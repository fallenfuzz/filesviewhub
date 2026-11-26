//! File details read in the background: how long a video or audio track is, and how many pages a
//! PDF has. Kept in the cache as `details.json` by file (path, size, date), so a file is read once
//! until it changes. A cloud-only file is never read (`decisions/2026/10/05/checked-folders-on-any-drive.md`).

use crate::scan::{class_of, Catalog, Class, Item, Role};
use crate::thumbs::Thumbs;
use crate::tools::Fail;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

/// What one file gave. Both empty: the file was read and gave nothing.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default)]
pub struct Detail {
    /// Seconds, for video and audio.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub s: Option<f32>,
    /// Pages, for a PDF.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p: Option<u32>,
}

#[derive(Serialize, Deserialize, Default)]
struct Index {
    v: u32,
    files: HashMap<String, Detail>,
}

struct Job {
    key: String,
    path: String,
    ext: String,
}

struct Inner {
    index: Index,
    /// File key by path, for the catalog as last planned.
    keys: HashMap<String, String>,
    todo: VecDeque<Job>,
    outbox: HashMap<String, Detail>,
    dirty: bool,
    saved: Instant,
}

pub struct Details {
    thumbs: Arc<Thumbs>,
    inner: Mutex<Inner>,
    wake: Condvar,
}

fn key_of(i: &Item) -> String {
    format!("{}|{}|{}", i.path, i.size, i.mtime)
}

fn index_file(dir: &Path) -> std::path::PathBuf {
    dir.join("details.json")
}

/// The files worth a detail: lessons and tracks for their length, a PDF for its pages.
fn wanted(i: &Item) -> bool {
    !i.cloud && i.role != Role::Code && i.role != Role::Cc && (matches!(class_of(&i.ext), Some(Class::Video | Class::Audio)) || i.ext == "pdf")
}

impl Details {
    pub fn new(thumbs: Arc<Thumbs>) -> Arc<Details> {
        let index = Self::load(&thumbs.dir());
        Arc::new(Details {
            thumbs,
            inner: Mutex::new(Inner { index, keys: HashMap::new(), todo: VecDeque::new(), outbox: HashMap::new(), dirty: false, saved: Instant::now() }),
            wake: Condvar::new(),
        })
    }

    fn load(dir: &Path) -> Index {
        let i: Index = crate::store::load(&index_file(dir));
        if i.v == 1 { i } else { Index { v: 1, files: HashMap::new() } }
    }

    /// Reads the index again after the cache moved.
    pub fn reload(&self) {
        let mut g = self.inner.lock().unwrap();
        g.index = Self::load(&self.thumbs.dir());
    }

    pub fn start(self: &Arc<Self>, app: AppHandle) {
        for _ in 0..2 {
            let t = self.clone();
            std::thread::spawn(move || t.work());
        }
        let t = self.clone();
        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_millis(1000));
            let (batch, save) = {
                let mut g = t.inner.lock().unwrap();
                let batch = (!g.outbox.is_empty()).then(|| std::mem::take(&mut g.outbox));
                let save = g.dirty && (g.todo.is_empty() || g.saved.elapsed() > Duration::from_secs(15));
                (batch, save)
            };
            if let Some(b) = batch {
                let _ = app.emit("details", b);
            }
            if save {
                t.save();
            }
        });
    }

    pub fn save(&self) {
        let text = {
            let mut g = self.inner.lock().unwrap();
            g.dirty = false;
            g.saved = Instant::now();
            serde_json::to_string(&g.index)
        };
        if let Ok(text) = text {
            let f = index_file(&self.thumbs.dir());
            let tmp = f.with_extension("json.tmp");
            let dir = f.parent().map(Path::to_path_buf).unwrap_or_default();
            if std::fs::create_dir_all(dir).and_then(|_| std::fs::write(&tmp, text)).and_then(|_| std::fs::rename(&tmp, &f)).is_err() {
                log::warn!("could not save {}", f.display());
            }
        }
    }

    /// Plans what to read for the catalog; details of files no Work holds any more are dropped.
    pub fn plan(&self, cat: &Catalog) {
        {
            let mut g = self.inner.lock().unwrap();
            let g = &mut *g;
            g.todo.clear();
            g.keys.clear();
            // A Section that is not connected keeps what is known; nothing is read for it.
            for (online, i) in cat.sections.iter().flat_map(|s| s.works.iter().flat_map(|w| &w.items).map(move |i| (s.online, i))).filter(|(_, i)| wanted(i)) {
                let key = key_of(i);
                if online && !g.index.files.contains_key(&key) {
                    g.todo.push_back(Job { key: key.clone(), path: i.path.clone(), ext: i.ext.clone() });
                }
                g.keys.insert(i.path.clone(), key);
            }
            let before = g.index.files.len();
            let used: std::collections::HashSet<&String> = g.keys.values().collect();
            g.index.files.retain(|k, _| used.contains(k));
            g.dirty |= g.index.files.len() != before;
            log::info!("file details: {} known, {} to read", g.index.files.len(), g.todo.len());
        }
        self.wake.notify_all();
    }

    /// Every detail known, by file path.
    pub fn ready(&self) -> HashMap<String, Detail> {
        let g = self.inner.lock().unwrap();
        g.keys.iter().filter_map(|(p, k)| g.index.files.get(k).map(|d| (p.clone(), *d))).collect()
    }

    fn work(&self) {
        loop {
            let job = {
                let mut g = self.inner.lock().unwrap();
                loop {
                    if let Some(j) = g.todo.pop_front() {
                        break j;
                    }
                    g = self.wake.wait(g).unwrap();
                }
            };
            let p = Path::new(&job.path);
            // The catalog may be older than the file: one made cloud-only since is not opened.
            let got = if crate::scan::now_cloud(&job.path) {
                Err(Fail::Retry("only in the cloud now".into()))
            } else if job.ext == "pdf" {
                match self.thumbs.tools.path("pdfium", 0) {
                    Some(dll) => crate::pdf::pages(&dll, p).map(|n| Detail { s: None, p: Some(n) }),
                    None => Err(Fail::Retry("no PDFium".into())),
                }
            } else if self.thumbs.tools.path("ffmpeg", 1).is_none() {
                Err(Fail::Retry("no ffprobe".into()))
            } else {
                match self.thumbs.seconds(p) {
                    Some(s) => Ok(Detail { s: Some(s as f32), p: None }),
                    None => Err(Fail::Bad("no length".into())),
                }
            };
            let mut g = self.inner.lock().unwrap();
            match got {
                Ok(d) => {
                    g.index.files.insert(job.key, d);
                    g.outbox.insert(job.path, d);
                    g.dirty = true;
                }
                Err(Fail::Bad(_)) => {
                    g.index.files.insert(job.key, Detail::default());
                    g.dirty = true;
                }
                Err(Fail::Retry(_)) => {}
            }
        }
    }
}
