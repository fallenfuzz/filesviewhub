//! Thumbnails (`decisions/2026/10/05/filesviewhub-is-a-tauri-app-shipped-portable.md`, `decisions/2026/10/07/app-data-in-storeddata-next-to-the-app.md`): one picture per Work, made in the background from its files and
//! kept in the cache as `thumbs/<2>/<key>.jpg`, with `thumbs.json` saying what each file gave.
//! A file is read only when it is on this PC: a cloud-only file is never touched (`decisions/2026/10/05/checked-folders-on-any-drive.md`). A
//! file that holds no picture is remembered until it changes; one that could not be read now is
//! tried again on the next start or scan. A video gives a strip of frames side by side, which the
//! card steps through when pointed at; a clip among Photos gives one frame. The titles of a collection get thumbnails of their own,
//! made when its dialog opens. A cover the person chose for a Work or a folder (`chosen.rs`,
//! `decisions/2026/10/22/covers-are-chosen-never-fetched-by-themselves.md`) comes before any its files give.

use crate::scan::{class_of, Catalog, Class, Item, Role, Work};
use crate::chosen::{Chosen, Covers};
use crate::tools::{self, Fail, Tools};
use image::{DynamicImage, GenericImageView};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

/// Thumbnails fit in a square this many pixels wide.
pub const BOX: u32 = 512;
/// Part of every key: raising it makes every thumbnail again. Videos have their own, for strips.
const RECIPE: &str = "t1";
const RECIPE_VIDEO: &str = "t2";
const RECIPE_COURSE: &str = "t3";
const RECIPE_CLIP: &str = "c1";
/// Frames in a video's strip.
const FRAMES: usize = 5;
/// A collection with more titles than this shows them as a list, without thumbnails.
pub const GRID_MAX: usize = 1000;
const TOOL_TIME: Duration = Duration::from_secs(120);

/// Pictures the app decodes itself; HEIC goes through ffmpeg.
const PICTURES: &[&str] = &["jpg", "jpeg", "png", "gif", "webp", "bmp", "tif", "tiff", "insp"];
/// Files a cover comes from, best first: a book's formats in the order their covers are best.
const DOCS: &[&str] = &["pdf", "epub", "azw3", "mobi", "fb2", "djvu", "djv", "cbz", "pptx", "docx", "odp", "odt"];
/// The most files tried for one Work's picture.
const TRIES: usize = 5;

/// What one source file gave: a thumbnail, or why it has none.
#[derive(Serialize, Deserialize, Clone, Debug)]
struct Entry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    ok: Option<(u32, u32)>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    no: Option<String>,
    /// Frames side by side; `ok` is the size of one.
    #[serde(default = "one", skip_serializing_if = "is_one")]
    n: u32,
}

fn one() -> u32 {
    1
}
fn is_one(n: &u32) -> bool {
    *n == 1
}

#[derive(Serialize, Deserialize, Default)]
struct Index {
    v: u32,
    files: HashMap<String, Entry>,
}

/// A Work's thumbnail, as the UI gets it.
#[derive(Serialize, Clone, Debug)]
pub struct Thumb {
    pub id: String,
    pub key: String,
    /// The size of one frame.
    pub w: u32,
    pub h: u32,
    /// Frames side by side in the picture: a video's strip.
    pub n: u32,
    /// Where a chosen cover came from: `own`, or the site it was picked from
    /// (`web::OPEN_LIBRARY`, `web::FANTLAB`); none for a picture from the Work's files.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
}

#[derive(Clone)]
struct Source {
    path: String,
    ext: String,
    key: String,
    /// A course's strip: a frame from each of these lessons, at this fraction of its length.
    shots: Vec<(String, f64)>,
    /// One frame of a video, not a strip: a clip among Photos.
    frame: bool,
    /// Where a chosen cover came from.
    from: Option<String>,
}

struct Inner {
    dir: PathBuf,
    index: Index,
    dirty: bool,
    saved: Instant,
    todo: HashMap<String, Vec<Source>>,
    queue: VecDeque<String>,
    urgent: VecDeque<String>,
    /// Thumbnails made only when asked for: the titles of a collection.
    lazy: HashMap<String, Vec<Source>>,
    /// Ids a worker is making now. An id on screen waits in both queues, and a second worker
    /// would make it again.
    busy: HashSet<String>,
    done: HashMap<String, Thumb>,
    /// The ids the UI asked for last, asked for again after each plan: the tiles on screen.
    wanted: Vec<String>,
    outbox: Vec<Thumb>,
    left_sent: usize,
}

pub struct Thumbs {
    pub tools: Tools,
    inner: Mutex<Inner>,
    /// The covers the person chose. Locked after `inner` when both are, never before.
    pub chosen: Mutex<Covers>,
    wake: Condvar,
    /// Held while a thumbnail is written and while the cache moves, so none lands in the old place.
    pub io: Mutex<()>,
}

#[derive(Serialize, Clone)]
pub struct Batch {
    pub done: Vec<Thumb>,
    pub left: usize,
}

/// A stable 64-bit hash (FNV-1a), so keys stay the same across builds of the app.
fn fnv(s: &str) -> String {
    fnv_bytes(s.as_bytes())
}

pub fn fnv_bytes(b: &[u8]) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in b {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{h:016x}")
}

fn key_of(i: &Item) -> String {
    let recipe = if class_of(&i.ext) == Some(Class::Video) { RECIPE_VIDEO } else { RECIPE };
    fnv(&format!("{recipe}|{}|{}|{}", i.path, i.size, i.mtime))
}

fn source(i: &Item) -> Source {
    Source { path: i.path.clone(), ext: i.ext.clone(), key: key_of(i), shots: vec![], frame: false, from: None }
}

/// A clip in an event: one frame, keyed apart from a video's strip.
fn clip(i: &Item) -> Source {
    Source { key: fnv(&format!("{RECIPE_CLIP}|{}|{}|{}", i.path, i.size, i.mtime)), frame: true, ..source(i) }
}

/// A file of a Work as its own thumbnail: a clip of an event gives one frame.
fn source_in(w: &Work, i: &Item) -> Source {
    if w.event && class_of(&i.ext) == Some(Class::Video) { clip(i) } else { source(i) }
}

/// Where a course's frames come from: `FRAMES` points spread evenly over its lessons, each taken
/// as the same length, so five or more lessons give one frame each and fewer give several.
fn course_shots(lessons: usize) -> Vec<(usize, f64)> {
    let mut out: Vec<(usize, f64)> = vec![];
    for k in 0..FRAMES {
        let x = (0.1 + 0.8 * k as f64 / (FRAMES - 1) as f64) * lessons as f64;
        let (i, f) = if lessons >= FRAMES {
            ((k * (lessons - 1) + (FRAMES - 1) / 2) / (FRAMES - 1), 0.4)
        } else {
            let i = (x as usize).min(lessons - 1);
            (i, (x - i as f64).clamp(0.1, 0.9))
        };
        if !out.contains(&(i, f)) {
            out.push((i, f));
        }
    }
    out
}

/// A course's strip from frames across all its lessons, keyed by the lessons it uses.
fn course(lessons: &[&Item]) -> Source {
    let shots: Vec<(String, f64)> = course_shots(lessons.len()).into_iter().map(|(i, f)| (lessons[i].path.clone(), f)).collect();
    let used: Vec<String> = course_shots(lessons.len()).into_iter().map(|(i, f)| format!("{}|{}|{}|{f}", lessons[i].path, lessons[i].size, lessons[i].mtime)).collect();
    let first = lessons[0];
    Source { path: first.path.clone(), ext: first.ext.clone(), key: fnv(&format!("{RECIPE_COURSE}|{}", used.join("|"))), shots, frame: false, from: None }
}

/// A chosen cover, made into a thumbnail like any picture.
fn chosen_source(covers: &Covers, c: &Chosen) -> Source {
    Source { path: covers.path(c).to_string_lossy().into(), ext: "jpg".into(), key: fnv(&format!("chosen|{}", c.file)), shots: vec![], frame: false, from: Some(c.from.clone()) }
}

/// The sources of a Work's picture: its chosen cover first, then its files.
fn sources_with(covers: &Covers, w: &Work) -> Vec<Source> {
    let mut srcs = sources(w);
    if let Some(c) = covers.get(&w.id) {
        srcs.insert(0, chosen_source(covers, c));
    }
    srcs
}

/// A folder Work whose titles are pictures or books shows them as a grid with thumbnails.
pub fn grid(w: &Work) -> bool {
    let main: Vec<&Item> = w.items.iter().filter(|i| i.role == Role::Main).collect();
    w.is_folder && main.len() > 1 && main.len() <= GRID_MAX && main.iter().all(|i| matches!(class_of(&i.ext), Some(Class::Image | Class::Book)) || (w.event && class_of(&i.ext) == Some(Class::Video)))
}

fn file_of(dir: &Path, key: &str) -> PathBuf {
    dir.join("thumbs").join(&key[..2]).join(format!("{key}.jpg"))
}

fn index_file(dir: &Path) -> PathBuf {
    dir.join("thumbs.json")
}

/// The files a Work's picture may come from, best first. A folder shows its own picture first
/// (a cover.jpg beats any other); a course shows a lesson's frame before a handout's page; a book
/// shows its cover.
fn sources(w: &Work) -> Vec<Source> {
    let local: Vec<&Item> = w.items.iter().filter(|i| !i.cloud && matches!(i.role, Role::Main | Role::Doc)).collect();
    let of = |c: Class| local.iter().filter(move |i| class_of(&i.ext) == Some(c)).copied();
    let mut pictures: Vec<&Item> = of(Class::Image).collect();
    let named = |i: &&Item| {
        let n = i.name.to_lowercase();
        !(n.contains("cover") || n.contains("folder") || n.contains("front"))
    };
    pictures.sort_by_key(named);
    let mut docs: Vec<&Item> = local.iter().filter(|i| DOCS.contains(&i.ext.as_str())).copied().collect();
    docs.sort_by_key(|i| DOCS.iter().position(|d| *d == i.ext).unwrap_or(99));
    let lessons: Vec<&Item> = of(Class::Video).collect();
    // A folder of several videos shows frames from across them; one that fails falls back to its
    // first video. An event's clips give one frame each.
    let videos: Vec<Source> = if w.event {
        lessons.iter().take(2).map(|i| clip(i)).collect()
    } else if w.is_folder && lessons.len() > 1 {
        vec![course(&lessons), source(lessons[0])]
    } else {
        lessons.iter().take(2).map(|i| source(i)).collect()
    };
    let list = |v: Vec<&Item>| v.into_iter().map(source).collect::<Vec<_>>();
    let audio = list(of(Class::Audio).take(1).collect());
    let pics = list(pictures.into_iter().take(1).collect());
    let docs = list(docs.into_iter().take(3).collect());
    let order = if w.is_folder && w.media.iter().any(|m| m == "Watching") {
        [pics, videos, docs, audio]
    } else if w.is_folder {
        [pics, docs, videos, audio]
    } else {
        [docs, pics, videos, audio]
    };
    order.into_iter().flatten().take(TRIES).collect()
}

enum State {
    Ready(Thumb),
    Todo,
    None,
}

fn state(index: &Index, id: &str, srcs: &[Source]) -> State {
    for s in srcs {
        match index.files.get(&s.key) {
            Some(Entry { ok: Some((w, h)), n, .. }) => return State::Ready(Thumb { id: id.into(), key: s.key.clone(), w: *w, h: *h, n: *n, from: s.from.clone() }),
            Some(_) => continue,
            None => return State::Todo,
        }
    }
    State::None
}

impl Inner {
    /// Makes these ids next, the titles of a collection included.
    fn ask(&mut self, ids: Vec<String>) {
        self.urgent.clear();
        for id in ids {
            if let Some(srcs) = self.lazy.remove(&id) {
                self.todo.insert(id.clone(), srcs);
            }
            if self.todo.contains_key(&id) {
                self.urgent.push_back(id);
            }
        }
    }
}

impl Thumbs {
    pub fn new(dir: PathBuf) -> Arc<Thumbs> {
        // Pages a DjVu tool left behind when it was stopped mid-way: only the app's own
        // `<key>-<page>.pnm` files, never anything else.
        for e in std::fs::read_dir(dir.join("tmp")).into_iter().flatten().filter_map(Result::ok) {
            let n = e.file_name().to_string_lossy().to_string();
            if n.ends_with(".pnm") && n.get(..16).is_some_and(|k| k.bytes().all(|b| b.is_ascii_hexdigit())) && n.get(16..17) == Some("-") {
                let _ = std::fs::remove_file(e.path());
            }
        }
        let index = Self::load(&dir);
        Arc::new(Thumbs {
            tools: Tools::find(),
            inner: Mutex::new(Inner {
                dir,
                index,
                dirty: false,
                saved: Instant::now(),
                todo: HashMap::new(),
                queue: VecDeque::new(),
                urgent: VecDeque::new(),
                lazy: HashMap::new(),
                busy: HashSet::new(),
                done: HashMap::new(),
                wanted: vec![],
                outbox: vec![],
                left_sent: usize::MAX,
            }),
            chosen: Mutex::new(Covers::default()),
            wake: Condvar::new(),
            io: Mutex::new(()),
        })
    }

    fn load(dir: &Path) -> Index {
        let i: Index = crate::store::load(&index_file(dir));
        if i.v == 1 { i } else { Index { v: 1, files: HashMap::new() } }
    }

    /// Starts the workers and the thread that tells the UI what is new.
    pub fn start(self: &Arc<Self>, app: AppHandle) {
        let n = std::thread::available_parallelism().map(|n| n.get() / 2).unwrap_or(2).clamp(2, 4);
        for _ in 0..n {
            let t = self.clone();
            std::thread::spawn(move || t.work());
        }
        let t = self.clone();
        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_millis(300));
            let (batch, save) = {
                let mut g = t.inner.lock().unwrap();
                let left = g.todo.len();
                let batch = if !g.outbox.is_empty() || left != g.left_sent {
                    g.left_sent = left;
                    Some(Batch { done: std::mem::take(&mut g.outbox), left })
                } else {
                    None
                };
                let save = g.dirty && (left == 0 || g.saved.elapsed() > Duration::from_secs(10));
                (batch, save)
            };
            if let Some(b) = batch {
                let _ = app.emit("thumbs", b);
            }
            if save {
                t.save();
            }
        });
    }

    pub fn save(&self) {
        let _io = self.io.lock().unwrap();
        self.write_index();
    }

    /// Writes the index; the caller holds `io`.
    pub fn write_index(&self) {
        let (dir, text) = {
            let mut g = self.inner.lock().unwrap();
            g.dirty = false;
            g.saved = Instant::now();
            (g.dir.clone(), serde_json::to_string(&g.index))
        };
        if let Ok(text) = text {
            let f = index_file(&dir);
            let tmp = f.with_extension("json.tmp");
            if std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(&tmp, text)).and_then(|_| std::fs::rename(&tmp, &f)).is_err() {
                log::warn!("could not save {}", f.display());
            }
        }
    }

    /// Plans the thumbnails for the catalog: what is made already, what to make. Files no Work
    /// uses any more are deleted from the cache.
    pub fn plan(&self, cat: &Catalog) {
        let mut gone = vec![];
        {
            let mut g = self.inner.lock().unwrap();
            let g = &mut *g;
            g.todo.clear();
            g.queue.clear();
            g.urgent.clear();
            g.lazy.clear();
            g.done.clear();
            let covers = self.chosen.lock().unwrap();
            let mut used = HashSet::new();
            // A Section that is not connected keeps its thumbnails; nothing is made for it, but a
            // cover chosen for one of its Works is kept in StoredData and made all the same.
            let works = cat.sections.iter().flat_map(|s| s.works.iter().map(move |w| (s.online, w)));
            // A folder tile shows only a cover chosen for it; any other comes from its Works.
            let folders = covers.iter().filter(|(id, _)| id.starts_with("dir:"));
            let plans = works.map(|(online, w)| (online, online || covers.get(&w.id).is_some(), w.id.clone(), sources_with(&covers, w), Some(w)))
                .chain(folders.map(|(id, c)| (true, true, id.clone(), vec![chosen_source(&covers, c)], None)));
            for (online, makes, id, srcs, w) in plans {
                used.extend(srcs.iter().map(|s| s.key.clone()));
                match state(&g.index, &id, &srcs) {
                    State::Ready(t) => {
                        g.done.insert(id, t);
                    }
                    State::Todo if makes => {
                        g.queue.push_back(id.clone());
                        g.todo.insert(id, srcs);
                    }
                    State::Todo | State::None => {}
                }
                let Some(w) = w else { continue };
                if grid(w) {
                    for i in w.items.iter().filter(|i| i.role == Role::Main && !i.cloud) {
                        let id = format!("{}#{}", w.id, i.key);
                        let srcs = vec![source_in(w, i)];
                        used.insert(srcs[0].key.clone());
                        match state(&g.index, &id, &srcs) {
                            State::Ready(t) => {
                                g.done.insert(id, t);
                            }
                            State::Todo if online => {
                                g.lazy.insert(id, srcs);
                            }
                            State::Todo => {}
                            State::None => {}
                        }
                    }
                }
            }
            let before = g.index.files.len();
            g.index.files.retain(|k, e| {
                let keep = used.contains(k);
                if !keep && e.ok.is_some() {
                    gone.push(k.clone());
                }
                keep
            });
            g.dirty |= g.index.files.len() != before;
            // A thumbnail made under the last plan and not sent yet goes only if this plan shows
            // it too: a removed cover does not come back.
            let done = &g.done;
            g.outbox.retain(|t| done.get(&t.id).is_some_and(|d| d.key == t.key));
            for k in &gone {
                let _ = std::fs::remove_file(file_of(&g.dir, k));
            }
            // What is on screen now comes first, without the UI asking again.
            let wanted = g.wanted.clone();
            g.ask(wanted);
            log::info!("thumbnails: {} ready, {} to make, {} removed", g.done.len(), g.todo.len(), gone.len());
        }
        self.wake.notify_all();
    }

    /// Forgets what these Works' files and grid titles gave, pictures and failures alike, so the
    /// next plan makes them again.
    pub fn forget(&self, cat: &Catalog, ids: &HashSet<String>) {
        let _io = self.io.lock().unwrap();
        let mut g = self.inner.lock().unwrap();
        let mut n = 0;
        for w in cat.sections.iter().flat_map(|s| &s.works).filter(|w| ids.contains(&w.id)) {
            // A grid's titles have thumbnails of their own, made again with the Work's.
            let titles = w.items.iter().filter(|i| grid(w) && i.role == Role::Main).map(|i| source_in(w, i));
            for s in sources(w).into_iter().chain(titles) {
                if g.index.files.remove(&s.key).is_some() {
                    let _ = std::fs::remove_file(file_of(&g.dir, &s.key));
                    n += 1;
                }
            }
        }
        g.dirty |= n > 0;
        log::info!("thumbnails of {} Works to make again ({n} files forgotten)", ids.len());
    }

    /// Takes the covers the person chose, kept in StoredData; the next plan shows them.
    pub fn set_chosen(&self, covers: Covers) {
        *self.chosen.lock().unwrap() = covers;
    }

    /// Takes over the covers the old lookups found on the web (`decisions/2026/10/22/covers-are-chosen-never-fetched-by-themselves.md`) as chosen covers from
    /// their site, from the thumbnails already made of them, then deletes the cache's `web.json`.
    /// A cover the person had refused is dropped.
    pub fn adopt_web_covers(&self) {
        #[derive(Deserialize, Default)]
        struct Old {
            #[serde(default)]
            by: HashMap<String, Lookup>,
        }
        #[derive(Deserialize)]
        struct Lookup {
            found: Option<Found>,
        }
        #[derive(Deserialize)]
        struct Found {
            url: String,
            from: String,
        }
        let dir = self.dir();
        let file = dir.join("web.json");
        if !file.exists() {
            return;
        }
        let old: Old = crate::store::load(&file);
        let mut covers = self.chosen.lock().unwrap();
        let (mut n, mut failed) = (0, 0);
        for (id, f) in old.by.iter().filter_map(|(id, l)| Some((id, l.found.as_ref()?))) {
            // A cover whose thumbnail was never made has no picture to keep, and is dropped.
            let Ok(b) = std::fs::read(file_of(&dir, &fnv(&format!("web|{}", f.url)))) else { continue };
            if covers.get(id).is_some() {
                continue;
            }
            match covers.keep(std::slice::from_ref(id), &b, &f.from) {
                Ok(_) => n += 1,
                Err(e) => {
                    failed += 1;
                    log::warn!("cover found on the web for {id} not kept: {e}");
                }
            }
        }
        // Kept for the next start when a cover could not be kept now.
        if failed == 0 {
            let _ = std::fs::remove_file(&file);
        }
        log::info!("{n} covers found on the web before kept as chosen covers; {failed} not kept{}", if failed == 0 { "; web.json deleted" } else { "" });
    }

    /// The thumbnails already in the cache for Works not in the catalog (a Section previewed as
    /// another kind): each one's first source that has a picture made. Makes none.
    pub fn made(&self, works: &[Work]) -> Vec<Thumb> {
        let g = self.inner.lock().unwrap();
        let covers = self.chosen.lock().unwrap();
        works
            .iter()
            .filter_map(|w| {
                let srcs = sources_with(&covers, w);
                srcs.iter().find_map(|s| match g.index.files.get(&s.key) {
                    Some(Entry { ok: Some((wd, h)), n, .. }) => Some(Thumb { id: w.id.clone(), key: s.key.clone(), w: *wd, h: *h, n: *n, from: s.from.clone() }),
                    _ => None,
                })
            })
            .collect()
    }

    /// The thumbnails made so far, by Work id, and how many are left to make.
    pub fn ready(&self) -> Batch {
        let g = self.inner.lock().unwrap();
        Batch { done: g.done.values().cloned().collect(), left: g.todo.len() }
    }

    /// Makes these Works' thumbnails next: the cards on screen now, or the titles in an open
    /// dialog (`<Work id>#<item key>`).
    pub fn want(&self, ids: Vec<String>) {
        let mut g = self.inner.lock().unwrap();
        g.wanted = ids.clone();
        g.ask(ids);
        drop(g);
        self.wake.notify_all();
    }

    pub fn dir(&self) -> PathBuf {
        self.inner.lock().unwrap().dir.clone()
    }

    /// Points at a moved cache. Call with `io` held, after `save`. The index is read from the new
    /// place: carried along, or empty when the new place starts empty.
    pub fn set_dir(&self, dir: PathBuf) {
        let mut g = self.inner.lock().unwrap();
        g.index = Self::load(&dir);
        g.dir = dir;
    }

    /// The thumbnail file for a key from the UI, or None for anything that is not a key.
    pub fn file(&self, key: &str) -> Option<PathBuf> {
        (key.len() == 16 && key.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())).then(|| file_of(&self.dir(), key))
    }

    fn next(&self) -> (String, Vec<Source>) {
        let mut g = self.inner.lock().unwrap();
        loop {
            while let Some(id) = g.urgent.pop_front().or_else(|| g.queue.pop_front()) {
                if let Some(srcs) = g.todo.get(&id).cloned().filter(|_| !g.busy.contains(&id)) {
                    g.busy.insert(id.clone());
                    return (id, srcs);
                }
            }
            g = self.wake.wait(g).unwrap();
        }
    }

    fn work(&self) {
        loop {
            let (id, srcs) = self.next();
            // A panic in a decoder must neither end the worker nor leave the id busy for good.
            let thumb = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.make_for(&id, &srcs))).unwrap_or(None);
            self.finish(id, &srcs, thumb);
        }
    }

    /// Takes a made thumbnail, when the id still waits for the same files. A new plan with the
    /// same files keeps the result; a newer job for the id (other files, a cover just chosen) is
    /// made next instead.
    fn finish(&self, id: String, srcs: &[Source], thumb: Option<Thumb>) {
        let mut g = self.inner.lock().unwrap();
        g.busy.remove(&id);
        let same = g.todo.get(&id).is_some_and(|now| now.iter().map(|s| &s.key).eq(srcs.iter().map(|s| &s.key)));
        if same {
            g.todo.remove(&id);
            if let Some(t) = thumb {
                g.done.insert(id, t.clone());
                g.outbox.push(t);
            }
        } else if g.todo.contains_key(&id) {
            g.urgent.push_front(id);
            drop(g);
            self.wake.notify_all();
        }
    }

    fn entry(&self, key: &str) -> Option<Entry> {
        self.inner.lock().unwrap().index.files.get(key).cloned()
    }

    fn make_for(&self, id: &str, srcs: &[Source]) -> Option<Thumb> {
        for s in srcs {
            match self.entry(&s.key) {
                Some(Entry { ok: Some((w, h)), n, .. }) => return Some(Thumb { id: id.into(), key: s.key.clone(), w, h, n, from: s.from.clone() }),
                Some(_) => continue,
                None => {}
            }
            // The catalog may be older than the file: one made cloud-only since is not opened. A
            // chosen cover is the app's own file.
            if s.from.is_none() && crate::scan::now_cloud(&s.path) {
                continue;
            }
            let made = self.make(s).and_then(|(img, n)| self.write(&s.key, &img, n));
            match made {
                Ok((w, h, n)) => return Some(Thumb { id: id.into(), key: s.key.clone(), w, h, n, from: s.from.clone() }),
                Err(Fail::Bad(why)) => {
                    log::info!("no thumbnail from {}: {why}", s.path);
                    let mut g = self.inner.lock().unwrap();
                    g.index.files.insert(s.key.clone(), Entry { ok: None, no: Some(why), n: 1 });
                    g.dirty = true;
                }
                Err(Fail::Retry(why)) => log::info!("no thumbnail yet from {} (tried again later): {why}", s.path),
            }
        }
        None
    }

    /// Writes a thumbnail; a strip of `n` frames is already made of frames that fit the box.
    fn write(&self, key: &str, img: &DynamicImage, n: u32) -> Result<(u32, u32, u32), Fail> {
        let fits = n > 1 || (img.width() <= BOX && img.height() <= BOX);
        let img = if fits { img.clone() } else { img.resize(BOX, BOX, image::imageops::FilterType::CatmullRom) };
        let rgb = flatten(&img);
        let mut buf = vec![];
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 85).encode_image(&rgb).map_err(|e| Fail::Bad(e.to_string()))?;
        let _io = self.io.lock().unwrap();
        let dir = self.dir();
        let f = file_of(&dir, key);
        std::fs::create_dir_all(f.parent().unwrap())?;
        let tmp = f.with_extension("tmp");
        std::fs::write(&tmp, &buf)?;
        std::fs::rename(&tmp, &f)?;
        let (w, h) = rgb.dimensions();
        let w = w / n.max(1);
        let mut g = self.inner.lock().unwrap();
        g.index.files.insert(key.into(), Entry { ok: Some((w, h)), no: None, n });
        g.dirty = true;
        Ok((w, h, n))
    }

    fn tool(&self, id: &str, n: usize) -> Result<PathBuf, Fail> {
        self.tools.path(id, n).ok_or_else(|| Fail::Retry(format!("{id} is not in the app's tools folder")))
    }

    /// A file's picture and how many frames side by side it holds.
    fn make(&self, s: &Source) -> Result<(DynamicImage, u32), Fail> {
        if !s.shots.is_empty() {
            return self.course_strip(&s.shots);
        }
        if class_of(&s.ext) == Some(Class::Video) && !s.frame {
            return self.strip(Path::new(&s.path));
        }
        self.still(s).map(|i| (i, 1))
    }

    fn still(&self, s: &Source) -> Result<DynamicImage, Fail> {
        let p = Path::new(&s.path);
        let e = s.ext.as_str();
        if PICTURES.contains(&e) {
            return picture(p);
        }
        let cover = |bytes: Vec<u8>| decode(&bytes).and_then(not_blank);
        match e {
            "heic" => self.ffmpeg_still(p, None),
            "pdf" => crate::pdf::cover(&self.tool("pdfium", 0)?, p),
            "djvu" | "djv" => self.djvu(s),
            "epub" => cover(crate::covers::epub(p)?),
            "fb2" => cover(crate::covers::fb2(p)?),
            "mobi" | "azw3" => cover(crate::covers::mobi(p)?),
            "cbz" => cover(crate::covers::cbz(p)?),
            "pptx" | "docx" | "odp" | "odt" => cover(crate::covers::office(p)?),
            _ => match class_of(e) {
                Some(Class::Video) => self.video(p),
                Some(Class::Audio) => self.ffmpeg_still(p, None).map_err(|f| match f {
                    Fail::Bad(_) => Fail::Bad("no cover picture in the file".into()),
                    f => f,
                }),
                _ => Err(Fail::Bad("no picture in this type of file".into())),
            },
        }
    }

    /// One picture from ffmpeg: a video frame at `at` seconds (skipping cover pictures stored in
    /// the file), or the file's own picture (a photo, an audio file's cover).
    fn ffmpeg_still(&self, p: &Path, at: Option<f64>) -> Result<DynamicImage, Fail> {
        let ff = self.tool("ffmpeg", 0)?;
        let mut a: Vec<std::ffi::OsString> = vec!["-v".into(), "error".into(), "-protocol_whitelist".into(), "file".into()];
        if let Some(t) = at {
            a.extend(["-ss".into(), format!("{t:.2}").into()]);
        }
        let map = if at.is_some() { "0:V:0" } else { "0:v:0" };
        let filt = format!("scale=iw*sar:ih,scale={BOX}:{BOX}:force_original_aspect_ratio=decrease");
        a.extend(["-i".into(), crate::store::named(p).into_owned().into(), "-map".into(), map.into(), "-frames:v".into(), "1".into(), "-vf".into(), filt.into()]);
        a.extend(["-f", "image2pipe", "-c:v", "png", "-an", "-"].map(Into::into));
        let a: Vec<&OsStr> = a.iter().map(|x| x.as_os_str()).collect();
        let out = tools::run(&ff, &a, TOOL_TIME)?;
        if out.is_empty() {
            return Err(Fail::Bad("no picture at that point".into()));
        }
        decode(&out)
    }

    /// A video's or audio file's length, when ffprobe can tell.
    pub fn seconds(&self, p: &Path) -> Option<f64> {
        let d = self.duration(p);
        (d > 0.0).then_some(d)
    }

    fn duration(&self, p: &Path) -> f64 {
        let Ok(probe) = self.tool("ffmpeg", 1) else { return 0.0 };
        let p = crate::store::named(p);
        let a: [&OsStr; 9] = ["-v".as_ref(), "error".as_ref(), "-protocol_whitelist".as_ref(), "file".as_ref(), "-show_entries".as_ref(), "format=duration".as_ref(), "-of".as_ref(), "csv=p=0".as_ref(), p.as_os_str()];
        tools::run(&probe, &a, Duration::from_secs(60)).ok().and_then(|o| String::from_utf8_lossy(&o).trim().parse().ok()).unwrap_or(0.0)
    }

    /// Frames from across a video that are not black or blank, side by side, all one height.
    fn strip(&self, p: &Path) -> Result<(DynamicImage, u32), Fail> {
        let d = self.duration(p);
        if d <= 0.0 {
            return self.video(p).map(|i| (i, 1));
        }
        let mut frames = vec![];
        let mut last = Fail::Bad("no frame".into());
        for k in 0..FRAMES {
            let t = d * (0.1 + 0.8 * k as f64 / (FRAMES - 1) as f64);
            match self.ffmpeg_still(p, Some(t)).and_then(not_blank) {
                Ok(img) => frames.push(img),
                Err(f @ Fail::Retry(_)) => return Err(f),
                Err(f) => last = f,
            }
        }
        side_by_side(frames, last)
    }

    /// A frame from each lesson at its point, or a little later when that one is black or blank;
    /// lessons only in the cloud are skipped.
    fn course_strip(&self, shots: &[(String, f64)]) -> Result<(DynamicImage, u32), Fail> {
        let mut frames = vec![];
        let mut last = Fail::Bad("no frame".into());
        for (path, f) in shots {
            let p = Path::new(path);
            if crate::scan::now_cloud(path) {
                continue;
            }
            let d = self.duration(p);
            let got = if d <= 0.0 {
                self.video(p)
            } else {
                self.ffmpeg_still(p, Some(d * f)).and_then(not_blank).or_else(|e| match e {
                    Fail::Bad(_) => self.ffmpeg_still(p, Some(d * (f + 0.3).min(0.95))).and_then(not_blank),
                    e => Err(e),
                })
            };
            match got {
                Ok(img) => frames.push(img),
                Err(f @ Fail::Retry(_)) => return Err(f),
                Err(f) => last = f,
            }
        }
        side_by_side(frames, last)
    }

    /// A frame that is not black or blank, at a tenth, three tenths and half the length.
    fn video(&self, p: &Path) -> Result<DynamicImage, Fail> {
        let d = self.duration(p);
        let times: Vec<f64> = if d > 0.0 { [0.1, 0.3, 0.5].iter().map(|f| d * f).collect() } else { vec![0.0] };
        let mut last = Fail::Bad("no frame".into());
        for t in times {
            match self.ffmpeg_still(p, Some(t)).and_then(not_blank) {
                Ok(img) => return Ok(img),
                Err(f @ Fail::Retry(_)) => return Err(f),
                Err(f) => last = f,
            }
        }
        Err(last)
    }

    /// The first of pages 1 to 3 that is not blank.
    fn djvu(&self, s: &Source) -> Result<DynamicImage, Fail> {
        let exe = self.tool("ddjvu", 0)?;
        let tmp = self.dir().join("tmp");
        std::fs::create_dir_all(&tmp)?;
        let size = format!("-size={BOX}x{BOX}");
        let file = crate::store::named(Path::new(&s.path));
        let mut last = Fail::Bad("no pages".into());
        for page in 1..=3 {
            let out = tmp.join(format!("{}-{page}.pnm", s.key));
            let pg = format!("-page={page}");
            let a: [&OsStr; 5] = ["-format=pnm".as_ref(), pg.as_ref(), size.as_ref(), file.as_os_str(), out.as_os_str()];
            let r = tools::run(&exe, &a, TOOL_TIME).and_then(|_| std::fs::read(&out).map_err(Fail::from));
            let _ = std::fs::remove_file(&out);
            match r.and_then(|b| decode(&b)).and_then(not_blank) {
                Ok(img) => return Ok(img),
                Err(f @ Fail::Retry(_)) => return Err(f),
                Err(f) => last = f,
            }
        }
        Err(last)
    }
}

/// Frames side by side, all the size of the first, and how many there are.
fn side_by_side(frames: Vec<DynamicImage>, last: Fail) -> Result<(DynamicImage, u32), Fail> {
    if frames.is_empty() {
        return Err(last);
    }
    let (w, h) = (frames[0].width(), frames[0].height());
    let mut out = image::RgbImage::new(w * frames.len() as u32, h);
    for (k, f) in frames.iter().enumerate() {
        let f = if f.dimensions() == (w, h) { f.to_rgb8() } else { f.resize_exact(w, h, image::imageops::FilterType::CatmullRom).to_rgb8() };
        image::imageops::replace(&mut out, &f, (k as u32 * w) as i64, 0);
    }
    Ok((DynamicImage::ImageRgb8(out), frames.len() as u32))
}

fn image_fail(e: image::ImageError) -> Fail {
    match e {
        image::ImageError::IoError(e) => Fail::Retry(e.to_string()),
        e => Fail::Bad(e.to_string()),
    }
}

fn decode(bytes: &[u8]) -> Result<DynamicImage, Fail> {
    image::load_from_memory(bytes).map_err(image_fail)
}

/// A photo or picture file, turned the way the camera says it was held.
fn picture(p: &Path) -> Result<DynamicImage, Fail> {
    use image::ImageDecoder;
    let r = image::ImageReader::open(crate::store::named(p))?.with_guessed_format()?;
    let mut dec = r.into_decoder().map_err(image_fail)?;
    let o = dec.orientation().unwrap_or(image::metadata::Orientation::NoTransforms);
    let mut img = DynamicImage::from_decoder(dec).map_err(image_fail)?;
    img.apply_orientation(o);
    Ok(img)
}

/// A picture held in memory, turned the way the camera says it was held.
pub fn picture_bytes(bytes: &[u8]) -> Result<DynamicImage, Fail> {
    use image::ImageDecoder;
    let r = image::ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format()?;
    let mut dec = r.into_decoder().map_err(image_fail)?;
    let o = dec.orientation().unwrap_or(image::metadata::Orientation::NoTransforms);
    let mut img = DynamicImage::from_decoder(dec).map_err(image_fail)?;
    img.apply_orientation(o);
    Ok(img)
}

/// A picture of one flat colour (a blank page, a black frame) is no cover. A page with only a line
/// of text is low in contrast too, so it also needs almost no ink: under 0.1 % of its pixels far
/// from the page's own colour.
pub fn blank(img: &DynamicImage) -> bool {
    let small = img.thumbnail(256, 256).to_luma8();
    let n = small.len();
    if n == 0 {
        return true;
    }
    let mean = small.iter().map(|&v| v as f64).sum::<f64>() / n as f64;
    let var = small.iter().map(|&v| (v as f64 - mean).powi(2)).sum::<f64>() / n as f64;
    let mut sorted = small.as_raw().clone();
    sorted.sort_unstable();
    let page = sorted[n / 2] as i32;
    let ink = small.iter().filter(|&&v| (v as i32 - page).abs() > 40).count();
    var.sqrt() < 10.0 && ink * 1000 < n
}

fn not_blank(img: DynamicImage) -> Result<DynamicImage, Fail> {
    if blank(&img) { Err(Fail::Bad("blank".into())) } else { Ok(img) }
}

/// RGB on white: a transparent part of a picture shows white, not black.
pub fn flatten(img: &DynamicImage) -> image::RgbImage {
    if !img.color().has_alpha() {
        return img.to_rgb8();
    }
    let rgba = img.to_rgba8();
    image::RgbImage::from_fn(rgba.width(), rgba.height(), |x, y| {
        let p = rgba.get_pixel(x, y).0;
        let a = p[3] as u32;
        let mix = |c: u8| ((c as u32 * a + 255 * (255 - a)) / 255) as u8;
        image::Rgb([mix(p[0]), mix(p[1]), mix(p[2])])
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Kind;

    fn item(name: &str, cloud: bool) -> Item {
        let ext = name.rsplit_once('.').map(|(_, e)| e.to_string()).unwrap_or_default();
        Item { name: name.into(), rel: name.into(), key: name.into(), path: format!("D:\\x\\{name}"), ext, size: 1, mtime: 1, cloud, title: name.into(), group: String::new(), role: Default::default(), lang: String::new(), at: None }
    }

    fn work(is_folder: bool, media: &[&str], items: Vec<Item>) -> Work {
        Work { id: "w".into(), title: "w".into(), path: "D:\\x".into(), folder: vec![], is_folder, items, media: media.iter().map(|m| m.to_string()).collect(), size: 0, added: 0, cloud: 0, year: None, taken: None, event: false, day_only: false }
    }

    fn names(w: &Work) -> Vec<String> {
        sources(w).into_iter().map(|s| s.path.rsplit('\\').next().unwrap().to_string()).collect()
    }

    #[test]
    fn a_book_shows_its_best_format_first() {
        let w = work(false, &["Reading"], vec![item("b.djvu", false), item("b.fb2", false), item("b.pdf", false)]);
        assert_eq!(names(&w), ["b.pdf", "b.fb2", "b.djvu"]);
    }

    #[test]
    fn a_folder_shows_its_cover_picture_first() {
        let w = work(true, &["Listening"], vec![item("01.mp3", false), item("scan.jpg", false), item("Cover.jpg", false)]);
        assert_eq!(names(&w), ["Cover.jpg", "01.mp3"]);
    }

    #[test]
    fn a_course_shows_a_lesson_before_a_handout() {
        let w = work(true, &["Watching", "Reading"], vec![item("notes.pdf", false), item("01.mp4", false)]);
        assert_eq!(names(&w), ["01.mp4", "notes.pdf"]);
    }

    #[test]
    fn a_course_takes_frames_from_across_its_lessons() {
        assert_eq!(course_shots(10).iter().map(|s| s.0).collect::<Vec<_>>(), [0, 2, 5, 7, 9]);
        assert_eq!(course_shots(5).iter().map(|s| s.0).collect::<Vec<_>>(), [0, 1, 2, 3, 4]);
        assert_eq!(course_shots(2).iter().map(|s| s.0).collect::<Vec<_>>(), [0, 0, 1, 1, 1]);
        let w = work(true, &["Watching"], vec![item("01.mp4", false), item("02.mp4", false), item("03.mp4", true)]);
        let s = sources(&w);
        assert_eq!(s[0].shots.iter().map(|(p, _)| p.rsplit('\\').next().unwrap()).collect::<Vec<_>>(), ["01.mp4", "01.mp4", "02.mp4", "02.mp4", "02.mp4"]);
        assert!(s[1].shots.is_empty());
    }

    #[test]
    fn a_clip_among_photos_is_one_frame() {
        let mut w = work(true, &[], vec![item("a.mp4", false), item("b.mov", false)]);
        w.event = true;
        let s = sources(&w);
        assert!(s.iter().all(|s| s.frame && s.shots.is_empty()));
        assert_eq!(names(&w), ["a.mp4", "b.mov"]);
        let one = source_in(&w, &w.items[0]);
        assert!(one.frame);
        assert_ne!(one.key, source(&w.items[0]).key);
        w.event = false;
        assert!(!source_in(&w, &w.items[0]).frame);
        assert!(!sources(&w)[0].shots.is_empty());
    }

    #[test]
    fn making_again_forgets_every_title_of_a_grid() {
        let dir = std::env::temp_dir().join(format!("fvh-forget-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut w = work(true, &[], vec![item("a.jpg", false), item("b.jpg", false), item("c.jpg", false), item("d.mp4", false)]);
        w.event = true;
        let keys: Vec<String> = w.items.iter().map(|i| source_in(&w, i).key).collect();
        let t = Thumbs::new(dir.clone());
        for k in &keys {
            t.inner.lock().unwrap().index.files.insert(k.clone(), Entry { ok: Some((10, 10)), no: None, n: 1 });
        }
        let (w2, s2) = (w.clone(), crate::scan::Section { id: "s".into(), name: "s".into(), path: "D:\\x".into(), drive: "D".into(), kind: Kind::Photos, guessed: false, online: true, gone: false, works: vec![], files: 0, scanned_at: 0, unlisted: vec![], empty: vec![], whole_tree: false, split_from: None, links: vec![] });
        let s = crate::scan::Section { works: vec![w], ..s2.clone() };
        t.forget(&Catalog { v: 1, sections: vec![s] }, &HashSet::from(["w".to_string()]));
        let g = t.inner.lock().unwrap();
        assert!(keys.iter().all(|k| !g.index.files.contains_key(k)), "kept: {:?}", keys.iter().filter(|k| g.index.files.contains_key(*k)).collect::<Vec<_>>());
        drop(g);
        // The titles on screen are made again without the UI asking a second time.
        t.want(vec!["w#a.jpg".into()]);
        let cat = Catalog { v: 1, sections: vec![crate::scan::Section { works: vec![w2], ..s2 }] };
        t.plan(&cat);
        let g = t.inner.lock().unwrap();
        assert!(g.todo.contains_key("w#a.jpg") && g.urgent.contains(&"w#a.jpg".to_string()), "a title on screen is not made again");
        assert!(g.lazy.contains_key("w#b.jpg"), "a title off screen is made before it is asked for");
        drop(g);
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn books(works: Vec<Work>, online: bool) -> Catalog {
        let s = crate::scan::Section { id: "s".into(), name: "s".into(), path: "D:\\x".into(), drive: "D".into(), kind: Kind::Books, guessed: false, online, gone: false, files: works.len(), works, scanned_at: 0, unlisted: vec![], empty: vec![], whole_tree: false, split_from: None, links: vec![] };
        Catalog { v: 1, sections: vec![s] }
    }

    fn png() -> Vec<u8> {
        let img = image::RgbImage::from_fn(300, 450, |x, y| image::Rgb([(x % 255) as u8, (y % 255) as u8, 90]));
        let mut out = std::io::Cursor::new(vec![]);
        DynamicImage::ImageRgb8(img).write_to(&mut out, image::ImageFormat::Png).unwrap();
        out.into_inner()
    }

    #[test]
    fn a_chosen_cover_comes_first_and_goes_back() {
        let dir = std::env::temp_dir().join(format!("fvh-chosen-thumbs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let w = work(false, &["Reading"], vec![item("b.pdf", false)]);
        let file_key = sources(&w)[0].key.clone();
        let cat = books(vec![w], true);
        let t = Thumbs::new(dir.join("cache"));
        t.set_chosen(Covers::load(&dir));
        t.inner.lock().unwrap().index.files.insert(file_key.clone(), Entry { ok: Some((10, 15)), no: None, n: 1 });
        let shown = |t: &Thumbs| t.ready().done.iter().find(|x| x.id == "w").map(|x| (x.key.clone(), x.from.clone()));
        t.plan(&cat);
        assert_eq!(shown(&t), Some((file_key.clone(), None)));
        // Chosen for the Work and a folder: each is made from the picture kept in StoredData.
        let ids = ["w".to_string(), "dir:s/Shelf".to_string()];
        t.chosen.lock().unwrap().keep(&ids, &png(), crate::chosen::OWN).unwrap();
        t.plan(&cat);
        let (id, srcs) = t.next();
        assert_eq!(srcs[0].from.as_deref(), Some("own"));
        let made = t.make_for(&id, &srcs);
        t.finish(id, &srcs, made);
        let (id, srcs) = t.next();
        let made = t.make_for(&id, &srcs);
        t.finish(id, &srcs, made);
        let b = t.ready();
        assert!(b.done.iter().any(|x| x.id == "dir:s/Shelf" && x.from.as_deref() == Some("own")), "the folder tile's cover");
        assert_eq!(shown(&t).and_then(|x| x.1).as_deref(), Some("own"));
        // Kept after a restart, and after Make its thumbnails again.
        t.save();
        let t2 = Thumbs::new(dir.join("cache"));
        t2.set_chosen(Covers::load(&dir));
        t2.forget(&cat, &HashSet::from(["w".to_string()]));
        t2.inner.lock().unwrap().index.files.insert(file_key.clone(), Entry { ok: Some((10, 15)), no: None, n: 1 });
        t2.plan(&cat);
        assert_eq!(shown(&t2).and_then(|x| x.1).as_deref(), Some("own"), "the pick survives a restart");
        // Removed: the card goes back to its file's cover, with nothing to make.
        t2.chosen.lock().unwrap().remove(&ids).unwrap();
        t2.plan(&cat);
        assert_eq!(shown(&t2), Some((file_key, None)));
        assert!(t2.ready().done.iter().all(|x| x.id != "dir:s/Shelf"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn no_cover_is_looked_up_without_a_request() {
        let dir = std::env::temp_dir().join(format!("fvh-no-lookup-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        // Books with no cover in their files: one only in the cloud, one whose file gave none.
        let cloud = work(false, &["Reading"], vec![item("a.fb2", true)]);
        let mut none = work(false, &["Reading"], vec![item("b.pdf", false)]);
        none.id = "v".into();
        let bad = sources(&none)[0].key.clone();
        let cat = books(vec![cloud, none], true);
        let before = crate::web::SENT.load(std::sync::atomic::Ordering::SeqCst);
        let t = Thumbs::new(dir.clone());
        t.set_chosen(Covers::load(&dir));
        t.inner.lock().unwrap().index.files.insert(bad, Entry { ok: None, no: Some("no cover".into()), n: 1 });
        t.plan(&cat);
        assert!(t.inner.lock().unwrap().todo.is_empty(), "nothing to make: no cover is fetched by itself");
        t.forget(&cat, &HashSet::from(["w".to_string(), "v".to_string()]));
        t.plan(&cat);
        let g = t.inner.lock().unwrap();
        assert!(g.todo.values().flatten().all(|s| s.from.is_none()), "made again only from its files");
        drop(g);
        assert_eq!(crate::web::SENT.load(std::sync::atomic::Ordering::SeqCst), before, "a request was sent");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn covers_found_before_are_kept_as_chosen() {
        let dir = std::env::temp_dir().join(format!("fvh-adopt-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let cache = dir.join("cache");
        let t = Thumbs::new(cache.clone());
        t.set_chosen(Covers::load(&dir));
        let url = "https://covers.openlibrary.org/b/id/1-L.jpg?default=false";
        let img = picture_bytes(&png()).unwrap();
        t.write(&fnv(&format!("web|{url}")), &img, 1).unwrap();
        let old = format!(r#"{{"v":1,"by":{{"w":{{"found":{{"url":"{url}","from":"openlibrary","title":"t"}}}},"r":{{"refused":{{"url":"u","from":"fantlab","title":"t"}}}}}}}}"#);
        std::fs::write(cache.join("web.json"), old).unwrap();
        t.adopt_web_covers();
        assert!(!cache.join("web.json").exists());
        let c = Covers::load(&dir);
        assert_eq!(c.get("w").map(|x| x.from.as_str()), Some(crate::web::OPEN_LIBRARY));
        assert!(c.get("r").is_none(), "a refused cover is dropped");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_thumbnail_is_made_by_one_worker_at_a_time() {
        let dir = std::env::temp_dir().join(format!("fvh-busy-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut v = work(false, &["Reading"], vec![item("v.pdf", false)]);
        v.id = "v".into();
        let w = work(false, &["Reading"], vec![item("w.pdf", false)]);
        let s = crate::scan::Section { id: "s".into(), name: "s".into(), path: "D:\\x".into(), drive: "D".into(), kind: Kind::Books, guessed: false, online: true, gone: false, works: vec![w, v], files: 2, scanned_at: 0, unlisted: vec![], empty: vec![], whole_tree: false, split_from: None, links: vec![] };
        let t = Thumbs::new(dir.clone());
        t.plan(&Catalog { v: 1, sections: vec![s] });
        // On screen and planned: "w" waits in both queues.
        t.want(vec!["w".into()]);
        assert_eq!(t.next().0, "w");
        assert_eq!(t.next().0, "v", "a second worker took the thumbnail the first is making");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_newer_job_for_a_busy_thumbnail_is_not_lost() {
        let dir = std::env::temp_dir().join(format!("fvh-newer-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let w = work(false, &["Reading"], vec![item("w.pdf", false)]);
        let t = Thumbs::new(dir.clone());
        let cat = books(vec![w], true);
        t.plan(&cat);
        let (id, old) = t.next();
        // While "w" is made from its file, the person chooses a cover for it.
        let mut covers = Covers::load(&dir);
        covers.keep(&["w".into()], &png(), crate::chosen::OWN).unwrap();
        t.set_chosen(covers);
        t.plan(&cat);
        // A second worker passes it over; the file then gives nothing.
        t.finish(id, &old, None);
        assert!(t.inner.lock().unwrap().todo.get("w").is_some_and(|s| s[0].from.is_some()), "the chosen cover's job was dropped");
        assert_eq!(t.next().1[0].from.as_deref(), Some("own"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn cloud_only_files_are_never_read() {
        let w = work(false, &["Reading"], vec![item("b.pdf", true), item("b.epub", false)]);
        assert_eq!(names(&w), ["b.epub"]);
    }

    /// Makes a thumbnail from each file in `.scratch/fixtures` (made by `tests/make_fixtures.py`)
    /// with the bundled tools, and checks each came from the right place. Needs `cargo build` first.
    #[test]
    #[ignore]
    fn fixtures() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../.scratch");
        let out = root.join("thumb-test");
        let _ = std::fs::remove_dir_all(&out);
        let t = Thumbs::new(out.clone());
        let mut fails = vec![];
        let mut files: Vec<_> = std::fs::read_dir(root.join("fixtures")).unwrap().map(|e| e.unwrap().path()).collect();
        files.sort();
        for f in files {
            let name = f.file_name().unwrap().to_string_lossy().to_string();
            let ext = f.extension().unwrap().to_string_lossy().to_lowercase();
            let s = Source { path: f.to_string_lossy().into(), ext, key: fnv(&name), shots: vec![], frame: false, from: None };
            let started = Instant::now();
            let r = t.make(&s).and_then(|(img, n)| t.write(&s.key, &img, n).map(|(w, h, _)| ((w, h), img)));
            let ms = started.elapsed().as_millis();
            let expect_bad = ["broken.pdf", "no-preview.docx", "no-cover.mp3"].contains(&name.as_str());
            match r {
                Ok(((w, h), img)) => {
                    println!("{name:<28} {w}x{h} in {ms} ms -> {}", file_of(&out, &s.key).display());
                    let ok = match name.as_str() {
                        "photo-rotated.jpg" => h > w,
                        "book.epub" | "book.fb2" | "comic.cbz" => h == BOX && !blank(&img),
                        n if n.ends_with(".mp4") || n.ends_with(".mkv") => img.width() >= w * 2 && !blank(&img),
                        "transparent.png" => flatten(&img).get_pixel(0, 0).0 == [255, 255, 255],
                        _ => !expect_bad && !blank(&img),
                    };
                    if !ok {
                        fails.push(name);
                    }
                }
                Err(e) => {
                    println!("{name:<28} {e:?} in {ms} ms");
                    if !expect_bad || matches!(e, Fail::Retry(_)) {
                        fails.push(name);
                    }
                }
            }
        }
        assert!(fails.is_empty(), "wrong: {fails:?}");
    }

    #[test]
    fn keys_are_stable() {
        assert_eq!(fnv(""), "cbf29ce484222325");
        assert_eq!(fnv("a"), "af63dc4c8601ec8c");
    }

    #[test]
    fn flat_pictures_are_blank() {
        let flat = DynamicImage::ImageRgb8(image::RgbImage::from_pixel(100, 100, image::Rgb([20, 20, 20])));
        assert!(blank(&flat));
        let mut line = image::RgbImage::from_pixel(256, 256, image::Rgb([255, 255, 255]));
        for x in 20..236 {
            line.put_pixel(x, 30, image::Rgb([0, 0, 0]));
        }
        assert!(!blank(&DynamicImage::ImageRgb8(line)), "a page with one line of text");
        let mut busy = image::RgbImage::new(100, 100);
        for (x, _, p) in busy.enumerate_pixels_mut() {
            *p = image::Rgb([(x * 2) as u8; 3]);
        }
        assert!(!blank(&DynamicImage::ImageRgb8(busy)));
    }
}

#[cfg(test)]
#[path = "qa_thumbs_clips.rs"]
mod qa_clips;
