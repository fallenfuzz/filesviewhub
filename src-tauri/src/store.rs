//! Where FilesViewHub keeps its files (`decisions/2026/10/07/app-data-in-storeddata-next-to-the-app.md`) and the settings and Marks it keeps there.
//! Every file here is listed on the "What this app does" screen (`decisions/2026/10/05/nothing-hidden-no-network-by-default.md`).

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// `StoredData` next to the app (`decisions/2026/10/07/app-data-in-storeddata-next-to-the-app.md`): catalog, Marks, settings, the log and by default the
/// cache. Replacing the app keeps it. On macOS it sits next to the `.app`, on Linux next to the
/// AppImage, not inside it.
/// `FVH_DATA_DIR` points it elsewhere, for tests that must not touch a person's own data.
pub fn data_dir() -> PathBuf {
    static DIR: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    DIR.get_or_init(|| {
        if let Some(d) = std::env::var_os("FVH_DATA_DIR") {
            return PathBuf::from(d);
        }
        // An AppImage runs from a read-only mount; `APPIMAGE` is the file the person started.
        #[cfg(target_os = "linux")]
        let exe = std::env::var_os("APPIMAGE").map(PathBuf::from).filter(|p| p.is_absolute()).unwrap_or_else(|| std::env::current_exe().unwrap_or_default());
        #[cfg(not(target_os = "linux"))]
        let exe = std::env::current_exe().unwrap_or_default();
        let mut dir = exe.parent().map(Path::to_path_buf).unwrap_or_default();
        if dir.ends_with("Contents/MacOS") && let Some(bundle) = dir.parent().and_then(Path::parent).and_then(Path::parent) {
            dir = bundle.to_path_buf();
        }
        dir.join("StoredData")
    })
    .clone()
}

/// Makes the data folder and proves the app can write in it. An error names the folder.
pub fn check_data_dir() -> Result<(), String> {
    let dir = data_dir();
    let probe = dir.join(".write-test");
    std::fs::create_dir_all(&dir)
        .and_then(|_| std::fs::write(&probe, b""))
        .and_then(|_| std::fs::remove_file(&probe))
        .map_err(|e| format!("{}: {e}", dir.display()))
}

pub fn default_cache_dir() -> PathBuf {
    data_dir().join("Cache")
}

pub fn log_dir() -> PathBuf {
    data_dir().join("Logs")
}

pub fn webview_dir() -> PathBuf {
    data_dir().join("WebView")
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Courses,
    Books,
    Collections,
    Files,
    /// Events of Photos by date, in years and months (`decisions/2026/10/08/photos-is-a-section-kind-in-the-app.md`).
    Photos,
}

/// A folder the person checked. It is found again by its Drive's serial number, so a new drive
/// letter does not lose it.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CheckedFolder {
    pub serial: u32,
    /// The place on the Drive, with forward slashes.
    pub rel: String,
    /// The full path when last seen, shown while the Drive is not connected.
    pub last_path: String,
    /// Subfolders unticked inside it, relative to it, with forward slashes.
    #[serde(default)]
    pub unchecked: Vec<String>,
    /// The Section's name when the person renamed it.
    #[serde(default)]
    pub name: Option<String>,
    /// The Section kind when the person chose one; otherwise it is guessed on every scan.
    #[serde(default)]
    pub kind: Option<Kind>,
    /// Its Drive as the library column names it, for when the Drive is not connected.
    #[serde(default)]
    pub drive: Option<String>,
    /// A Split-out Section (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`): a folder inside another checked folder, made a Section of its
    /// own. Its `rel` is that folder's `rel`, `/` and its path there, so its id is the folder's key
    /// and its Works keep their ids. The Section it came from leaves it out.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub split: bool,
}

impl CheckedFolder {
    /// Sets the Section's kind, and its name only with `rename`: a kind change or a guess again
    /// keeps the name as it was, so one never chosen keeps following the folder's.
    pub fn set_section(&mut self, name: Option<String>, kind: Option<Kind>, rename: bool) {
        if rename {
            self.name = name.filter(|n| !n.trim().is_empty());
        }
        self.kind = kind;
    }

    /// The Section's name: the one the person gave it, or its folder's, or for a whole drive its
    /// label, else its letter or mount point.
    pub fn shown_name(&self) -> String {
        if let Some(n) = &self.name {
            return n.clone();
        }
        if self.rel.is_empty() {
            return crate::scan::drive_title(self.drive.as_deref().unwrap_or(&self.last_path));
        }
        crate::scan::title_of(self.rel.rsplit('/').next().unwrap_or(""), false)
    }

    pub fn id(&self) -> String {
        format!("{:08X}:{}", self.serial, self.rel)
    }

    /// The path of folder `other` inside this one, as `rel_in` compares; None when it is not inside.
    pub fn rel_of(&self, other: &CheckedFolder) -> Option<String> {
        rel_in(&other.id(), &self.id())
    }
}

/// A folder's own kind and views inside a Section (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`): None is "Same as" the folder above,
/// and a level with no view picked shows the one above it. Kept by Drive and path.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct FolderSetting {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<Kind>,
    /// The view picked at each level, by level id, as the UI keeps a Section's.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub views: std::collections::BTreeMap<String, String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Settings {
    pub v: u32,
    pub folders: Vec<CheckedFolder>,
    /// None means the default, `Cache` inside the data folder.
    #[serde(default)]
    pub cache_dir: Option<String>,
    /// How the window looks (column widths, what a folder shows), kept as the UI writes it.
    #[serde(default)]
    pub ui: serde_json::Value,
    /// The program a type opens in, by extension, when the person picked one (`decisions/2026/10/05/filesviewhub-is-a-tauri-app-shipped-portable.md`);
    /// any other type opens in the system's own program.
    #[serde(default)]
    pub open_with: std::collections::BTreeMap<String, String>,
    /// The gap in hours that starts a new Photos Event (`decisions/2026/10/04/an-event-is-the-work-in-photos.md`).
    #[serde(default = "gap_hours")]
    pub photo_gap: u32,
    /// The folder a Photos Section's set-aside photos move into, by Section id, when the person
    /// picked one; otherwise `Not needed` inside the photo folder (`decisions/2026/10/09/review-moves-set-aside-photos-itself-with-undo.md`).
    #[serde(default)]
    pub move_to: std::collections::BTreeMap<String, String>,
    /// The folder of the last Marks backup saved or added; the backup dialogs start there.
    #[serde(default)]
    pub backup_dir: Option<String>,
    /// Folder kinds and views (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`), by Drive and path: `{serial}:{path on the Drive}`, as a
    /// Checked folder's id. The nearest folder with a setting wins.
    #[serde(default)]
    pub folder_kinds: std::collections::BTreeMap<String, FolderSetting>,
    /// The Sections whose offer of a kind per folder was answered or put away, by id (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`): it
    /// is made once per Section, and again only from its Section settings.
    #[serde(default, skip_serializing_if = "std::collections::BTreeSet::is_empty")]
    pub offered: std::collections::BTreeSet<String>,
}

fn gap_hours() -> u32 {
    crate::photos::GAP_HOURS
}

impl Default for Settings {
    fn default() -> Self {
        Settings { v: 1, folders: vec![], cache_dir: None, ui: serde_json::Value::Null, open_with: Default::default(), photo_gap: gap_hours(), move_to: Default::default(), backup_dir: None, folder_kinds: Default::default(), offered: Default::default() }
    }
}

impl Settings {
    /// The Folder kinds inside a checked folder's Section, by path from its folder as `case` gives it.
    pub fn kinds_in(&self, cf: &CheckedFolder) -> crate::scan::Kinds {
        let id = cf.id();
        self.folder_kinds.iter().filter_map(|(k, f)| Some((case(&rel_in(k, &id)?), f.kind?))).collect()
    }

    /// Sets a folder's own kind and views by its id (Drive and path); none of either drops it.
    /// Where names set case aside (`CASE_ASIDE`), every key that names the same folder with other
    /// case goes first, so a folder renamed only in case never keeps an old setting in force.
    pub fn set_folder_kind(&mut self, id: &str, set: FolderSetting) {
        let low = case(id);
        self.folder_kinds.retain(|k, _| case(k) != low);
        if set != FolderSetting::default() {
            self.folder_kinds.insert(id.to_string(), set);
        }
    }

    /// The nearest checked folder holding folder `cf`: for a Split-out Section, the Section it came
    /// from (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`). None for any other checked folder.
    pub fn container_of(&self, cf: &CheckedFolder) -> Option<&CheckedFolder> {
        self.folders.iter().filter(|f| f.rel_of(cf).is_some()).max_by_key(|f| f.rel.len())
    }

    /// The Split-out Sections inside checked folder `cf`, at any depth, as paths from it: a read of
    /// it leaves their folders out, as it does an unticked one.
    pub fn splits_in(&self, cf: &CheckedFolder) -> Vec<String> {
        self.folders.iter().filter(|f| f.split).filter_map(|f| cf.rel_of(f)).collect()
    }

    /// Checked folder `cf` as a read takes it: the Split-out Sections inside it left out, as its
    /// unticked folders are (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`).
    pub fn as_read(&self, cf: &CheckedFolder) -> CheckedFolder {
        CheckedFolder { unchecked: [cf.unchecked.clone(), self.splits_in(cf)].concat(), ..cf.clone() }
    }

    /// Makes the folder with key `id` (`{Section id}/{path}`) a Split-out Section of kind `kind`,
    /// out of the nearest checked folder holding it. Its Folder kind goes; the folders unticked
    /// inside it stay unticked, and the Folder kinds below it apply in it as they did. It goes last
    /// among the Sections, or at `at` (Undo of Put back), which keeps the others' order and
    /// colours. Returns its id.
    pub fn split_out(&mut self, id: &str, kind: Kind, at: Option<usize>) -> Result<String, String> {
        let low = case(id);
        if self.folders.iter().any(|f| case(&f.id()) == low) {
            return Err("This folder is a Section already.".into());
        }
        let (from, rel) = self.folders.iter().filter_map(|f| Some((f, rel_in(id, &f.id())?))).max_by_key(|(f, _)| f.rel.len()).ok_or("This folder is not inside a checked folder.")?;
        let under = |u: &str| rel_in(&format!("@/{u}"), &format!("@/{rel}"));
        // It, or a folder above it, unticked.
        if from.unchecked.iter().any(|u| case(u) == case(&rel) || rel_in(&format!("@/{rel}"), &format!("@/{u}")).is_some()) {
            return Err("This folder is not ticked.".into());
        }
        let sep = if from.last_path.contains('\\') { "\\" } else { "/" };
        let cf = CheckedFolder {
            serial: from.serial,
            rel: if from.rel.is_empty() { rel.clone() } else { format!("{}/{rel}", from.rel) },
            last_path: [from.last_path.trim_end_matches(sep).to_string(), rel.replace('/', sep)].join(sep),
            unchecked: from.unchecked.iter().filter_map(|u| under(u)).collect(),
            name: None,
            kind: Some(kind),
            drive: from.drive.clone(),
            split: true,
        };
        let new = cf.id();
        self.folder_kinds.retain(|k, _| case(k) != low);
        self.folders.insert(at.unwrap_or(usize::MAX).min(self.folders.len()), cf);
        Ok(new)
    }

    /// Puts Split-out Section `id` back into the Section it came from, as a folder with the Folder
    /// kind `kind` and the views `views` (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`). The Split-out Sections inside it stay, now out
    /// of that Section. Returns that Section's id.
    pub fn put_back(&mut self, id: &str, kind: Kind, views: std::collections::BTreeMap<String, String>) -> Result<String, String> {
        let at = self.folders.iter().position(|f| f.split && f.id() == id).ok_or("This Section is not split out of another.")?;
        let into = self.container_of(&self.folders[at]).map(CheckedFolder::id).ok_or("The Section it came from is not checked any more.")?;
        self.folders.remove(at);
        self.set_folder_kind(id, FolderSetting { kind: Some(kind), views });
        Ok(into)
    }

    /// Adds to `folders`, the tree's new ticks, the Split-out Sections on a connected Drive, each
    /// with the folders the tree leaves unticked inside it (full paths in `unchecked`). One on a
    /// Drive not connected is not added: it stays with the other folders there, which the tree
    /// could not show.
    pub fn carry_splits(&self, folders: &mut Vec<CheckedFolder>, drives: &[crate::drives::Drive], unchecked: &[String]) {
        for f in self.folders.iter().filter(|f| f.split) {
            let Some(d) = drives.iter().find(|d| d.serial == f.serial) else { continue };
            let base = crate::drives::join_on_drive(d, &f.rel);
            let mut f = f.clone();
            f.unchecked = unchecked
                .iter()
                .map(PathBuf::from)
                .filter(|u| within(u, &base) && !within(&base, u))
                .map(|u| u.components().skip(base.components().count()).map(|c| c.as_os_str().to_string_lossy().to_string()).collect::<Vec<_>>().join("/"))
                .collect();
            if !folders.iter().any(|o| o.id() == f.id()) {
                folders.push(f);
            }
        }
    }

    /// Keeps only the offers answered for Sections still checked (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`): a folder checked
    /// again is offered again.
    pub fn prune_offered(&mut self) {
        let ids: std::collections::HashSet<String> = self.folders.iter().map(CheckedFolder::id).collect();
        self.offered.retain(|id| ids.contains(id));
    }

    /// Drops each Split-out Section no checked folder holds any more, or whose folder is unticked in
    /// the one that holds it: the person unticked or removed that folder, and the tree showed the
    /// Split-out Section ticked as part of it.
    pub fn drop_loose_splits(&mut self) -> Vec<String> {
        let mut gone = vec![];
        while let Some(i) = self.folders.iter().position(|f| {
            f.split && self.container_of(f).is_none_or(|c| c.rel_of(f).is_some_and(|r| {
                let r = case(&r);
                c.unchecked.iter().map(|u| case(u)).any(|u| r == u || r.starts_with(&format!("{u}/")))
            }))
        }) {
            gone.push(self.folders.remove(i).id());
        }
        gone
    }

    /// Drops the Folder kinds whose folder is gone from a connected Drive (renamed, moved or
    /// deleted), so the Section's kind applies there again, and returns each as (Section name,
    /// folder name) for the one toast that says so (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`). Nothing is matched again. A folder
    /// whose Section is gone or not connected, or that is under an unticked folder, keeps its own.
    pub fn drop_lost_kinds(&mut self, drives: &[crate::drives::Drive]) -> Vec<(String, String)> {
        let mut lost = vec![];
        let mut listed = std::collections::HashMap::new();
        let keys: Vec<String> = self.folder_kinds.keys().cloned().collect();
        for key in keys {
            let Some((cf, rel)) = self.folders.iter().find_map(|f| Some((f, rel_in(&key, &f.id())?))) else { continue };
            let Some(d) = drives.iter().find(|d| d.serial == cf.serial) else { continue };
            let root = crate::drives::join_on_drive(d, &cf.rel);
            let unticked = cf.unchecked.iter().map(|u| case(u)).any(|u| {
                let r = case(&rel);
                r == u || r.starts_with(&format!("{u}/"))
            });
            // Only a folder surely not there is lost: a path the system cannot look at now (no
            // access, a network blip) keeps its kind. Where names keep their case, a drive that sets
            // case aside (FAT, exFAT, a share) finds `X` as `x`, so there each name must also be
            // listed as written.
            let gone = match std::fs::metadata(named(&rel.split('/').fold(root.clone(), |p, x| p.join(x)))) {
                Ok(m) => !m.is_dir() || (!CASE_ASIDE && !listed_as_written(&root, &rel, &mut listed)),
                Err(e) => e.kind() == std::io::ErrorKind::NotFound,
            };
            if !named(&root).is_dir() || unticked || !gone {
                continue;
            }
            let folder = crate::scan::title_of(rel.rsplit('/').next().unwrap_or_default(), false);
            log::info!("{key} is not there any more: its own kind is dropped, {} shows it as the Section does", cf.shown_name());
            lost.push((cf.shown_name(), folder));
            self.folder_kinds.remove(&key);
        }
        lost
    }

    /// A cache on a drive that is not connected falls back to the default until the drive is back.
    pub fn cache_dir(&self) -> PathBuf {
        // Its drive: `E:\` on Windows, `/Volumes/<name>` on macOS.
        let root = |p: &Path| -> PathBuf {
            if cfg!(target_os = "macos") && p.starts_with("/Volumes") { p.iter().take(3).collect() } else { p.ancestors().last().map(Path::to_path_buf).unwrap_or_default() }
        };
        match self.cache_dir.as_ref().map(PathBuf::from) {
            Some(p) if root(&p).exists() => p,
            _ => default_cache_dir(),
        }
    }
}

/// A Mark: Next or Done, set by hand, with when it was set (seconds since 1970).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Mark {
    pub m: String,
    pub at: u64,
}

/// A Review choice on one Photo (`decisions/2026/10/04/photos-take-no-marks-review-is-the-progress.md`): kept, or set aside for the move script. It holds for
/// the file of that size only, so a new photo at an old path waits for Review.
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct Choice {
    pub keep: bool,
    pub at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
}

/// Marks by Work id, and Ticks (`decisions/2026/09/29/ticks-offer-the-mark-never-set-it.md`, `decisions/2026/10/01/marks-and-ticks-carry-when-they-were-set.md`): by Work id, each ticked item's Tick key with when
/// it was set. Review choices by Photo, keyed `{Section id}/{path in the Section}` (`shots.rs`
/// `set_aside`, `review.ts` `photoKey`). When each Work was last opened from the app, by Work id
///. Version 1 had no Ticks and reads as version 2 with none.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Marks {
    pub v: u32,
    pub marks: HashMap<String, Mark>,
    #[serde(default)]
    pub ticks: HashMap<String, HashMap<String, u64>>,
    #[serde(default)]
    pub review: HashMap<String, Choice>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub opened: HashMap<String, u64>,
}

impl Default for Marks {
    fn default() -> Self {
        Marks { v: 2, marks: HashMap::new(), ticks: HashMap::new(), review: HashMap::new(), opened: HashMap::new() }
    }
}

impl Marks {
    /// Ticks or unticks items of a Work. Ticking an item already ticked keeps its date.
    pub fn tick(&mut self, id: &str, keys: &[String], on: bool) {
        let t = self.ticks.entry(id.to_string()).or_default();
        for k in keys {
            if on {
                t.entry(k.clone()).or_insert_with(now);
            } else {
                t.remove(k);
            }
        }
        if t.is_empty() {
            self.ticks.remove(id);
        }
    }

    /// Notes Work `id` opened at `at`: never at or before the last Work opened, so two opened in
    /// one second keep the order they were opened in.
    pub fn open(&mut self, id: &str, at: u64) {
        let last = self.opened.values().max().map_or(0, |t| t + 1);
        self.opened.insert(id.to_string(), at.max(last));
    }

    /// Forgets the Review choices of these Photos, so they wait for Review again; returns the
    /// choices forgotten, for Undo.
    pub fn forget_review(&mut self, keys: &[String]) -> HashMap<String, Choice> {
        keys.iter().filter_map(|k| self.review.remove_entry(k)).collect()
    }

    /// Adds a backup's Marks, Ticks, Review choices and times opened: where both have a Mark for a
    /// Work, a choice for a Photo or a time opened, the later one stays; a Tick both have keeps the earlier date. Returns
    /// how many Marks it added or changed.
    pub fn merge(&mut self, other: Marks) -> usize {
        let mut added = 0;
        for (id, m) in other.marks {
            match self.marks.get(&id) {
                Some(mine) if mine.at >= m.at => {}
                _ => {
                    self.marks.insert(id, m);
                    added += 1;
                }
            }
        }
        for (id, t) in other.ticks {
            let into = self.ticks.entry(id).or_default();
            for (k, at) in t {
                let e = into.entry(k).or_insert(at);
                *e = (*e).min(at);
            }
        }
        for (k, c) in other.review {
            if self.review.get(&k).is_none_or(|mine| mine.at < c.at) {
                self.review.insert(k, c);
            }
        }
        for (id, at) in other.opened {
            let e = self.opened.entry(id).or_insert(at);
            *e = (*e).max(at);
        }
        added
    }

    /// Moves a Mark, its Ticks and its time opened to another Work, picked by the person (`decisions/2026/09/30/work-id-is-its-path.md`).
    /// A Mark the other Work has is kept, and so is the date of a Tick it has; the later time opened stays.
    pub fn carry(&mut self, from: &str, to: &str) {
        if let Some(m) = self.marks.remove(from) {
            self.marks.entry(to.to_string()).or_insert(m);
        }
        if let Some(t) = self.ticks.remove(from) {
            let into = self.ticks.entry(to.to_string()).or_default();
            for (k, at) in t {
                into.entry(k).or_insert(at);
            }
        }
        if let Some(at) = self.opened.remove(from) {
            let e = self.opened.entry(to.to_string()).or_insert(at);
            *e = (*e).max(at);
        }
    }

    /// Forgets Works' Marks, Ticks and times opened, and single Ticks as (Work id, item).
    pub fn forget(&mut self, ids: &[String], items: &[(String, String)]) {
        for id in ids {
            self.marks.remove(id);
            self.ticks.remove(id);
            self.opened.remove(id);
        }
        for (id, k) in items {
            if let Some(t) = self.ticks.get_mut(id) {
                t.remove(k);
                if t.is_empty() {
                    self.ticks.remove(id);
                }
            }
        }
    }
}

/// The start of every key inside checked folder or folder `id`: `{id}/`, or for a whole drive's
/// Section (id `{serial}:`) the id alone, so a key is always its Drive and its path on it
/// (`decisions/2026/10/05/checked-folders-on-any-drive.md`) and a Work keeps its id whether its drive or a folder on it is ticked.
pub fn inside(id: &str) -> String {
    let drive = id.len() == 9 && id.ends_with(':') && id.bytes().take(8).all(|b| b.is_ascii_hexdigit());
    if drive { id.to_string() } else { format!("{id}/") }
}

/// The key of path `rel` inside checked folder or folder `id`; `id` itself when `rel` is empty.
pub fn key_in(id: &str, rel: &str) -> String {
    if rel.is_empty() { id.to_string() } else { format!("{}{rel}", inside(id)) }
}

/// The path inside `id` of the key `key`, as it is written; None for `id` itself or a key
/// outside it.
pub fn under<'a>(key: &'a str, id: &str) -> Option<&'a str> {
    key.strip_prefix(&inside(id)).filter(|r| !r.is_empty())
}

/// A key saved in an earlier version inside a whole drive's Section (id `{serial}:`) as `{serial}:/path`,
/// as it is now, `{serial}:path`; None for any other key. `drives`: the whole drives' ids.
pub fn rekey(key: &str, drives: &[String]) -> Option<String> {
    drives.iter().find_map(|id| key.strip_prefix(id.as_str())?.strip_prefix('/').map(|rest| format!("{id}{rest}")))
}

/// `rekey` on every key of a map; a key already there in the new form keeps its own value.
/// Returns how many it changed.
pub fn rekey_map<M, V>(map: &mut M, drives: &[String]) -> usize
where
    M: Default + IntoIterator<Item = (String, V)> + Extend<(String, V)>,
{
    // No bound on `&M`: on macOS another crate's IntoIterator impl sends inference in circles there.
    if drives.is_empty() {
        return 0;
    }
    let mut new: Vec<(String, V)> = vec![];
    let mut old: Vec<(String, V)> = vec![];
    for (k, v) in std::mem::take(map) {
        match rekey(&k, drives) {
            Some(n) => old.push((n, v)),
            None => new.push((k, v)),
        }
    }
    let n = old.len();
    let have: std::collections::HashSet<String> = new.iter().map(|(k, _)| k.clone()).collect();
    map.extend(new);
    map.extend(old.into_iter().filter(|(k, _)| !have.contains(k)));
    n
}

impl Settings {
    /// The ids of the whole drives (`{serial}:`) whose keys may still be kept as `{serial}:/path`:
    /// those of a whole drive's Section, and of a Split-out Section made from one in an earlier version,
    /// whose `rel` starts with `/`.
    pub fn whole_drives(&self) -> Vec<String> {
        let mut d: Vec<String> = self.folders.iter().filter(|f| f.rel.is_empty() || f.rel.starts_with('/')).map(|f| format!("{:08X}:", f.serial)).collect();
        d.sort();
        d.dedup();
        d
    }

    /// Rewrites what is kept by `{serial}:/path` on the drives `drives` (`rekey`): the Split-out
    /// Sections' `rel`, Folder kinds, move folders, offers answered, and the window's views,
    /// merges and sorts by Section id. Returns how many it changed.
    pub fn rekey(&mut self, drives: &[String]) -> usize {
        let mut n = 0;
        for f in self.folders.iter_mut().filter(|f| f.rel.starts_with('/')) {
            f.rel = f.rel.trim_start_matches('/').to_string();
            n += 1;
        }
        let offered: Vec<String> = self.offered.iter().cloned().collect();
        self.offered = offered.into_iter().map(|k| rekey(&k, drives).inspect(|_| n += 1).unwrap_or(k)).collect();
        for part in ["views", "merge", "sort"] {
            let Some(map) = self.ui.get_mut(part).and_then(|v| v.as_object_mut()) else { continue };
            let old = std::mem::take(map);
            for (k, mut v) in old {
                if let Some(to) = v.as_str().and_then(|x| rekey(x, drives)) {
                    v = serde_json::Value::String(to);
                    n += 1;
                }
                let k = rekey(&k, drives).inspect(|_| n += 1).unwrap_or(k);
                map.insert(k, v);
            }
        }
        n + rekey_map(&mut self.folder_kinds, drives) + rekey_map(&mut self.move_to, drives)
    }
}

impl Marks {
    /// Rewrites the Marks, Ticks, Review choices and times opened kept by `{serial}:/path` (`rekey`); how many.
    pub fn rekey(&mut self, drives: &[String]) -> usize {
        rekey_map(&mut self.marks, drives) + rekey_map(&mut self.ticks, drives) + rekey_map(&mut self.review, drives) + rekey_map(&mut self.opened, drives)
    }
}

/// A folder's path inside the checked folder `id`, from its key `{id}/{path}`, None when it is not
/// inside. The serial's letters match in either case; the path's as `CASE_ASIDE` says, case aside
/// letter by letter, so a letter whose lower case is longer never shifts the cut.
pub fn rel_in(key: &str, id: &str) -> Option<String> {
    if !CASE_ASIDE {
        let want = inside(id);
        let n = if want.as_bytes().get(8) == Some(&b':') && want.bytes().take(8).all(|b| b.is_ascii_hexdigit()) { 9 } else { 0 };
        let rel = key.get(..n).filter(|s| s.eq_ignore_ascii_case(&want[..n])).and_then(|_| key[n..].strip_prefix(&want[n..]))?;
        return (!rel.is_empty()).then(|| rel.to_string());
    }
    let want = inside(id).to_lowercase();
    let mut got = String::new();
    for (i, c) in key.char_indices() {
        got.extend(c.to_lowercase());
        if got.len() >= want.len() {
            let rel = &key[i + c.len_utf8()..];
            return (got == want && !rel.is_empty()).then(|| rel.to_string());
        }
    }
    None
}

pub fn settings_file() -> PathBuf {
    data_dir().join("settings.json")
}
pub fn marks_file() -> PathBuf {
    data_dir().join("marks.json")
}
pub fn catalog_file() -> PathBuf {
    data_dir().join("catalog.json")
}

/// Reads a JSON file. A file that cannot be read is set aside as `<name>.damaged` and the
/// default is used, so a broken file never stops the app or is silently overwritten.
pub fn load<T: DeserializeOwned + Default>(path: &Path) -> T {
    load_noting(path, &data_dir())
}

/// The data files `load` set aside, by name, until the window asks for them.
static SET_ASIDE: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

/// The data files set aside since the last ask, for the toast at start.
pub fn take_set_aside() -> Vec<String> {
    std::mem::take(&mut *SET_ASIDE.lock().unwrap())
}

/// As `load`, noting a file set aside that lies in `data` (the cache's lists and a person's backups do not).
fn load_noting<T: DeserializeOwned + Default>(path: &Path, data: &Path) -> T {
    let Ok(text) = std::fs::read_to_string(path) else { return T::default() };
    match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => {
            let aside = path.with_extension("json.damaged");
            match std::fs::rename(path, &aside) {
                Ok(()) => log::warn!("{} could not be read ({e}); set aside as {}", path.display(), aside.display()),
                Err(r) => log::warn!("{} could not be read ({e}), nor set aside as {} ({r})", path.display(), aside.display()),
            }
            if path.parent() == Some(data) && let Some(name) = path.file_name() {
                SET_ASIDE.lock().unwrap().push(name.to_string_lossy().into_owned());
            }
            T::default()
        }
    }
}

/// Writes a JSON file through a temporary file, so a crash mid-write leaves the old one whole.
pub fn save<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("json.tmp");
    // Written as it is made, so a large catalog is never held as one string first.
    let mut out = std::io::BufWriter::new(std::fs::File::create(&tmp).map_err(|e| e.to_string())?);
    serde_json::to_writer(&mut out, value).map_err(|e| e.to_string())?;
    std::io::Write::flush(&mut out).map_err(|e| e.to_string())?;
    drop(out);
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())
}

/// A path as the system finds it: `.` and `..` worked out, trailing dots and spaces dropped as
/// Windows drops them, and links and junctions followed for the part that exists. Two spellings
/// of one folder then compare equal. A file or folder whose name does end in a dot or a space (a
/// share served by Samba, WSL or a Mac can hold one) is kept as named, with the `\\?\` start
/// that stops Windows dropping that end.
pub fn real(p: &Path) -> PathBuf {
    use std::path::Component;
    let (mut lex, mut raw) = (PathBuf::new(), PathBuf::new());
    for c in p.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                lex.pop();
                raw.pop();
            }
            Component::Normal(n) if cfg!(windows) => {
                lex.push(n.to_string_lossy().trim_end_matches(['.', ' ']));
                raw.push(n);
            }
            c => {
                lex.push(c);
                raw.push(c);
            }
        }
    }
    if raw != lex && let Some(v) = verbatim(&raw).filter(|v| v.exists()) {
        lex = v;
    }
    let mut base = lex.clone();
    let mut rest = vec![];
    while !base.exists() {
        match (base.file_name().map(|n| n.to_os_string()), base.parent()) {
            (Some(n), Some(up)) => {
                rest.push(n);
                base = up.to_path_buf();
            }
            _ => return lex,
        }
    }
    let mut out = match std::fs::canonicalize(&base) {
        // Windows answers `\\?\E:\...`; the plain form compares with the rest, unless a name
        // ends in a dot or a space.
        Ok(c) => match c.to_string_lossy().strip_prefix(r"\\?\") {
            Some(r) if !r.starts_with("UNC") && !r.split('\\').any(|n| n.ends_with(['.', ' '])) => PathBuf::from(r),
            _ => c,
        },
        Err(_) => base,
    };
    out.extend(rest.into_iter().rev());
    out
}

/// `p`, a whole path with `.` and `..` worked out, with the `\\?\` start Windows takes each
/// name of as written; `None` for a relative or device path.
pub fn verbatim(p: &Path) -> Option<PathBuf> {
    use std::path::{Component, Prefix};
    let Some(Component::Prefix(x)) = p.components().next() else { return None };
    let t = || p.to_string_lossy().replace('/', r"\");
    Some(PathBuf::from(match x.kind() {
        Prefix::Disk(_) => format!(r"\\?\{}", t()),
        Prefix::UNC(..) => format!(r"\\?\UNC\{}", &t()[2..]),
        Prefix::Verbatim(_) | Prefix::VerbatimUNC(..) | Prefix::VerbatimDisk(_) => return Some(p.to_path_buf()),
        Prefix::DeviceNS(_) => return None,
    }))
}

/// The names each folder lists, kept for one pass; None for a folder that could not be listed whole.
type Listed = std::collections::HashMap<PathBuf, Option<std::collections::HashSet<std::ffi::OsString>>>;

/// Each folder name of `rel` is listed, as written, in the folder above it, from `root` down. A
/// folder that cannot be listed whole (no access, a network blip part way) counts as listing it:
/// only a name surely not there says no. Each folder is listed once per `listed`.
fn listed_as_written(root: &Path, rel: &str, listed: &mut Listed) -> bool {
    let mut dir = root.to_path_buf();
    for name in rel.split('/') {
        let names = listed.entry(dir.clone()).or_insert_with(|| {
            std::fs::read_dir(named(&dir)).ok()?.map(|e| e.map(|e| e.file_name())).collect::<Result<_, _>>().ok()
        });
        match names {
            Some(names) if !names.contains(std::ffi::OsStr::new(name)) => return false,
            Some(_) => dir.push(name),
            None => return true,
        }
    }
    true
}

/// A name in `p` ends in a dot or a space, which Windows drops from a path given without the `\\?\`
/// start.
pub fn ends_oddly(p: &Path) -> bool {
    p.components().any(|c| matches!(c, std::path::Component::Normal(n) if n.to_string_lossy().ends_with(['.', ' '])))
}

/// `p` as the system must be given it to read the file or folder named there. On Windows a whole
/// path with a name ending in a dot or a space gets the `\\?\` start, with its `.` and `..` worked
/// out first, since that start takes each name as written: without it Windows drops that end and
/// reads the sibling named without it. Any other path, and every path on macOS and Linux,
/// as it is.
pub fn named(p: &Path) -> std::borrow::Cow<'_, Path> {
    use std::path::Component;
    if !cfg!(windows) || !ends_oddly(p) {
        return std::borrow::Cow::Borrowed(p);
    }
    let mut whole = PathBuf::new();
    for c in p.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                whole.pop();
            }
            c => whole.push(c),
        }
    }
    match verbatim(&whole).filter(|_| ends_oddly(&whole)) {
        Some(v) => std::borrow::Cow::Owned(v),
        None => std::borrow::Cow::Borrowed(p),
    }
}

/// `p` is `dir` or inside it, compared by whole folder names: with case aside on Windows and macOS
/// (as a default macOS volume does, a case-sensitive one is not told apart yet), and with case
/// as written elsewhere, as Linux names are. `\\?\E:` is the drive `E:`.
pub fn within(p: &Path, dir: &Path) -> bool {
    within_by(p, dir, CASE_ASIDE)
}

/// Folder names are compared with case aside on Windows and macOS (as a default macOS volume does;
/// a case-sensitive one is not told apart yet), and with case as written elsewhere, as Linux
/// names are.
pub const CASE_ASIDE: bool = cfg!(any(windows, target_os = "macos"));

/// Name or path `s` as folder names are compared (`CASE_ASIDE`): in lower case, or as written.
pub fn case(s: &str) -> String {
    if CASE_ASIDE { s.to_lowercase() } else { s.to_string() }
}

/// `within` with case aside on every system, for a check that refuses a path in `dir`: on Linux a
/// drive can still set case aside (a FAT or exFAT stick, a share), so another spelling of `dir`
/// may be `dir` itself.
pub fn within_any_case(p: &Path, dir: &Path) -> bool {
    within_by(p, dir, true)
}

/// `within`, with case aside when `fold` is set.
fn within_by(p: &Path, dir: &Path, fold: bool) -> bool {
    use std::path::{Component, Prefix};
    let case = |s: std::borrow::Cow<str>| if fold { s.to_lowercase() } else { s.into_owned() };
    let part = |c: Component| match c {
        Component::Prefix(x) if matches!(x.kind(), Prefix::VerbatimDisk(_)) => case(x.as_os_str().to_string_lossy()[4..].into()),
        c => case(c.as_os_str().to_string_lossy()),
    };
    let parts = |p: &Path| p.components().map(part).collect::<Vec<_>>();
    let (p, d) = (parts(p), parts(dir));
    !d.is_empty() && p.len() >= d.len() && p[..d.len()] == d[..]
}

pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// The total size of the files in a folder, for the "What this app does" screen.
pub fn folder_size(path: &Path) -> u64 {
    walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(Result::ok)
        .filter_map(|e| e.metadata().ok())
        .filter(|m| m.is_file())
        .map(|m| m.len())
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// On Windows a whole path with a name ending in a dot or a space is read through its `\\?\`
    /// form; any other path is kept as it is.
    #[test]
    fn a_name_ending_in_a_dot_or_a_space_is_read_with_the_verbatim_start() {
        let named = |p: &str| named(Path::new(p)).to_string_lossy().into_owned();
        for plain in [r"Q:\tmp\Books\a.pdf", r"Q:\tmp\x.\..\a.pdf", r"Books.\a.pdf", r"\\?\Q:\tmp\Books.\a.pdf"] {
            assert_eq!(named(plain), plain, "kept as it is");
        }
        if cfg!(windows) {
            assert_eq!(named(r"Q:\tmp\Books.\a.pdf"), r"\\?\Q:\tmp\Books.\a.pdf");
            assert_eq!(named(r"Q:\tmp\Books\a.pdf "), r"\\?\Q:\tmp\Books\a.pdf ");
            assert_eq!(named(r"\\server\share\Books.\a.pdf"), r"\\?\UNC\server\share\Books.\a.pdf");
            assert_eq!(named(r"Q:\tmp\.\x\..\Books.\a.pdf"), r"\\?\Q:\tmp\Books.\a.pdf", "`.` and `..` worked out first");
        } else {
            assert_eq!(named("/tmp/Books./a.pdf"), "/tmp/Books./a.pdf");
        }
    }

    /// Linux names keep their case, so `BOOKS` is not in `Books`; Windows and macOS set case aside
    ///.
    #[test]
    fn within_keeps_case_only_on_linux() {
        let (books, upper) = (Path::new("/media/x/Books"), Path::new("/media/x/BOOKS/f.pdf"));
        assert!(within_by(upper, books, true));
        assert!(!within_by(upper, books, false));
        assert!(within_by(Path::new("/media/x/Books/f.pdf"), books, false));
        assert!(!within_by(Path::new("/media/x/Books2/f.pdf"), books, false));
        assert_eq!(within(upper, books), cfg!(any(windows, target_os = "macos")));
        assert!(within_any_case(upper, books), "a check that refuses sets case aside everywhere");
    }

    /// A damaged data file is set aside and named for the toast at start; one elsewhere is set
    /// aside but not named.
    #[test]
    fn a_damaged_data_file_is_set_aside_and_named() {
        let data = std::env::temp_dir().join(format!("fvh-store-aside-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&data);
        std::fs::create_dir_all(data.join("Cache")).unwrap();
        std::fs::write(data.join("aside-marks.json"), "{not json").unwrap();
        std::fs::write(data.join("Cache").join("aside-index.json"), "{not json").unwrap();
        let m: Marks = load_noting(&data.join("aside-marks.json"), &data);
        let _: Marks = load_noting(&data.join("Cache").join("aside-index.json"), &data);
        assert!(m.marks.is_empty());
        assert!(data.join("aside-marks.json.damaged").exists() && !data.join("aside-marks.json").exists());
        assert!(data.join("Cache").join("aside-index.json.damaged").exists());
        let named = take_set_aside();
        assert!(named.contains(&"aside-marks.json".to_string()), "{named:?}");
        assert!(!named.contains(&"aside-index.json".to_string()), "{named:?}");
        let _ = std::fs::remove_dir_all(&data);
    }

    #[test]
    fn a_kind_change_keeps_the_name_as_it_was() {
        let mut f = CheckedFolder { serial: 1, rel: "Kids Stuff".into(), last_path: String::new(), unchecked: vec![], name: None, split: false, kind: None, drive: None };
        f.set_section(Some("Kids Stuff".into()), Some(Kind::Collections), false);
        assert_eq!((f.name.as_deref(), f.kind), (None, Some(Kind::Collections)), "a name never chosen stays unset");
        f.set_section(Some("Crafts".into()), Some(Kind::Collections), true);
        f.set_section(None, None, false);
        assert_eq!((f.name.as_deref(), f.kind), (Some("Crafts"), None), "guess again keeps a chosen name");
        f.set_section(Some("  ".into()), None, true);
        assert_eq!(f.name, None, "an emptied name goes back to the folder's");
    }

    /// A folder with a Folder kind renamed on disk loses it, once; one under an unticked folder,
    /// or in a Section whose drive is not connected, keeps it (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`).
    #[test]
    fn a_renamed_folder_loses_its_kind_once() {
        let top = Path::new(env!("CARGO_MANIFEST_DIR")).join("../.scratch/folder-kind-rename-test");
        let _ = std::fs::remove_dir_all(&top);
        for d in ["S/Books", "S/Курсы 🎨", "S/Off/Inner"] {
            std::fs::create_dir_all(top.join(d)).unwrap();
        }
        let drive = crate::drives::Drive { path: top.to_string_lossy().into(), label: String::new(), serial: 1, kind: "fixed".into(), free: 0, total: 0, system: false };
        let cf = |serial, rel: &str| CheckedFolder { serial, rel: rel.into(), last_path: String::new(), unchecked: vec!["Off".into()], name: None, split: false, kind: None, drive: None };
        let mut s = Settings { folders: vec![cf(1, "S"), cf(2, "Away")], ..Settings::default() };
        let own = |k| FolderSetting { kind: Some(k), views: Default::default() };
        for (id, k) in [("00000001:S/Books", Kind::Books), ("00000001:S/Курсы 🎨", Kind::Courses), ("00000001:S/Off/Inner", Kind::Files), ("00000002:Away/X", Kind::Books)] {
            s.folder_kinds.insert(id.into(), own(k));
        }
        assert_eq!(s.kinds_in(&s.folders[0]).len(), 3);
        assert!(s.drop_lost_kinds(std::slice::from_ref(&drive)).is_empty(), "every folder is there");
        std::fs::rename(top.join("S/Курсы 🎨"), top.join("S/Курсы")).unwrap();
        std::fs::remove_dir_all(top.join("S/Off")).unwrap();
        assert_eq!(s.drop_lost_kinds(std::slice::from_ref(&drive)), [("S".to_string(), "Курсы 🎨".to_string())]);
        assert!(s.drop_lost_kinds(std::slice::from_ref(&drive)).is_empty(), "said once");
        let mut left: Vec<&str> = s.folder_kinds.keys().map(String::as_str).collect();
        left.sort();
        assert_eq!(left, ["00000001:S/Books", "00000001:S/Off/Inner", "00000002:Away/X"]);
        // The renamed folder is read as the Section's kind: it has none of its own.
        assert!(!s.kinds_in(&s.folders[0]).contains_key("курсы"));
        let _ = std::fs::remove_dir_all(&top);
    }

    /// A folder renamed only in case keeps its kind where case is set aside (one folder, `decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`); on Linux it is another folder, so its kind is lost even on a drive that finds `X`
    /// as `x` (FAT, exFAT, a share, or a Windows drive under WSL, `/mnt/<letter>`). On a
    /// drive that keeps case, `X` and `x` side by side are two folders and neither is lost.
    #[test]
    fn a_folder_renamed_only_in_case_is_lost_only_on_linux() {
        let top = Path::new(env!("CARGO_MANIFEST_DIR")).join("../.scratch/folder-kind-case-test");
        let _ = std::fs::remove_dir_all(&top);
        std::fs::create_dir_all(top.join("Books/X/Inner")).unwrap();
        let drive = crate::drives::Drive { path: top.to_string_lossy().into(), label: String::new(), serial: 1, kind: "fixed".into(), free: 0, total: 0, system: false };
        let cf = CheckedFolder { serial: 1, rel: "Books".into(), last_path: String::new(), unchecked: vec![], name: None, split: false, kind: None, drive: None };
        let mut s = Settings { folders: vec![cf], ..Settings::default() };
        let own = |k| FolderSetting { kind: Some(k), views: Default::default() };
        s.set_folder_kind("00000001:Books/X", own(Kind::Photos));
        s.set_folder_kind("00000001:Books/X/Inner", own(Kind::Files));
        assert!(s.drop_lost_kinds(std::slice::from_ref(&drive)).is_empty(), "every folder is there");
        std::fs::rename(top.join("Books/X"), top.join("Books/x")).unwrap();
        let lost = s.drop_lost_kinds(std::slice::from_ref(&drive));
        if CASE_ASIDE {
            assert!(lost.is_empty(), "x is X");
            assert_eq!(s.folder_kinds.len(), 2);
        } else {
            let mut lost: Vec<&str> = lost.iter().map(|(_, f)| f.as_str()).collect();
            lost.sort();
            assert_eq!(lost, ["Inner", "X"], "X and X/Inner are not there as written");
            assert!(s.folder_kinds.is_empty(), "the old keys are dropped");
        }
        let _ = std::fs::remove_dir_all(&top);
    }

    /// A name is listed only as written, on any drive; a folder that cannot be listed says yes.
    #[test]
    fn a_name_is_listed_only_as_written() {
        let top = Path::new(env!("CARGO_MANIFEST_DIR")).join("../.scratch/listed-as-written-test");
        let _ = std::fs::remove_dir_all(&top);
        std::fs::create_dir_all(top.join("A/Bé")).unwrap();
        let mut listed = Default::default();
        assert!(listed_as_written(&top, "A/Bé", &mut listed));
        assert!(!listed_as_written(&top, "A/bé", &mut listed), "other case");
        assert!(!listed_as_written(&top, "a/Bé", &mut listed), "other case above");
        assert!(listed_as_written(&top.join("Gone"), "A", &mut Default::default()), "cannot be listed");
        let _ = std::fs::remove_dir_all(&top);
    }

    /// On a drive that keeps case, `X` and `x` are two folders: a kind on each stays.
    #[cfg(target_os = "linux")]
    #[test]
    fn two_folders_apart_by_case_keep_their_kinds() {
        let top = std::env::temp_dir().join(format!("fvh-case-kinds-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&top);
        for d in ["Books/X", "Books/x"] {
            std::fs::create_dir_all(top.join(d)).unwrap();
        }
        let drive = crate::drives::Drive { path: top.to_string_lossy().into(), label: String::new(), serial: 1, kind: "fixed".into(), free: 0, total: 0, system: false };
        let cf = CheckedFolder { serial: 1, rel: "Books".into(), last_path: String::new(), unchecked: vec![], name: None, split: false, kind: None, drive: None };
        let mut s = Settings { folders: vec![cf], ..Settings::default() };
        let own = |k| FolderSetting { kind: Some(k), views: Default::default() };
        s.set_folder_kind("00000001:Books/X", own(Kind::Photos));
        s.set_folder_kind("00000001:Books/x", own(Kind::Courses));
        assert!(s.drop_lost_kinds(std::slice::from_ref(&drive)).is_empty(), "both are there");
        std::fs::remove_dir_all(top.join("Books/X")).unwrap();
        assert_eq!(s.drop_lost_kinds(std::slice::from_ref(&drive)), [("Books".to_string(), "X".to_string())]);
        assert_eq!(s.folder_kinds.keys().collect::<Vec<_>>(), ["00000001:Books/x"]);
        let _ = std::fs::remove_dir_all(&top);
    }

    /// A whole drive's Section: its keys are its Drive and the path on it, as a folder's,
    /// so its folders' kinds, a Split-out Section of one and its name all work as for a folder.
    #[test]
    fn a_whole_drive_is_a_section_like_a_folder() {
        let drive = CheckedFolder { serial: 0xAB, rel: String::new(), last_path: if cfg!(windows) { r"W:\".into() } else { "/media/STICK".into() }, unchecked: vec!["Off".into()], name: None, split: false, kind: None, drive: None };
        let books = CheckedFolder { rel: "Books".into(), ..drive.clone() };
        assert_eq!((drive.id().as_str(), books.id().as_str()), ("000000AB:", "000000AB:Books"));
        assert_eq!((inside(&drive.id()), inside(&books.id())), ("000000AB:".to_string(), "000000AB:Books/".to_string()));
        assert_eq!(key_in(&drive.id(), "Books/a.pdf"), "000000AB:Books/a.pdf");
        assert_eq!(key_in(&drive.id(), ""), "000000AB:");
        assert_eq!(under("000000AB:Books/a.pdf", &drive.id()), Some("Books/a.pdf"));
        assert_eq!(under("000000AB:", &drive.id()), None);
        assert_eq!(rel_in("000000ab:BOOKS", &drive.id()).as_deref(), Some("BOOKS"));
        assert_eq!(rel_in("000000AB:", &drive.id()), None, "the drive itself is no folder inside it");
        assert_eq!(drive.rel_of(&books).as_deref(), Some("Books"));
        assert_eq!(rel_in("000000ABC:x", &drive.id()), None, "another serial's id is not inside it");
        // A folder named like a drive id is still a folder.
        assert_eq!(inside("000000AB:Books/0000000A:"), "000000AB:Books/0000000A:/");

        let mut s = Settings { folders: vec![drive.clone()], ..Settings::default() };
        s.set_folder_kind("000000AB:Books", FolderSetting { kind: Some(Kind::Books), views: Default::default() });
        assert_eq!(s.kinds_in(&drive).get(&case("Books")), Some(&Kind::Books));
        // Split out, its id is the folder's key, as a tick of that folder alone would give.
        let id = s.split_out("000000AB:Books", Kind::Books, None).unwrap();
        assert_eq!(id, books.id());
        assert_eq!(s.folders[1].rel, "Books");
        assert_eq!(s.folders[1].last_path, if cfg!(windows) { r"W:\Books" } else { "/media/STICK/Books" });
        assert_eq!(s.container_of(&s.folders[1]).map(CheckedFolder::id), Some(drive.id()));
        assert_eq!(s.splits_in(&drive), ["Books"]);
        assert!(s.split_out("000000AB:Off", Kind::Files, None).is_err(), "an unticked folder is not split out");

        // Its name: the drive's label, else its letter or mount point.
        let named = |d: &str| CheckedFolder { drive: Some(d.into()), ..drive.clone() }.shown_name();
        if cfg!(target_os = "linux") {
            assert_eq!([named("My stick 📚"), named("/media/STICK")], ["My stick 📚", "/media/STICK"]);
        } else {
            assert_eq!([named(r"W: Книги и фото"), named(r"W:\")], ["Книги и фото", "W:"]);
        }
        assert_eq!(CheckedFolder { name: Some("Mine".into()), ..drive.clone() }.shown_name(), "Mine");
        assert_eq!(drive.shown_name(), if cfg!(windows) { "W:" } else { "/media/STICK" }, "no drive name kept: its path");
    }

    /// Keys of a whole drive's Section saved as `{serial}:/path` in an earlier version take the form a
    /// read gives them now; a key already in the new form keeps its own value.
    #[test]
    fn old_whole_drive_keys_are_rewritten() {
        let drive = CheckedFolder { serial: 0xAB, rel: String::new(), last_path: "/media/STICK".into(), unchecked: vec![], name: None, split: false, kind: None, drive: None };
        let folder = CheckedFolder { serial: 0xCD, rel: "Books".into(), ..drive.clone() };
        // A Split-out Section made from it in an earlier version kept `rel` "/Split".
        let split = CheckedFolder { rel: "/Split".into(), split: true, ..drive.clone() };
        let mut s = Settings { folders: vec![drive, folder, split], ..Settings::default() };
        s.offered.insert("000000AB:/Split".into());
        s.ui = serde_json::json!({ "views": { "000000AB:/Split": { "section": "grid" } }, "merge": { "000000AB:/Split": "000000CD:Books" }, "sort": { "000000AB:/Split": "-name", "home": "added" }, "size": "normal" });
        let d = s.whole_drives();
        assert_eq!(d, ["000000AB:"]);
        assert_eq!(rekey("000000AB:/Books/a", &d).as_deref(), Some("000000AB:Books/a"));
        assert_eq!(rekey("000000AB:Books/a", &d), None);
        assert_eq!(rekey("000000CD:Books/a", &d), None);
        let own = |k| FolderSetting { kind: Some(k), views: Default::default() };
        s.folder_kinds.insert("000000AB:/Books".into(), own(Kind::Books));
        s.folder_kinds.insert("000000CD:Books/Old".into(), own(Kind::Files));
        s.move_to.insert("000000AB:/Photos".into(), "/x".into());
        assert_eq!(s.rekey(&d), 7);
        assert_eq!(s.kinds_in(&s.folders[0]).get(&case("Books")), Some(&Kind::Books));
        assert_eq!(s.folders[2].id(), "000000AB:Split");
        assert_eq!(s.container_of(&s.folders[2]).map(CheckedFolder::id).as_deref(), Some("000000AB:"));
        assert!(s.offered.contains("000000AB:Split"));
        assert_eq!(s.ui["views"]["000000AB:Split"]["section"], "grid");
        assert_eq!(s.ui["merge"]["000000AB:Split"], "000000CD:Books");
        assert_eq!(s.ui["sort"]["000000AB:Split"], "-name");
        assert_eq!(s.ui["sort"]["home"], "added");
        assert_eq!(s.rekey(&d), 0, "once only");
        assert!(s.folder_kinds.contains_key("000000CD:Books/Old") && s.move_to.contains_key("000000AB:Photos"));
        let mut m = Marks::default();
        let mark = |at| Mark { m: "done".into(), at };
        m.marks.insert("000000AB:/Books/a".into(), mark(1));
        m.marks.insert("000000AB:/Books/b".into(), mark(1));
        m.marks.insert("000000AB:Books/b".into(), mark(2));
        m.review.insert("000000AB:/Photos/x.jpg".into(), Choice { keep: false, at: 1, size: Some(3) });
        assert_eq!(m.rekey(&d), 3);
        assert_eq!(m.marks.len(), 2);
        assert_eq!(m.marks["000000AB:Books/b"].at, 2, "the new key's own value stays");
        assert!(m.marks.contains_key("000000AB:Books/a") && m.review.contains_key("000000AB:Photos/x.jpg"));
        assert_eq!(m.rekey(&d), 0, "once only");
    }

    /// Where case is set aside, a folder renamed only in case gets one setting, and Same as clears
    /// it (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`); on Linux that is another folder (`folder_settings_keep_case_only_on_linux`).
    #[test]
    fn a_folder_kind_is_one_setting_whatever_its_case() {
        let cf = CheckedFolder { serial: 1, rel: "İstanbul".into(), last_path: String::new(), unchecked: vec![], name: None, split: false, kind: None, drive: None };
        let mut s = Settings { folders: vec![cf.clone()], ..Settings::default() };
        let own = |k| FolderSetting { kind: Some(k), views: Default::default() };
        s.set_folder_kind("00000001:İstanbul/books", own(Kind::Books));
        if !CASE_ASIDE {
            return;
        }
        assert_eq!(rel_in("00000001:i̇stanbul/Books", &cf.id()).as_deref(), Some("Books"), "a longer lower case does not shift the cut");
        assert_eq!(rel_in("00000001:İstanbul", &cf.id()), None, "the Section's own folder is no folder inside it");
        s.set_folder_kind("00000001:İSTANBUL/Books", own(Kind::Courses));
        assert_eq!(s.folder_kinds.len(), 1);
        assert_eq!(s.kinds_in(&cf).get("books"), Some(&Kind::Courses));
        s.set_folder_kind("00000001:İstanbul/BOOKS", FolderSetting::default());
        assert!(s.folder_kinds.is_empty(), "Same as clears it");
    }

    /// Put back keeps the Split-out Section's kind and views as its Folder kind; the Sections split
    /// out of it stay, now out of the Section above (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`).
    #[test]
    fn put_back_keeps_kind_and_views() {
        let cf = CheckedFolder { serial: 1, rel: "S".into(), last_path: r"E:\S".into(), unchecked: vec!["Media/Sub/Off".into()], name: None, split: false, kind: None, drive: Some("E:".into()) };
        let mut s = Settings { folders: vec![cf.clone()], ..Settings::default() };
        let media = s.split_out("00000001:S/Media", Kind::Courses, None).unwrap();
        let sub = s.split_out("00000001:S/Media/Sub", Kind::Photos, None).unwrap();
        assert_eq!((s.folders[1].last_path.as_str(), s.folders[2].last_path.as_str()), (r"E:\S\Media", r"E:\S\Media\Sub"));
        assert_eq!(s.folders[2].unchecked, ["Off"]);
        let views = std::collections::BTreeMap::from([("folder".to_string(), "tiles".to_string())]);
        assert!(s.put_back("00000001:S", Kind::Books, views.clone()).is_err(), "no Split-out Section");
        assert_eq!(s.put_back(&media, Kind::Courses, views.clone()).unwrap(), "00000001:S");
        assert_eq!(s.folder_kinds.get(&media), Some(&FolderSetting { kind: Some(Kind::Courses), views }));
        assert_eq!(s.kinds_in(&cf).get(&case("Media")), Some(&Kind::Courses));
        assert_eq!(s.folders.iter().map(CheckedFolder::id).collect::<Vec<_>>(), ["00000001:S", sub.as_str()]);
        assert_eq!(s.container_of(&s.folders[1]).map(CheckedFolder::id).as_deref(), Some("00000001:S"));
        // Split out again, its Folder kind goes: a folder has one setting, as a folder or as a Section.
        s.split_out(&media, Kind::Courses, Some(1)).unwrap();
        assert!(s.folder_kinds.is_empty());
        assert_eq!(s.folders[1].id(), media, "Undo puts it back in its place, so the others keep their colours");
    }

    /// Folder settings compare names as `case` does: on Linux `Books/X` and `Books/x` are two
    /// folders, each with a setting of its own, and `Books/off` is not the unticked `Books/Off`; on
    /// Windows and macOS each pair is one folder.
    #[test]
    fn folder_settings_keep_case_only_on_linux() {
        let cf = CheckedFolder { serial: 1, rel: "Books".into(), last_path: "/media/x/Books".into(), unchecked: vec!["Off".into()], name: None, split: false, kind: None, drive: None };
        let mut s = Settings { folders: vec![cf.clone()], ..Settings::default() };
        let own = |k| FolderSetting { kind: Some(k), views: Default::default() };
        s.set_folder_kind("00000001:Books/X", own(Kind::Photos));
        let kinds = s.kinds_in(&cf);
        assert_eq!(crate::scan::kind_at(&kinds, Kind::Books, "X/a"), Kind::Photos);
        assert_eq!(crate::scan::kind_at(&kinds, Kind::Books, "x/a"), if CASE_ASIDE { Kind::Photos } else { Kind::Books }, "a kind on X applies to x only with case aside");
        assert_eq!(rel_in("00000001:BOOKS/X", &cf.id()).is_some(), CASE_ASIDE, "BOOKS/X is in Books only with case aside");
        assert_eq!(rel_in("00000001:Books/X", "00000001:Books").as_deref(), Some("X"));
        s.set_folder_kind("00000001:Books/x", own(Kind::Courses));
        if CASE_ASIDE {
            assert_eq!(s.folder_kinds.len(), 1, "one folder, one setting");
            assert_eq!(s.kinds_in(&cf).get("x"), Some(&Kind::Courses));
            assert!(s.split_out("00000001:Books/off", Kind::Files, None).is_err(), "unticked as Off");
        } else {
            assert_eq!(s.folder_kinds.len(), 2, "two folders, two settings");
            let kinds = s.kinds_in(&cf);
            assert_eq!((kinds.get("X"), kinds.get("x")), (Some(&Kind::Photos), Some(&Kind::Courses)));
            s.split_out("00000001:Books/x", Kind::Files, None).unwrap();
            assert_eq!(s.folder_kinds.keys().collect::<Vec<_>>(), ["00000001:Books/X"], "splitting x out keeps X's kind");
            assert!(s.split_out("00000001:BOOKS", Kind::Files, None).is_err(), "BOOKS is not in Books");
            assert!(s.split_out("00000001:Books/x", Kind::Files, None).is_err(), "a Section already");
            s.split_out("00000001:Books/off", Kind::Files, None).unwrap();
            assert!(s.drop_loose_splits().is_empty(), "off is ticked: only Off is not");
        }
    }

    /// A Split-out Section goes with the folder holding it: unticked, or removed (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`).
    #[test]
    fn a_split_out_section_goes_with_its_folder() {
        let cf = CheckedFolder { serial: 1, rel: "S".into(), last_path: String::new(), unchecked: vec![], name: None, split: false, kind: None, drive: None };
        let mut s = Settings { folders: vec![cf], ..Settings::default() };
        s.split_out("00000001:S/A", Kind::Files, None).unwrap();
        s.split_out("00000001:S/A/B", Kind::Files, None).unwrap();
        s.split_out("00000001:S/C", Kind::Files, None).unwrap();
        assert!(s.drop_loose_splits().is_empty());
        s.folders[0].unchecked.push("c".into());
        if CASE_ASIDE {
            assert_eq!(s.drop_loose_splits(), ["00000001:S/C"], "its folder unticked, case aside");
        } else {
            assert!(s.drop_loose_splits().is_empty(), "`c` is another folder on Linux");
            s.folders[0].unchecked.push("C".into());
            assert_eq!(s.drop_loose_splits(), ["00000001:S/C"], "its folder unticked");
        }
        s.folders.remove(0);
        let mut gone = s.drop_loose_splits();
        gone.sort();
        assert_eq!(gone, ["00000001:S/A", "00000001:S/A/B"], "the folder holding them removed, nested ones too");
    }

    /// Saving the tree twice keeps one of each Split-out Section, on a connected Drive or not.
    #[test]
    fn saving_the_tree_keeps_each_split_out_section_once() {
        let cf = |serial: u32| CheckedFolder { serial, rel: "S".into(), last_path: String::new(), unchecked: vec![], name: None, split: false, kind: None, drive: None };
        let mut s = Settings { folders: vec![cf(1), cf(2)], ..Settings::default() };
        s.split_out("00000001:S/A", Kind::Files, None).unwrap();
        s.split_out("00000002:S/B", Kind::Files, None).unwrap();
        let here = crate::drives::Drive { path: if cfg!(windows) { r"E:\".into() } else { "/e".into() }, label: String::new(), serial: 1, kind: "fixed".into(), free: 0, total: 0, system: false };
        let off = crate::drives::join_on_drive(&here, "S/A/Off").to_string_lossy().to_string();
        for _ in 0..2 {
            // As set_folders does: the tree's ticks on connected Drives, the splits, then the rest.
            let mut folders = vec![s.folders[0].clone()];
            s.carry_splits(&mut folders, std::slice::from_ref(&here), std::slice::from_ref(&off));
            folders.extend(s.folders.iter().filter(|f| f.serial != 1).cloned());
            s.folders = folders;
            assert!(s.drop_loose_splits().is_empty());
        }
        let mut ids: Vec<String> = s.folders.iter().map(CheckedFolder::id).collect();
        ids.sort();
        assert_eq!(ids, ["00000001:S", "00000001:S/A", "00000002:S", "00000002:S/B"]);
        assert_eq!(s.folders.iter().find(|f| f.id() == "00000001:S/A").unwrap().unchecked, ["Off"]);
    }

    #[test]
    fn a_backup_adds_marks_and_the_later_mark_stays() {
        let mut m: Marks = serde_json::from_str(r#"{"v":2,"marks":{"a":{"m":"next","at":5},"b":{"m":"done","at":9}},"ticks":{"a":{"x":7}}}"#).unwrap();
        let b: Marks = serde_json::from_str(r#"{"v":2,"marks":{"a":{"m":"done","at":8},"b":{"m":"next","at":3},"c":{"m":"next","at":1}},"ticks":{"a":{"x":2,"y":4}}}"#).unwrap();
        m.merge(b);
        assert_eq!(m.marks["a"].m, "done");
        assert_eq!(m.marks["b"].m, "done");
        assert_eq!(m.marks["c"].m, "next");
        assert_eq!(m.ticks["a"]["x"], 2);
        assert_eq!(m.ticks["a"]["y"], 4);
    }

    #[test]
    fn two_works_opened_in_one_second_keep_their_order() {
        let mut m = Marks::default();
        m.open("a", 100);
        m.open("b", 100);
        assert!(m.opened["b"] > m.opened["a"]);
        m.open("a", 500);
        assert_eq!(m.opened["a"], 500);
    }

    #[test]
    fn a_backup_adds_times_opened_and_the_later_one_stays() {
        let mut m: Marks = serde_json::from_str(r#"{"v":2,"marks":{},"opened":{"a":5,"b":9}}"#).unwrap();
        let b: Marks = serde_json::from_str(r#"{"v":2,"marks":{},"opened":{"a":8,"b":3,"c":1}}"#).unwrap();
        m.merge(b);
        assert_eq!(m.opened, HashMap::from([("a".into(), 8), ("b".into(), 9), ("c".into(), 1)]));
        m.carry("c", "a");
        assert_eq!(m.opened, HashMap::from([("a".into(), 8), ("b".into(), 9)]));
        m.forget(&["b".into()], &[]);
        assert_eq!(m.opened, HashMap::from([("a".into(), 8)]));
        // A file with none opened stays as it was written in an earlier version.
        assert!(!serde_json::to_string(&Marks::default()).unwrap().contains("opened"));
    }

    #[test]
    fn a_backup_adds_review_choices_and_the_later_one_stays() {
        let mut m: Marks = serde_json::from_str(r#"{"v":2,"marks":{},"review":{"a":{"keep":true,"at":5},"b":{"keep":false,"at":9}}}"#).unwrap();
        let b: Marks = serde_json::from_str(r#"{"v":2,"marks":{},"review":{"a":{"keep":false,"at":8},"b":{"keep":true,"at":3},"c":{"keep":false,"at":1}}}"#).unwrap();
        m.merge(b);
        assert!(!m.review["a"].keep && !m.review["b"].keep && !m.review["c"].keep);
    }

    #[test]
    fn version_1_reads_with_no_ticks() {
        let m: Marks = serde_json::from_str(r#"{"v":1,"marks":{"a":{"m":"done","at":5}}}"#).unwrap();
        assert_eq!(m.marks["a"].at, 5);
        assert!(m.ticks.is_empty());
    }

    #[test]
    fn ticking_again_keeps_the_date() {
        let mut m = Marks::default();
        m.ticks.insert("w".into(), HashMap::from([("1.mp4".into(), 7)]));
        m.tick("w", &["1.mp4".into(), "2.mp4".into()], true);
        assert_eq!(m.ticks["w"]["1.mp4"], 7);
        assert_eq!(m.ticks["w"].len(), 2);
        m.tick("w", &["1.mp4".into(), "2.mp4".into()], false);
        assert!(!m.ticks.contains_key("w"));
    }

    #[test]
    fn carrying_keeps_what_the_other_work_has() {
        let mut m = Marks::default();
        m.marks.insert("old".into(), Mark { m: "done".into(), at: 1 });
        m.ticks.insert("old".into(), HashMap::from([("a".into(), 1), ("b".into(), 1)]));
        m.ticks.insert("new".into(), HashMap::from([("a".into(), 9)]));
        m.carry("old", "new");
        assert_eq!(m.marks["new"].m, "done");
        assert_eq!(m.ticks["new"]["a"], 9);
        assert_eq!(m.ticks["new"]["b"], 1);
        assert!(!m.marks.contains_key("old") && !m.ticks.contains_key("old"));
    }

    #[test]
    fn forgetting_one_tick_drops_an_empty_work() {
        let mut m = Marks::default();
        m.ticks.insert("w".into(), HashMap::from([("a".into(), 1)]));
        m.forget(&[], &[("w".into(), "a".into())]);
        assert!(m.ticks.is_empty());
    }
}
