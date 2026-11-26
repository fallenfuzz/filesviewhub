//! Reads the checked folders and makes the catalog: one Section per checked folder, its Works
//! made by its Section kind (`decisions/2026/10/05/section-kind-guessed-and-changeable.md`). Reads names, sizes, dates and attributes only, never file
//! contents, and never touches a cloud-only file (`decisions/2026/10/05/checked-folders-on-any-drive.md`). Every file it walks ends up in a Work
//! or in the Section's unlisted files, with the reason (`decisions/2026/09/30/hub-lists-every-file.md`).

use crate::drives::{self, Drive};
use crate::store::{CheckedFolder, Kind};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

/// What an item is to its Work: a lesson, track or title takes a Tick; a handout, captions or a
/// code file does not (`decisions/2026/09/29/ticks-offer-the-mark-never-set-it.md`).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    #[default]
    Main,
    Doc,
    Cc,
    Code,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Item {
    pub name: String,
    /// The path from the Work's folder (or the Work's own folder for a file Work); in an Event, from
    /// its Photos Section's or Photos folder's.
    pub rel: String,
    pub path: String,
    pub ext: String,
    pub size: u64,
    pub mtime: u64,
    /// Only in the cloud (Dropbox or OneDrive online-only): not read until it is on disk.
    pub cloud: bool,
    /// Its Tick key, as the Hub makes it (`decisions/2026/09/30/work-id-is-its-path.md`): `rel`, without the extension for a course's
    /// lessons, so a lesson's video and handout of one name are one Tick. A handout or captions
    /// share the key of the lesson they belong to.
    #[serde(default)]
    pub key: String,
    /// The name made to read well: no lesson number, no extension.
    #[serde(default)]
    pub title: String,
    /// The subfolder it is in below the Work's folder, folder titles joined by " / "; empty for
    /// the Work's own files.
    #[serde(default)]
    pub group: String,
    #[serde(default)]
    pub role: Role,
    /// The language of captions, as "EN", when the file name says it.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub lang: String,
    /// In a Photos Section, when it was taken (`photos.rs`); None when undated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Work {
    pub id: String,
    pub title: String,
    /// The Work's folder, or its first file.
    pub path: String,
    /// The folders between the Section and the Work.
    pub folder: Vec<String>,
    pub is_folder: bool,
    pub items: Vec<Item>,
    /// Watching, Reading, Listening; empty for pictures and other files.
    pub media: Vec<String>,
    pub size: u64,
    /// The newest file date in the Work.
    pub added: u64,
    pub cloud: usize,
    /// A year the name gives: "[2015] Title".
    #[serde(default)]
    pub year: Option<u16>,
    /// An Event's first and last time taken (`photos.rs`); None for every other Work, and for
    /// the Undated Event.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub taken: Option<(i64, i64)>,
    /// An Event of a Photos Section, the Undated one included.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub event: bool,
    /// An Event of Photos whose names give only the day they were taken: `taken` is that day's
    /// start and the time is not known (`photos::name_day`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub day_only: bool,
}

/// Why no Work holds a file (`decisions/2026/09/30/hub-lists-every-file.md`).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Reason {
    /// A type the app shows, in a place no Work takes it from.
    Unplaced,
    /// A type the app does not know.
    Unknown,
    /// Captions that match no video.
    Captions,
    /// Code, config and logs.
    Aside,
    /// Inside a program's folder (one that holds an .exe).
    Program,
    /// Moved into the `Not needed` folder of a Photos Section or Photos folder by a Review move script (`decisions/2026/10/04/sorting-ends-in-a-move-script-never-a-delete.md`).
    #[serde(rename = "setaside")]
    SetAside,
    /// A 360 camera's small preview copy (.lrv) of a video beside it (.insv).
    Preview,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Unlisted {
    pub name: String,
    /// The path from the Section's folder, with forward slashes.
    pub rel: String,
    pub path: String,
    /// The folder titles where it shows: its own folder, or the folder of the Work holding it.
    pub folder: Vec<String>,
    pub reason: Reason,
    pub size: u64,
    /// Its date, as an item's: a Section previewed as another kind dates its Works by it.
    #[serde(default)]
    pub mtime: u64,
    pub cloud: bool,
    /// The Work whose folder holds it; it also shows in that Work's dialog.
    #[serde(default)]
    pub work: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Section {
    pub id: String,
    pub name: String,
    pub path: String,
    pub drive: String,
    pub kind: Kind,
    pub guessed: bool,
    pub online: bool,
    /// Its Drive is connected but the folder is not there: moved, renamed or deleted.
    #[serde(default)]
    pub gone: bool,
    pub works: Vec<Work>,
    pub files: usize,
    pub scanned_at: u64,
    #[serde(default)]
    pub unlisted: Vec<Unlisted>,
    /// The folders read with no file in them or below, as paths from the Section's folder: with the
    /// Works and the unlisted files they give back the whole tree read (`tree_of`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub empty: Vec<String>,
    /// Read since the catalog kept `empty` and unlisted files' dates: only then is a preview exact.
    #[serde(default)]
    pub whole_tree: bool,
    /// A Split-out Section's: the id of the Section it came from (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub split_from: Option<String>,
    /// The Split-out Sections made from its folders, each shown in its place as a link to it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<Link>,
}

/// A folder of a Section split out as a Section of its own (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`): that Section's id, the
/// folder's path from the Section's folder, and the titles of the folders down to it, as Works'
/// `folder` names them.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Link {
    pub id: String,
    pub rel: String,
    pub folder: Vec<String>,
}

/// A Work of `sec` outside folder `key` (`{Section id}/{path}`) holds a file inside it, as a book
/// holds the files of its format folder: that folder cannot be a Section of its own (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`).
/// A Photos Section's Events gather pictures from every folder and are made again without it.
pub fn splits_a_work(sec: &Section, key: &str) -> bool {
    let Some(rel) = crate::store::rel_in(key, &sec.id) else { return false };
    let dir = rel.split('/').fold(PathBuf::from(&sec.path), |p, x| p.join(x));
    let low = crate::store::case(key);
    sec.works
        .iter()
        .filter(|w| !(w.event && place_of(w) == sec.id))
        .filter(|w| {
            let id = crate::store::case(&w.id);
            !(id.starts_with(&format!("{low}/")) || id.starts_with(&format!("{low}#")))
        })
        .any(|w| w.items.iter().any(|i| crate::store::within(Path::new(&i.path), &dir)))
}

/// Puts on each Section of the catalog the Section it was split out of and the links to the
/// Sections split out of it, from the checked folders. Reads nothing.
pub fn link_sections(cat: &mut Catalog, s: &crate::store::Settings) {
    for sec in &mut cat.sections {
        let Some(cf) = s.folders.iter().find(|f| f.id() == sec.id) else { continue };
        sec.split_from = s.container_of(cf).filter(|_| cf.split).map(CheckedFolder::id);
        sec.links = s
            .folders
            .iter()
            .filter(|f| f.split && s.container_of(f).is_some_and(|c| c.id() == sec.id))
            .filter_map(|f| {
                let rel = cf.rel_of(f)?;
                Some(Link { id: f.id(), folder: link_titles(sec, &rel), rel })
            })
            .collect();
    }
    // A Split-out Section with no name of its own is titled as its row in the Section it came from.
    let titles: Vec<(String, String)> = cat.sections.iter().flat_map(|c| c.links.iter().filter_map(|l| Some((l.id.clone(), l.folder.last()?.clone())))).collect();
    for sec in &mut cat.sections {
        let named = s.folders.iter().find(|f| f.id() == sec.id).is_some_and(|f| f.name.is_some());
        if let Some((_, t)) = titles.iter().find(|(id, _)| *id == sec.id).filter(|_| !named) {
            sec.name = t.clone();
        }
    }
}

/// The folder titles of a link's path inside its Section, as its Works show them: a folder that
/// holds a Work has that Work's title for it; any other keeps its number as a numbered folder
/// beside it with Works does, else as `walk_as` decided without it, as it was left out of the
/// read: cut beside other folders, kept alone (`numbered`).
fn link_titles(sec: &Section, rel: &str) -> Vec<String> {
    if sec.kind == Kind::Photos {
        return rel.split('/').map(|p| title_of(p, false)).collect();
    }
    let parts: Vec<&str> = rel.split('/').collect();
    // Each Work's folders on disk (a folder Work's own last), the titles of those it shows in,
    // and a folder Work's title.
    let homes: Vec<(Vec<&str>, &[String], Option<&str>)> = sec
        .works
        .iter()
        .filter_map(|w| {
            let mut on_disk: Vec<&str> = crate::store::under(place_of(w), &sec.id)?.split('/').collect();
            if !w.event {
                on_disk.pop(); // the file, or "" after a folder Work's own folder
            }
            let n = on_disk.len().min(w.folder.len());
            let own = (w.is_folder && !w.event && on_disk.len() == n + 1).then_some(w.title.as_str());
            Some((on_disk, &w.folder[..n], own))
        })
        .collect();
    (0..parts.len())
        .map(|i| {
            let up = &parts[..i];
            if let Some((_, titles, _)) = homes.iter().find(|(d, t, _)| t.len() > i && d[..=i] == parts[..=i]) {
                return titles[i].clone();
            }
            let beside: Vec<_> = homes.iter().filter(|(d, _, _)| d.len() > i && d[..i] == *up).collect();
            // A folder beside it whose number a read would cut shows whether it was kept.
            let kept = beside.iter().find_map(|(d, t, own)| {
                if title_of(d[i], false) == title_in(d[i], false, true) {
                    return None;
                }
                Some(if t.len() > i { t[i] != title_of(d[i], false) } else { (*own)? != clean_title(d[i], false).0 })
            });
            let keep = kept.unwrap_or_else(|| beside.is_empty() && numbered([parts[i]], false));
            title_in(parts[i], false, keep)
        })
        .collect()
}

/// Every Section read. Written and read in the compact form of `packed.rs`.
#[derive(Clone, Debug, Default)]
pub struct Catalog {
    /// The version of the file it was read from (`packed::VERSION` for one made here).
    pub v: u32,
    pub sections: Vec<Section>,
}

/// The Folder kinds inside one Section (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`), by path from its folder as `store::case` gives it.
pub type Kinds = HashMap<String, Kind>;

/// The kind a folder at `rel` (from the Section's folder) is read as: its own, or the nearest
/// folder above it with one, else the Section's.
pub fn kind_at(kinds: &Kinds, section: Kind, rel: &str) -> Kind {
    let rel = crate::store::case(rel);
    let mut at = rel.as_str();
    loop {
        if let Some(k) = kinds.get(at) {
            return *k;
        }
        match at.rsplit_once('/') {
            Some((up, _)) => at = up,
            None => return section,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Class {
    Video,
    Audio,
    Book,
    Image,
}

pub fn class_of(ext: &str) -> Option<Class> {
    Some(match ext {
        "mp4" | "mkv" | "avi" | "mov" | "wmv" | "m4v" | "webm" | "ts" | "flv" | "mpg" | "mpeg" | "3gp" | "insv" | "lrv" => Class::Video,
        "mp3" | "m4a" | "m4b" | "aac" | "flac" | "ogg" | "opus" | "wav" | "wma" => Class::Audio,
        "pdf" | "epub" | "djvu" | "djv" | "fb2" | "mobi" | "azw3" | "kfx" | "cbz" | "cbr" | "doc" | "docx" | "rtf" | "txt"
        | "odt" | "chm" | "ppt" | "pptx" | "ppsx" | "pps" | "odp" | "xls" | "xlsx" | "md" => Class::Book,
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp" | "tif" | "tiff" | "heic" | "insp" => Class::Image,
        _ => return None,
    })
}

fn medium(c: Class) -> Option<&'static str> {
    match c {
        Class::Video => Some("Watching"),
        Class::Audio => Some("Listening"),
        Class::Book => Some("Reading"),
        Class::Image => None,
    }
}

/// A folder named after a format holds files of the Work in the folder above (Books).
const FORMAT_DIRS: &[&str] = &["pdf", "djvu", "epub", "fb2", "mobi", "azw3", "kfx", "doc", "docx", "txt", "rtf", "chm", "cbz", "cbr"];
/// The format a book opens in first.
const PRIORITY: &[&str] = &["pdf", "epub", "mobi", "azw3", "kfx", "fb2", "djvu", "djv", "chm", "cbz", "cbr", "pptx", "ppt", "ppsx", "pps", "docx", "doc", "odt", "rtf", "txt", "md", "xlsx", "xls"];
const CAPTIONS: &[&str] = &["srt", "vtt"];
const CAPTION_DIRS: &[&str] = &["captions", "subtitles", "subs"];
/// A book's or a talk's attachments: a folder of these beside one Work holds its code files.
const EXTRA_DIRS: &[&str] = &["code", "files"];
/// Course attachments a course lists among its files.
const EXTRA: &[&str] = &["zip", "gz", "vsd", "pcapng", "7z", "rar"];
/// Code, config and logs: a Work lists them as its code files, never as lessons.
const ASIDE: &[&str] = &[
    "py", "sh", "c", "h", "cpp", "js", "ts", "awk", "sql", "yml", "yaml", "j2", "template", "cfg", "conf", "ini", "xml", "json", "log",
    "pyc", "retry", "gv", "m3u", "swf", "exe", "dll", "db", "bat", "cmd", "ps1", "ipynb", "html", "htm", "css",
];

struct F {
    name: String,
    ext: String,
    class: Option<Class>,
    size: u64,
    mtime: u64,
    cloud: bool,
    path: PathBuf,
    /// A 360 camera's preview copy of a video beside it, shown only as unlisted (`class` is None).
    preview: bool,
}

impl F {
    fn caption(&self) -> bool {
        CAPTIONS.contains(&self.ext.as_str())
    }
    fn aside(&self) -> bool {
        ASIDE.contains(&self.ext.as_str()) || self.name.starts_with("~$")
    }
    fn media(&self) -> bool {
        matches!(self.class, Some(Class::Video | Class::Audio))
    }
}

struct Dir {
    name: String,
    files: Vec<F>,
    dirs: Vec<Dir>,
    /// Holds a program (an .exe): its files are listed, never made into Works.
    program: bool,
}

impl Dir {
    fn count(&self, c: Class) -> usize {
        self.files.iter().filter(|f| f.class == Some(c)).count() + self.dirs.iter().map(|d| d.count(c)).sum::<usize>()
    }
    fn total(&self) -> usize {
        self.files.len() + self.dirs.iter().map(Dir::total).sum::<usize>()
    }
    fn has_media(&self) -> bool {
        self.count(Class::Video) + self.count(Class::Audio) > 0
    }
    fn content(&self) -> usize {
        self.files.iter().filter(|f| f.class.is_some()).count() + self.dirs.iter().map(Dir::content).sum::<usize>()
    }
    fn own_media(&self) -> usize {
        self.files.iter().filter(|f| f.media()).count()
    }
}

/// Compares names the way a person reads them: "Part 2" before "Part 10", case aside.
pub fn natural(a: &str, b: &str) -> Ordering {
    let (mut x, mut y) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (x.peek().copied(), y.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, _) => return Ordering::Less,
            (_, None) => return Ordering::Greater,
            (Some(c), Some(d)) if c.is_ascii_digit() && d.is_ascii_digit() => {
                let mut n = String::new();
                while let Some(c) = x.peek().copied().filter(char::is_ascii_digit) {
                    n.push(c);
                    x.next();
                }
                let mut m = String::new();
                while let Some(d) = y.peek().copied().filter(char::is_ascii_digit) {
                    m.push(d);
                    y.next();
                }
                let (n2, m2) = (n.trim_start_matches('0'), m.trim_start_matches('0'));
                let o = n2.len().cmp(&m2.len()).then_with(|| n2.cmp(m2));
                if o != Ordering::Equal {
                    return o;
                }
            }
            (Some(c), Some(d)) => {
                let o = c.to_lowercase().cmp(d.to_lowercase());
                if o != Ordering::Equal {
                    return o;
                }
                x.next();
                y.next();
            }
        }
    }
}

fn starts_with_digit(s: &str) -> bool {
    s.chars().next().is_some_and(|c| c.is_ascii_digit())
}

fn read_tree(path: &Path, rel: &str, unchecked: &HashSet<String>, seen: &mut usize, progress: &dyn Fn(usize)) -> Dir {
    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let mut dir = Dir { name, files: vec![], dirs: vec![], program: false };
    let Ok(rd) = std::fs::read_dir(crate::store::named(path)) else { return dir };
    for e in rd.filter_map(Result::ok) {
        let Ok(md) = e.metadata() else { continue };
        // As written, without the `\\?\` start `named` may give: the catalog keeps plain paths.
        let p = path.join(e.file_name());
        if drives::is_hidden(&p, &md) {
            continue;
        }
        let n = e.file_name().to_string_lossy().to_string();
        if md.is_dir() {
            let r = if rel.is_empty() { n.clone() } else { format!("{rel}/{n}") };
            if !unchecked.contains(&r) {
                dir.dirs.push(read_tree(&p, &r, unchecked, seen, progress));
            }
            continue;
        }
        // "2.4 Dockerfile" has no extension.
        let ext = p.extension().map(|x| x.to_string_lossy().to_lowercase()).filter(|x| !x.contains(' ')).unwrap_or_default();
        if n.eq_ignore_ascii_case("desktop.ini") || n.eq_ignore_ascii_case("thumbs.db") {
            continue;
        }
        *seen += 1;
        if seen.is_multiple_of(500) {
            progress(*seen);
        }
        dir.files.push(F {
            class: class_of(&ext),
            name: n,
            ext,
            size: md.len(),
            mtime: md.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0),
            cloud: is_cloud(&md),
            path: p,
            preview: false,
        });
    }
    finish(&mut dir);
    dir
}

/// What a folder's files say once they are all in it: whether it holds a program, which files
/// are a 360 camera's previews, and the order of its files and folders.
fn finish(dir: &mut Dir) {
    dir.program = dir.files.iter().any(|f| f.ext == "exe");
    // An Insta360 camera saves a small copy of each video beside it: LRV_20210304_070809_01_003.lrv
    // of VID_20210304_070809_00_003.insv. The video is shown; the copy is not shown a second time.
    let videos: HashSet<String> = dir.files.iter().filter(|f| f.ext == "insv").filter_map(|f| insta360_key(&f.name)).collect();
    for f in dir.files.iter_mut().filter(|f| f.ext == "lrv") {
        if insta360_key(&f.name).is_some_and(|k| videos.contains(&k)) {
            f.preview = true;
            f.class = None;
        }
    }
    dir.files.sort_by(|a, b| natural(&a.name, &b.name));
    dir.dirs.sort_by(|a, b| natural(&a.name, &b.name));
}

/// The folders under `d` (at `rel`) with no file in them or below.
fn empty_dirs(d: &Dir, rel: &str, out: &mut Vec<String>) {
    for c in &d.dirs {
        let r = join(rel, &c.name);
        if c.total() == 0 {
            out.push(r.clone());
        }
        empty_dirs(c, &r, out);
    }
}

/// A Section's folder tree as it was last read, made again from its catalog: every file read is
/// in a Work or unlisted, and `empty` holds the folders with none. Reads nothing from the disk.
fn tree_of(sec: &Section) -> Dir {
    let mut root = Dir { name: String::new(), files: vec![], dirs: vec![], program: false };
    fn at<'a>(d: &'a mut Dir, parts: &[&str]) -> &'a mut Dir {
        let Some((first, rest)) = parts.split_first() else { return d };
        let i = match d.dirs.iter().position(|c| c.name == *first) {
            Some(i) => i,
            None => {
                d.dirs.push(Dir { name: first.to_string(), files: vec![], dirs: vec![], program: false });
                d.dirs.len() - 1
            }
        };
        at(&mut d.dirs[i], rest)
    }
    let base = sec.path.trim_end_matches(std::path::MAIN_SEPARATOR);
    let mut seen = HashSet::new();
    let files = sec.works.iter().flat_map(|w| &w.items).map(|i| (&i.path, &i.name, i.size, i.mtime, i.cloud));
    let loose = sec.unlisted.iter().map(|u| (&u.path, &u.name, u.size, u.mtime, u.cloud));
    for (path, name, size, mtime, cloud) in files.chain(loose) {
        let Some(rel) = path.strip_prefix(base) else { continue };
        if !seen.insert(path) {
            continue;
        }
        // By the system's own separators: on macOS a name may hold a backslash.
        let parts: Vec<String> = Path::new(rel).components().filter_map(|c| match c {
            std::path::Component::Normal(x) => Some(x.to_string_lossy().into_owned()),
            _ => None,
        }).collect();
        let parts: Vec<&str> = parts.iter().map(String::as_str).collect();
        let Some((_, dirs)) = parts.split_last() else { continue };
        let ext = Path::new(name).extension().map(|x| x.to_string_lossy().to_lowercase()).filter(|x| !x.contains(' ')).unwrap_or_default();
        at(&mut root, dirs).files.push(F { class: class_of(&ext), name: name.clone(), ext, size, mtime, cloud, path: PathBuf::from(path), preview: false });
    }
    for e in &sec.empty {
        at(&mut root, &e.split('/').collect::<Vec<_>>());
    }
    fn all(d: &mut Dir) {
        finish(d);
        d.dirs.iter_mut().for_each(all);
    }
    all(&mut root);
    root
}

#[cfg(test)]
pub fn preview(sec: &Section, kind: Option<Kind>, cache: Option<&Path>, gap: i64) -> (Kind, Vec<Work>, bool) {
    preview_or_stop(sec, kind, cache, gap, &Kinds::new(), &|| false).expect("never stopped")
}

/// What a Section would show as `kind`, or as the kind the app guesses with None: its kind, its
/// Works, made by the same grouping as a read, from the files read last time, and whether that is
/// exactly what a read makes (the catalog holds the whole tree and every file it read). No folder is
/// read and nothing is saved. A Photos preview reads the date of a picture on this PC that
/// no read has dated yet, as a read would, and keeps it nowhere. `stop` is asked before each file
/// is looked at on the disk: once it says yes, the preview ends with None. `kinds` are the
/// Folder kinds a read would use (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`).
pub fn preview_or_stop(sec: &Section, kind: Option<Kind>, cache: Option<&Path>, gap: i64, kinds: &Kinds, stop: &dyn Fn() -> bool) -> Option<(Kind, Vec<Work>, bool)> {
    let tree = tree_of(sec);
    let exact = sec.whole_tree && tree.total() == sec.files;
    let kind = kind.unwrap_or_else(|| guess(&tree));
    let mut works = vec![];
    let mut unlisted = vec![];
    // A picture whose name gives no date is opened for its date: one only in the cloud since the
    // last read is not, as a read would not open it either.
    fn recheck(d: &mut Dir, stop: &dyn Fn() -> bool) {
        for f in d.files.iter_mut().filter(|f| !f.cloud && f.class.is_some() && crate::photos::name_date(&f.name).is_none()) {
            if stop() {
                return;
            }
            f.cloud = now_cloud(&f.path.to_string_lossy());
        }
        d.dirs.iter_mut().for_each(|c| recheck(c, stop));
    }
    if kind == Kind::Photos {
        let mut tree = tree;
        recheck(&mut tree, stop);
        if stop() {
            return None;
        }
        let mut dates = crate::photos::Dates::load(cache);
        photos_section(&sec.id, &tree, Path::new(&sec.path), &mut dates, gap, kinds, &mut works, &mut unlisted, stop);
        if stop() {
            return None;
        }
    } else {
        // A folder with the Folder kind Photos dates its pictures as a read would.
        let photos = kinds.values().any(|k| *k == Kind::Photos);
        let mut tree = tree;
        for rel in kinds.iter().filter(|(_, k)| **k == Kind::Photos).map(|(r, _)| r) {
            let mut at = Some(&mut tree);
            for part in rel.split('/') {
                at = at.and_then(|d| d.dirs.iter_mut().find(|c| crate::store::case(&c.name) == part));
            }
            if let Some(d) = at {
                recheck(d, stop);
            }
        }
        let mut dates = crate::photos::Dates::load(cache.filter(|_| photos));
        let mut ctx = Ctx::new(sec.id.clone(), kind, kinds, &mut works, &mut unlisted);
        ctx.dates = Some(&mut dates);
        ctx.gap = gap;
        ctx.stop = stop;
        walk(&mut ctx, &tree, Path::new(&sec.path), "", &mut vec![], true);
        if stop() {
            return None;
        }
        sort_works(&mut works);
    }
    Some((kind, works, exact))
}

/// What an Insta360 video or preview name says of the shot, with its prefix and lens number left out:
/// `VID_20210304_070809_00_003.insv` and `LRV_20210304_070809_01_003.lrv` are both
/// `20210304_070809_003`.
pub fn insta360_key(name: &str) -> Option<String> {
    let stem = name.rsplit_once('.')?.0;
    let p: Vec<&str> = stem.split('_').collect();
    let digits = |s: &str, n: usize| s.len() == n && s.bytes().all(|c| c.is_ascii_digit());
    let insta = p[0].eq_ignore_ascii_case("VID") || p[0].eq_ignore_ascii_case("LRV");
    (p.len() == 5 && insta && digits(p[1], 8) && digits(p[2], 6) && digits(p[3], 2) && !p[4].is_empty() && digits(p[4], p[4].len())).then(|| format!("{}_{}_{}", p[1], p[2], p[4]))
}

/// A file only in the cloud now: opening it would make the sync client download it.
pub fn now_cloud(path: &str) -> bool {
    std::fs::metadata(crate::store::named(Path::new(path))).is_ok_and(|m| is_cloud(&m))
}

pub fn is_cloud(md: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const OFFLINE: u32 = 0x1000;
        const RECALL_ON_OPEN: u32 = 0x40000;
        const RECALL_ON_DATA_ACCESS: u32 = 0x400000;
        return md.file_attributes() & (OFFLINE | RECALL_ON_OPEN | RECALL_ON_DATA_ACCESS) != 0;
    }
    #[allow(unreachable_code)]
    {
        let _ = md;
        false
    }
}

/// The end of a read's log line, saying how many files under the folder titles `under` (all
/// of them when empty) are only in the cloud, so the person sees none was downloaded.
pub fn cloud_note(sec: &Section, under: &[String]) -> String {
    let n = sec.works.iter().filter(|w| w.folder.starts_with(under)).map(|w| w.cloud).sum::<usize>()
        + sec.unlisted.iter().filter(|u| u.cloud && u.folder.starts_with(under)).count();
    match n {
        0 => String::new(),
        1 => "; 1 file only in the cloud was not read or downloaded".into(),
        n => format!("; {n} files only in the cloud were not read or downloaded"),
    }
}

/// A name made to read well: no extension, no leading "!" or lesson number, no underscores.
pub fn title_of(name: &str, strip_ext: bool) -> String {
    title_in(name, strip_ext, false)
}

/// `title_of`, its lesson number kept when `keep`: the numbers of the names beside it give
/// their order (`numbered`).
fn title_in(name: &str, strip_ext: bool, keep: bool) -> String {
    let base = name_base(name, strip_ext);
    // A lesson number is "01 ", "1. ", "2) " or "3 - ", not the start of a title like "101 Dalmatians",
    // "3-D Printing" (a "-" needs a space beside it) or "2.1 Intro" (a lesson's own number).
    let digits = base.chars().take_while(|c| c.is_ascii_digit()).count();
    let after = base[digits..].trim_start();
    let next = |m: char| after.strip_prefix(m).map(|r| r.chars().next());
    let dash = next('-').is_some_and(|c| base[digits..].starts_with(char::is_whitespace) || c.is_none_or(char::is_whitespace));
    let marked = next('.').is_some() || next(')').is_some() || dash;
    let rest = after.trim_start_matches(['.', ')', '-']).trim_start();
    let decimal = next('.').flatten().is_some_and(|c| c.is_ascii_digit());
    let t = if !keep && !decimal && (1..=3).contains(&digits) && !rest.is_empty() && (marked || base.starts_with('0')) { rest } else { base.as_str() };
    t.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn name_base(name: &str, strip_ext: bool) -> String {
    let base = if strip_ext { name.rsplit_once('.').map(|(a, _)| a).unwrap_or(name) } else { name };
    base.trim_start_matches('!').replace('_', " ")
}

/// Names side by side keep their leading numbers when more than half of them start with one
/// ("1 ", "02.", "3) ", "10 - "): the numbers give their order, 2 before 10.
fn numbered<'a>(names: impl IntoIterator<Item = &'a str>, strip_ext: bool) -> bool {
    let (mut n, mut all) = (0, 0);
    for name in names {
        let base = name_base(name, strip_ext);
        let digits = base.chars().take_while(|c| c.is_ascii_digit()).count();
        let rest = base[digits..].trim_start_matches([' ', '.', ')', '-']);
        // A number a title would lose, or one standing apart: "1 Intro", "2.1 Intro".
        let apart = base[digits..].starts_with([' ', '.', ')']) || base[digits..].starts_with("- ");
        all += 1;
        if title_in(name, strip_ext, false) != title_in(name, strip_ext, true) || ((1..=3).contains(&digits) && apart && !rest.is_empty()) {
            n += 1;
        }
    }
    n * 2 > all
}

/// A Work's title and the year its name gives, as the Hub reads them: "[2015] Title",
/// "Title - pdf - 2015", and no "(Z-Library)", "(from kfx)" or "- YouTube".
pub fn clean_title(name: &str, strip_ext: bool) -> (String, Option<u16>) {
    clean_title_in(name, strip_ext, false)
}

/// `clean_title`, its lesson number kept when `keep` (`title_in`).
fn clean_title_in(name: &str, strip_ext: bool, keep: bool) -> (String, Option<u16>) {
    let mut t = title_in(name, strip_ext, keep);
    let mut year = None;
    let four = |s: &str| (s.len() == 4 && s.bytes().all(|b| b.is_ascii_digit())).then(|| s.parse::<u16>().ok()).flatten();
    if let Some(r) = t.strip_prefix('[') && let Some(y) = r.get(..4).and_then(four) && r[4..].starts_with([']', '}']) {
        year = Some(y);
        t = r[5..].trim_start().to_string();
    }
    let lower = t.to_lowercase();
    if let Some(i) = lower.rfind(" - pdf - ") && let Some(y) = four(t[i + 9..].trim()) {
        year = year.or(Some(y));
        t.truncate(i);
    }
    for junk in ["(Z-Library)", "(z-library)", "(from kfx)"] {
        t = t.replace(junk, "");
    }
    if t.to_lowercase().ends_with("- youtube") {
        t.truncate(t.len() - "- youtube".len());
    }
    let t = t.split_whitespace().collect::<Vec<_>>().join(" ");
    (t.trim_matches([' ', '-']).to_string(), year)
}

/// The name that groups a book's formats: case, spaces, "_" and "(from kfx)" aside; a book named
/// by its ISBN groups by the number.
fn stem_key(name: &str) -> String {
    let s = name.rsplit_once('.').map(|(a, _)| a).unwrap_or(name).to_lowercase().replace("(from kfx)", "").replace('{', "[").replace('}', "]");
    let s = s.replace('_', " ").split_whitespace().collect::<Vec<_>>().join(" ");
    let digits = s.chars().take_while(|c| c.is_ascii_digit()).count();
    if (10..=13).contains(&digits) && !s[digits..].starts_with(|c: char| c.is_alphanumeric()) {
        return s[..digits].to_string();
    }
    s
}

/// A chapter word at the start of a name ("Module 1", "Week2", "Урок 3"), as the Hub reads it.
fn chapter_word(name: &str) -> bool {
    const WORDS: &[&str] = &[
        "module", "chapter", "lesson", "lecture", "part", "section", "session", "week", "day", "unit", "episode", "урок", "лекция",
        "занятие", "тема", "часть", "модуль", "неделя", "день", "глава", "раздел",
    ];
    let n = name.trim().to_lowercase();
    WORDS.iter().any(|w| n.strip_prefix(w).is_some_and(|r| !r.starts_with(|c: char| c.is_alphabetic())))
}

/// Named like a part of one course: a leading number that is not a year, or a chapter word.
fn chapter_like(name: &str) -> bool {
    let n = name.trim();
    let digits = n.chars().take_while(|c| c.is_ascii_digit()).count();
    (digits > 0 && digits != 4) || chapter_word(n)
}

/// Chapter folders: two or more folders with media whose names differ only in their numbers
/// ("01 Basics", "02 Tools"; "Week 1", "Week 2"; "Глава 1", "Глава 2").
fn chapter_dirs(d: &Dir) -> usize {
    let with_media: Vec<&Dir> = d.dirs.iter().filter(|c| c.has_media()).collect();
    let shape = |n: &str| n.chars().take_while(|c| !c.is_ascii_digit()).collect::<String>().trim().to_lowercase();
    let numbered: Vec<&&Dir> = with_media.iter().filter(|c| c.name.chars().any(|ch| ch.is_ascii_digit())).collect();
    if numbered.len() < 2 {
        return 0;
    }
    let first = shape(&numbered[0].name);
    if numbered.iter().all(|c| shape(&c.name) == first) { numbered.len() } else { 0 }
}

fn is_course(d: &Dir) -> bool {
    d.own_media() >= 2 || chapter_dirs(d) >= 2
}

/// A folder of separate courses rather than one course made of chapters: most of its folders
/// with media are courses of their own (not named like a chapter, and with chapters, or with
/// five lessons or more), or one of them holds separate courses itself.
fn holds_courses(d: &Dir) -> bool {
    let kids: Vec<&Dir> = d.dirs.iter().filter(|c| c.has_media() && !c.program).collect();
    if kids.len() < 2 {
        return false;
    }
    if kids.iter().any(|k| holds_courses(k)) {
        return true;
    }
    let lessons = |k: &Dir| k.count(Class::Video) + k.count(Class::Audio);
    let courses = kids
        .iter()
        .filter(|k| !chapter_word(&k.name) && is_course(k) && (chapter_dirs(k) >= 2 || (!chapter_like(&k.name) && lessons(k) >= 5)))
        .count();
    2 * courses > d.own_media() + kids.len()
}

fn numbered_course(d: &Dir) -> bool {
    let media: Vec<&F> = d.files.iter().filter(|f| f.media()).collect();
    let numbered = media.iter().filter(|f| starts_with_digit(&f.name)).count();
    (media.len() >= 3 && numbered * 2 >= media.len()) || chapter_dirs(d) >= 2
}

/// Names a part of one Work: CD1, Audio, UK, US, a bare number, resources, glossary, Звук.
fn part_name(name: &str) -> bool {
    let n = name.trim().to_lowercase();
    let whole = ["uk", "us", "activities", "resource", "resources", "glossary", "dictionary", "picture dictionary", "google books", "скан", "scan", "scans"];
    if whole.contains(&n.as_str()) || (!n.is_empty() && n.chars().all(|c| c.is_ascii_digit())) {
        return true;
    }
    let cd = |w: &str| w.strip_prefix("cd").is_some_and(|r| r.chars().all(|c| c.is_ascii_digit()));
    let words: Vec<&str> = n.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).collect();
    words.iter().enumerate().any(|(i, w)| *w == "audio" || *w == "звук" || cd(w) || (*w == "cd" && words.get(i + 1).is_none_or(|x| x.chars().all(|c| c.is_ascii_digit()))))
}

/// A whole name that only says which part it is: "CD1", "Disc 2", "Version 2", "Vol. 3", "Часть 1",
/// "Том 2" or, for a folder, a number of up to three digits; a file named "300" or "2010" is a title.
fn part_label(name: &str, is_folder: bool) -> bool {
    const WORDS: &[&str] = &["cd", "disc", "disk", "диск", "version", "vol", "volume", "part", "pt", "book", "книга", "часть", "том"];
    let n = name.trim().to_lowercase();
    let word = WORDS.iter().filter(|w| n.starts_with(*w)).max_by_key(|w| w.len()).map_or("", |w| w);
    let num = n[word.len()..].trim_start_matches([' ', '.', '_', '-']);
    (1..=3).contains(&num.len()) && num.bytes().all(|b| b.is_ascii_digit()) && (!word.is_empty() || (is_folder && num.len() == n.len()))
}

/// A Work named only by its part takes the title of the folder above, then the part: "The Time
/// Machine - CD1". A folder above named as a part itself ("Audio", "CD 1") is passed over.
fn part_title(title: String, folder: &[String], is_folder: bool) -> String {
    if !part_label(&title, is_folder) {
        return title;
    }
    let above = folder.iter().rev().map(|f| clean_title(f, false).0).find(|f| !part_label(f, true) && !["audio", "звук"].contains(&f.to_lowercase().as_str()));
    match above {
        Some(a) if !a.is_empty() => format!("{a} - {title}"),
        _ => title,
    }
}

/// Holds audio and nothing else to show.
fn audio_only(d: &Dir) -> bool {
    let c = d.content();
    c > 0 && d.count(Class::Audio) == c
}

/// What a person would count as things: a file, but a folder of three or more audio tracks
/// (an audiobook, an album) is one.
fn units(d: &Dir, out: &mut [usize; 4]) {
    let audio = d.files.iter().filter(|f| f.class == Some(Class::Audio)).count();
    for f in &d.files {
        if let Some(c) = f.class && (c != Class::Audio || audio < 3) {
            out[c as usize] += 1;
        }
    }
    if audio >= 3 {
        out[Class::Audio as usize] += 1;
    }
    for c in &d.dirs {
        units(c, out);
    }
}

fn any_dir(d: &Dir, test: &dyn Fn(&Dir) -> bool) -> bool {
    d.dirs.iter().any(|c| test(c) || any_dir(c, test))
}

/// Counts book files in packs: folders of six or more, most of them numbered and few named with an
/// author ("Title - Author"), as worksheets and activity pages come. A library of books has few.
fn pack_books(d: &Dir, out: &mut usize) {
    let books: Vec<&F> = d.files.iter().filter(|f| f.class == Some(Class::Book)).collect();
    let stem = |f: &F| f.name.rsplit_once('.').map_or(f.name.as_str(), |(s, _)| s).trim_end().to_string();
    let numbered = books.iter().filter(|f| starts_with_digit(&f.name) || stem(f).ends_with(|c: char| c.is_ascii_digit())).count();
    let authored = books.iter().filter(|f| f.name.contains(" - ")).count();
    if books.len() >= 6 && numbered * 2 >= books.len() && authored * 2 < books.len() {
        *out += books.len();
    }
    for c in &d.dirs {
        pack_books(c, out);
    }
}

/// Counts pictures and videos, and those named as cameras and phones name them.
fn camera_files(d: &Dir, cam: &mut usize, shots: &mut usize) {
    for f in d.files.iter().filter(|f| matches!(f.class, Some(Class::Image | Class::Video))) {
        *shots += 1;
        *cam += usize::from(crate::photos::camera_name(&f.name));
    }
    for c in &d.dirs {
        camera_files(c, cam, shots);
    }
}

/// The kind a folder's contents suggest; the person can change it.
fn guess(root: &Dir) -> Kind {
    let mut u = [0usize; 4];
    units(root, &mut u);
    let (video, audio, book, image) = (u[0], u[1], u[2], u[3]);
    let total = video + audio + book + image;
    if total == 0 {
        return Kind::Files;
    }
    // Nearly all pictures and clips, most named as cameras and phones name them. Any other file
    // in a Photos Section is unlisted, so a folder of photos and documents stays a Collection.
    let (mut cam, mut shots) = (0, 0);
    camera_files(root, &mut cam, &mut shots);
    if image >= 20 && shots * 100 >= root.content() * 97 && cam * 2 >= shots {
        return Kind::Photos;
    }
    if book * 2 >= total {
        // Mostly packs of pages (a kids' folder of activity sheets): each folder is one card.
        let mut packed = 0;
        pack_books(root, &mut packed);
        return if packed * 2 > book { Kind::Collections } else { Kind::Books };
    }
    if (video + audio) * 2 >= total && any_dir(root, &numbered_course) {
        return Kind::Courses;
    }
    if image * 5 >= total * 2 {
        return Kind::Collections;
    }
    Kind::Files
}

/// A top-level folder of a Section whose files suggest another kind than the Section's (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`): its name on disk, its title as the folder column shows it, the kind the app guesses for
/// it and how many files it holds.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Suggestion {
    pub rel: String,
    pub title: String,
    pub kind: Kind,
    pub files: usize,
}

/// A top-level folder's guess is offered when it holds this share of its Section's files, in
/// percent, or this many files (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`).
const OFFER_SHARE: usize = 10;
const OFFER_FILES: usize = 50;

/// The top-level folders of `tree`, a Section of kind `kind` with `all` files, whose guessed kind
/// differs from it and which hold 10% of its files or 50 files (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`). Left out: a folder with a Folder kind of its
/// own in `kinds`, and in a Photos Section a year, Undated, Review's Not needed, a folder of only
/// pictures and one of mostly camera pictures and clips (a trip with its tickets), which are its
/// calendar's; a folder of videos not named by a camera (courses, films) is offered. Only
/// offered: nothing here sets a kind.
fn suggestions(tree: &Dir, kind: Kind, kinds: &Kinds, all: usize) -> Vec<Suggestion> {
    let calendar = |c: &Dir| {
        let n = c.name.trim().to_lowercase();
        let (mut cam, mut shots) = (0, 0);
        camera_files(c, &mut cam, &mut shots);
        let camera = cam * 2 >= shots && shots * 100 >= c.content() * 80;
        (n.len() == 4 && n.bytes().all(|b| b.is_ascii_digit())) || n == "undated" || n == crate::shots::NOT_NEEDED.to_lowercase() || c.count(Class::Image) == c.content() || camera
    };
    // Titled as the rows below the offer: their numbers kept as the read keeps them (`walk_as`).
    let keep = kind != Kind::Photos && numbered(tree.dirs.iter().filter(|c| !c.program && c.total() > 0).map(|c| c.name.as_str()), false);
    tree.dirs
        .iter()
        .filter(|c| c.content() > 0 && !kinds.contains_key(&crate::store::case(&c.name)))
        .filter(|c| kind != Kind::Photos || !calendar(c))
        .filter(|c| c.total() * 100 >= all * OFFER_SHARE || c.total() >= OFFER_FILES)
        .filter_map(|c| {
            let g = guess(c);
            (g != kind).then(|| Suggestion { rel: c.name.clone(), title: title_in(&c.name, false, keep), kind: g, files: c.total() })
        })
        .collect()
}

/// What the app offers for Section `sec` (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`), from the files read last time: a kind for each
/// of its top-level folders that looks like another kind, once per Section, so none after the
/// person answered or put the offer away, unless `again` (Section settings' Suggest kinds for its
/// folders). A folder the Section makes into one Work (a course, a Collection's folder) is left
/// out: its guess is for one Work, never for a kind. So is a folder whose files are part of a Work
/// outside it (a book's format folder), which can take neither shape. Reads no folder and changes
/// nothing.
pub fn offer(s: &crate::store::Settings, sec: &Section, again: bool) -> Vec<Suggestion> {
    let Some(cf) = s.folders.iter().find(|f| f.id() == sec.id) else { return vec![] };
    if !again && s.offered.contains(&sec.id) {
        return vec![];
    }
    let works: HashSet<String> = sec.works.iter().filter(|w| w.is_folder).map(|w| crate::store::case(w.id.trim_end_matches('/'))).collect();
    suggestions(&tree_of(sec), sec.kind, &s.kinds_in(cf), sec.files)
        .into_iter()
        .filter(|x| {
            let key = crate::store::key_in(&sec.id, &x.rel);
            !works.contains(&crate::store::case(&key)) && !splits_a_work(sec, &key)
        })
        .collect()
}

struct Ctx<'a> {
    id_base: String,
    kind: Kind,
    kinds: &'a Kinds,
    out: &'a mut Vec<Work>,
    unlisted: &'a mut Vec<Unlisted>,
    /// For a folder with the Folder kind Photos: the dates read before (none keeps them in memory
    /// only), the Event gap in seconds, and `stop`, asked before each picture's date is read.
    dates: Option<&'a mut crate::photos::Dates>,
    gap: i64,
    stop: &'a dyn Fn() -> bool,
    /// The folder walked now keeps the number its name starts with in its title, as the folders
    /// beside it do (`numbered`); `walk_as` sets it for each subfolder before walking it.
    keep: bool,
}

fn never() -> bool {
    false
}

impl<'a> Ctx<'a> {
    fn new(id_base: String, kind: Kind, kinds: &'a Kinds, out: &'a mut Vec<Work>, unlisted: &'a mut Vec<Unlisted>) -> Self {
        Ctx { id_base, kind, kinds, out, unlisted, dates: None, gap: 4 * 3600, stop: &never, keep: false }
    }
    /// The folder at `rel` has a Folder kind, or holds one below it: it is never one Work, so the
    /// folder with the kind stays a folder and is read as that kind (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`).
    fn holds_kind(&self, rel: &str) -> bool {
        let rel = crate::store::case(rel);
        let below = format!("{rel}/");
        self.kinds.keys().any(|k| *k == rel || k.starts_with(&below))
    }
    /// Lists file `f` (at `rel` from the Section) as unlisted, shown in `folder`.
    fn unlist(&mut self, f: &F, rel: &str, folder: &[String], reason: Reason, work: Option<&str>) {
        self.unlisted.push(Unlisted {
            name: f.name.clone(),
            rel: rel.to_string(),
            path: f.path.to_string_lossy().into(),
            folder: folder.to_vec(),
            reason,
            size: f.size,
            mtime: f.mtime,
            cloud: f.cloud,
            work: work.map(String::from),
        });
    }
    /// Lists every file in a folder and below: in a program folder with `reason`, else each with its own.
    fn unlist_all(&mut self, d: &Dir, rel: &str, folder: &[String], reason: Option<Reason>) {
        for f in &d.files {
            self.unlist(f, &join(rel, &f.name), folder, reason.unwrap_or_else(|| reason_of(f)), None);
        }
        for c in &d.dirs {
            self.unlist_all(c, &join(rel, &c.name), folder, reason);
        }
    }
}

/// Why a file that is not shown was left out.
fn reason_of(f: &F) -> Reason {
    if f.preview {
        Reason::Preview
    } else if f.caption() {
        Reason::Captions
    } else if f.aside() {
        Reason::Aside
    } else if f.class.is_none() {
        Reason::Unknown
    } else {
        Reason::Unplaced
    }
}

/// Items side by side in one folder, when most of their names start with a number (`numbered`):
/// in a grid each keeps it, as Works do; a list numbers its rows itself, so each loses it, a
/// bare "10 " among "01 " and "02 " too.
fn keep_numbers(items: &mut [Item], grid: bool) {
    let dir_of = |i: &Item| i.rel.rsplit_once('/').map_or(String::new(), |(a, _)| a.to_string());
    let mut by: BTreeMap<String, Vec<&str>> = BTreeMap::new();
    for i in items.iter().filter(|i| matches!(i.role, Role::Main | Role::Doc)) {
        by.entry(dir_of(i)).or_default().push(&i.name);
    }
    let kept: HashSet<String> = by.into_iter().filter(|(_, names)| numbered(names.iter().copied(), true)).map(|(d, _)| d).collect();
    for i in items.iter_mut().filter(|i| matches!(i.role, Role::Main | Role::Doc) && kept.contains(&dir_of(i))) {
        let t = clean_title_in(&i.name, true, grid).0;
        let digits = t.chars().take_while(|c| c.is_ascii_digit()).count();
        let rest = t[digits..].trim_start();
        i.title = if !grid && (1..=3).contains(&digits) && t[digits..].starts_with(' ') && !rest.is_empty() { rest.to_string() } else { t };
    }
}

fn item(f: &F, rel: String, role: Role, group: String) -> Item {
    Item {
        name: f.name.clone(),
        key: rel.clone(),
        title: clean_title(&f.name, true).0,
        rel,
        path: f.path.to_string_lossy().into(),
        ext: f.ext.clone(),
        size: f.size,
        mtime: f.mtime,
        cloud: f.cloud,
        group,
        role,
        lang: String::new(),
        at: None,
    }
}

#[allow(clippy::too_many_arguments)]
fn make_work(ctx: &mut Ctx, rel: &str, name: &str, strip_ext: bool, keep: bool, path: String, folder: &[String], is_folder: bool, items: Vec<Item>) {
    let mut media: Vec<String> = vec![];
    for i in items.iter().filter(|i| matches!(i.role, Role::Main | Role::Doc)) {
        if let Some(m) = class_of(&i.ext).and_then(medium) && !media.iter().any(|x| x == m) {
            media.push(m.into());
        }
    }
    // A Work's first Medium is what most of its items are: a book with its audio reads first.
    let main: Vec<&str> = items.iter().filter(|i| i.role == Role::Main).filter_map(|i| class_of(&i.ext).and_then(medium)).collect();
    media.sort_by_key(|m| std::cmp::Reverse(main.iter().filter(|x| *x == m).count()));
    let (title, year) = clean_title_in(name, strip_ext, keep);
    let title = part_title(title, folder, is_folder);
    ctx.out.push(Work {
        id: crate::store::key_in(&ctx.id_base, rel),
        title,
        year,
        path,
        folder: folder.to_vec(),
        is_folder,
        size: items.iter().map(|i| i.size).sum(),
        added: items.iter().map(|i| i.mtime).max().unwrap_or(0),
        cloud: items.iter().filter(|i| i.cloud).count(),
        items,
        media,
        taken: None,
        event: false,
        day_only: false,
    });
}

fn join(rel: &str, name: &str) -> String {
    if rel.is_empty() { name.to_string() } else { format!("{rel}/{name}") }
}

fn priority(ext: &str) -> usize {
    PRIORITY.iter().position(|p| *p == ext).unwrap_or(PRIORITY.len())
}

/// Files with the same name in one folder are one Work, whatever their formats; a book opens in
/// its best format first. Captions join the video of their name. `rel` is the folder's path.
fn grouped(ctx: &mut Ctx, files: Vec<(&F, String)>, rel: &str, folder: &[String]) {
    let mut groups: BTreeMap<String, Vec<(&F, String)>> = BTreeMap::new();
    let mut caps = vec![];
    for (f, r) in files {
        if f.caption() {
            caps.push((f, r));
        } else {
            groups.entry(stem_key(&f.name)).or_default().push((f, r));
        }
    }
    let mut ordered: Vec<Vec<(&F, String)>> = groups.into_values().collect();
    ordered.sort_by(|a, b| natural(&a[0].0.name, &b[0].0.name));
    let keep = numbered(ordered.iter().map(|fs| fs[0].0.name.as_str()), true);
    let mut taken = HashSet::new();
    for mut fs in ordered {
        fs.sort_by_key(|(f, _)| (!f.media(), priority(&f.ext)));
        let first = fs[0].0;
        let base = first.name.rsplit_once('.').map(|(a, _)| a).unwrap_or(&first.name).to_string();
        let mut items: Vec<Item> = fs.iter().map(|(f, r)| item(f, r.clone(), Role::Main, String::new())).collect();
        for i in items.iter_mut().filter(|_| keep) {
            i.title = clean_title_in(&i.name, true, true).0;
        }
        if first.class == Some(Class::Video) {
            let want = caption_key(&first.name).0;
            for (c, r) in caps.iter().filter(|(c, _)| caption_key(&c.name).0 == want) {
                let lang = caption_key(&c.name).1;
                taken.insert(r.clone());
                items.push(Item { lang, key: items[0].key.clone(), ..item(c, r.clone(), Role::Cc, String::new()) });
            }
        }
        make_work(ctx, &join(rel, &base), &first.name, true, keep, first.path.to_string_lossy().into(), folder, false, items);
    }
    for (c, r) in caps {
        if !taken.contains(&r) {
            ctx.unlist(c, &join(rel, &r), folder, Reason::Captions, None);
        }
    }
}

fn all_files<'a>(d: &'a Dir, rel: &str, group: &str, out: &mut Vec<(&'a F, String, String)>) {
    for f in &d.files {
        out.push((f, join(rel, &f.name), group.to_string()));
    }
    // Headings keep their numbers as folders side by side do (`numbered`).
    let keep = numbered(d.dirs.iter().filter(|c| !c.program && c.total() > 0).map(|c| c.name.as_str()), false);
    for c in &d.dirs {
        let t = title_in(&c.name, false, keep);
        let g = if group.is_empty() { t } else { format!("{group} / {t}") };
        all_files(c, &join(rel, &c.name), &g, out);
    }
}

/// A Tick key without the extension: `Part 1/01 Intro.mp4` is `Part 1/01 Intro`.
fn lesson_key(rel: &str) -> String {
    match rel.rsplit_once('.') {
        Some((a, _)) if !a.ends_with('/') && !a.is_empty() && !rel[a.len()..].contains('/') => a.to_string(),
        _ => rel.to_string(),
    }
}

/// (name to match, language) for a caption or video: case, "-", "_" and a trailing video
/// extension ("lesson.mp4.srt") aside, and a language ("-EN", ".es") set apart.
fn caption_key(name: &str) -> (String, String) {
    let mut s = name.rsplit_once('.').map(|(a, _)| a).unwrap_or(name).to_string();
    if let Some((a, e)) = s.rsplit_once('.') && class_of(&e.to_lowercase()) == Some(Class::Video) {
        s = a.to_string();
    }
    let mut lang = String::new();
    let lower = s.to_lowercase();
    for l in ["english", "eng", "spa", "rus", "heb", "en", "es", "ru", "he", "de", "fr"] {
        if let Some(head) = lower.strip_suffix(l) && head.ends_with(['.', '_', ' ', '-']) {
            lang = l[..2].to_uppercase();
            s.truncate(head.trim_end_matches(['.', '_', ' ', '-']).len());
            break;
        }
    }
    let key = s.to_lowercase().split(|c: char| c.is_whitespace() || matches!(c, '.' | '_' | '-')).filter(|w| !w.is_empty()).collect::<Vec<_>>().join(" ");
    (key, lang)
}

fn lesson_number(name: &str) -> Option<u32> {
    let digits: String = name.trim_start().chars().take_while(|c| c.is_ascii_digit()).collect();
    let rest = name.trim_start()[digits.len()..].chars().next();
    (!digits.is_empty() && rest.is_some_and(|c| c.is_whitespace() || matches!(c, '.' | '_' | '-'))).then(|| digits.parse().ok()).flatten()
}

/// The folder of a Work: its files at any depth, with subfolders as groups. In a course or an
/// album the lessons or tracks take Ticks and the rest are its files; in a collection every
/// title does. Captions join their lesson; code and config are its code files; what it cannot
/// show is unlisted, in its dialog.
fn folder_work(ctx: &mut Ctx, d: &Dir, path: &Path, rel: &str, folder: &[String]) {
    let id = format!("{}/", crate::store::key_in(&ctx.id_base, rel));
    let mut fs = vec![];
    all_files(d, "", "", &mut fs);
    let has_media = fs.iter().any(|(f, _, _)| f.media());
    let kind = ctx.kind;
    let courses = kind == Kind::Courses;
    let main_role = |f: &F| -> Option<Role> {
        let c = f.class?;
        Some(if kind == Kind::Collections || !has_media || matches!(c, Class::Video | Class::Audio) { Role::Main } else { Role::Doc })
    };
    // A course's items are lessons; an audiobook's tracks and a collection's files keep their extension.
    let lessons = courses && fs.iter().any(|(f, _, _)| f.class == Some(Class::Video));
    let key_of = |r: &str| if lessons { lesson_key(r) } else { r.to_string() };
    let mut items = vec![];
    let mut caps = vec![];
    for (f, r, g) in &fs {
        if let Some(role) = main_role(f) {
            items.push(Item { key: key_of(r), ..item(f, r.clone(), role, g.clone()) });
        } else if courses && EXTRA.contains(&f.ext.as_str()) {
            items.push(item(f, r.clone(), Role::Doc, g.clone()));
        } else if f.caption() {
            caps.push((*f, r.clone(), g.clone()));
        } else if f.aside() {
            items.push(item(f, r.clone(), Role::Code, g.clone()));
        } else {
            ctx.unlist(f, &join(rel, r), folder, reason_of(f), Some(&id));
        }
    }
    // Captions: by name in their folder or the folder above a captions folder, else by a lesson
    // number only one video in that folder has.
    for (c, r, g) in caps {
        let dir_of = |x: &str| x.rsplit_once('/').map(|(a, _)| a.to_string()).unwrap_or_default();
        let mut home = dir_of(&r);
        if CAPTION_DIRS.contains(&home.rsplit('/').next().unwrap_or("").to_lowercase().as_str()) {
            home = dir_of(&home);
        }
        let videos: Vec<&Item> = items.iter().filter(|i| class_of(&i.ext) == Some(Class::Video) && dir_of(&i.rel) == home).collect();
        let (want, lang) = caption_key(&c.name);
        let found = videos.iter().find(|v| caption_key(&v.name).0 == want).or_else(|| {
            let n = lesson_number(&c.name)?;
            let same: Vec<&&Item> = videos.iter().filter(|v| lesson_number(&v.name) == Some(n)).collect();
            (same.len() == 1).then(|| same[0])
        });
        match found.map(|v| v.key.clone()) {
            Some(key) => items.push(Item { key, lang, ..item(c, r, Role::Cc, g) }),
            None => ctx.unlist(c, &join(rel, &r), folder, Reason::Captions, Some(&id)),
        }
    }
    let keep = ctx.keep;
    make_work(ctx, &format!("{rel}/"), &d.name, false, keep, path.to_string_lossy().into(), folder, true, items);
    // A grid of titles shows their numbers; a list numbers its rows itself (a course's lessons, tracks).
    let w = ctx.out.last_mut().unwrap();
    let grid = crate::thumbs::grid(w);
    keep_numbers(&mut w.items, grid);
}

/// What a folder with files of its own and subfolders is in a collection (`decisions/2026/09/30/a-folder-is-a-work-or-a-place-never-both.md`): one Work
/// when every subfolder holds a part of it (named as one, or holding only audio), else a Shelf.
fn work_with_parts(d: &Dir) -> bool {
    d.dirs.iter().all(|c| part_name(&c.name) || audio_only(c))
}

fn walk(ctx: &mut Ctx, d: &Dir, path: &Path, rel: &str, folder: &mut Vec<String>, root: bool) {
    if d.program {
        return ctx.unlist_all(d, rel, folder, Some(Reason::Program));
    }
    // A folder with a Folder kind is read as that kind from here down, as its Section's folder
    // would be: it stays a folder, never one Work (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`).
    if !rel.is_empty() && let Some(k) = ctx.kinds.get(&crate::store::case(rel)).copied() {
        if k == Kind::Photos {
            return photo_folder(ctx, d, path, rel, folder);
        }
        let was = std::mem::replace(&mut ctx.kind, k);
        walk_as(ctx, d, path, rel, folder, true);
        ctx.kind = was;
        return;
    }
    walk_as(ctx, d, path, rel, folder, root);
}

/// A folder with the Folder kind Photos (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`): its Events from here down, made as a Photos
/// Section's with this folder in place of the Section's folder. Their ids start with its path, so
/// an Event's id is the one it has if this folder is ever a Photos Section of its own; their
/// folders start with its titles; its own `Not needed` holds what Review's move set aside. A
/// Folder kind set below it is not used: a Photos folder shows no folders, as a Photos Section.
fn photo_folder(ctx: &mut Ctx, d: &Dir, path: &Path, rel: &str, folder: &[String]) {
    let mut kept_nowhere = crate::photos::Dates::load(None);
    let dates = ctx.dates.as_deref_mut().unwrap_or(&mut kept_nowhere);
    let (mut works, mut unlisted) = (vec![], vec![]);
    events_or_stop(&crate::store::key_in(&ctx.id_base, rel), d, path, dates, ctx.gap, &mut works, &mut unlisted, &HashSet::new(), ctx.stop);
    for mut w in works {
        w.folder.splice(0..0, folder.iter().cloned());
        ctx.out.push(w);
    }
    for mut u in unlisted {
        u.rel = join(rel, &u.rel);
        u.folder.splice(0..0, folder.iter().cloned());
        ctx.unlisted.push(u);
    }
}

/// The part of a Work's id that is a folder: an Event's id is its Photos folder's, `#`, and its
/// first Photo's path in that folder without the extension (`events`), so that path is cut off
/// the end, never at the first `#`, which a folder's name may hold. It is the Photos folder
/// Review covers. Any other Work's id is its own.
pub fn place_of(w: &Work) -> &str {
    let Some(first) = w.items.first().filter(|_| w.event) else { return &w.id };
    let stem = first.rel.rsplit_once('.').map_or(first.rel.as_str(), |(s, _)| s);
    w.id.strip_suffix(stem).and_then(|x| x.strip_suffix('#')).unwrap_or(&w.id)
}

/// `walk` for a folder read as `ctx.kind`. `root`: the Section's folder, or one with a Folder kind.
fn walk_as(ctx: &mut Ctx, d: &Dir, path: &Path, rel: &str, folder: &mut Vec<String>, root: bool) {
    let root = root || ctx.holds_kind(rel);
    // Subfolders with a Folder kind, or holding one: always walked as folders of their own.
    let kept: HashSet<&str> = d.dirs.iter().filter(|c| ctx.holds_kind(&join(rel, &c.name))).map(|c| c.name.as_str()).collect();
    let format_dir = |c: &Dir| FORMAT_DIRS.contains(&c.name.to_lowercase().as_str()) && !kept.contains(c.name.as_str());
    let parent = |folder: &Vec<String>| folder[..folder.len().saturating_sub(1)].to_vec();
    let own: Vec<&F> = d.files.iter().filter(|f| f.class.is_some()).collect();
    let mut loose: Vec<(&F, String)> = vec![];
    match ctx.kind {
        Kind::Files => loose = d.files.iter().filter(|f| f.class.is_some() || f.caption()).map(|f| (f, f.name.clone())).collect(),
        Kind::Books => {
            let audio = d.files.iter().filter(|f| f.class == Some(Class::Audio)).count();
            if !root && audio >= 3 && audio * 2 >= own.len() {
                return folder_work(ctx, d, path, rel, &parent(folder)); // an audiobook
            }
            loose = d.files.iter().filter(|f| f.class.is_some()).map(|f| (f, f.name.clone())).collect();
            for c in d.dirs.iter().filter(|c| format_dir(c)) {
                for f in &c.files {
                    let r = format!("{}/{}", c.name, f.name);
                    if f.class.is_some() {
                        loose.push((f, r));
                    } else {
                        ctx.unlist(f, &join(rel, &r), folder, reason_of(f), None);
                    }
                }
                // Folders inside a format folder hold no formats of a book here: their files are unlisted.
                for sub in &c.dirs {
                    ctx.unlist_all(sub, &join(rel, &format!("{}/{}", c.name, sub.name)), folder, None);
                }
            }
        }
        Kind::Courses => {
            if !root && is_course(d) && !holds_courses(d) {
                return folder_work(ctx, d, path, rel, &parent(folder));
            }
            loose = d.files.iter().filter(|f| f.class.is_some() || f.caption()).map(|f| (f, f.name.clone())).collect();
        }
        Kind::Photos => unreachable!("a Photos Section is read by events()"),
        Kind::Collections => {
            if !root && !own.is_empty() && work_with_parts(d) {
                return folder_work(ctx, d, path, rel, &parent(folder));
            }
            loose = d.files.iter().filter(|f| f.class.is_some()).map(|f| (f, f.name.clone())).collect();
        }
    }
    let placed: HashSet<&str> = loose.iter().map(|(f, _)| f.name.as_str()).collect();
    for f in d.files.iter().filter(|f| !placed.contains(f.name.as_str())) {
        ctx.unlist(f, &join(rel, &f.name), folder, reason_of(f), None);
    }
    let before = ctx.out.len();
    grouped(ctx, loose, rel, folder);
    // A Code or Files folder beside the one Work made here holds that Work's code files.
    let extra = |c: &Dir| EXTRA_DIRS.contains(&c.name.to_lowercase().as_str()) && !c.has_media() && !kept.contains(c.name.as_str());
    if ctx.out.len() == before + 1 {
        for c in d.dirs.iter().filter(|c| extra(c)) {
            let mut fs = vec![];
            all_files(c, &c.name, "", &mut fs);
            let w = ctx.out.last_mut().unwrap();
            w.items.extend(fs.iter().map(|(f, r, _)| item(f, r.clone(), Role::Code, String::new())));
            w.size += fs.iter().map(|(f, _, _)| f.size).sum::<u64>();
        }
    }
    // The folders shown beside each other here: their numbers give their order or are cut.
    let one = ctx.out.len() == before + 1;
    // Program folders and empty ones show nowhere, so they take no part.
    let keep = numbered(d.dirs.iter().filter(|c| !c.program && c.total() > 0 && !(ctx.kind == Kind::Books && format_dir(c)) && !(one && extra(c))).map(|c| c.name.as_str()), false);
    for c in &d.dirs {
        if ctx.kind == Kind::Books && format_dir(c) {
            continue;
        }
        if ctx.out.len() == before + 1 && extra(c) {
            continue;
        }
        folder.push(title_in(&c.name, false, keep));
        ctx.keep = keep;
        walk(ctx, c, &path.join(&c.name), &join(rel, &c.name), folder, false);
        folder.pop();
    }
}

/// Reads one checked folder into a Section. `None` when its Drive is not connected.
/// `gap` is the Photos Event gap in seconds; `kinds` are its Folder kinds (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`).
pub fn scan_folder(cf: &CheckedFolder, drives: &[Drive], cache: Option<&Path>, gap: i64, kinds: &Kinds, progress: &dyn Fn(usize)) -> Option<Section> {
    let drive = drives.iter().find(|d| d.serial == cf.serial)?;
    let path = drives::join_on_drive(drive, &cf.rel);
    if !crate::store::named(&path).is_dir() {
        return None;
    }
    let unchecked: HashSet<String> = cf.unchecked.iter().cloned().collect();
    let mut seen = 0;
    let tree = read_tree(&path, "", &unchecked, &mut seen, progress);
    // A drive unplugged during the read leaves folders it could not read looking empty: what was
    // read last time stays instead.
    if !crate::store::named(&path).is_dir() {
        return None;
    }
    let guessed = cf.kind.is_none();
    let kind = cf.kind.unwrap_or_else(|| guess(&tree));
    let (mut works, mut unlisted, mut empty) = (vec![], vec![], vec![]);
    empty_dirs(&tree, "", &mut empty);
    if kind == Kind::Photos {
        let mut dates = crate::photos::Dates::load(cache);
        photos_section(&cf.id(), &tree, &path, &mut dates, gap, kinds, &mut works, &mut unlisted, &never);
        dates.save();
    } else {
        let mut dates = crate::photos::Dates::load(cache.filter(|_| kinds.values().any(|k| *k == Kind::Photos)));
        let mut ctx = Ctx::new(cf.id(), kind, kinds, &mut works, &mut unlisted);
        ctx.dates = Some(&mut dates);
        ctx.gap = gap;
        walk(&mut ctx, &tree, &path, "", &mut vec![], true);
        dates.save();
        sort_works(&mut works);
    }
    Some(Section {
        id: cf.id(),
        name: cf.shown_name(),
        path: path.to_string_lossy().into(),
        drive: drive_name(drive),
        kind,
        guessed,
        online: true,
        gone: false,
        files: tree.total(),
        works,
        scanned_at: crate::store::now(),
        unlisted,
        empty,
        whole_tree: true,
        split_from: None,
        links: vec![],
    })
}

/// A checked folder `scan_folder` could not read: its Drive is not connected, or its folder is gone
/// from a connected one (`gone`), a Split-out Section's too (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`). It stays listed, with what
/// was read `last` time if anything, so the person sees it and can remove it or tick its new place.
pub fn not_read(cf: &CheckedFolder, drives: &[Drive], last: Option<&Section>) -> Section {
    let gone = drives.iter().any(|d| d.serial == cf.serial);
    let mut sec = last.cloned().unwrap_or_else(|| Section {
        id: cf.id(),
        name: cf.shown_name(),
        path: cf.last_path.clone(),
        drive: drives.iter().find(|d| d.serial == cf.serial).map(drive_name).or_else(|| cf.drive.clone()).unwrap_or_else(|| cf.last_path.split('\\').next().unwrap_or_default().to_string()),
        kind: cf.kind.unwrap_or(Kind::Files),
        guessed: cf.kind.is_none(),
        online: false,
        gone,
        works: vec![],
        files: 0,
        scanned_at: 0,
        unlisted: vec![],
        empty: vec![],
        whole_tree: false,
        split_from: None,
        links: vec![],
    });
    sec.online = false;
    sec.gone = gone;
    sec
}

/// Marks as not connected each Section read last time whose Drive is not `present` now, keeping
/// what was read, and returns their ids. A Section on a network drive whose letter is still
/// there stays, as its serial is not asked. Nothing is read; a Drive back stays not connected until
/// its folders are read again.
pub fn mark_away(cat: &mut Catalog, folders: &[CheckedFolder], present: &drives::Present) -> Vec<String> {
    let mut away = vec![];
    for sec in cat.sections.iter_mut().filter(|s| s.online) {
        let mapped = present.network.iter().any(|r| sec.path.get(..r.len()).is_some_and(|p| p.eq_ignore_ascii_case(r)));
        if !mapped && folders.iter().any(|f| f.id() == sec.id && !present.serials.contains(&f.serial)) {
            sec.online = false;
            away.push(sec.id.clone());
        }
    }
    away
}

/// A Photos Section's Works: Events of its pictures and clips from every subfolder, in the order
/// they were taken, then one Undated Event (`decisions/2026/10/04/an-event-is-the-work-in-photos.md`, `decisions/2026/10/08/photos-is-a-section-kind-in-the-app.md`). Any other file is unlisted, and so
/// is every file in the `Not needed` folder the Review move scripts fill (`decisions/2026/10/04/sorting-ends-in-a-move-script-never-a-delete.md`).
#[cfg(test)]
fn events(id_base: &str, tree: &Dir, root: &Path, dates: &mut crate::photos::Dates, gap: i64, out: &mut Vec<Work>, unlisted: &mut Vec<Unlisted>) {
    events_or_stop(id_base, tree, root, dates, gap, out, unlisted, &HashSet::new(), &|| false);
}

/// The Folder kind of a Photos Section's top-level folder `name` that takes it out of the Events:
/// any kind but Photos, which is the Section's own (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`).
pub fn other_kind(kinds: &Kinds, name: &str) -> Option<Kind> {
    kinds.get(&crate::store::case(name)).copied().filter(|k| *k != Kind::Photos)
}

/// A Photos Section's Works (`decisions/2026/10/08/photos-is-a-section-kind-in-the-app.md`, `decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`): the Events of its pictures and clips, then the Works
/// of each top-level folder with another Folder kind, read as that kind from there down as in a
/// folder Section, with the same ids. Their pictures are in no Event. A Folder kind deeper than
/// those folders is not used, as a Photos Section shows no folders.
#[allow(clippy::too_many_arguments)]
fn photos_section(id: &str, tree: &Dir, root: &Path, dates: &mut crate::photos::Dates, gap: i64, kinds: &Kinds, out: &mut Vec<Work>, unlisted: &mut Vec<Unlisted>, stop: &dyn Fn() -> bool) {
    let others: Vec<&Dir> = tree.dirs.iter().filter(|c| other_kind(kinds, &c.name).is_some()).collect();
    let skip: HashSet<String> = others.iter().map(|c| crate::store::case(&c.name)).collect();
    events_or_stop(id, tree, root, dates, gap, out, unlisted, &skip, stop);
    if others.is_empty() || stop() {
        return;
    }
    let mut ctx = Ctx::new(id.to_string(), Kind::Photos, kinds, out, unlisted);
    ctx.dates = Some(dates);
    ctx.gap = gap;
    ctx.stop = stop;
    for c in others {
        walk(&mut ctx, c, &root.join(&c.name), &c.name, &mut vec![title_of(&c.name, false)], false);
    }
    sort_works(out);
}

/// `events`, asking `stop` before each picture's date is read; once it says yes, it ends with
/// what it has, which the caller throws away. The top-level folders in `skip` (as `store::case`
/// gives their names) are left out.
#[allow(clippy::too_many_arguments)]
fn events_or_stop(id_base: &str, tree: &Dir, root: &Path, dates: &mut crate::photos::Dates, gap: i64, out: &mut Vec<Work>, unlisted: &mut Vec<Unlisted>, skip: &HashSet<String>, stop: &dyn Fn() -> bool) {
    use crate::photos;
    let mut files = vec![];
    all_files(tree, "", "", &mut files);
    files.retain(|(_, rel, _)| rel.split_once('/').is_none_or(|(top, _)| !skip.contains(&crate::store::case(top))));
    let mut shots: Vec<(Option<i64>, &F, String)> = vec![];
    let aside = format!("{}/", crate::shots::NOT_NEEDED).to_lowercase();
    for (f, rel, _) in files {
        if stop() {
            return;
        }
        let set_aside = rel.to_lowercase().starts_with(&aside);
        match f.class {
            Some(c @ (Class::Image | Class::Video)) if !set_aside => shots.push((dates.taken(&f.name, &f.path, f.size, f.mtime, f.cloud, c == Class::Image), f, rel)),
            _ => {
                let folder: Vec<String> = rel.split('/').rev().skip(1).collect::<Vec<_>>().into_iter().rev().map(|n| title_of(n, false)).collect();
                let reason = if set_aside { Reason::SetAside } else { reason_of(f) };
                unlisted.push(Unlisted { name: f.name.clone(), rel: rel.clone(), path: f.path.to_string_lossy().into(), folder, reason, size: f.size, mtime: f.mtime, cloud: f.cloud, work: None });
            }
        }
    }
    shots.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| natural(&a.2, &b.2)));
    let undated = shots.iter().take_while(|s| s.0.is_none()).count();
    let dated = &shots[undated..];
    let times: Vec<i64> = dated.iter().map(|s| s.0.unwrap_or_default()).collect();
    let make = |run: &[(Option<i64>, &F, String)], folder: Vec<String>, title: String, taken: Option<(i64, i64)>| {
        let first = &run[0];
        let dir = first.2.rsplit_once('/').map_or(root.to_path_buf(), |(d, _)| d.split('/').fold(root.to_path_buf(), |p, x| p.join(x)));
        let items: Vec<Item> = run.iter().map(|(t, f, rel)| Item { at: *t, ..item(f, rel.clone(), Role::Main, String::new()) }).collect();
        let stem = first.2.rsplit_once('.').map_or(first.2.as_str(), |(s, _)| s);
        Work {
            id: format!("{id_base}#{stem}"),
            title,
            year: None,
            path: dir.to_string_lossy().into(),
            folder,
            is_folder: true,
            size: items.iter().map(|i| i.size).sum(),
            added: items.iter().map(|i| i.mtime).max().unwrap_or(0),
            cloud: items.iter().filter(|i| i.cloud).count(),
            items,
            media: vec![],
            taken,
            event: true,
            day_only: false,
        }
    };
    for r in photos::runs(&times, gap) {
        let (a, b) = (times[r.start], times[r.end - 1]);
        out.push(make(&dated[r], photos::folder(a), photos::title(a, b), Some((a, b))));
    }
    // A name that gives only the day (WhatsApp's IMG-20180212-WA0001, which keeps no EXIF) puts
    // the Photo in an Event of its own for that day, with no time: a time made up would join it to
    // an Event it was not part of.
    let mut by_day = std::collections::BTreeMap::<i64, Vec<_>>::new();
    let mut still = vec![];
    for (_, f, rel) in &shots[..undated] {
        match photos::name_day(&f.name) {
            Some(d) => by_day.entry(d).or_default().push((Some(d), *f, rel.clone())),
            None => still.push((None, *f, rel.clone())),
        }
    }
    for (d, run) in by_day {
        out.push(Work { day_only: true, ..make(&run, photos::folder(d), photos::title(d, d), Some((d, d))) });
    }
    if !still.is_empty() {
        out.push(make(&still, vec!["Undated".into()], "Undated".into(), None));
    }
}

/// A Drive as the library column heads its Sections: "E: Data", or "E:\\" with no label.
pub fn drive_name(drive: &Drive) -> String {
    // A Linux drive is named by its label, else its mount point.
    if cfg!(target_os = "linux") {
        return if drive.label.is_empty() { drive.path.clone() } else { drive.label.clone() };
    }
    if drive.label.is_empty() { drive.path.clone() } else { format!("{} {}", drive.path.trim_end_matches('\\'), drive.label) }
}

/// A whole drive's Section name, from the name `drive_name` gives its Drive: the label,
/// else the letter (`D:`) or mount point.
pub fn drive_title(name: &str) -> String {
    if cfg!(target_os = "linux") {
        return name.to_string();
    }
    match name.split_once(' ') {
        Some((letter, label)) if letter.ends_with(':') && !label.is_empty() => label.to_string(),
        _ => name.trim_end_matches('\\').to_string(),
    }
}

/// Works by folder and title; Events, of a Photos folder (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`), keep the order they were
/// taken in, after the rest.
fn sort_works(works: &mut Vec<Work>) {
    let (mut events, mut rest): (Vec<Work>, Vec<Work>) = std::mem::take(works).into_iter().partition(|w| w.event);
    rest.sort_by(|a, b| {
        let f = a.folder.iter().zip(&b.folder).map(|(x, y)| natural(x, y)).find(|o| o.is_ne()).unwrap_or(a.folder.len().cmp(&b.folder.len()));
        f.then_with(|| natural(&a.title, &b.title))
    });
    rest.append(&mut events);
    *works = rest;
}

/// Reads one folder of a Section again, the one the folder column shows as `under` (folder
/// titles from the Section down), and puts its Works and unlisted files in place of the old ones.
/// Makes its Works as a whole read would, with the kind it has there (`kind_at`). `None` when the
/// Drive is not connected, no Work shows where the folder is on disk, or it is a year or month of
/// a folder with the Folder kind Photos; the caller then reads the whole Section. `cache` and `gap`
/// are as `scan_folder`'s.
#[allow(clippy::too_many_arguments)]
pub fn scan_under(cf: &CheckedFolder, drives: &[Drive], sec: &Section, under: &[String], kinds: &Kinds, cache: Option<&Path>, gap: i64, progress: &dyn Fn(usize)) -> Option<Section> {
    // Years and months are no folders on disk.
    if sec.kind == Kind::Photos {
        return None;
    }
    let drive = drives.iter().find(|d| d.serial == cf.serial)?;
    let root = drives::join_on_drive(drive, &cf.rel);
    // Each folder title is one folder on disk, so a Work under it holds its real name.
    let w = sec.works.iter().find(|w| w.folder.len() >= under.len() && w.folder[..under.len()] == *under)?;
    let rel: Vec<&str> = crate::store::under(place_of(w), &sec.id)?.split('/').take(under.len()).collect();
    if rel.len() < under.len() {
        return None;
    }
    // Titles drop numbers, so "01 Books" and "Books" are one folder on screen: with more than one
    // folder on disk under these titles, only a whole read reads them all.
    let joined = rel.join("/");
    let other = sec.works.iter().filter(|w| w.folder.len() >= under.len() && w.folder[..under.len()] == *under).any(|w| {
        crate::store::under(place_of(w), &sec.id).is_some_and(|r| r.split('/').take(under.len()).collect::<Vec<_>>().join("/") != joined)
    });
    if other {
        return None;
    }
    let rel = rel.join("/");
    let path = rel.split('/').fold(root.clone(), |p, x| p.join(x));
    let prefix = format!("{}/", crate::store::key_in(&sec.id, &rel));
    // Its Works, and its Events when it is a Photos folder.
    let me = crate::store::key_in(&sec.id, &rel);
    let inside = |w: &Work| w.id.starts_with(&prefix) || place_of(w) == me;
    // The folder above may take this folder's files into its own Works (a book's format folder, a
    // Work's Code folder), or start to once a Folder kind is gone: only a whole read is exact then.
    let name = rel.rsplit('/').next().unwrap_or_default().to_lowercase();
    let taken = sec.works.iter().filter(|w| !inside(w)).flat_map(|w| &w.items).any(|i| Path::new(&i.path).starts_with(&path));
    if taken || FORMAT_DIRS.contains(&name.as_str()) || EXTRA_DIRS.contains(&name.as_str()) {
        return None;
    }
    let mut out = sec.clone();
    let old: Vec<Work> = out.works.iter().filter(|w| inside(w)).cloned().collect();
    out.works.retain(|w| !inside(w));
    let gone_unlisted = out.unlisted.len();
    out.unlisted.retain(|u| !u.rel.starts_with(&format!("{rel}/")));
    let gone_unlisted = gone_unlisted - out.unlisted.len();
    let (mut works, mut unlisted) = (vec![], vec![]);
    out.empty.retain(|e| *e != rel && !e.starts_with(&format!("{rel}/")));
    if crate::store::named(&path).is_dir() {
        let unchecked: HashSet<String> = cf.unchecked.iter().cloned().collect();
        let mut seen = 0;
        let tree = read_tree(&path, &rel, &unchecked, &mut seen, progress);
        // Gone during the read, its drive perhaps unplugged: the whole Section is read instead.
        if !crate::store::named(&path).is_dir() {
            return None;
        }
        // The kind above it: `walk` takes the folder's own, if it has one.
        let above = rel.rsplit_once('/').map_or("", |(up, _)| up);
        let kind = if above.is_empty() { out.kind } else { kind_at(kinds, out.kind, above) };
        // Inside a Photos folder no folder is shown: its folder is read again.
        if kind == Kind::Photos {
            return None;
        }
        let mut dates = crate::photos::Dates::load(cache.filter(|_| kinds.values().any(|k| *k == Kind::Photos)));
        let mut ctx = Ctx::new(cf.id(), kind, kinds, &mut works, &mut unlisted);
        ctx.dates = Some(&mut dates);
        ctx.gap = gap;
        // The folder keeps its number in its title when the folders beside it did.
        ctx.keep = under.last().is_some_and(|t| *t != title_of(rel.rsplit('/').next().unwrap_or_default(), false));
        walk(&mut ctx, &tree, &path, &rel, &mut under.to_vec(), false);
        dates.save();
        if tree.total() == 0 {
            out.empty.push(rel.clone());
        }
        empty_dirs(&tree, &rel, &mut out.empty);
    }
    let count = |ws: &[Work]| ws.iter().map(|w| w.items.len()).sum::<usize>();
    out.files = (out.files + count(&works) + unlisted.len()).saturating_sub(count(&old) + gone_unlisted);
    out.works.extend(works);
    out.unlisted.extend(unlisted);
    // A folder above it that lost its last file is empty now.
    let parts: Vec<&str> = rel.split('/').collect();
    for n in 1..parts.len() {
        let up = parts[..n].join("/");
        let at = up.split('/').fold(root.clone(), |p, x| p.join(x));
        let holds = |p: &str| Path::new(p).starts_with(&at);
        if crate::store::named(&at).is_dir() && !out.empty.contains(&up) && !out.works.iter().flat_map(|w| &w.items).any(|i| holds(&i.path)) && !out.unlisted.iter().any(|u| holds(&u.path)) {
            out.empty.push(up);
        }
    }
    sort_works(&mut out.works);
    out.online = true;
    Some(out)
}

/// Sections the compact catalog is tested on (packed.rs): a sample tree read by each kind, with
/// its paths written as on Windows (`D:\Lib`), on macOS and Linux (`/media/me/Lib`) and as a whole
/// drive (`Q:\`), and every date 1, so the sample is the same on every run and system.
#[cfg(test)]
pub fn samples_for_packing() -> Vec<Section> {
    static RUN: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!("fvh-packing-{}-{}", std::process::id(), RUN.fetch_add(1, std::sync::atomic::Ordering::SeqCst)));
    let _ = std::fs::remove_dir_all(&dir);
    for f in [
        "Course A/01 Intro.mp4", "Course A/01 Intro.en.srt", "Course A/02 Next Step.mp4", "Course A/Handouts/01 Intro.pdf", "Course A/code/run.py",
        "Course A/Part 2/03 Deep.mp4", "Author Б/Book One.pdf", "Author Б/Book One.epub", "Author Б/Series 1/Книга 😀 (2001).fb2",
        "Author Б/Book Two/pdf/Book Two.pdf", "Audio/Talk/01 a.mp3", "Audio/Talk/02 b.mp3", "Audio/Talk/03 c.mp3", "Loose.Name.With.Dots.PDF", "noext",
        "Photos/IMG_20200101_120000.jpg", "Photos/IMG_20200101_120500.jpg", "Photos/2021/IMG_20210305_090000.jpg", "Photos/IMG-20180212-WA0001.jpg",
        "Photos/cat.jpg", "Collection X/Part 1/a.pdf", "Collection X/Part 2/b.pdf", "readme.zzz", "[2015] Year Book.pdf",
    ] {
        let p = dir.join(f);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, "x").unwrap();
    }
    std::fs::create_dir_all(dir.join("Empty")).unwrap();
    let tree = read_tree(&dir, "", &HashSet::new(), &mut 0, &|_| {});
    let at = dir.to_string_lossy().to_string();
    let to = |p: &mut String, from: &str, into: &str, slash: Option<(char, char)>| {
        if let Some(rest) = p.strip_prefix(from) {
            let rest = slash.map_or(rest.to_string(), |(a, b)| rest.replace(a, &b.to_string()));
            *p = format!("{into}{rest}");
        }
    };
    let paths = |s: &mut Section, from: &str, into: &str, slash: Option<(char, char)>| {
        to(&mut s.path, from, into, slash);
        for w in &mut s.works {
            to(&mut w.path, from, into, slash);
            w.items.iter_mut().for_each(|i| to(&mut i.path, from, into, slash));
        }
        s.unlisted.iter_mut().for_each(|u| to(&mut u.path, from, into, slash));
    };
    let mut out = vec![];
    for kind in [Kind::Courses, Kind::Books, Kind::Collections, Kind::Files, Kind::Photos] {
        let (mut works, mut unlisted, mut empty) = (vec![], vec![], vec![]);
        empty_dirs(&tree, "", &mut empty);
        if kind == Kind::Photos {
            events("S", &tree, &dir, &mut crate::photos::Dates::load(None), 4 * 3600, &mut works, &mut unlisted);
        } else {
            let none = Kinds::new();
            let mut ctx = Ctx::new("S".into(), kind, &none, &mut works, &mut unlisted);
            walk(&mut ctx, &tree, &dir, "", &mut vec![], true);
            sort_works(&mut works);
        }
        for w in &mut works {
            w.items.iter_mut().for_each(|i| i.mtime = 1);
            w.added = 1;
        }
        unlisted.iter_mut().for_each(|u| u.mtime = 1);
        let mut sec = Section { id: "S".into(), name: "S".into(), path: at.clone(), drive: "D:\\".into(), kind, guessed: false, online: true, gone: false,
            files: tree.total(), works, scanned_at: 0, unlisted, empty, whole_tree: true, split_from: None, links: vec![] };
        paths(&mut sec, &at, "D:\\Lib", Some(('/', '\\')));
        let mut unix = sec.clone();
        paths(&mut unix, "D:\\Lib", "/media/me/Lib", Some(('\\', '/')));
        let mut drive = sec.clone();
        paths(&mut drive, "D:\\Lib\\", "Q:\\", None);
        drive.path = "Q:\\".into();
        let key = |k: &mut String| {
            if let Some(rest) = k.strip_prefix("S/").or_else(|| k.strip_prefix('S').filter(|r| r.starts_with('#') || r.is_empty())) {
                *k = format!("0A1B2C3D:{rest}");
            }
        };
        key(&mut drive.id);
        drive.works.iter_mut().for_each(|w| key(&mut w.id));
        drive.unlisted.iter_mut().filter_map(|u| u.work.as_mut()).for_each(key);
        out.extend([sec, unix, drive]);
    }
    let _ = std::fs::remove_dir_all(&dir);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// On Windows `Books.` beside `Books` is read as named: its own files and folders, with paths
    /// as written, never the `\\?\` start; the folder tree lists both, each as named.
    #[test]
    #[cfg(windows)]
    fn a_folder_ending_in_a_dot_is_read_as_named() {
        let n = |p: &Path| crate::store::named(p).into_owned();
        let base = std::env::temp_dir().join(format!("fvh-dot-scan-{}", std::process::id()));
        let (dot, sib) = (base.join("Books."), base.join("Books"));
        for (d, file, sub) in [(&dot, "Dot only.pdf", "Part."), (&sib, "Plain only.pdf", "Other")] {
            std::fs::create_dir_all(n(&d.join(sub))).unwrap();
            std::fs::write(n(&d.join(file)), file).unwrap();
            std::fs::write(n(&d.join(sub).join("x.epub")), sub).unwrap();
        }
        let tree = read_tree(&dot, "", &HashSet::new(), &mut 0, &|_| {});
        assert_eq!(tree.files.iter().map(|f| (f.name.as_str(), f.path.clone())).collect::<Vec<_>>(), [("Dot only.pdf", dot.join("Dot only.pdf"))]);
        assert_eq!(tree.dirs.iter().map(|d| d.name.as_str()).collect::<Vec<_>>(), ["Part."]);
        assert_eq!(tree.dirs[0].files[0].path, dot.join("Part.").join("x.epub"));
        let names = |p: &Path| crate::drives::folders(p).unwrap().into_iter().map(|f| (f.name, f.path)).collect::<Vec<_>>();
        let s = |p: PathBuf| p.to_string_lossy().into_owned();
        assert_eq!(names(&base), [("Books".into(), s(sib.clone())), ("Books.".into(), s(dot.clone()))]);
        assert_eq!(names(&dot), [("Part.".into(), s(dot.join("Part.")))]);
        // A photo read for Review is the named one.
        std::fs::write(n(&dot.join("a.jpg")), "dot").unwrap();
        std::fs::write(sib.join("a.jpg"), "sibling").unwrap();
        assert_eq!(crate::shots::read(&dot.join("a.jpg")).unwrap().sha, sha1_smol::Sha1::from("dot").digest().to_string());
        let _ = std::fs::remove_dir_all(n(&dot));
        std::fs::remove_dir_all(&base).unwrap();
    }

    fn photo(name: &str) -> F {
        F { name: name.into(), ext: "jpg".into(), class: Some(Class::Image), size: 1, mtime: 1, cloud: true, path: PathBuf::from(name), preview: false }
    }

    #[test]
    fn cloud_note_counts_the_files_left_in_the_cloud() {
        let work = |folder: &[&str], cloud: usize| Work { id: "S/w".into(), title: "w".into(), path: "D:\\S\\w".into(), folder: folder.iter().map(|f| f.to_string()).collect(),
            is_folder: true, items: vec![], media: vec![], size: 0, added: 0, cloud, year: None, taken: None, event: false, day_only: false };
        let loose = |folder: &[&str], cloud: bool| Unlisted { name: "x.bin".into(), rel: "x.bin".into(), path: "D:\\S\\x.bin".into(),
            folder: folder.iter().map(|f| f.to_string()).collect(), reason: Reason::Unknown, size: 1, mtime: 1, cloud, work: None };
        let mut sec = Section { id: "S".into(), name: "S".into(), path: "D:\\S".into(), drive: "D:".into(), kind: Kind::Books, guessed: false, online: true, gone: false,
            works: vec![work(&["A"], 2), work(&["B"], 0)], files: 4, scanned_at: 0, unlisted: vec![loose(&["B"], true), loose(&["B"], false)], empty: vec![], whole_tree: false, split_from: None, links: vec![] };
        assert_eq!(cloud_note(&sec, &[]), "; 3 files only in the cloud were not read or downloaded");
        assert_eq!(cloud_note(&sec, &["B".to_string()]), "; 1 file only in the cloud was not read or downloaded");
        sec.works[0].cloud = 0;
        sec.unlisted[0].cloud = false;
        assert_eq!(cloud_note(&sec, &[]), "");
    }

    #[test]
    fn an_insta360_preview_beside_its_video_is_unlisted() {
        assert_eq!(insta360_key("LRV_20210304_070809_01_003.lrv").as_deref(), Some("20210304_070809_003"));
        assert_eq!(insta360_key("VID_20210304_070809_00_003.insv").as_deref(), Some("20210304_070809_003"));
        assert_eq!(insta360_key("VID_20210304_070809_00_003_edited.insv"), None);
        assert_eq!(insta360_key("trip.lrv"), None);
        assert_eq!(insta360_key("PXL_20210304_070809_00_003.lrv"), None);
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../.scratch/insta360-pair-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for n in ["VID_20210304_070809_00_003.insv", "LRV_20210304_070809_01_003.lrv", "LRV_20210304_080000_01_004.lrv"] {
            std::fs::write(dir.join(n), b"x").unwrap();
        }
        let tree = read_tree(&dir, "", &HashSet::new(), &mut 0, &|_| {});
        let previews: Vec<&str> = tree.files.iter().filter(|f| f.preview).map(|f| f.name.as_str()).collect();
        assert_eq!(previews, ["LRV_20210304_070809_01_003.lrv"]);
        let (mut w, mut u) = (vec![], vec![]);
        events("S", &tree, &dir, &mut crate::photos::Dates::load(None), 4 * 3600, &mut w, &mut u);
        let shown: Vec<&str> = w.iter().flat_map(|w| &w.items).map(|i| i.name.as_str()).collect();
        assert_eq!(shown, ["VID_20210304_070809_00_003.insv", "LRV_20210304_080000_01_004.lrv"]);
        assert_eq!(u.iter().map(|u| (u.name.as_str(), u.reason)).collect::<Vec<_>>(), [("LRV_20210304_070809_01_003.lrv", Reason::Preview)]);
        let mut works = vec![];
        let none = Kinds::new();
        let mut ctx = Ctx::new("S".into(), Kind::Files, &none, &mut works, &mut u);
        walk(&mut ctx, &tree, &dir, "", &mut vec![], true);
        assert_eq!(works.len(), 2);
        assert_eq!(u.iter().filter(|u| u.reason == Reason::Preview).count(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Review's move takes an Insta360 preview with its video, and Undo puts both back; a preview
    /// whose video stays, stays too.
    #[test]
    fn a_preview_moves_with_its_video_and_comes_back() {
        let dir = std::env::temp_dir().join(format!("fvh-insta360-move-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("2025")).unwrap();
        let files = ["2025/VID_20210304_070809_00_003.insv", "2025/LRV_20210304_070809_01_003.lrv", "2025/VID_20251206_140000_00_004.insv", "2025/LRV_20251206_140000_01_004.lrv"];
        for f in files {
            std::fs::write(dir.join(f), f).unwrap();
        }
        let tree = read_tree(&dir, "", &HashSet::new(), &mut 0, &|_| {});
        let (mut works, mut unlisted) = (vec![], vec![]);
        events("S", &tree, &dir, &mut crate::photos::Dates::load(None), 4 * 3600, &mut works, &mut unlisted);
        let sec = Section { id: "S".into(), name: "S".into(), path: dir.to_string_lossy().into(), drive: "E:".into(), kind: Kind::Photos, guessed: false, online: true,
            gone: false, files: 4, works, scanned_at: 0, unlisted, empty: vec![], whole_tree: false, split_from: None, links: vec![] };
        let review = HashMap::from([("S/2025/VID_20210304_070809_00_003.insv".to_string(), crate::store::Choice { keep: false, at: 1, size: None })]);
        let (photos, previews) = crate::shots::to_move(&sec, "S", &review, None);
        assert_eq!(photos.iter().map(|p| p.rel.as_str()).collect::<Vec<_>>(), [files[0], files[1]]);
        assert_eq!(previews, 1);
        let both = HashMap::from([files[0], files[2]].map(|f| (format!("S/{f}"), crate::store::Choice { keep: false, at: 1, size: None })));
        let ticked = HashSet::from([files[2].to_string()]);
        let (only, n) = crate::shots::to_move(&sec, "S", &both, Some(&ticked));
        assert_eq!((only.iter().map(|p| p.rel.as_str()).collect::<Vec<_>>(), n), (vec![files[2], files[3]], 1), "an unticked video stays, and its preview with it");
        // A dual-lens shot: two videos, one preview, which moves only when both are ticked.
        let lens = "2025/VID_20210304_070809_10_003.insv";
        std::fs::write(dir.join(lens), lens).unwrap();
        let mut sec2 = sec.clone();
        let mut pair = sec2.works.iter().find(|w| w.items.iter().any(|i| i.rel == files[0])).unwrap().items.iter().find(|i| i.rel == files[0]).unwrap().clone();
        (pair.rel, pair.name) = (lens.into(), lens[5..].into());
        sec2.works.iter_mut().find(|w| w.items.iter().any(|i| i.rel == files[0])).unwrap().items.push(pair);
        let two = HashMap::from([files[0], lens].map(|f| (format!("S/{f}"), crate::store::Choice { keep: false, at: 1, size: None })));
        let rels = |t: &[&str]| crate::shots::to_move(&sec2, "S", &two, Some(&t.iter().map(|x| x.to_string()).collect())).0.into_iter().map(|p| p.rel).collect::<Vec<_>>();
        assert_eq!(rels(&[files[0]]), [files[0]], "half a shot ticked: its preview stays with the other half");
        assert_eq!(rels(&[files[0], lens]), [files[0], lens, files[1]]);
        let one = HashMap::from([(format!("S/{}", files[0]), crate::store::Choice { keep: false, at: 1, size: None })]);
        assert_eq!(crate::shots::to_move(&sec2, "S", &one, None).0.len(), 1, "half a shot not needed: its preview stays with the half kept");
        std::fs::remove_file(dir.join(lens)).unwrap();
        let jf = dir.with_file_name("insta360-move-test-data").join("moves.json");
        std::fs::create_dir_all(jf.parent().unwrap()).unwrap();
        let dest = dir.join(crate::shots::NOT_NEEDED);
        let out = crate::moves::move_photos(&jf, "S", &dir, &dest, &photos, &|_, _| {}).unwrap();
        assert_eq!((out.moved, out.skipped.len()), (1, 0), "{:?}", out.skipped);
        assert!(dest.join(files[1]).exists() && !dir.join(files[1]).exists());
        assert!(dir.join(files[3]).exists(), "a preview whose video stays, stays");
        let back = crate::moves::undo(&jf, out.id, &|_, _| {}).unwrap();
        assert_eq!(back.moved, 1, "the preview is put back but not counted");
        for f in files {
            assert_eq!(std::fs::read_to_string(dir.join(f)).unwrap(), f);
        }
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(jf.parent().unwrap());
    }

    #[test]
    fn photos_split_by_the_gap_and_skip_not_needed() {
        let tree = Dir {
            name: "P".into(),
            files: ["2018-02-12 19.32.38.jpg", "2018-02-12 21.00.00.jpg", "2018-02-13 10.00.00.jpg"].map(photo).into(),
            dirs: vec![Dir { name: "Not needed".into(), files: vec![photo("2018-02-12 19.32.39.jpg")], dirs: vec![], program: false }],
            program: false,
        };
        let read = |hours: i64| {
            let (mut w, mut u) = (vec![], vec![]);
            events("S", &tree, Path::new("P"), &mut crate::photos::Dates::load(None), hours * 3600, &mut w, &mut u);
            (w, u)
        };
        let (w, u) = read(4);
        assert_eq!(w.iter().map(|w| w.items.len()).collect::<Vec<_>>(), [2, 1]);
        assert_eq!(w[0].items[0].at, crate::photos::name_date("2018-02-12 19.32.38"));
        assert_eq!(u.len(), 1);
        assert_eq!(u[0].reason, Reason::SetAside);
        assert_eq!(read(1).0.len(), 3);
        assert_eq!(read(24).0.len(), 1);
    }

    /// A made-up library of folders named Cooking, Learning, Books and Kids Stuff, named in
    /// `FVH_DEMO_LIBRARY`, for the two tests below.
    fn demo_library_folder() -> std::path::PathBuf {
        let dir = std::env::var("FVH_DEMO_LIBRARY").expect("set FVH_DEMO_LIBRARY to a demo library folder");
        Path::new(&dir).canonicalize().unwrap()
    }

    /// Reads the demo library and prints what each folder becomes.
    #[test]
    #[ignore]
    fn demo_library() {
        let root = demo_library_folder();
        let ds = drives::list();
        for name in ["Cooking", "Learning", "Books", "Kids Stuff"] {
            let p = root.join(name);
            let d = drives::drive_of(&ds, &p).unwrap();
            let rel = drives::rel_on_drive(d, &p);
            let cf = CheckedFolder { serial: d.serial, rel, last_path: p.to_string_lossy().into(), unchecked: vec![], name: None, split: false, drive: None, kind: None };
            let s = scan_folder(&cf, &ds, None, 4 * 3600, &Kinds::new(), &|_| {}).unwrap();
            println!("== {} {:?} files {} works {}", s.name, s.kind, s.files, s.works.len());
            for w in &s.works {
                println!("   {:<45} {:<28} {} items {:?}", w.title, w.folder.join(" / "), w.items.len(), w.media);
            }
        }
    }

    /// Reads the folder in FVH_SCAN (its kind guessed) and prints a summary, for checks by eye.
    #[test]
    #[ignore]
    fn real_folder() {
        let p = PathBuf::from(std::env::var("FVH_SCAN").unwrap());
        let ds = drives::list();
        let d = drives::drive_of(&ds, &p).unwrap();
        let cf = CheckedFolder { serial: d.serial, rel: drives::rel_on_drive(d, &p), last_path: String::new(), unchecked: vec![], name: None, split: false, drive: None,
            kind: std::env::var("FVH_KIND").ok().and_then(|k| serde_json::from_str(&format!("\"{k}\"")).ok()) };
        let s = scan_folder(&cf, &ds, None, 4 * 3600, &Kinds::new(), &|_| {}).unwrap();
        println!("== {} {:?}: {} files, {} Works, {} unlisted", s.name, s.kind, s.files, s.works.len(), s.unlisted.len());
        let mut why: BTreeMap<String, usize> = BTreeMap::new();
        for u in &s.unlisted {
            *why.entry(format!("{:?}", u.reason)).or_default() += 1;
        }
        println!("   unlisted by reason: {why:?}");
        let items: usize = s.works.iter().map(|w| w.items.len()).sum();
        println!("   items {items}, unlisted {}, files {}", s.unlisted.len(), s.files);
        let show = std::env::var("FVH_SHOW").unwrap_or_default();
        for w in s.works.iter().filter(|w| !show.is_empty() && w.folder.join("/").contains(&show)) {
            let roles = |r: Role| w.items.iter().filter(|i| i.role == r).count();
            println!("   {:<50} | {:<40} | {}y main {} doc {} cc {} code {} {:?}", w.title, w.folder.join(" / "), w.year.map(|y| y.to_string()).unwrap_or_default(), roles(Role::Main), roles(Role::Doc), roles(Role::Cc), roles(Role::Code), w.media);
        }
    }

    #[test]
    fn packs_of_pages_are_guessed_as_collections() {
        let f = |name: &str| F { name: name.into(), ext: "pdf".into(), class: Some(Class::Book), size: 1, mtime: 0, cloud: false, path: PathBuf::new(), preview: false };
        let dir = |name: &str, files: Vec<F>| Dir { name: name.into(), files, dirs: vec![], program: false };
        let pages = |k: &str| dir(k, (1..=8).map(|i| f(&format!("{k} {i}.pdf"))).collect());
        let books = |k: &str| dir(k, (1..=8).map(|i| f(&format!("Title {k}{i} - Author.pdf"))).collect());
        let root = |dirs: Vec<Dir>| Dir { name: "root".into(), files: vec![], dirs, program: false };
        assert_eq!(guess(&root(vec![pages("Ages 3-4"), pages("Ages 5-6"), books("x")])), Kind::Collections);
        assert_eq!(guess(&root(vec![pages("Magazine 1993"), books("x"), books("y")])), Kind::Books);
    }

    /// A folder of `n` files of extension `ext`: books named with an author, or pictures.
    fn files_dir(name: &str, n: usize, ext: &str) -> Dir {
        let class = class_of(ext);
        let f = |i: usize| F { name: format!("Title {i} - Author.{ext}"), ext: ext.into(), class, size: 1, mtime: 0, cloud: false, path: PathBuf::new(), preview: false };
        Dir { name: name.into(), files: (0..n).map(f).collect(), dirs: vec![], program: false }
    }

    #[test]
    fn a_folder_is_offered_from_a_tenth_of_the_files_or_fifty() {
        let root = |dirs: Vec<Dir>| Dir { name: String::new(), files: vec![], dirs, program: false };
        let none = Kinds::new();
        let rels = |t: &Dir, kind: Kind, kinds: &Kinds| suggestions(t, kind, kinds, t.total()).into_iter().map(|x| (x.rel, x.kind, x.files)).collect::<Vec<_>>();
        // 10 pictures of 100 files: a tenth, offered as a Collection; the Books folder is the Section's kind.
        let tenth = root(vec![files_dir("Books", 90, "pdf"), files_dir("Pics", 10, "png")]);
        assert_eq!(rels(&tenth, Kind::Books, &none), [("Pics".to_string(), Kind::Collections, 10)]);
        // 9 of 99 is less than a tenth.
        assert!(rels(&root(vec![files_dir("Books", 90, "pdf"), files_dir("Pics", 9, "png")]), Kind::Books, &none).is_empty());
        // 50 of 1050 is under a tenth but 50 files; 49 is neither.
        assert_eq!(rels(&root(vec![files_dir("Books", 1000, "pdf"), files_dir("Pics", 50, "png")]), Kind::Books, &none), [("Pics".to_string(), Kind::Collections, 50)]);
        assert!(rels(&root(vec![files_dir("Books", 1000, "pdf"), files_dir("Pics", 49, "png")]), Kind::Books, &none).is_empty());
        // A folder with a Folder kind of its own is not offered again.
        assert!(rels(&tenth, Kind::Books, &kinds_of([("Pics", Kind::Files)])).is_empty());
        // In a Photos Section a year and a folder of only pictures are its calendar's; books are not.
        let photos = root(vec![files_dir("2019", 30, "png"), files_dir("Phone", 30, "jpg"), files_dir("Books", 30, "pdf")]);
        assert_eq!(rels(&photos, Kind::Photos, &none), [("Books".to_string(), Kind::Books, 30)]);
        // A trip of camera pictures with its tickets stays in the calendar too.
        let mut trip = files_dir("Trip", 3, "pdf");
        trip.files.extend((0..30).map(|i| F { name: format!("IMG_20240701_1200{i:02}.jpg"), ext: "jpg".into(), class: Some(Class::Image), size: 1, mtime: 0, cloud: false, path: PathBuf::new(), preview: false }));
        assert!(rels(&root(vec![trip, files_dir("Books", 30, "pdf")]), Kind::Photos, &none).iter().all(|x| x.0 != "Trip"));
        // Videos not named by a camera (a course, films) are offered: only pictures stay in the calendar.
        assert_eq!(rels(&root(vec![files_dir("Phone", 60, "jpg"), files_dir("Lessons", 30, "mp4")]), Kind::Photos, &none), [("Lessons".to_string(), Kind::Files, 30)]);
    }

    #[test]
    fn the_offer_is_made_once_per_section_and_sets_no_kind() {
        let top = std::env::temp_dir().join(format!("fvh-offer-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&top);
        for (dir, n, ext) in [("Books", 90, "pdf"), ("Pics", 10, "png")] {
            std::fs::create_dir_all(top.join(dir)).unwrap();
            for i in 0..n {
                std::fs::write(top.join(dir).join(format!("Title {i} - Author.{ext}")), b"x").unwrap();
            }
        }
        let (_, mut sec) = read_as(&top, Kind::Books);
        let cf = CheckedFolder { serial: 1, rel: "S".into(), last_path: String::new(), unchecked: vec![], name: None, split: false, kind: Some(Kind::Books), drive: None };
        sec.id = cf.id();
        for w in &mut sec.works {
            w.id = w.id.replacen("S/", &format!("{}/", cf.id()), 1);
        }
        let mut s = crate::store::Settings { folders: vec![cf], ..Default::default() };
        let (before, works) = (serde_json::to_string(&s).unwrap(), shape(&sec.works));
        assert_eq!(offer(&s, &sec, false).iter().map(|x| (x.rel.as_str(), x.kind)).collect::<Vec<_>>(), [("Pics", Kind::Collections)]);
        // Never applied by the app: the settings and the cards are as they were.
        assert_eq!(serde_json::to_string(&s).unwrap(), before);
        assert_eq!(shape(&sec.works), works);
        assert!(s.folder_kinds.is_empty() && s.folders.len() == 1 && s.offered.is_empty());
        // Answered or put away: not offered again, except when asked from Section settings.
        s.offered.insert(sec.id.clone());
        assert!(offer(&s, &sec, false).is_empty());
        assert_eq!(offer(&s, &sec, true).len(), 1);
        // Kept in settings.json.
        let back: crate::store::Settings = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert!(back.offered.contains(&sec.id));
        let _ = std::fs::remove_dir_all(&top);
    }

    #[test]
    fn a_folder_part_of_a_work_outside_it_is_not_offered() {
        // A book's `files` folder of pictures guesses Collections, but its files are that book's.
        let top = std::env::temp_dir().join(format!("fvh-offer-extra-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&top);
        std::fs::create_dir_all(top.join("files")).unwrap();
        std::fs::write(top.join("Novel - Writer.pdf"), b"x").unwrap();
        for i in 0..30 {
            std::fs::write(top.join("files").join(format!("Map {i}.png")), b"x").unwrap();
        }
        let (tree, mut sec) = read_as(&top, Kind::Books);
        assert_eq!(suggestions(&tree, Kind::Books, &Kinds::new(), tree.total()).len(), 1, "alone it guesses another kind");
        let cf = CheckedFolder { serial: 1, rel: "S".into(), last_path: String::new(), unchecked: vec![], name: None, split: false, kind: Some(Kind::Books), drive: None };
        sec.id = cf.id();
        for w in &mut sec.works {
            w.id = w.id.replacen("S/", &format!("{}/", cf.id()), 1);
        }
        let s = crate::store::Settings { folders: vec![cf], ..Default::default() };
        assert!(offer(&s, &sec, true).is_empty());
        let _ = std::fs::remove_dir_all(&top);
    }

    #[test]
    fn answered_offers_go_with_their_sections() {
        let cf = |rel: &str| CheckedFolder { serial: 1, rel: rel.into(), last_path: String::new(), unchecked: vec![], name: None, split: false, kind: None, drive: None };
        let mut s = crate::store::Settings { folders: vec![cf("A"), cf("B")], ..Default::default() };
        s.offered.extend(["00000001:A".to_string(), "00000001:B".to_string()]);
        s.folders.retain(|f| f.rel != "B");
        s.prune_offered();
        assert_eq!(s.offered.iter().collect::<Vec<_>>(), ["00000001:A"]);
    }

    #[test]
    fn a_folder_that_is_one_work_is_not_offered() {
        // A course of numbered lessons guesses Videos and files on its own, but in a Courses
        // Section it is one course: no other kind is offered for it.
        let top = std::env::temp_dir().join(format!("fvh-offer-course-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&top);
        for c in ["Watercolor", "Oil"] {
            std::fs::create_dir_all(top.join(c)).unwrap();
            for i in 1..=8 {
                std::fs::write(top.join(c).join(format!("{i:02} Lesson {i}.mp4")), b"x").unwrap();
            }
        }
        let (tree, mut sec) = read_as(&top, Kind::Courses);
        assert_eq!(suggestions(&tree, Kind::Courses, &Kinds::new(), tree.total()).len(), 2, "each course alone guesses another kind");
        let cf = CheckedFolder { serial: 1, rel: "S".into(), last_path: String::new(), unchecked: vec![], name: None, split: false, kind: Some(Kind::Courses), drive: None };
        sec.id = cf.id();
        for w in &mut sec.works {
            w.id = w.id.replacen("S/", &format!("{}/", cf.id()), 1);
        }
        let s = crate::store::Settings { folders: vec![cf], ..Default::default() };
        assert!(offer(&s, &sec, true).is_empty());
        let _ = std::fs::remove_dir_all(&top);
    }

    #[test]
    fn lessons_tick_without_the_extension() {
        assert_eq!(lesson_key("01 Intro.mp4"), "01 Intro");
        assert_eq!(lesson_key("Part 1/01 Intro.mp4"), "Part 1/01 Intro");
        assert_eq!(lesson_key("v1.2/Intro"), "v1.2/Intro");
        assert_eq!(lesson_key(".hidden"), ".hidden");
    }

    #[test]
    fn names_sort_as_people_read_them() {
        let mut v = vec!["Part 10", "part 2", "Part 1", "Appendix", "Part 02b"];
        v.sort_by(|a, b| natural(a, b));
        assert_eq!(v, ["Appendix", "Part 1", "part 2", "Part 02b", "Part 10"]);
    }

    #[test]
    fn years_and_junk_leave_the_title() {
        assert_eq!(clean_title("[2015] The Martian.epub", true), ("The Martian".into(), Some(2015)));
        assert_eq!(clean_title("Networking - pdf - 2019.pdf", true), ("Networking".into(), Some(2019)));
        assert_eq!(clean_title("Allegiant (Veronica Roth) (Z-Library).azw3", true), ("Allegiant (Veronica Roth)".into(), None));
        assert_eq!(clean_title("Knife Skills - YouTube.mp4", true), ("Knife Skills".into(), None));
    }

    #[test]
    fn parts_of_one_work_are_named_as_parts() {
        for n in ["CD1", "CD 2", "Audio", "UK", "us", "123", "Resources", "Picture Dictionary", "The Time Machine Звук", "Audio CD"] {
            assert!(part_name(n), "{n}");
        }
        for n in ["LEVEL 2", "Graded Readers", "Cdrom tools", "Audiobooks of 2020x"] {
            assert!(!part_name(n), "{n}");
        }
    }

    #[test]
    fn parts_take_the_title_of_the_folder_above() {
        let f = |p: &[&str]| p.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        assert_eq!(part_title("CD1".into(), &f(&["Kids", "The Time Machine"]), true), "The Time Machine - CD1");
        assert_eq!(part_title("Version 2".into(), &f(&["Love or Money", "Audio"]), true), "Love or Money - Version 2");
        assert_eq!(part_title("Vol. 3".into(), &f(&["Series"]), false), "Series - Vol. 3");
        assert_eq!(part_title("Часть 1".into(), &f(&["Война и мир"]), true), "Война и мир - Часть 1");
        assert_eq!(part_title("03".into(), &f(&["[2015] Album", "CD 2"]), true), "Album - 03");
        assert_eq!(part_title("CD1".into(), &[], true), "CD1");
        assert_eq!(part_title("300".into(), &f(&["Films"]), false), "300");
        for t in ["2010", "1984", "CD Player", "Volumes", "Part One", "Disco 2", "Version 2 Final", "Booklet 2", "The Time Machine"] {
            assert_eq!(part_title(t.into(), &f(&["Kids"]), true), t);
        }
    }

    #[test]
    fn captions_match_their_video() {
        assert_eq!(caption_key("01 Intro.mp4.srt"), ("01 intro".into(), String::new()));
        assert_eq!(caption_key("01_Intro-EN.vtt"), ("01 intro".into(), "EN".into()));
        assert_eq!(caption_key("01 Intro.mp4").0, "01 intro");
        assert_eq!(lesson_number("3 - Skies.srt"), Some(3));
        assert_eq!(lesson_number("2001 A Space Odyssey"), Some(2001));
        assert_eq!(lesson_number("Intro.srt"), None);
    }

    #[test]
    fn books_group_by_name_and_isbn() {
        assert_eq!(stem_key("The_Martian.epub"), stem_key("the martian.pdf"));
        assert_eq!(stem_key("9780307887443 The Martian.pdf"), "9780307887443");
        assert_eq!(stem_key("Dune (from kfx).azw3"), stem_key("Dune.epub"));
    }

    #[test]
    fn chapters_are_named_as_chapters() {
        assert!(chapter_like("01 Basics") && chapter_like("Module1") && chapter_like("Урок 3") && chapter_like("Week 2"));
        assert!(!chapter_like("2019 Conference") && !chapter_like("Python for Everybody") && !chapter_like("Partners"));
    }

    #[test]
    fn titles_read_well() {
        assert_eq!(title_of("01 Paper, Paint and Brushes.mp4", true), "Paper, Paint and Brushes");
        assert_eq!(title_of("03. Skies.mp4", true), "Skies");
        assert_eq!(title_of("!Learning Videos", false), "Learning Videos");
        assert_eq!(title_of("2001 A Space Odyssey.mkv", true), "2001 A Space Odyssey");
        assert_eq!(title_of("1984.epub", true), "1984");
        assert_eq!(title_of("101 Dalmatians.epub", true), "101 Dalmatians");
        assert_eq!(title_of("2) Wet on Wet.mp4", true), "Wet on Wet");
        assert_eq!(title_of("3D Printing.pdf", true), "3D Printing");
        assert_eq!(title_of("Knife_Skills_101.mp4", true), "Knife Skills 101");
        assert_eq!(title_of("3-D Printing.pdf", true), "3-D Printing");
        assert_eq!(title_of("3 -Intro.mp4", true), "Intro");
        assert_eq!(title_of("3 - Intro.mp4", true), "Intro");
        assert_eq!(title_of("01-Intro.mp4", true), "Intro");
        assert_eq!(title_of("1-Intro.mp4", true), "1-Intro");
        assert_eq!(title_of("2.1 Что такое DevOps.mp4", true), "2.1 Что такое DevOps");
        assert_eq!(title_of("1.5 Half step.mp4", true), "1.5 Half step");
        assert_eq!(title_of("1.Intro.mp4", true), "Intro");
    }

    #[test]
    fn numbers_that_give_the_order_are_kept() {
        assert!(numbered(["1. Intro", "2) Basics", "Notes"], false));
        assert!(numbered(["2.4 Dockerfile.mp4", "2.10 Images.mp4", "10 - End.mp4"], true));
        assert!(!numbered(["01 Intro", "Basics", "Advanced"], false), "one numbered name of three");
        assert!(!numbered(["1. Intro", "Basics"], false), "half is not most");
        assert!(!numbered(["2001 A Space Odyssey", "1984", "3D Printing", "3-D Scanning"], false), "years and 3D are no lesson numbers");
        assert!(numbered(["01-Intro", "02-Next", "Notes"], false), "a number a title would lose counts");
        assert_eq!(title_in("2. Введение в DevOps", false, true), "2. Введение в DevOps");
        assert_eq!(title_in("2. Введение в DevOps", false, false), "Введение в DevOps");
        assert_eq!(title_in("!01_Start.mp4", true, true), "01 Start");
    }

    #[test]
    fn numbered_folders_keep_their_numbers_and_order() {
        let top = std::env::temp_dir().join(format!("fvh-numbered-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&top);
        let files = ["Course/1. Evolution/1.1 Start.mp4", "Course/2. Intro/2.1 What.mp4", "Course/10. Testing/10.1 Tests.mp4", "Course/Course.docx",
            "Plain/Beta/b.mp4", "Plain/Alpha/a.mp4", "Plain/03. Gamma/c.mp4", "Top/02 Next/Lesson/y.mp4", "Top/01 Start/Lesson/x.mp4",
            "10. Sheet.pdf", "2. Sheet.pdf", "1. Sheet.pdf", "Mixed/1. A/a.mp4", "Mixed/Notes/n.mp4", "Few/1. A/a.mp4", "Few/2. B/b.mp4",
            "Few/Tool/setup.exe"];
        for f in files {
            let p = top.join(f);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, b"x").unwrap();
        }
        for empty in ["Few/x", "Few/y", "Few/z"] {
            std::fs::create_dir_all(top.join(empty)).unwrap();
        }
        let works = |kind: Kind, under: &str| -> Vec<(String, String)> {
            let (_, sec) = read_as(&top, kind);
            sec.works.iter().filter(|w| w.folder.first().map_or("", String::as_str) == under).map(|w| (w.folder.join(" / "), w.title.clone())).collect()
        };
        let pairs = |v: &[(&str, &str)]| v.iter().map(|(f, t)| (f.to_string(), t.to_string())).collect::<Vec<_>>();
        // Lessons that are folders: each one Work, with its number, 1, 2, 10; the document unnumbered.
        assert_eq!(works(Kind::Collections, "Course"), pairs(&[("Course", "1. Evolution"), ("Course", "2. Intro"), ("Course", "10. Testing"), ("Course", "Course")]));
        // One numbered folder of three: its number is cut, as before.
        assert_eq!(works(Kind::Collections, "Plain"), pairs(&[("Plain", "Alpha"), ("Plain", "Beta"), ("Plain", "Gamma")]));
        // Numbered folders shown as folders keep the number too.
        assert_eq!(works(Kind::Collections, "Top"), pairs(&[("Top / 01 Start", "Lesson"), ("Top / 02 Next", "Lesson")]));
        // Files side by side, in every kind that makes a Work of each.
        for kind in [Kind::Collections, Kind::Books, Kind::Files] {
            assert_eq!(works(kind, ""), pairs(&[("", "1. Sheet"), ("", "2. Sheet"), ("", "10. Sheet")]), "{kind:?}");
        }
        // Folders, in a kind that makes no Work of a folder.
        assert_eq!(works(Kind::Files, "Course").iter().map(|(f, _)| f.as_str()).collect::<Vec<_>>(), ["Course", "Course / 1. Evolution", "Course / 2. Intro", "Course / 10. Testing"]);
        // A link to a Split-out Section titles its folders as the Works beside it do.
        let (_, sec) = read_as(&top, Kind::Collections);
        assert_eq!(link_titles(&sec, "Top/01 Start/Lesson"), ["Top", "01 Start", "Lesson"]);
        assert_eq!(link_titles(&sec, "Course/11. Later"), ["Course", "11. Later"]);
        assert_eq!(link_titles(&sec, "Plain/04. Delta"), ["Plain", "Delta"]);
        // The Split-out Section's folder is left out of the read: 1 of 2 numbered beside it is cut.
        assert_eq!(works(Kind::Collections, "Mixed"), pairs(&[("Mixed", "A"), ("Mixed", "Notes")]));
        assert_eq!(link_titles(&sec, "Mixed/3. C"), ["Mixed", "C"]);
        // Empty and program folders show nowhere and take no part.
        assert_eq!(works(Kind::Collections, "Few"), pairs(&[("Few", "1. A"), ("Few", "2. B")]));
        assert_eq!(link_titles(&sec, "Few/3. C"), ["Few", "3. C"]);
        assert_eq!(link_titles(&sec, "Alone/3. C"), ["Alone", "3. C"], "alone, its number is kept");
        // The offer of other kinds titles the folders as their rows.
        let lessons = |n: &str| Dir { name: n.into(), files: (1..=3).map(|i| F { name: format!("{i:02} Lesson.mp4"), ext: "mp4".into(), class: Some(Class::Video), size: 1, mtime: 0, cloud: false, path: PathBuf::new(), preview: false }).collect(), dirs: vec![], program: false };
        let tree = Dir { name: "S".into(), files: vec![], dirs: vec![lessons("1. Start"), lessons("2. Next")], program: false };
        assert_eq!(suggestions(&tree, Kind::Books, &Kinds::new(), 6).iter().map(|x| x.title.as_str()).collect::<Vec<_>>(), ["1. Start", "2. Next"]);
        let _ = std::fs::remove_dir_all(&top);
    }

    #[test]
    fn numbers_side_by_side_inside_a_work_are_kept_in_a_grid() {
        let top = std::env::temp_dir().join(format!("fvh-numbered-in-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&top);
        let files = ["Sheets/1. Sheet.pdf", "Sheets/2. Sheet.pdf", "Sheets/10. Sheet.pdf", "Mixed/01 Intro.pdf", "Mixed/Basics.pdf", "Mixed/Advanced.pdf",
            "Course/1. Start/01 Hello.mp4", "Course/1. Start/02 World.mp4", "Course/2. Next/01 Again.mp4", "Course/10. End/01 Bye.mp4",
            "Lessons/01 Hello.mp3", "Lessons/02 World.mp3", "Lessons/10 Again.mp3"];
        for f in files {
            let p = top.join(f);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, b"x").unwrap();
        }
        let (_, sec) = read_as(&top, Kind::Collections);
        let titles = |w: &str| sec.works.iter().find(|x| x.title == w).unwrap().items.iter().map(|i| i.title.clone()).collect::<Vec<_>>();
        assert_eq!(titles("Sheets"), ["1. Sheet", "2. Sheet", "10. Sheet"], "with their numbers, 2 before 10");
        let mut mixed = titles("Mixed");
        mixed.sort();
        assert_eq!(mixed, ["Advanced", "Basics", "Intro"], "one numbered of three: cut");
        let ordered = titles("Lessons");
        assert_eq!(ordered, ["Hello", "World", "Again"], "a list numbers its rows itself: the file's number is cut, a bare 10 too");
        let (_, sec) = read_as(&top, Kind::Courses);
        let course = sec.works.iter().find(|w| w.title == "Course").unwrap();
        let groups: Vec<&str> = course.items.iter().map(|i| i.group.as_str()).collect();
        assert_eq!(groups, ["1. Start", "1. Start", "2. Next", "10. End"], "headings keep their numbers");
        assert_eq!(course.items.iter().map(|i| i.title.as_str()).collect::<Vec<_>>(), ["Hello", "World", "Again", "Bye"], "lessons have their running number");
        let _ = std::fs::remove_dir_all(&top);
    }

    #[test]
    fn a_split_out_section_is_named_as_its_row() {
        let (top, drive, cf) = section_of("split-out-numbered", &["Data/1. A/x.pdf", "Data/2. B/y.pdf", "Data/3. C/z.pdf"]);
        let ds = std::slice::from_ref(&drive);
        let mut set = crate::store::Settings { folders: vec![cf], ..Default::default() };
        let b = set.split_out("00000001:S/Data/2. B", Kind::Files, None).unwrap();
        let read = |set: &crate::store::Settings| {
            let mut cat = Catalog { v: 1, sections: set.folders.iter().map(|f| scan_folder(&set.as_read(f), ds, None, 4 * 3600, &set.kinds_in(f), &|_| {}).unwrap()).collect() };
            link_sections(&mut cat, set);
            cat
        };
        let cat = read(&set);
        let sec = |cat: &Catalog, id: &str| cat.sections.iter().find(|s| s.id == id).unwrap().clone();
        assert_eq!(sec(&cat, "00000001:S").links[0].folder, ["Data", "2. B"]);
        assert_eq!(sec(&cat, &b).name, "2. B", "named as its row");
        set.folders.iter_mut().find(|f| f.id() == b).unwrap().name = Some("Mine".into());
        assert_eq!(sec(&read(&set), &b).name, "Mine", "a name of its own stays");
        let _ = std::fs::remove_dir_all(&top);
    }

    /// A Section read as `kind` from `dir`, as `scan_folder` makes it, with no Drive.
    fn read_as(dir: &Path, kind: Kind) -> (Dir, Section) {
        let tree = read_tree(dir, "", &HashSet::new(), &mut 0, &|_| {});
        let (mut works, mut unlisted, mut empty) = (vec![], vec![], vec![]);
        empty_dirs(&tree, "", &mut empty);
        if kind == Kind::Photos {
            events("S", &tree, dir, &mut crate::photos::Dates::load(None), 4 * 3600, &mut works, &mut unlisted);
        } else {
            let none = Kinds::new();
            let mut ctx = Ctx::new("S".into(), kind, &none, &mut works, &mut unlisted);
            walk(&mut ctx, &tree, dir, "", &mut vec![], true);
            sort_works(&mut works);
        }
        let sec = Section { id: "S".into(), name: "S".into(), path: dir.to_string_lossy().into(), drive: "D".into(), kind, guessed: false, online: true, gone: false,
            files: tree.total(), works, scanned_at: 0, unlisted, empty, whole_tree: true, split_from: None, links: vec![] };
        (tree, sec)
    }

    /// What a person sees of Works: ids, titles, folders and each item's place, key and role.
    fn shape(ws: &[Work]) -> Vec<String> {
        ws.iter().map(|w| format!("{}|{}|{:?}|{:?}|{}|{:?}", w.id, w.title, w.folder, w.taken, w.added, w.items.iter().map(|i| (&i.path, &i.key, i.role, &i.group, &i.lang)).collect::<Vec<_>>())).collect()
    }

    const KINDS: [Kind; 5] = [Kind::Courses, Kind::Books, Kind::Collections, Kind::Files, Kind::Photos];

    /// The preview of each kind, made from a Section read as any kind, is what a read as that kind makes.
    fn preview_is_a_read(dir: &Path) {
        for from in KINDS {
            let (tree, sec) = read_as(dir, from);
            assert_eq!(tree_of(&sec).total(), tree.total(), "{from:?}: every file read is in the catalog");
            assert_eq!(preview(&sec, None, None, 4 * 3600).0, guess(&tree), "{from:?}: the guess");
            assert!(preview(&sec, None, None, 4 * 3600).2, "{from:?}: exact");
            for to in KINDS {
                let (_, want) = read_as(dir, to);
                let (k, got, _) = preview(&sec, Some(to), None, 4 * 3600);
                assert_eq!(k, to);
                assert_eq!(shape(&got), shape(&want.works), "{} read as {from:?}, previewed as {to:?}", dir.display());
            }
        }
    }

    #[test]
    fn a_preview_makes_what_a_read_would() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../.scratch/preview-test");
        let _ = std::fs::remove_dir_all(&dir);
        for f in [
            "Course A/01 Intro.mp4", "Course A/02 Next.mp4", "Course A/02 Next.en.srt", "Course A/notes.pdf", "Course A/code/run.py",
            "Course B/Week 1/01 a.mp4", "Course B/Week 2/01 b.mp4", "Course B/Week 2/handout.docx",
            "Books/The Night Ferry.epub", "Books/The Night Ferry.pdf", "Books/pdf/Old/x.pdf", "Books/Glass City.fb2", "Books/epub/Glass City.epub", "Books/Audio Book/01.mp3", "Books/Audio Book/02.mp3", "Books/Audio Book/03.mp3",
            "Pack/page 1.pdf", "Pack/page 2.pdf", "Pack/cover.jpg", "Pack/CD1/track.mp3", "Pack/Shelf/one.pdf", "Pack/Shelf/two.pdf",
            "Camera/IMG_20180212_193238.jpg", "Camera/IMG_20180212_194002.jpg", "Camera/VID_20210304_070809_00_003.insv", "Camera/LRV_20210304_070809_01_003.lrv", "Camera/IMG-20180212-WA0001.jpg", "Camera/plain.png",
            "Tool/setup.exe", "Tool/readme.txt", "loose.mp4", "loose.srt", "odd.xyz", "Not needed/old.jpg",
        ] {
            let p = dir.join(f);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, b"x").unwrap();
        }
        // An empty folder beside a part makes a Collection's folder a Shelf rather than one Work.
        for d in ["Pack/Empty", "Pack/Shelf/Nothing/Deeper", "Empty top"] {
            std::fs::create_dir_all(dir.join(d)).unwrap();
        }
        preview_is_a_read(&dir);
        // A preview stopped (Section settings closed) ends with nothing, after asking once per file at most.
        let (_, sec) = read_as(&dir, Kind::Files);
        let asked = std::cell::Cell::new(0);
        assert!(preview_or_stop(&sec, Some(Kind::Photos), None, 4 * 3600, &Kinds::new(), &|| { asked.set(asked.get() + 1); true }).is_none());
        assert!(asked.get() < sec.files, "it stops early, not after looking at every file");
        assert!(preview_or_stop(&sec, Some(Kind::Books), None, 4 * 3600, &Kinds::new(), &|| false).is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The same on each folder of the demo library (`FVH_DEMO_LIBRARY`).
    #[test]
    #[ignore]
    fn a_preview_makes_what_a_read_would_in_the_demo_library() {
        let lib = demo_library_folder();
        for d in std::fs::read_dir(&lib).unwrap().filter_map(Result::ok).filter(|d| d.path().is_dir()) {
            preview_is_a_read(&d.path());
            println!("{}: every kind previews as it reads", d.file_name().to_string_lossy());
        }
    }

    #[test]
    fn a_folder_inside_a_format_folder_is_unlisted_in_books() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../.scratch/format-dirs-test");
        let _ = std::fs::remove_dir_all(&dir);
        for f in ["Glass City.fb2", "pdf/Glass City.pdf", "pdf/Old/Glass City v1.pdf", "pdf/Old/notes.xyz"] {
            let p = dir.join(f);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, b"x").unwrap();
        }
        let tree = read_tree(&dir, "", &HashSet::new(), &mut 0, &|_| {});
        let (mut works, mut unlisted) = (vec![], vec![]);
        let none = Kinds::new();
        let mut ctx = Ctx::new("S".into(), Kind::Books, &none, &mut works, &mut unlisted);
        walk(&mut ctx, &tree, &dir, "", &mut vec![], true);
        let shown: usize = works.iter().map(|w| w.items.len()).sum();
        assert_eq!(shown + unlisted.len(), tree.total(), "every file read is in a Work or unlisted");
        let mut u: Vec<(&str, Reason)> = unlisted.iter().map(|u| (u.rel.as_str(), u.reason)).collect();
        u.sort_by_key(|x| x.0);
        assert_eq!(u, [("pdf/Old/Glass City v1.pdf", Reason::Unplaced), ("pdf/Old/notes.xyz", Reason::Unknown)]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// After one folder is read again, the preview of every kind is still what a whole read makes:
    /// a folder that lost its last file, the folders above it, and a folder deleted.
    #[test]
    fn a_preview_after_reading_one_folder_again_makes_what_a_read_would() {
        let top = Path::new(env!("CARGO_MANIFEST_DIR")).join("../.scratch/preview-under-test");
        let _ = std::fs::remove_dir_all(&top);
        let dir = top.join("S");
        for f in ["Pack/page 1.pdf", "Pack/Shelf/a.pdf", "Pack/Shelf/Sub/b.pdf", "Top/Mid/Leaf/z.pdf", "Gone/Inner/c.pdf", "Gone/Inner/d.pdf"] {
            let p = dir.join(f);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, b"x").unwrap();
        }
        let drive = Drive { path: top.to_string_lossy().into(), label: String::new(), serial: 1, kind: "fixed".into(), free: 0, total: 0, system: false };
        let cf = CheckedFolder { serial: 1, rel: "S".into(), last_path: String::new(), unchecked: vec![], name: None, split: false, kind: Some(Kind::Collections), drive: None };
        let read = || scan_folder(&cf, std::slice::from_ref(&drive), None, 4 * 3600, &Kinds::new(), &|_| {}).unwrap();
        let mut sec = read();
        let under = |names: &[&str]| names.iter().map(|n| n.to_string()).collect::<Vec<_>>();
        std::fs::remove_file(dir.join("Pack/Shelf/Sub/b.pdf")).unwrap();
        sec = scan_under(&cf, std::slice::from_ref(&drive), &sec, &under(&["Pack", "Shelf"]), &Kinds::new(), None, 4 * 3600, &|_| {}).unwrap();
        std::fs::remove_file(dir.join("Top/Mid/Leaf/z.pdf")).unwrap();
        sec = scan_under(&cf, std::slice::from_ref(&drive), &sec, &under(&["Top", "Mid"]), &Kinds::new(), None, 4 * 3600, &|_| {}).unwrap();
        std::fs::remove_dir_all(dir.join("Gone")).unwrap();
        sec = scan_under(&cf, std::slice::from_ref(&drive), &sec, &under(&["Gone"]), &Kinds::new(), None, 4 * 3600, &|_| {}).unwrap();
        let mut empty = sec.empty.clone();
        empty.sort();
        assert_eq!(empty, ["Pack/Shelf/Sub", "Top", "Top/Mid", "Top/Mid/Leaf"]);
        assert!(preview(&sec, None, None, 4 * 3600).2, "exact after reading folders again");
        let fresh = read();
        for to in KINDS {
            assert_eq!(shape(&preview(&sec, Some(to), None, 4 * 3600).1), shape(&preview(&fresh, Some(to), None, 4 * 3600).1), "{to:?}");
        }
        let _ = std::fs::remove_dir_all(&top);
    }

    /// Folder kinds by their folders' paths as written, keyed as `Settings::kinds_in` keys them.
    fn kinds_of<const N: usize>(list: [(&str, Kind); N]) -> Kinds {
        list.into_iter().map(|(rel, k)| (crate::store::case(rel), k)).collect()
    }

    /// On Linux a Folder kind on `X` leaves `x` beside it read as its Section is. The
    /// system's temp folder, since `.scratch` on a Windows drive sets case aside.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_folder_kind_keeps_case_on_linux() {
        let top = std::env::temp_dir().join(format!("fvh-kind-case-{}", std::process::id()));
        for f in ["S/X/01 a.mp4", "S/X/02 b.mp4", "S/x/01 a.mp4", "S/x/02 b.mp4"] {
            let p = top.join(f);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, b"x").unwrap();
        }
        let drive = Drive { path: top.to_string_lossy().into(), label: String::new(), serial: 1, kind: "fixed".into(), free: 0, total: 0, system: false };
        let cf = CheckedFolder { serial: 1, rel: "S".into(), last_path: String::new(), unchecked: vec![], name: None, split: false, kind: Some(Kind::Books), drive: None };
        let kinds = kinds_of([("X", Kind::Photos)]);
        let sec = scan_folder(&cf, std::slice::from_ref(&drive), None, 4 * 3600, &kinds, &|_| {}).unwrap();
        let _ = std::fs::remove_dir_all(&top);
        let events: Vec<_> = sec.works.iter().filter(|w| w.event).flat_map(|w| w.items.iter().map(|i| i.path.clone())).collect();
        assert_eq!(events.len(), 2, "X's two clips are an Event: {events:?}");
        assert!(events.iter().all(|p| p.contains("/X/")), "x's are not: {events:?}");
    }

    /// On Linux a Photos Section's folder `trip` beside `Trip`, which has the Folder kind Books,
    /// stays in the Events: only `Trip` leaves them.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_photos_folder_beside_one_of_another_kind_keeps_case_on_linux() {
        let top = std::env::temp_dir().join(format!("fvh-photos-case-{}", std::process::id()));
        for f in ["S/Trip/Glass.epub", "S/trip/IMG_20240301_101500.jpg"] {
            let p = top.join(f);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, b"x").unwrap();
        }
        let drive = Drive { path: top.to_string_lossy().into(), label: String::new(), serial: 1, kind: "fixed".into(), free: 0, total: 0, system: false };
        let cf = CheckedFolder { serial: 1, rel: "S".into(), last_path: String::new(), unchecked: vec![], name: None, split: false, kind: Some(Kind::Photos), drive: None };
        let sec = scan_folder(&cf, std::slice::from_ref(&drive), None, 4 * 3600, &kinds_of([("Trip", Kind::Books)]), &|_| {}).unwrap();
        let _ = std::fs::remove_dir_all(&top);
        let in_events: Vec<_> = sec.works.iter().filter(|w| w.event).flat_map(|w| w.items.iter().map(|i| i.path.clone())).collect();
        assert!(in_events.iter().any(|p| p.ends_with("trip/IMG_20240301_101500.jpg")), "trip's picture is in an Event: {in_events:?}");
        assert!(sec.works.iter().any(|w| !w.event && w.items.iter().any(|i| i.path.ends_with("Trip/Glass.epub"))), "Trip is read as Books");
    }

    /// A temporary Section `S` of these files under `.scratch/<name>`, with its Drive.
    fn section_of(name: &str, files: &[&str]) -> (PathBuf, Drive, CheckedFolder) {
        let top = Path::new(env!("CARGO_MANIFEST_DIR")).join("../.scratch").join(name);
        let _ = std::fs::remove_dir_all(&top);
        for f in files {
            let p = top.join("S").join(f);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, b"x").unwrap();
        }
        let drive = Drive { path: top.to_string_lossy().into(), label: String::new(), serial: 1, kind: "fixed".into(), free: 0, total: 0, system: false };
        let cf = CheckedFolder { serial: 1, rel: "S".into(), last_path: String::new(), unchecked: vec![], name: None, split: false, kind: Some(Kind::Files), drive: None };
        (top, drive, cf)
    }

    /// A whole drive as a Section: its loose files are Works, and a Work in a folder on it
    /// has the id it has when only that folder is ticked, so Marks stay either way. Reading one
    /// of its folders again works as in any Section.
    #[test]
    fn a_whole_drive_reads_as_a_section() {
        let (top, drive, folder) = section_of("whole-drive-test", &["Pack/a.pdf", "Pack/Sub/b.pdf", "c.pdf"]);
        std::fs::write(top.join("loose.pdf"), b"x").unwrap();
        let whole = CheckedFolder { rel: String::new(), ..folder.clone() };
        let ds = std::slice::from_ref(&drive);
        let read = |cf: &CheckedFolder| scan_folder(cf, ds, None, 4 * 3600, &Kinds::new(), &|_| {}).unwrap();
        let (all, part) = (read(&whole), read(&folder));
        assert_eq!(all.id, "00000001:");
        let ids = |sec: &Section| sec.works.iter().map(|w| w.id.clone()).collect::<Vec<_>>();
        assert!(ids(&all).contains(&"00000001:loose".to_string()), "{:?}", ids(&all));
        for id in ids(&part) {
            assert!(ids(&all).contains(&id), "{id} is in the folder's Section, not the drive's: {:?}", ids(&all));
        }
        let under = |names: &[&str]| names.iter().map(|n| n.to_string()).collect::<Vec<_>>();
        std::fs::remove_file(top.join("S/Pack/Sub/b.pdf")).unwrap();
        let again = scan_under(&whole, ds, &all, &under(&["S", "Pack"]), &Kinds::new(), None, 4 * 3600, &|_| {}).unwrap();
        assert_eq!(shape(&again.works), shape(&read(&whole).works));
        let _ = std::fs::remove_dir_all(&top);
    }

    const MIXED: &[&str] = &[
        "loose.pdf",
        "Media/Watercolor/01 Paper.mp4", "Media/Watercolor/02 Skies.mp4",
        "Media/Books/Glass.epub", "Media/Books/pdf/Glass.pdf",
        "Media/Books/Deep/Fox.epub", "Media/Books/Deep/pdf/Fox.pdf",
        "Media/Lessons/01 Intro.mp4", "Media/Lessons/02 Next.mp4", "Media/Lessons/Notes/a.pdf",
    ];

    fn mixed_kinds() -> Kinds {
        kinds_of([("Media", Kind::Courses), ("Media/Books", Kind::Books), ("Media/Lessons/Notes", Kind::Files)])
    }

    #[test]
    fn the_nearest_folder_with_a_kind_wins() {
        let k = mixed_kinds();
        assert_eq!(kind_at(&k, Kind::Files, ""), Kind::Files);
        assert_eq!(kind_at(&k, Kind::Files, "Other"), Kind::Files);
        assert_eq!(kind_at(&k, Kind::Files, "Media"), Kind::Courses);
        assert_eq!(kind_at(&k, Kind::Files, "Media/Watercolor"), Kind::Courses);
        let shout = if crate::store::CASE_ASIDE { Kind::Books } else { Kind::Files };
        assert_eq!(kind_at(&k, Kind::Files, "MEDIA/Books/Deep"), shout, "case aside, or on Linux another folder");
        let (top, drive, cf) = section_of("folder-kind-test", MIXED);
        let ids = |kinds: &Kinds| {
            let mut v: Vec<String> = scan_folder(&cf, std::slice::from_ref(&drive), None, 4 * 3600, kinds, &|_| {}).unwrap().works.iter().map(|w| format!("{} {}", w.id.trim_start_matches("00000001:S/"), w.items.len())).collect();
            v.sort();
            v
        };
        assert_eq!(ids(&Kinds::new()), [
            "Media/Books/Deep/Fox 1", "Media/Books/Deep/pdf/Fox 1", "Media/Books/Glass 1", "Media/Books/pdf/Glass 1",
            "Media/Lessons/01 Intro 1", "Media/Lessons/02 Next 1", "Media/Lessons/Notes/a 1",
            "Media/Watercolor/01 Paper 1", "Media/Watercolor/02 Skies 1", "loose 1",
        ], "as Videos and files, every file is a card");
        assert_eq!(ids(&mixed_kinds()), [
            // Books from Media/Books down: a book's formats are one card, in Deep too.
            "Media/Books/Deep/Fox 2", "Media/Books/Glass 2",
            // Lessons holds a folder with a kind of its own, so it stays a folder, read as Courses.
            "Media/Lessons/01 Intro 1", "Media/Lessons/02 Next 1", "Media/Lessons/Notes/a 1",
            // Courses from Media down: the course is one card.
            "Media/Watercolor/ 2",
            // The Section's own kind above them.
            "loose 1",
        ]);
        let _ = std::fs::remove_dir_all(&top);
    }

    /// The dry run with Folder kinds makes what a read with them makes, and reading one folder
    /// again with a new kind makes what a whole read does.
    #[test]
    fn a_folder_kind_dry_run_and_read_again_make_what_a_read_would() {
        let (top, drive, cf) = section_of("folder-kind-dry-run-test", MIXED);
        let ds = std::slice::from_ref(&drive);
        let before = scan_folder(&cf, ds, None, 4 * 3600, &Kinds::new(), &|_| {}).unwrap();
        let kinds = mixed_kinds();
        let read = scan_folder(&cf, ds, None, 4 * 3600, &kinds, &|_| {}).unwrap();
        let (_, dry, exact) = preview_or_stop(&before, Some(before.kind), None, 4 * 3600, &kinds, &|| false).unwrap();
        assert!(exact);
        assert_eq!(shape(&dry), shape(&read.works));
        // Media is the top folder holding every folder whose kind changed: reading it again is exact.
        let again = scan_under(&cf, ds, &before, &["Media".to_string()], &kinds, None, 4 * 3600, &|_| {}).unwrap();
        assert_eq!(shape(&again.works), shape(&read.works));
        assert_eq!(again.files, read.files);
        // Back to Same as: one folder read again under a folder that keeps its kind.
        let mut less = mixed_kinds();
        less.remove("media/books");
        let back = scan_under(&cf, ds, &read, &["Media".to_string(), "Books".to_string()], &less, None, 4 * 3600, &|_| {}).unwrap();
        assert_eq!(shape(&back.works), shape(&scan_folder(&cf, ds, None, 4 * 3600, &less, &|_| {}).unwrap().works));
        let _ = std::fs::remove_dir_all(&top);
    }

    /// A format folder at the top of a Books Section is taken into the books beside it: setting a
    /// kind inside it, and putting it back, read the whole Section, so no card is made twice.
    #[test]
    fn reading_a_format_folder_again_is_a_whole_read() {
        let (top, drive, mut cf) = section_of("folder-kind-format-test", &["Glass.epub", "pdf/Glass.pdf", "pdf/sub/Fox.pdf", "pdf/sub/Fox.epub", "Other/Night.pdf"]);
        cf.kind = Some(Kind::Books);
        let ds = std::slice::from_ref(&drive);
        let sec = scan_folder(&cf, ds, None, 4 * 3600, &Kinds::new(), &|_| {}).unwrap();
        let kinds = kinds_of([("pdf/sub", Kind::Files)]);
        assert!(scan_under(&cf, ds, &sec, &["pdf".to_string()], &kinds, None, 4 * 3600, &|_| {}).is_none(), "its files are in a book outside it");
        let with = scan_folder(&cf, ds, None, 4 * 3600, &kinds, &|_| {}).unwrap();
        assert!(scan_under(&cf, ds, &with, &["pdf".to_string()], &Kinds::new(), None, 4 * 3600, &|_| {}).is_none(), "putting it back joins it to the books again");
        assert!(scan_under(&cf, ds, &sec, &["Other".to_string()], &Kinds::new(), None, 4 * 3600, &|_| {}).is_some(), "any other folder is read alone");
        let _ = std::fs::remove_dir_all(&top);
    }

    /// A folder with the Folder kind Photos shows Events from there down (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`): their ids use that
    /// folder in place of the Section's, as when it is a Photos Section of its own; their folders
    /// start with its titles; a Folder kind below it is not used. The dry run and reading it again
    /// make what a read makes.
    #[test]
    fn events_of_a_photos_folder_use_its_path() {
        let files = ["loose.pdf", "Books/Glass.epub", "Data/Phone/IMG_20240301_101500.jpg", "Data/Phone/IMG_20240301_101800.jpg", "Data/Phone/Sub/VID_20240502_090000.mp4",
            "Data/Phone/notes.txt", "Data/Phone/Not needed/IMG_20240101_000000.jpg", "Data/Phone/scan.jpg"];
        let (top, drive, cf) = section_of("photo-folder-test", &files);
        let ds = std::slice::from_ref(&drive);
        let kinds = kinds_of([("Data/Phone", Kind::Photos), ("Data/Phone/Sub", Kind::Books)]);
        let sec = scan_folder(&cf, ds, None, 4 * 3600, &kinds, &|_| {}).unwrap();
        let ids = |s: &Section| {
            let mut v: Vec<String> = s.works.iter().map(|w| format!("{} {}", w.id.trim_start_matches("00000001:S/"), w.folder.join(" / "))).collect();
            v.sort();
            v
        };
        assert_eq!(ids(&sec), [
            "Books/Glass Books",
            "Data/Phone#IMG_20240301_101500 Data / Phone / 2024 / 03 March",
            "Data/Phone#Sub/VID_20240502_090000 Data / Phone / 2024 / 05 May",
            "Data/Phone#scan Data / Phone / Undated",
            "loose ",
        ]);
        let mut u: Vec<(&str, String, Reason)> = sec.unlisted.iter().map(|u| (u.rel.as_str(), u.folder.join(" / "), u.reason)).collect();
        u.sort_by(|a, b| a.0.cmp(b.0));
        assert_eq!(u, [("Data/Phone/Not needed/IMG_20240101_000000.jpg", "Data / Phone / Not needed".to_string(), Reason::SetAside), ("Data/Phone/notes.txt", "Data / Phone".to_string(), Reason::Unplaced)]);
        // The same folder checked as a Photos Section of its own makes the same Event ids.
        let own = CheckedFolder { rel: "S/Data/Phone".into(), kind: Some(Kind::Photos), ..cf.clone() };
        let alone: HashSet<String> = scan_folder(&own, ds, None, 4 * 3600, &Kinds::new(), &|_| {}).unwrap().works.into_iter().map(|w| w.id).collect();
        assert_eq!(alone, sec.works.iter().filter(|w| w.event).map(|w| w.id.clone()).collect());
        // Its dry run, and reading it or the folder above it again, make what a read makes.
        let before = scan_folder(&cf, ds, None, 4 * 3600, &Kinds::new(), &|_| {}).unwrap();
        let (_, dry, exact) = preview_or_stop(&before, Some(before.kind), None, 4 * 3600, &kinds, &|| false).unwrap();
        assert!(exact);
        assert_eq!(shape(&dry), shape(&sec.works));
        let s = |x: &[&str]| x.iter().map(|n| n.to_string()).collect::<Vec<_>>();
        for under in [s(&["Data"]), s(&["Data", "Phone"])] {
            let again = scan_under(&cf, ds, &sec, &under, &kinds, None, 4 * 3600, &|_| {}).unwrap();
            assert_eq!(shape(&again.works), shape(&sec.works), "{under:?}");
            assert_eq!(again.files, sec.files);
        }
        assert_eq!(shape(&scan_under(&cf, ds, &before, &s(&["Data"]), &kinds, None, 4 * 3600, &|_| {}).unwrap().works), shape(&sec.works), "made a Photos folder");
        assert!(scan_under(&cf, ds, &sec, &s(&["Data", "Phone", "2024"]), &kinds, None, 4 * 3600, &|_| {}).is_none(), "a year is no folder");
        let _ = std::fs::remove_dir_all(&top);
    }

    /// A `#` in a folder's name is no Event's `#`: reading such a folder again keeps the folders
    /// beside it, and a Photos folder named with one is its own place for Review's move.
    #[test]
    fn a_hash_in_a_folder_name_is_kept() {
        let (top, drive, cf) = section_of("photo-folder-hash-test", &["C# One/a.pdf", "C# Two/b.pdf", "Pics #2/IMG_20240301_101500.jpg"]);
        let ds = std::slice::from_ref(&drive);
        let kinds = kinds_of([("Pics #2", Kind::Photos)]);
        let sec = scan_folder(&cf, ds, None, 4 * 3600, &kinds, &|_| {}).unwrap();
        let again = scan_under(&cf, ds, &sec, &["C# One".to_string()], &kinds, None, 4 * 3600, &|_| {}).unwrap();
        assert_eq!(shape(&again.works), shape(&sec.works), "C# Two and Pics #2 stay");
        let ev = sec.works.iter().find(|w| w.event).unwrap();
        assert_eq!(place_of(ev), "00000001:S/Pics #2");
        let review = HashMap::from([("00000001:S/Pics #2/IMG_20240301_101500.jpg".to_string(), crate::store::Choice { keep: false, at: 1, size: None })]);
        assert_eq!(crate::shots::to_move(&sec, "00000001:S/Pics #2", &review, None).0.len(), 1);
        let again = scan_under(&cf, ds, &sec, &["Pics #2".to_string()], &kinds, None, 4 * 3600, &|_| {}).unwrap();
        assert_eq!(shape(&again.works), shape(&sec.works));
        let _ = std::fs::remove_dir_all(&top);
    }

    /// Two folders whose titles are one ("01 Books" and "Books") are read again together, by a
    /// whole read, and each takes its Folder kind.
    #[test]
    fn folders_sharing_a_title_are_read_again_together() {
        let (top, drive, cf) = section_of("folder-kind-title-test", &["01 Books/Glass.epub", "01 Books/pdf/Glass.pdf", "Books/Fox.epub", "Books/pdf/Fox.pdf", "Other/a.pdf"]);
        let ds = std::slice::from_ref(&drive);
        let sec = scan_folder(&cf, ds, None, 4 * 3600, &Kinds::new(), &|_| {}).unwrap();
        assert!(sec.works.iter().filter(|w| w.folder.first().map(String::as_str) == Some("Books")).count() >= 4, "both show under one title");
        let kinds = kinds_of([("01 Books", Kind::Books), ("Books", Kind::Books)]);
        assert!(scan_under(&cf, ds, &sec, &["Books".to_string()], &kinds, None, 4 * 3600, &|_| {}).is_none(), "two folders on disk: a whole read");
        let read = scan_folder(&cf, ds, None, 4 * 3600, &kinds, &|_| {}).unwrap();
        let mut ids: Vec<&str> = read.works.iter().map(|w| w.id.trim_start_matches("00000001:S/")).collect();
        ids.sort();
        assert_eq!(ids, ["01 Books/Glass", "Books/Fox", "Other/a"], "each folder is read as Books");
        let _ = std::fs::remove_dir_all(&top);
    }

    /// A folder split out as a Section (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`), and one split out of that: each Work is listed
    /// once, under the nearest Section, with the id it had, so Marks stay; the Section it came from
    /// links to it; put back, the folder's Works are its own again.
    #[test]
    fn a_split_out_section_lists_its_works_once_and_nests() {
        let files = ["loose.pdf", "Data/Books/Glass.epub", "Data/Books/Deep/Fox.pdf", "Data/Books/Deep/Inner/Owl.pdf", "Data/Books/Off/x.pdf", "Data/Other/a.pdf"];
        let (top, drive, mut cf) = section_of("split-out-test", &files);
        let ds = std::slice::from_ref(&drive);
        cf.unchecked = vec!["Data/Books/Off".into()];
        let mut set = crate::store::Settings { folders: vec![cf.clone()], ..Default::default() };
        set.set_folder_kind("00000001:S/Data/Books", crate::store::FolderSetting { kind: Some(Kind::Books), views: Default::default() });
        let read = |set: &crate::store::Settings| -> Vec<Section> {
            set.folders.iter().map(|f| scan_folder(&set.as_read(f), ds, None, 4 * 3600, &set.kinds_in(f), &|_| {}).unwrap()).collect()
        };
        let ids = |secs: &[Section]| {
            let mut v: Vec<String> = secs.iter().flat_map(|s| s.works.iter().map(|w| w.id.clone())).collect();
            v.sort();
            v
        };
        let whole = ids(&read(&set));
        assert_eq!(whole.len(), 5, "{whole:?}");
        let books = set.split_out("00000001:S/Data/Books", Kind::Books, None).unwrap();
        assert_eq!(books, "00000001:S/Data/Books");
        assert!(set.folder_kinds.is_empty(), "its Folder kind is now the Section's kind");
        assert_eq!(set.folders[1].unchecked, ["Off"], "a folder unticked inside it stays unticked");
        assert!(set.split_out("00000001:s/data/BOOKS", Kind::Files, None).is_err(), "a Section already");
        assert!(set.split_out("00000001:S/Data/Books/Off", Kind::Files, None).is_err(), "not ticked");
        let inner = set.split_out("00000001:S/Data/Books/Deep/Inner", Kind::Books, None).unwrap();
        assert_eq!(set.container_of(&set.folders[2]).map(CheckedFolder::id).as_deref(), Some(books.as_str()), "split out of the nearest Section");
        assert_eq!(set.splits_in(&cf), ["Data/Books", "Data/Books/Deep/Inner"]);
        let secs = read(&set);
        assert_eq!(ids(&secs), whole, "every Work once, with the id it had");
        let of = |id: &str| secs.iter().find(|s| s.id == id).unwrap();
        assert_eq!(of(&inner).works.iter().map(|w| w.id.as_str()).collect::<Vec<_>>(), ["00000001:S/Data/Books/Deep/Inner/Owl"]);
        assert!(of("00000001:S").works.iter().all(|w| !w.id.starts_with("00000001:S/Data/Books/")));
        let mut cat = Catalog { v: 1, sections: secs.clone() };
        link_sections(&mut cat, &set);
        let sec = |id: &str| cat.sections.iter().find(|s| s.id == id).unwrap();
        assert_eq!(sec("00000001:S").links, [Link { id: books.clone(), rel: "Data/Books".into(), folder: vec!["Data".into(), "Books".into()] }]);
        assert_eq!(sec(&books).split_from.as_deref(), Some("00000001:S"));
        assert_eq!(sec(&books).links.iter().map(|l| (l.id.as_str(), l.rel.as_str())).collect::<Vec<_>>(), [(inner.as_str(), "Deep/Inner")]);
        assert_eq!(sec(&inner).split_from.as_deref(), Some(books.as_str()));
        // Put back, Books is a folder of S again, with its kind; Inner is now split out of S.
        assert_eq!(set.put_back(&books, Kind::Books, Default::default()).unwrap(), "00000001:S");
        assert_eq!(set.container_of(&set.folders[1]).map(CheckedFolder::id).as_deref(), Some("00000001:S"));
        assert_eq!(ids(&read(&set)), whole);
        assert!(read(&set)[0].works.iter().any(|w| w.id == "00000001:S/Data/Books/Glass"));
        let _ = std::fs::remove_dir_all(&top);
    }

    /// A Split-out Section whose folder is gone from a connected drive shows as missing, as a gone
    /// checked folder does, and stays until the person removes it (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`).
    #[test]
    fn a_missing_split_out_section_shows_as_gone() {
        let (top, drive, cf) = section_of("split-out-gone-test", &["Data/Books/Glass.epub", "Data/Other/a.pdf"]);
        let ds = std::slice::from_ref(&drive);
        let mut set = crate::store::Settings { folders: vec![cf.clone()], ..Default::default() };
        let id = set.split_out("00000001:S/Data/Books", Kind::Books, None).unwrap();
        let books = set.folders[1].clone();
        let was = scan_folder(&set.as_read(&books), ds, None, 4 * 3600, &set.kinds_in(&books), &|_| {}).unwrap();
        std::fs::remove_dir_all(top.join("S/Data/Books")).unwrap();
        assert!(scan_folder(&set.as_read(&books), ds, None, 4 * 3600, &set.kinds_in(&books), &|_| {}).is_none());
        let sec = not_read(&books, ds, Some(&was));
        assert!(sec.gone && !sec.online, "missing, its drive connected");
        assert_eq!(sec.works.len(), 1, "what was read last time");
        assert!(set.drop_loose_splits().is_empty(), "it stays until removed");
        let away = not_read(&books, &[], None);
        assert!(!away.gone && !away.online, "a drive not connected is not missing");
        assert_eq!((away.id, away.name), (id, "Books".to_string()));
        let _ = std::fs::remove_dir_all(&top);
    }

    /// A drive unplugged while the app is open makes its Sections, a Split-out one too, not connected
    /// with what was read last time; a Section on another drive and one already missing stay.
    #[test]
    fn a_drive_gone_marks_its_sections_away() {
        let (top, drive, cf) = section_of("drive-away-test", &["Data/Books/Glass.epub", "Data/Other/a.pdf"]);
        let ds = std::slice::from_ref(&drive);
        let mut set = crate::store::Settings { folders: vec![cf.clone()], ..Default::default() };
        set.split_out("00000001:S/Data/Books", Kind::Books, None).unwrap();
        let other = CheckedFolder { serial: 2, ..cf.clone() };
        set.folders.push(other.clone());
        let mut cat = Catalog { v: 1, sections: set.folders.iter().filter(|f| f.serial == 1).map(|f| scan_folder(&set.as_read(f), ds, None, 4 * 3600, &set.kinds_in(f), &|_| {}).unwrap()).collect() };
        let mut elsewhere = cat.sections[0].clone();
        elsewhere.id = other.id();
        cat.sections.push(elsewhere);
        cat.sections[0].gone = true;
        cat.sections[0].online = false;
        let only = |serials: &[u32]| drives::Present { serials: serials.to_vec(), network: vec![] };
        let away = mark_away(&mut cat, &set.folders, &only(&[2]));
        assert_eq!(away, [cat.sections[1].id.clone()], "only the Split-out Section, still online");
        assert!(!cat.sections[1].online && !cat.sections[1].gone && cat.sections[1].works.len() == 1, "what was read last time stays");
        assert!(cat.sections[0].gone, "a missing one stays missing");
        assert!(cat.sections[2].online, "a Section on a drive still in");
        assert!(mark_away(&mut cat, &set.folders, &only(&[1, 2])).is_empty(), "a drive back stays away until read");
        assert!(!cat.sections[1].online);
        // A network drive still mapped is not asked for its serial, so its Sections stay.
        let mapped = drives::Present { serials: vec![], network: vec![cat.sections[2].path.get(..2).unwrap().to_lowercase()] };
        assert!(mark_away(&mut cat, &set.folders, &mapped).is_empty(), "on a network drive still mapped");
        assert!(cat.sections[2].online);
        let _ = std::fs::remove_dir_all(&top);
    }

    /// A drive unplugged during a read gives no Section, so what was read last time stays, rather
    /// than one emptied by the folders it could no longer read.
    #[test]
    fn a_drive_gone_during_a_read_reads_nothing() {
        let names: Vec<String> = (0..500).map(|i| format!("A/{i}.pdf")).chain(["B/b.pdf".to_string()]).collect();
        let (top, drive, cf) = section_of("drive-gone-mid-read-test", &names.iter().map(String::as_str).collect::<Vec<_>>());
        let gone = || {
            let _ = std::fs::remove_dir_all(top.join("S"));
        };
        let sec = scan_folder(&cf, std::slice::from_ref(&drive), None, 4 * 3600, &Default::default(), &|_| gone());
        assert!(!top.join("S").exists(), "the folder went during the read");
        assert!(sec.is_none(), "read {:?} files", sec.map(|s| s.files));
        let _ = std::fs::remove_dir_all(&top);
    }

    /// A folder split out with the kind it has in its Section keeps every Work id, whatever that
    /// kind (courses, packs, books with format folders); one whose files belong to a card outside
    /// it cannot be split out (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`).
    #[test]
    fn work_ids_stay_when_a_folder_is_split_out_as_its_kind() {
        let files = ["other.pdf", "Media/Watercolor/01 Paper.mp4", "Media/Watercolor/02 Skies.mp4", "Media/Knife/01 Grip.mp4", "Media/Knife/02 Dicing.mp4",
            "Media/Pack/page 1.pdf", "Media/Pack/page 2.pdf", "Media/Pack/cover.jpg", "Media/Books/Glass.epub", "Media/Books/pdf/Glass.pdf", "Media/Books/Fox.fb2"];
        for kind in [Kind::Courses, Kind::Books, Kind::Collections, Kind::Files] {
            let (top, drive, mut cf) = section_of(&format!("split-kind-test-{kind:?}"), &files);
            let ds = std::slice::from_ref(&drive);
            cf.kind = Some(kind);
            let mut set = crate::store::Settings { folders: vec![cf], ..Default::default() };
            let ids = |set: &crate::store::Settings| {
                let mut v: Vec<String> = set.folders.iter().flat_map(|f| scan_folder(&set.as_read(f), ds, None, 4 * 3600, &set.kinds_in(f), &|_| {}).unwrap().works).map(|w| w.id).collect();
                v.sort();
                v
            };
            let whole = ids(&set);
            let sec = scan_folder(&set.folders[0], ds, None, 4 * 3600, &Kinds::new(), &|_| {}).unwrap();
            assert!(!splits_a_work(&sec, "00000001:S/Media"), "{kind:?}");
            if kind == Kind::Books {
                assert!(splits_a_work(&sec, "00000001:S/Media/Books/pdf"), "a format folder is part of its book");
            }
            set.split_out("00000001:S/Media", kind, None).unwrap();
            assert_eq!(ids(&set), whole, "{kind:?}");
            let _ = std::fs::remove_dir_all(&top);
        }
    }

    /// A Photos Section's top-level folder with another Folder kind is read as that kind, with the
    /// ids it has in a folder Section and once split out; its pictures leave the Events, and the
    /// other Events keep their ids. Every file is listed once. A Folder kind of Photos there is the
    /// Section's own; one deeper is used only inside a folder of another kind (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`).
    #[test]
    fn a_photos_section_reads_a_folder_of_another_kind_as_that_kind() {
        let files = ["notes.txt", "Phone/IMG_20240301_101500.jpg", "Phone/IMG_20240301_101800.jpg", "Phone/Sub/IMG_20240301_102000.jpg",
            "Books/Glass.epub", "Books/pdf/Glass.pdf", "Books/IMG_20240301_101600.jpg", "Books/Deep/a.pdf", "Books/Deep/b.pdf",
            "Camera/IMG_20240502_090000.jpg", "Camera/IMG_20240502_091000.jpg"];
        let (top, drive, mut cf) = section_of("photos-other-kind-test", &files);
        let ds = std::slice::from_ref(&drive);
        cf.kind = Some(Kind::Photos);
        let read = |kinds: &Kinds| scan_folder(&cf, ds, None, 4 * 3600, kinds, &|_| {}).unwrap();
        let ids = |s: &Section| {
            let mut v: Vec<String> = s.works.iter().map(|w| format!("{} {} {}", w.id.trim_start_matches("00000001:S"), w.folder.join(" / "), w.items.len())).collect();
            v.sort();
            v
        };
        let plain = read(&Kinds::new());
        assert_eq!(ids(&plain), ["#Camera/IMG_20240502_090000 2024 / 05 May 2", "#Phone/IMG_20240301_101500 2024 / 03 March 4"]);
        // Photos at the top is the Section's own kind; a kind inside a Photos folder is not used.
        assert_eq!(ids(&read(&kinds_of([("Phone", Kind::Photos), ("Phone/Sub", Kind::Books)]))), ids(&plain));
        let kinds = kinds_of([("Books", Kind::Books), ("Books/Deep", Kind::Collections), ("Phone/Sub", Kind::Books)]);
        let sec = read(&kinds);
        assert_eq!(ids(&sec), [
            "#Camera/IMG_20240502_090000 2024 / 05 May 2",
            "#Phone/IMG_20240301_101500 2024 / 03 March 3",
            "/Books/Deep/a Books / Deep 1",
            "/Books/Deep/b Books / Deep 1",
            "/Books/Glass Books 2",
            "/Books/IMG_20240301_101600 Books 1",
        ]);
        // Each file once, in a Work or unlisted.
        let mut paths: Vec<&str> = sec.works.iter().flat_map(|w| &w.items).map(|i| i.path.as_str()).chain(sec.unlisted.iter().map(|u| u.path.as_str())).collect();
        let n = paths.len();
        paths.sort();
        paths.dedup();
        assert_eq!((paths.len(), n, sec.files), (files.len(), files.len(), files.len()));
        assert_eq!(sec.works.iter().filter(|w| w.event).count(), 2, "Events after the folder's Works");
        assert!(sec.works.iter().skip_while(|w| !w.event).all(|w| w.event));
        // The dry run makes what a read makes.
        let (_, dry, exact) = preview_or_stop(&plain, Some(Kind::Photos), None, 4 * 3600, &kinds, &|| false).unwrap();
        assert!(exact);
        assert_eq!(shape(&dry), shape(&sec.works));
        // Split out as its kind, the folder keeps its Works' ids; one with the Section's kind has
        // its own Events, and the Events left keep theirs.
        assert!(!splits_a_work(&sec, "00000001:S/Phone"), "Events are made again without it");
        let mut set = crate::store::Settings { folders: vec![cf.clone()], ..Default::default() };
        set.set_folder_kind("00000001:S/Books", crate::store::FolderSetting { kind: Some(Kind::Books), views: Default::default() });
        set.set_folder_kind("00000001:S/Books/Deep", crate::store::FolderSetting { kind: Some(Kind::Collections), views: Default::default() });
        let all = |set: &crate::store::Settings| {
            let mut v: Vec<String> = set.folders.iter().flat_map(|f| scan_folder(&set.as_read(f), ds, None, 4 * 3600, &set.kinds_in(f), &|_| {}).unwrap().works).map(|w| w.id).collect();
            v.sort();
            v
        };
        let whole = all(&set);
        set.split_out("00000001:S/Books", Kind::Books, None).unwrap();
        assert_eq!(all(&set), whole);
        set.split_out("00000001:S/Phone", Kind::Photos, None).unwrap();
        let now = all(&set);
        assert!(now.contains(&"00000001:S#Camera/IMG_20240502_090000".to_string()) && now.contains(&"00000001:S/Phone#IMG_20240301_101500".to_string()), "{now:?}");
        assert_eq!(now.len(), whole.len());
        let _ = std::fs::remove_dir_all(&top);
    }
}
