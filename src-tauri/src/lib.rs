mod chosen;
mod covers;
mod details;
mod drives;
mod logs;
mod moves;
mod packed;
mod pdf;
mod photos;
mod programs;
mod scan;
mod shots;
mod store;
mod thumbs;
mod tools;
mod web;
#[cfg(test)]
mod qa_tests;

use scan::Catalog;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use store::{CheckedFolder, Kind, Marks, Settings};
use thumbs::Thumbs;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const BUILD: &str = env!("FVH_BUILD");
pub const PDF_FLAG: &str = pdf::FLAG;

/// WebView2 left alone connects to Google, Cloudflare and Microsoft servers on its own (secure
/// DNS checks, settings and component downloads). The window loads nothing from the web, so its
/// background connections are turned off and every web address goes to a proxy that does not
/// exist (`decisions/2026/10/05/nothing-hidden-no-network-by-default.md`). The first part is wry's own default, which these arguments replace.
#[cfg(windows)]
const WEBVIEW_ARGS: &str = "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection,DnsOverHttps,msEdgeDNSOverHttps \
    --disable-background-networking --disable-component-update --disable-domain-reliability --no-pings \
    --proxy-server=http://127.0.0.1:9 --host-resolver-rules=\"MAP * ~NOTFOUND, EXCLUDE localhost\"";

/// The second copy of the app that reads one PDF's cover (see `pdf.rs`).
pub fn pdf_helper(args: &[String]) -> i32 {
    pdf::helper(args)
}

struct AppState {
    settings: Mutex<Settings>,
    /// Held while the catalog is saved (`save_catalog`): a command that takes it runs off the
    /// main thread, `#[tauri::command(async)]`, so the window never waits for a save.
    catalog: Mutex<Catalog>,
    /// Raised, under the catalog's lock, each time its Works change: a part the page asks for
    /// names the catalog it began with.
    catalog_stamp: std::sync::atomic::AtomicU64,
    marks: Mutex<Marks>,
    /// One scan at a time.
    scanning: std::sync::atomic::AtomicBool,
    /// Section settings' previews: a preview started before the last `stop_preview` ends early.
    preview_gen: std::sync::atomic::AtomicU64,
    thumbs: Arc<Thumbs>,
    details: Arc<details::Details>,
    shots: Arc<shots::Shots>,
    /// What the Review window shows: its events, title, files and the Photo it opens at,
    /// as the main window asked, read by that window when it opens or is asked again.
    review: Mutex<Option<serde_json::Value>>,
    /// Held while the Review window is looked for and made, so two quick requests make one window.
    review_window: Mutex<()>,
}

fn err(e: impl ToString) -> String {
    e.to_string()
}

/// A Drive as the page gets it: with whether its folder names keep their case, as `store::case`
/// compares them, so the tree and the folder settings compare them as the core does.
#[derive(Serialize)]
struct PageDrive {
    #[serde(flatten)]
    drive: drives::Drive,
    keeps_case: bool,
}

#[tauri::command]
fn drives() -> Vec<PageDrive> {
    drives::list().into_iter().map(|drive| PageDrive { drive, keeps_case: !store::CASE_ASIDE }).collect()
}

#[tauri::command]
fn folders(path: String) -> Result<Vec<drives::Folder>, String> {
    drives::folders(Path::new(&path)).map_err(err)
}

#[tauri::command]
fn settings(state: State<AppState>) -> Settings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
fn set_ui(state: State<AppState>, ui: serde_json::Value) -> Result<(), String> {
    let mut s = state.settings.lock().unwrap();
    s.ui = ui;
    store::save(&store::settings_file(), &*s)
}

/// The tree's ticks, as full paths on the drives connected now.
#[derive(Serialize)]
struct Picked {
    checked: Vec<String>,
    unchecked: Vec<String>,
}

#[tauri::command]
fn picked(state: State<AppState>) -> Picked {
    let ds = drives::list();
    let s = state.settings.lock().unwrap();
    let mut p = Picked { checked: vec![], unchecked: vec![] };
    // A Split-out Section is no tick of its own: it is ticked as part of the folder holding it.
    for cf in s.folders.iter().filter(|f| !f.split) {
        let base = match ds.iter().find(|d| d.serial == cf.serial) {
            Some(d) => drives::join_on_drive(d, &cf.rel),
            None => PathBuf::from(&cf.last_path),
        };
        p.unchecked.extend(cf.unchecked.iter().map(|u| u.split('/').fold(base.clone(), |a, x| a.join(x)).to_string_lossy().to_string()));
        p.checked.push(base.to_string_lossy().into());
    }
    p
}

/// Path `p` as `set_folders` compares ticks: as names compare on this system (`store::case`), so
/// `Books` and `BOOKS` are two folders on Linux, with no separator at its end.
fn tick_key(p: &str) -> String {
    store::case(p).trim_end_matches(std::path::MAIN_SEPARATOR).to_string()
}

/// The top-most of the ticked folders `checked`, each once: a folder inside another ticked one is
/// part of it, unless it is on another mounted drive (`mount`).
fn top_ticks<'a>(checked: &'a [String], mount: &dyn Fn(&str) -> Option<String>) -> Vec<&'a String> {
    let sep = std::path::MAIN_SEPARATOR;
    let mut tops: Vec<&String> = checked.iter().filter(|c| !checked.iter().any(|o| o != *c && tick_key(c).starts_with(&format!("{}{sep}", tick_key(o))) && mount(c) == mount(o))).collect();
    tops.sort_by_key(|c| tick_key(c));
    tops.dedup_by_key(|c| tick_key(c));
    tops
}

/// The folders of `unchecked` inside ticked folder `c`, as paths from it with `/`, cut letter by
/// letter as `store::rel_in` cuts, so a letter whose lower case is longer or shorter never shifts it.
fn unticked_in(c: &str, unchecked: &[String]) -> Vec<String> {
    let sep = std::path::MAIN_SEPARATOR;
    let slash = |p: &str| p.trim_end_matches(sep).replace(sep, "/");
    unchecked.iter().filter_map(|u| store::rel_in(&slash(u), &slash(c))).collect()
}

/// Saves the tree's ticks. A folder already checked keeps its name and kind.
#[tauri::command]
fn set_folders(state: State<AppState>, checked: Vec<String>, unchecked: Vec<String>) -> Result<Settings, String> {
    let ds = drives::list();
    let cache = state.thumbs.dir();
    let mut s = state.settings.lock().unwrap();
    let mount = |p: &str| drives::mount_of(&ds, Path::new(p)).map(|d| d.path.clone());
    let mut folders = vec![];
    for c in top_ticks(&checked, &mount) {
        let path = PathBuf::from(c);
        let Some(d) = drives::drive_of(&ds, &path) else { continue };
        let rel = drives::rel_on_drive(d, &path);
        // All of the system drive or Computer is never a Section, only folders on it.
        if rel.is_empty() && ds.iter().any(|x| x.system && x.serial == d.serial) {
            continue;
        }
        let un = unticked_in(c, &unchecked);
        let old = s.folders.iter().find(|f| f.serial == d.serial && tick_key(&f.rel) == tick_key(&rel));
        // The app's own folders are never read as part of a checked folder (`decisions/2026/10/05/app-data-local-cache-movable.md`, `decisions/2026/10/07/app-data-in-storeddata-next-to-the-app.md`), nor
        // the drives mounted inside it, which are drives of their own.
        let mut un = un;
        let here = mount(c);
        let mounted = ds.iter().filter(|x| !drives::is_home(x) && Some(&x.path) != here.as_ref()).map(|x| PathBuf::from(&x.path));
        for mine in [store::data_dir(), cache.clone()].into_iter().chain(mounted) {
            if store::within_any_case(&mine, &path) && !store::within_any_case(&path, &mine) {
                let inside: Vec<String> = mine.components().skip(path.components().count()).map(|c| c.as_os_str().to_string_lossy().to_string()).collect();
                let inside = inside.join("/");
                if !un.iter().any(|u| tick_key(u) == tick_key(&inside)) {
                    un.push(inside);
                }
            }
        }
        // Two drives with one serial (two shares of one server's disk) would be one Section.
        if folders.iter().any(|f: &CheckedFolder| f.serial == d.serial && tick_key(&f.rel) == tick_key(&rel)) {
            log::warn!("{c} is left out: it has the id of a folder ticked on another drive with the same serial");
            continue;
        }
        folders.push(CheckedFolder {
            serial: d.serial,
            rel,
            last_path: c.clone(),
            unchecked: un,
            name: old.and_then(|o| o.name.clone()),
            kind: old.and_then(|o| o.kind),
            drive: Some(scan::drive_name(d)),
            split: false,
        });
    }
    s.carry_splits(&mut folders, &ds, &unchecked);
    // Folders on a Drive that is not connected stay checked: the tree could not show them.
    for f in s.folders.iter().filter(|f| !ds.iter().any(|d| d.serial == f.serial)) {
        folders.push(f.clone());
    }
    s.folders = folders;
    for id in s.drop_loose_splits() {
        log::info!("{id} is no Section of its own any more: the folder holding it is not ticked");
    }
    s.prune_offered();
    store::save(&store::settings_file(), &*s)?;
    Ok(s.clone())
}

/// What a part asked of a catalog changed since says: the page starts again from the heads.
const CHANGED: &str = "The catalog changed while it was sent.";

/// The catalog's Sections without their Works and unlisted files, with how many of each it has;
/// the page asks for them in parts (`catalog_part`, `catalog_unlisted`), so no reply nears the
/// longest string the page holds.
#[tauri::command(async)]
fn catalog(state: State<AppState>) -> Result<tauri::ipc::Response, String> {
    heads_of(&state)
}

fn heads_of(state: &AppState) -> Result<tauri::ipc::Response, String> {
    let c = state.catalog.lock().unwrap();
    let stamp = state.catalog_stamp.load(std::sync::atomic::Ordering::SeqCst);
    Ok(tauri::ipc::Response::new(serde_json::to_string(&packed::heads(&c, stamp)).map_err(err)?))
}

/// The Works of Section `id` from `from`, `packed::PART` of them at most, packed; an error when
/// the catalog is no longer the one numbered `stamp`.
#[tauri::command(async)]
fn catalog_part(state: State<AppState>, stamp: u64, id: String, from: usize) -> Result<tauri::ipc::Response, String> {
    let c = state.catalog.lock().unwrap();
    if state.catalog_stamp.load(std::sync::atomic::Ordering::SeqCst) != stamp {
        return Err(CHANGED.into());
    }
    let part = packed::part(&c, &id, from).ok_or(CHANGED)?;
    Ok(tauri::ipc::Response::new(serde_json::to_string(&part).map_err(err)?))
}

/// The unlisted files of Section `id` from `from`, `packed::UNLISTED_PART` of them at most,
/// packed; an error when the catalog is no longer the one numbered `stamp`.
#[tauri::command(async)]
fn catalog_unlisted(state: State<AppState>, stamp: u64, id: String, from: usize) -> Result<tauri::ipc::Response, String> {
    let c = state.catalog.lock().unwrap();
    if state.catalog_stamp.load(std::sync::atomic::Ordering::SeqCst) != stamp {
        return Err(CHANGED.into());
    }
    let part = packed::unlisted_part(&c, &id, from).ok_or(CHANGED)?;
    Ok(tauri::ipc::Response::new(serde_json::to_string(&part).map_err(err)?))
}

#[derive(Serialize, Clone)]
struct Progress {
    section: String,
    files: usize,
}

/// Reads every checked folder again, or only the Section `only`, or only its folder `under`
/// (folder titles from the Section down). A Section on a Drive that is not connected keeps what
/// was read last time, shown as offline.
#[tauri::command]
async fn scan(app: AppHandle, only: Option<String>, under: Option<Vec<String>>) -> Result<tauri::ipc::Response, String> {
    let state = app.state::<AppState>();
    use std::sync::atomic::Ordering;
    if state.scanning.swap(true, Ordering::SeqCst) {
        return Err("A scan is already running.".into());
    }
    struct Done<'a>(&'a std::sync::atomic::AtomicBool);
    impl Drop for Done<'_> {
        fn drop(&mut self) {
            self.0.store(false, Ordering::SeqCst);
        }
    }
    let _done = Done(&state.scanning);
    // Each checked folder leaves out the Split-out Sections inside it, as it does an unticked folder (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`).
    let (folders, kinds, gap) = {
        let s = state.settings.lock().unwrap();
        let folders = s.folders.iter().map(|f| s.as_read(f)).collect::<Vec<_>>();
        (folders, s.folders.iter().map(|f| s.kinds_in(f)).collect::<Vec<_>>(), i64::from(s.photo_gap.max(1)) * 3600)
    };
    let old = state.catalog.lock().unwrap().clone();
    let cache = state.thumbs.dir();
    let app2 = app.clone();
    let sections = tauri::async_runtime::spawn_blocking(move || {
        let ds = drives::list();
        let mut out = vec![];
        let mine = [store::real(&store::data_dir()), store::real(&cache)];
        for (cf, kinds) in folders.iter().zip(&kinds) {
            let cf = &own_folders_left_out(cf, &ds, &mine);
            if only.as_ref().is_some_and(|o| *o != cf.id()) && let Some(prev) = old.sections.iter().find(|s| s.id == cf.id()) {
                out.push(prev.clone());
                continue;
            }
            let name = cf.shown_name();
            let _ = app2.emit("scan-progress", Progress { section: name.clone(), files: 0 });
            let started = std::time::Instant::now();
            let progress = |n: usize| {
                let _ = app2.emit("scan-progress", Progress { section: name.clone(), files: n });
            };
            let prev = old.sections.iter().find(|s| s.id == cf.id());
            let part = match (&under, prev) {
                (Some(u), Some(p)) if !u.is_empty() => scan::scan_under(cf, &ds, p, u, kinds, Some(&cache), gap, &progress),
                _ => None,
            };
            if let Some(sec) = part {
                let u = under.as_deref().unwrap_or_default();
                log::info!("read {} again in {}: {} Works in {:.1} s{}", u.join(" / "), sec.path, sec.works.len(), started.elapsed().as_secs_f32(), scan::cloud_note(&sec, u));
                out.push(sec);
                continue;
            }
            match scan::scan_folder(cf, &ds, Some(&cache), gap, kinds, &progress) {
                Some(sec) => {
                    log::info!("scanned {} ({} files, {} Works) in {:.1} s{}", sec.path, sec.files, sec.works.len(), started.elapsed().as_secs_f32(), scan::cloud_note(&sec, &[]));
                    out.push(sec)
                }
                None => {
                    // The drives as they are now: one unplugged during the read is not connected, not missing.
                    let sec = scan::not_read(cf, &drives::list(), old.sections.iter().find(|s| s.id == cf.id()));
                    log::info!("{} {}", cf.last_path, if sec.gone { "is not there any more" } else { "is not connected" });
                    out.push(sec);
                }
            }
        }
        out
    })
    .await
    .map_err(err)?;
    // A catalog kept in an older version (`packed::migrate`) stays so.
    let mut cat = Catalog { v: old.v, sections };
    // A Section renamed while it was read (rename_section) keeps its new name.
    {
        let s = state.settings.lock().unwrap();
        for cf in &s.folders {
            if let Some(sec) = cat.sections.iter_mut().find(|x| x.id == cf.id()) {
                // A whole drive's name follows its label as read now.
                let drive = if cf.rel.is_empty() && sec.online { Some(sec.drive.clone()) } else { cf.drive.clone() };
                sec.name = CheckedFolder { drive, unchecked: vec![], ..cf.clone() }.shown_name();
            }
        }
        scan::link_sections(&mut cat, &s);
    }
    store::save(&store::catalog_file(), &cat)?;
    // Raised before the catalog changes, so a drive check listed before this read is dropped.
    READS.fetch_add(1, Ordering::SeqCst);
    {
        let mut c = state.catalog.lock().unwrap();
        *c = cat.clone();
        state.catalog_stamp.fetch_add(1, Ordering::SeqCst);
    }
    // A folder with a Folder kind renamed or moved on disk loses it, and the page says so once.
    let lost = {
        let mut s = state.settings.lock().unwrap();
        let lost = s.drop_lost_kinds(&drives::list());
        if !lost.is_empty() {
            store::save(&store::settings_file(), &*s)?;
        }
        lost
    };
    if !lost.is_empty() {
        let _ = app.emit("kinds-lost", lost);
    }
    state.thumbs.plan(&cat);
    state.details.plan(&cat);
    state.shots.plan(&cat);
    heads_of(&state)
}

/// How many reads have changed the catalog: a drive check whose listing is older than a read is
/// dropped, since the read saw the drives later.
static READS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Marks the Sections on a Drive unplugged since they were read as not connected, with no folder
/// read and no network drive asked; the page asks when its window gets focus and every 10 s.
/// The ids marked, for the page to grey; none while the folders are read, since that read decides.
#[tauri::command]
async fn drives_away(app: AppHandle) -> Result<Vec<String>, String> {
    use std::sync::atomic::{AtomicBool, Ordering};
    // A drive slow to answer can hold the listing up: one check at a time.
    static CHECKING: AtomicBool = AtomicBool::new(false);
    let state = app.state::<AppState>();
    if state.scanning.load(Ordering::SeqCst) || CHECKING.swap(true, Ordering::SeqCst) {
        return Ok(vec![]);
    }
    let reads = READS.load(Ordering::SeqCst);
    let ds = tauri::async_runtime::spawn_blocking(drives::present).await;
    CHECKING.store(false, Ordering::SeqCst);
    let ds = ds.map_err(err)?;
    let folders = state.settings.lock().unwrap().folders.clone();
    let (away, cat) = {
        let mut c = state.catalog.lock().unwrap();
        if state.scanning.load(Ordering::SeqCst) || READS.load(Ordering::SeqCst) != reads {
            return Ok(vec![]);
        }
        let away = scan::mark_away(&mut c, &folders, &ds);
        if away.is_empty() {
            return Ok(away);
        }
        for sec in c.sections.iter().filter(|s| away.contains(&s.id)) {
            log::info!("{} is not connected", sec.path);
        }
        store::save(&store::catalog_file(), &*c)?;
        (away, c.clone())
    };
    state.thumbs.plan(&cat);
    state.details.plan(&cat);
    state.shots.plan(&cat);
    Ok(away)
}

/// Stops showing a checked folder: one gone from its Drive, or on a Drive that is not connected,
/// which the folder tree cannot show to untick. Its Marks stay, as Orphaned Marks. Off the main
/// thread, as the catalog's save takes a second on a large one.
#[tauri::command(async)]
fn forget_folder(state: State<AppState>, id: String) -> Result<tauri::ipc::Response, String> {
    // The catalog's lock before the settings', so a command waiting on a save holds neither;
    // a read started after this check takes the settings after this change.
    let mut c = state.catalog.lock().unwrap();
    let mut s = state.settings.lock().unwrap();
    if state.scanning.load(std::sync::atomic::Ordering::SeqCst) {
        return Err("The folders are being read. Try again when that is done.".into());
    }
    s.folders.retain(|f| f.id() != id);
    // The Split-out Sections made from its folders go with it.
    let mut gone = s.drop_loose_splits();
    gone.push(id.clone());
    s.prune_offered();
    store::save(&store::settings_file(), &*s)?;
    c.sections.retain(|x| !gone.contains(&x.id));
    state.catalog_stamp.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    scan::link_sections(&mut c, &s);
    drop(s);
    save_catalog(&store::catalog_file(), &c)?;
    log::info!("stopped showing {id}");
    drop(c);
    heads_of(&state)
}

/// Saves the catalog while its lock is still held, after the change made under it: two changes in
/// flight then reach the file in the order they were made, and the file never holds an older
/// catalog than the app shows. A command that changes the catalog takes its lock before the
/// settings' and lets the settings go first, so the commands still on the main thread that take
/// them never wait for this save; those that take the catalog's lock run off the main thread.
fn save_catalog(file: &Path, c: &std::sync::MutexGuard<Catalog>) -> Result<(), String> {
    store::save(file, &**c)
}

/// A checked folder with StoredData and the cache added to what it leaves out, found by real
/// path, so a junction or another spelling of the folder never makes the app read its own files
/// (`decisions/2026/10/05/app-data-local-cache-movable.md`, `decisions/2026/10/07/app-data-in-storeddata-next-to-the-app.md`).
fn own_folders_left_out(cf: &CheckedFolder, ds: &[drives::Drive], mine: &[PathBuf]) -> CheckedFolder {
    let mut cf = cf.clone();
    let Some(d) = ds.iter().find(|d| d.serial == cf.serial) else { return cf };
    let root = store::real(&drives::join_on_drive(d, &cf.rel));
    for m in mine {
        if store::within_any_case(m, &root) && !store::within_any_case(&root, m) {
            let inside = m.components().skip(root.components().count()).map(|c| c.as_os_str().to_string_lossy().to_string()).collect::<Vec<_>>().join("/");
            if !cf.unchecked.iter().any(|u| u.eq_ignore_ascii_case(&inside)) {
                cf.unchecked.push(inside);
            }
        }
    }
    cf
}

/// Renames a Section in place: its name in the settings and in the catalog, with no read. An
/// empty name gives it its folder's name again, as a read would. Off the main thread, as
/// `forget_folder`.
#[tauri::command(async)]
fn rename_section(state: State<AppState>, id: String, name: Option<String>) -> Result<tauri::ipc::Response, String> {
    // The catalog's lock before the settings', so a command waiting on a save holds neither;
    // a read started after this check takes the settings after this change.
    let mut c = state.catalog.lock().unwrap();
    let mut s = state.settings.lock().unwrap();
    if state.scanning.load(std::sync::atomic::Ordering::SeqCst) {
        return Err("The folders are being read. Try again when that is done.".into());
    }
    let f = s.folders.iter_mut().find(|f| f.id() == id).ok_or("no such Section")?;
    let kind = f.kind;
    f.set_section(name, kind, true);
    let shown = f.shown_name();
    store::save(&store::settings_file(), &*s)?;
    if let Some(sec) = c.sections.iter_mut().find(|x| x.id == id) {
        sec.name = shown;
    }
    // A Split-out Section's name, cleared, follows its row again.
    scan::link_sections(&mut c, &s);
    drop(s);
    save_catalog(&store::catalog_file(), &c)?;
    drop(c);
    heads_of(&state)
}

/// Ends every Section settings preview still running: the dialog closed.
#[tauri::command]
fn stop_preview(state: State<AppState>) {
    state.preview_gen.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
}

#[tauri::command]
fn set_section(state: State<AppState>, id: String, name: Option<String>, kind: Option<Kind>, rename: bool) -> Result<Settings, String> {
    let mut s = state.settings.lock().unwrap();
    let f = s.folders.iter_mut().find(|f| f.id() == id).ok_or("no such Section")?;
    f.set_section(name, kind, rename);
    store::save(&store::settings_file(), &*s)?;
    Ok(s.clone())
}

/// What Section settings shows for a kind: the cards, as many as `show`, of all `total`.
#[derive(Serialize)]
struct KindPreview {
    kind: Kind,
    total: usize,
    works: Vec<scan::Work>,
    /// Every card's title: a Books Section tells authors apart by the names met across it.
    titles: Vec<String>,
    /// The thumbnails already made for the cards shown.
    thumbs: Vec<thumbs::Thumb>,
    /// Works with a Mark or Ticks that would be set aside: gone, or with a ticked item gone.
    marks: usize,
    /// Works with a Mark or Ticks that come back: Orphaned now, made again by this kind.
    back: usize,
    /// The cards are what a read makes; false for a catalog kept before every folder was kept.
    exact: bool,
}

/// A Section's cards as `kind` (None: the kind the app would guess), from the files read last
/// time and the thumbnails already made. Reads no folder and saves nothing. `stop_preview`
/// ends it early, with an error.
#[tauri::command]
async fn preview_kind(app: AppHandle, id: String, kind: Option<Kind>, show: usize) -> Result<KindPreview, String> {
    let state = app.state::<AppState>();
    let started = state.preview_gen.load(std::sync::atomic::Ordering::SeqCst);
    let sec = state.catalog.lock().unwrap().sections.iter().find(|s| s.id == id).cloned().ok_or("no such Section")?;
    let (gap, kinds) = preview_settings(&state, &id)?;
    let marks = state.marks.lock().unwrap().clone();
    let cache = state.thumbs.dir();
    let app2 = app.clone();
    let (kind, works, set_aside, exact) = tauri::async_runtime::spawn_blocking(move || {
        let stop = || app2.state::<AppState>().preview_gen.load(std::sync::atomic::Ordering::SeqCst) != started;
        let (kind, works, exact) = scan::preview_or_stop(&sec, kind, Some(&cache), gap, &kinds, &stop).ok_or("stopped")?;
        Ok::<_, String>((kind, works.clone(), marks_moved(&sec, &works, &marks), exact))
    })
    .await
    .map_err(err)??;
    let shown: Vec<scan::Work> = works.iter().take(show).cloned().collect();
    Ok(KindPreview {
        kind,
        total: works.len(),
        titles: if kind == Kind::Books { works.iter().map(|w| w.title.clone()).collect() } else { vec![] },
        thumbs: state.thumbs.made(&shown),
        works: shown,
        marks: set_aside.0,
        back: set_aside.1,
        exact,
    })
}

/// The Photos gap in seconds and a Section's Folder kinds, as a read would use them.
fn preview_settings(state: &AppState, id: &str) -> Result<(i64, scan::Kinds), String> {
    let s = state.settings.lock().unwrap();
    let cf = s.folders.iter().find(|f| f.id() == id).ok_or("no such Section")?;
    Ok((i64::from(s.photo_gap.max(1)) * 3600, s.kinds_in(cf)))
}

/// How many of a Section's Works with a Mark or Ticks `works` would set aside, as Settings, Marks
/// finds orphans (a Work gone, or a ticked item no longer in its Work), and how many Orphaned ones
/// it brings back. A Section whose drive is not connected keeps its cards until it is read, so none.
fn marks_moved(sec: &scan::Section, works: &[scan::Work], marks: &Marks) -> (usize, usize) {
    if !sec.online {
        return (0, 0);
    }
    let now: HashSet<&str> = sec.works.iter().map(|w| w.id.as_str()).collect();
    let back = works.iter().filter(|w| !now.contains(w.id.as_str()) && (marks.marks.contains_key(&w.id) || marks.ticks.contains_key(&w.id))).count();
    let keys: HashMap<&str, HashSet<&str>> = works.iter().map(|w| (w.id.as_str(), w.items.iter().map(|i| if i.key.is_empty() { i.rel.as_str() } else { i.key.as_str() }).collect())).collect();
    let aside = sec
        .works
        .iter()
        .filter(|w| marks.marks.contains_key(&w.id) || marks.ticks.contains_key(&w.id))
        .filter(|w| match keys.get(w.id.as_str()) {
            None => true,
            Some(k) => marks.ticks.get(&w.id).is_some_and(|t| t.keys().any(|x| !k.contains(x.as_str()))),
        })
        .count();
    (aside, back)
}

/// What Folder settings shows for a kind (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`): a folder's cards with `kind` as its Folder
/// kind, None for Same as the folder above, from the files read last time. `rel` is the folder's
/// path from the Section's folder; `more` are other folders of the Section given a kind at the same
/// time (a Photos Section's Folders with other things), counted with it. Counts the cards in
/// those folders and the Marks set aside in the Section; reads no folder and saves nothing.
/// `stop_preview` ends it early, with an error.
#[tauri::command]
async fn preview_folder(app: AppHandle, id: String, rel: String, kind: Option<Kind>, more: Option<std::collections::BTreeMap<String, Option<Kind>>>) -> Result<KindPreview, String> {
    let state = app.state::<AppState>();
    let started = state.preview_gen.load(std::sync::atomic::Ordering::SeqCst);
    let sec = state.catalog.lock().unwrap().sections.iter().find(|s| s.id == id).cloned().ok_or("no such Section")?;
    let (gap, mut kinds) = preview_settings(&state, &id)?;
    let set: Vec<(String, Option<Kind>)> = std::iter::once((rel.clone(), kind)).chain(more.unwrap_or_default()).collect();
    // A Photos Section shows no folders: there only its top-level folders take a kind, and the
    // folders inside one with another kind (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`).
    let photos = sec.kind == Kind::Photos;
    if photos && set.iter().any(|(r, _)| r.split_once('/').is_some_and(|(top, _)| scan::other_kind(&kinds, top).is_none())) {
        return Err("A Photos Section shows no folders.".into());
    }
    // The kind it then has: its own, or the one above it.
    let at = kind.unwrap_or_else(|| scan::kind_at(&kinds, sec.kind, rel.rsplit_once('/').map_or("", |(up, _)| up)));
    for (r, k) in &set {
        match k {
            Some(k) => kinds.insert(r.to_lowercase(), *k),
            None => kinds.remove(&r.to_lowercase()),
        };
    }
    let marks = state.marks.lock().unwrap().clone();
    let cache = state.thumbs.dir();
    let app2 = app.clone();
    // Their Works, and their Events when one is a Photos folder or, in a Photos Section, its own
    // Events with a picture in one.
    let dir = |r: &str| r.split('/').fold(PathBuf::from(&sec.path), |p, x| p.join(x));
    let places: Vec<(String, String, PathBuf)> = set.iter().map(|(r, _)| (format!("{}/", store::key_in(&id, r)), store::key_in(&id, r), dir(r))).collect();
    let (works, set_aside, exact) = tauri::async_runtime::spawn_blocking(move || {
        let stop = || app2.state::<AppState>().preview_gen.load(std::sync::atomic::Ordering::SeqCst) != started;
        let (_, works, exact) = scan::preview_or_stop(&sec, Some(sec.kind), Some(&cache), gap, &kinds, &stop).ok_or("stopped")?;
        let n = marks_moved(&sec, &works, &marks);
        let own = |w: &scan::Work| photos && w.event && scan::place_of(w) == id;
        let inside = |w: &scan::Work| places.iter().any(|(prefix, me, d)| w.id.starts_with(prefix) || scan::place_of(w) == me || (own(w) && w.items.iter().any(|i| store::within(std::path::Path::new(&i.path), d))));
        Ok::<_, String>((works.iter().filter(|w| inside(w)).count(), n, exact))
    })
    .await
    .map_err(err)??;
    Ok(KindPreview { kind: at, total: works, works: vec![], titles: vec![], thumbs: vec![], marks: set_aside.0, back: set_aside.1, exact })
}

/// Sets a folder's own kind and views inside a Section (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`), by its id: Drive and path, as
/// `{serial}:{path on the Drive}`. No kind and no view drops its entry. A kind change is read
/// again by the page after, which puts the old one back if that read fails.
#[tauri::command]
fn set_folder(state: State<AppState>, id: String, kind: Option<Kind>, views: std::collections::BTreeMap<String, String>) -> Result<Settings, String> {
    let mut s = state.settings.lock().unwrap();
    if !s.folders.iter().any(|f| store::rel_in(&id, &f.id()).is_some()) {
        return Err("This folder is not inside a checked folder.".into());
    }
    s.set_folder_kind(&id, store::FolderSetting { kind, views });
    store::save(&store::settings_file(), &*s)?;
    log::info!("folder {id}: kind {}", kind.map_or("same as above".into(), |k| format!("{k:?}")));
    Ok(s.clone())
}

/// Makes the folder with key `id` (`{Section id}/{path}`) a Split-out Section of kind `kind` (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`), last among the Sections or at `at`, and returns its id. The page then reads again the Section it came from, which reads the
/// new one too, as it has never been read.
#[tauri::command(async)]
fn split_folder(state: State<AppState>, id: String, kind: Kind, at: Option<usize>) -> Result<String, String> {
    // The catalog's lock first, as `rename_section`.
    let c = state.catalog.lock().unwrap();
    let mut s = state.settings.lock().unwrap();
    if state.scanning.load(std::sync::atomic::Ordering::SeqCst) {
        return Err("The folders are being read. Try again when that is done.".into());
    }
    let from = s.folders.iter().filter(|f| store::rel_in(&id, &f.id()).is_some()).max_by_key(|f| f.rel.len()).ok_or("This folder is not inside a checked folder.")?;
    if !drives::list().iter().any(|d| d.serial == from.serial) {
        return Err("Its drive is not connected.".into());
    }
    // A folder whose files are part of a card outside it (a book's format folder) is no place of its own.
    if let Some(sec) = c.sections.iter().find(|x| x.id == from.id()) && scan::splits_a_work(sec, &id) {
        return Err("Its files are part of a card outside it.".into());
    }
    let new = s.split_out(&id, kind, at)?;
    store::save(&store::settings_file(), &*s)?;
    log::info!("{new} is a Section of its own now ({kind:?})");
    Ok(new)
}

/// The top-level folders of Section `id` the app offers a kind of their own (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`), from the
/// files read last time: none once the offer was answered or put away, unless `again`. Reads no
/// folder and changes nothing.
#[tauri::command]
async fn suggest_kinds(app: AppHandle, id: String, again: bool) -> Result<Vec<scan::Suggestion>, String> {
    let state = app.state::<AppState>();
    let sec = state.catalog.lock().unwrap().sections.iter().find(|s| s.id == id).cloned().ok_or("no such Section")?;
    let s = state.settings.lock().unwrap().clone();
    tauri::async_runtime::spawn_blocking(move || scan::offer(&s, &sec, again)).await.map_err(err)
}

/// Keeps that the offer of a kind per folder for Section `id` was answered or put away, or with
/// `offered` false, that it is made again (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`).
#[tauri::command]
fn set_offered(state: State<AppState>, id: String, offered: bool) -> Result<Settings, String> {
    let mut s = state.settings.lock().unwrap();
    if offered {
        s.offered.insert(id);
    } else {
        s.offered.remove(&id);
    }
    store::save(&store::settings_file(), &*s)?;
    Ok(s.clone())
}

/// Puts Split-out Section `id` back into the Section it came from as a folder with the Folder kind
/// `kind` and the views `views` (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`), and returns that Section's id, which the page reads again.
#[tauri::command(async)]
fn put_back(state: State<AppState>, id: String, kind: Kind, views: std::collections::BTreeMap<String, String>) -> Result<String, String> {
    // The catalog's lock first, as `rename_section`.
    let c = state.catalog.lock().unwrap();
    let mut s = state.settings.lock().unwrap();
    if state.scanning.load(std::sync::atomic::Ordering::SeqCst) {
        return Err("The folders are being read. Try again when that is done.".into());
    }
    if !s.folders.iter().any(|f| f.id() == id && drives::list().iter().any(|d| d.serial == f.serial)) {
        return Err("Its drive is not connected.".into());
    }
    let into = s.put_back(&id, kind, views.clone())?;
    s.prune_offered();
    // Photos inside a Photos Section is its own kind: the folder keeps only its views (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`).
    if kind == Kind::Photos && c.sections.iter().any(|x| x.id == into && x.kind == Kind::Photos) {
        s.set_folder_kind(&id, store::FolderSetting { kind: None, views });
    }
    store::save(&store::settings_file(), &*s)?;
    log::info!("{id} is back in {into} as a folder ({kind:?})");
    Ok(into)
}

/// The thumbnails made so far and how many are left; new ones arrive as `thumbs` events.
#[tauri::command]
fn thumbs(state: State<AppState>) -> thumbs::Batch {
    state.thumbs.ready()
}

/// Makes these Works' thumbnails first: the cards on screen.
#[tauri::command]
fn want_thumbs(state: State<AppState>, ids: Vec<String>) {
    state.thumbs.want(ids);
}

/// Lengths and page counts read so far, by file path; new ones arrive as `details` events.
#[tauri::command]
fn details(state: State<AppState>) -> std::collections::HashMap<String, details::Detail> {
    state.details.ready()
}

/// Makes these Works' thumbnails again, including those that gave none before.
#[tauri::command(async)]
fn rebuild_thumbs(state: State<AppState>, ids: Vec<String>) {
    let cat = state.catalog.lock().unwrap().clone();
    state.thumbs.forget(&cat, &ids.into_iter().collect());
    state.thumbs.plan(&cat);
}

/// Choose a cover… (`decisions/2026/10/22/covers-are-chosen-never-fetched-by-themselves.md`). Its ids are a Work's and its copies' on other drives, or a folder
/// tile's (`dir:` and its place). Nothing here goes online but `find_covers`, `cover_preview` and
/// `use_web_cover`, each only when the person presses a button for it.
fn b64(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// Runs a step that may wait on the network or the disk off the command threads.
async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Result<T, String> + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(err)?
}

/// Plans the thumbnails again after a cover was chosen or removed.
fn replan(state: &AppState) {
    let cat = state.catalog.lock().unwrap().clone();
    state.thumbs.plan(&cat);
}

/// The search words the picker starts with, from a Work's or folder's name. Sends nothing.
#[tauri::command]
fn cover_words(name: String) -> String {
    web::words_for(&name)
}

/// Searches Open Library and FantLab for these words: the person pressed Search.
#[tauri::command]
async fn find_covers(words: String) -> Result<Vec<web::Results>, String> {
    blocking(move || Ok(web::search(&words))).await
}

/// Starts a new round of small pictures: a new search, or the picker closed. Pictures still
/// waiting from an earlier round are not fetched.
#[tauri::command]
fn cover_round() -> u64 {
    web::next_round()
}

/// A cover's small picture to choose by, as a `data:` address (the window loads nothing from
/// the web itself), while `round` is the picker's.
#[tauri::command]
async fn cover_preview(url: String, round: u64) -> Result<String, String> {
    blocking(move || Ok(format!("data:image/jpeg;base64,{}", b64(&web::picture(&url, "small cover picture to choose from, in Choose a cover", Some(round))?)))).await
}

/// Fetches the cover picked from a search in full and keeps it as these ids' cover.
#[tauri::command]
async fn use_web_cover(app: AppHandle, ids: Vec<String>, url: String, from: String) -> Result<(), String> {
    if from != web::OPEN_LIBRARY && from != web::FANTLAB {
        return Err(format!("not a cover source: {from}"));
    }
    blocking(move || {
        let bytes = web::picture(&url, "the cover picked in Choose a cover", None)?;
        app.state::<AppState>().thumbs.chosen.lock().unwrap().writable()?;
        let c = chosen::store(&store::data_dir(), &bytes, &from)?;
        let state = app.state::<AppState>();
        state.thumbs.chosen.lock().unwrap().set(&ids, c)?;
        log::info!("cover from {from} chosen for {}", ids.join(", "));
        replan(&state);
        Ok(())
    })
    .await
}

/// A picture file the person picks for a cover, as it would be kept, for the picker to show
/// before Use it. None when cancelled.
#[tauri::command]
async fn pick_cover_file(app: AppHandle) -> Result<Option<String>, String> {
    let d = app.dialog().file().add_filter("Pictures", &["jpg", "jpeg", "png", "webp", "gif", "bmp", "tif", "tiff"]);
    let Some(f) = d.blocking_pick_file() else { return Ok(None) };
    let path = f.into_path().map_err(err)?;
    blocking(move || {
        let bytes = std::fs::read(store::named(&path)).map_err(err)?;
        Ok(Some(b64(&chosen::checked(&bytes)?)))
    })
    .await
}

/// A pasted picture (base64) as it would be kept, for the picker to show before Use it; an
/// error says it is no picture the app can read.
#[tauri::command]
async fn read_cover(data: String) -> Result<String, String> {
    blocking(move || {
        use base64::Engine;
        let bytes = base64::engine::general_purpose::STANDARD.decode(data.trim()).map_err(err)?;
        Ok(b64(&chosen::checked(&bytes)?))
    })
    .await
}

/// Keeps a picture of one's own (picked or pasted, as base64) as these ids' cover.
#[tauri::command]
async fn keep_cover(app: AppHandle, ids: Vec<String>, data: String) -> Result<(), String> {
    blocking(move || {
        use base64::Engine;
        let bytes = base64::engine::general_purpose::STANDARD.decode(data.trim()).map_err(err)?;
        app.state::<AppState>().thumbs.chosen.lock().unwrap().writable()?;
        let c = chosen::store(&store::data_dir(), &bytes, chosen::OWN)?;
        let state = app.state::<AppState>();
        state.thumbs.chosen.lock().unwrap().set(&ids, c)?;
        log::info!("a picture of one's own chosen as the cover of {}", ids.join(", "));
        replan(&state);
        Ok(())
    })
    .await
}

/// True when any of these ids has a chosen cover: the picker then offers Remove this cover.
#[tauri::command]
fn has_cover(state: State<AppState>, ids: Vec<String>) -> bool {
    state.thumbs.chosen.lock().unwrap().any(&ids)
}

/// Removes the cover chosen for these ids: they show the cover their files give again. Gives each
/// id's cover removed, for Undo.
#[tauri::command(async)]
fn remove_cover(state: State<AppState>, ids: Vec<String>) -> Result<std::collections::BTreeMap<String, chosen::Chosen>, String> {
    let gone = state.thumbs.chosen.lock().unwrap().remove(&ids)?;
    if !gone.is_empty() {
        log::info!("chosen cover of {} removed", ids.join(", "));
        replan(&state);
    }
    Ok(gone)
}

/// Undo of Remove this cover: each id gets its cover back, unless it was given another since.
#[tauri::command(async)]
fn restore_cover(state: State<AppState>, covers: std::collections::BTreeMap<String, chosen::Chosen>) -> Result<(), String> {
    let ids: Vec<String> = covers.keys().cloned().collect();
    state.thumbs.chosen.lock().unwrap().restore(covers)?;
    log::info!("chosen cover of {} put back", ids.join(", "));
    replan(&state);
    Ok(())
}

/// Review's notes for every Photos Section (`decisions/2026/10/04/photos-take-no-marks-review-is-the-progress.md`); new measures arrive as `shots` events with
/// how many pictures are left to read.
#[tauri::command(async)]
fn photo_sort(state: State<AppState>) -> shots::Sorting {
    let cat = state.catalog.lock().unwrap().clone();
    state.shots.sorting(&cat)
}

/// Keeps or sets aside Photos in Review, as (key, file size): the Section id, `/`, the path in it.
#[tauri::command]
fn set_review(state: State<AppState>, photos: Vec<(String, u64)>, keep: bool) -> Result<Marks, String> {
    let mut m = state.marks.lock().unwrap();
    let at = store::now();
    for (k, size) in photos {
        m.review.insert(k, store::Choice { keep, at, size: Some(size) });
    }
    store::save(&store::marks_file(), &*m)?;
    Ok(m.clone())
}

/// Opens Review in its own window, or shows the one open with what is asked now. Async, as
/// a window made in a command that is not deadlocks on Windows.
#[tauri::command]
async fn open_review(app: AppHandle, request: serde_json::Value) -> Result<(), String> {
    let state = app.state::<AppState>();
    *state.review.lock().unwrap() = Some(request);
    // A window being made is not found yet: a second request waits here until it is.
    let _one = state.review_window.lock().unwrap();
    if let Some(w) = app.get_webview_window("review") {
        let _ = w.unminimize();
        w.show().map_err(err)?;
        w.set_focus().map_err(err)?;
        return w.emit("review-request", ()).map_err(err);
    }
    let main = app.config().app.windows[0].clone();
    let window = tauri::WebviewWindowBuilder::new(&app, "review", tauri::WebviewUrl::App("index.html?review".into()))
        .title("Review - FilesViewHub")
        .inner_size(main.width, main.height)
        .min_inner_size(main.min_width.unwrap_or(900.0), main.min_height.unwrap_or(560.0))
        .decorations(false)
        .resizable(true)
        .data_directory(store::webview_dir());
    #[cfg(windows)]
    let window = window.additional_browser_args(WEBVIEW_ARGS);
    window.build().map_err(err)?;
    Ok(())
}

/// What the Review window is asked to show.
#[tauri::command]
fn review_request(state: State<AppState>) -> Option<serde_json::Value> {
    state.review.lock().unwrap().clone()
}

/// Forgets Review choices, so their Photos wait for Review again.
#[tauri::command]
fn forget_review(state: State<AppState>, keys: Vec<String>) -> Result<Marks, String> {
    let mut m = state.marks.lock().unwrap();
    m.forget_review(&keys);
    store::save(&store::marks_file(), &*m)?;
    Ok(m.clone())
}

/// Puts Review choices back as they were, date and size too: Undo of Review again. A choice made
/// since stays.
#[tauri::command]
fn restore_review(state: State<AppState>, choices: HashMap<String, store::Choice>) -> Result<Marks, String> {
    let mut m = state.marks.lock().unwrap();
    for (k, c) in choices {
        if m.review.get(&k).is_none_or(|now| now.at < c.at) {
            m.review.insert(k, c);
        }
    }
    store::save(&store::marks_file(), &*m)?;
    Ok(m.clone())
}

/// Forgets the Review choice of each photo an Undo put back (`moves::put_back`, read before the
/// marks are locked, as it may ask the drives), when the choice is older than its move, so it
/// waits for Review again. Returns how many it forgot.
fn forget_put_back(m: &mut Marks, back: &[(String, i64)]) -> usize {
    let mut n = 0;
    for (k, at) in back {
        if m.review.get(k).is_some_and(|c| c.at as i64 <= *at) {
            m.review.remove(k);
            n += 1;
        }
    }
    n
}

/// Saves the script that moves a Photos Section's set-aside photos into `Not needed`, for the
/// system the app runs on, where the person picks; never inside a checked folder (`decisions/2026/10/04/sorting-ends-in-a-move-script-never-a-delete.md`, `decisions/2026/10/04/a-move-script-for-each-system.md`,
/// `decisions/2026/10/05/app-data-local-cache-movable.md`). Takes only the photos ticked in Move (`files`, paths inside the photo folder). Returns
/// where, or None when they cancelled.
#[tauri::command]
async fn save_move_script(app: AppHandle, id: String, files: Vec<String>) -> Result<Option<String>, String> {
    let state = app.state::<AppState>();
    let (root, photos, _) = set_aside_of(&state, &id, Some(&files.into_iter().collect()))?;
    let files: Vec<String> = photos.into_iter().map(|p| p.rel).collect();
    if files.is_empty() {
        return Err("No photo is marked Not needed here.".into());
    }
    let (name, ext) = if cfg!(windows) { ("Move Not needed photos.ps1", "ps1") } else { ("Move Not needed photos.sh", "sh") };
    let mut d = app.dialog().file().set_file_name(name).add_filter("Script", &[ext]);
    if let Some(dir) = app.path().download_dir().ok().filter(|d| !in_checked(d, &checked_paths(&state))) {
        d = d.set_directory(dir);
    }
    let Some(f) = d.blocking_save_file() else { return Ok(None) };
    let path = f.into_path().map_err(err)?;
    let real = store::real(&path);
    if checked_paths(&state).iter().any(|c| store::within_any_case(&real, &store::real(c))) {
        return Err("Save the script outside your checked folders: the app never writes into them.".into());
    }
    std::fs::write(&path, shots::move_script(&shots::script_root(&root, &files), &files, cfg!(windows))).map_err(err)?;
    log::info!("move script for {} files saved to {}", files.len(), path.display());
    Ok(Some(path.to_string_lossy().into()))
}

/// The folder of a Photos Section, or of a folder with the Folder kind Photos (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`), by its
/// id: the Section's id, or that and `/` and its path in the Section, as its Events' ids start. As
/// its drive is found now, never a drive letter remembered from before, with its photos set aside
/// in Review (only those in `only`, when given), then the Insta360 previews that go with them. Also
/// returns how many previews there are.
fn set_aside_of(state: &AppState, id: &str, only: Option<&HashSet<String>>) -> Result<(PathBuf, Vec<moves::Photo>, usize), String> {
    // The settings let go before the catalog is taken: never both at once here.
    let folders = state.settings.lock().unwrap().folders.clone();
    let cat = state.catalog.lock().unwrap();
    let sec = cat.sections.iter().filter(|s| id == s.id || id.starts_with(&store::inside(&s.id))).max_by_key(|s| s.id.len()).ok_or("no such Section")?;
    let cf = folders.iter().find(|f| f.id() == sec.id).cloned().ok_or("no such Section")?;
    let drive = drives::list().into_iter().find(|d| d.serial == cf.serial).filter(|_| sec.online);
    let Some(d) = drive else { return Err("This Section's drive is not connected. Connect it and read the folders again first.".into()) };
    let marks = state.marks.lock().unwrap();
    let (photos, n) = shots::to_move(sec, id, &marks.review, only);
    let root = store::under(id, &sec.id).unwrap_or_default().split('/').filter(|x| !x.is_empty()).fold(drives::join_on_drive(&d, &cf.rel), |p, x| p.join(x));
    Ok((root, photos, n))
}

/// Where Review's move puts a Section's photos, and what it would take (`decisions/2026/10/09/review-moves-set-aside-photos-itself-with-undo.md`).
#[derive(serde::Serialize)]
struct MovePlan {
    /// The photos set aside, and how many of them are only in the cloud and stay.
    count: usize,
    cloud: usize,
    /// Each photo set aside, for the list Move ticks them in: its path inside the photo folder, how
    /// many Insta360 previews go with its shot (on the shot's first video), and the shot, or "".
    /// A preview moves only when every video of its shot is ticked.
    photos: Vec<(String, usize, String)>,
    /// The Insta360 previews (.lrv) that go with the set-aside videos; not in `count`.
    previews: usize,
    /// The folder they go into, and whether the person picked it.
    to: String,
    picked: bool,
    /// The synced folder (Dropbox, OneDrive) the photos are in when the folder they go into is
    /// outside it: to the sync client the move is a deletion.
    leaves_sync: Option<String>,
    /// The last move Undo can still take back: its id, when, how many and where.
    last: Option<(u64, i64, usize, String)>,
}

/// The last move of a Section that Undo can still take back: its id, when, how many and where.
#[tauri::command]
fn last_move(id: String) -> Option<(u64, i64, usize, String)> {
    moves::last(&moves::journal_file(), &id).map(|m| (m.id, m.at, m.files.iter().filter(|e| e.state == moves::State::Moved && !e.preview).count(), m.to))
}

#[tauri::command(async)]
fn move_plan(state: State<AppState>, id: String) -> Result<MovePlan, String> {
    let (root, photos, previews) = set_aside_of(&state, &id, None)?;
    let marked = &photos[..photos.len() - previews];
    // A preview goes with the videos in its folder that have its Insta360 key: the shot, two
    // videos on a dual-lens camera. Its previews are counted on the shot's first video only.
    let key = |rel: &str| { let (d, n) = rel.rsplit_once('/').unwrap_or(("", rel)); scan::insta360_key(n).map(|k| (d.to_string(), k)) };
    let shot = |p: &moves::Photo| key(&p.rel).filter(|_| p.rel.to_lowercase().ends_with(".insv"));
    let mut seen = HashSet::new();
    let list = marked
        .iter()
        .map(|p| {
            let k = shot(p);
            let n = k.as_ref().filter(|k| seen.insert((*k).clone())).map_or(0, |k| photos[marked.len()..].iter().filter(|v| key(&v.rel).as_ref() == Some(k)).count());
            (p.rel.clone(), n, k.map(|(d, k)| format!("{d}/{k}")).unwrap_or_default())
        })
        .collect();
    let picked = state.settings.lock().unwrap().move_to.get(&id).cloned();
    let to = picked.clone().map_or_else(|| root.join(shots::NOT_NEEDED), PathBuf::from);
    let leaves_sync = moves::sync_root(&root).filter(|(_, r)| !store::within(&store::real(&to), &store::real(r))).map(|(n, _)| n);
    let last = last_move(id.clone());
    Ok(MovePlan { count: marked.len(), cloud: marked.iter().filter(|p| p.cloud).count(), photos: list, previews, to: to.to_string_lossy().into(), picked: picked.is_some(), leaves_sync, last })
}

/// A folder the person picked for Review's move: not a whole drive, not in or around a checked
/// folder, not the app's own. Checked again at each move, as folders can be checked after.
fn check_move_to(state: &AppState, p: &Path) -> Result<(), String> {
    let real = store::real(p);
    if real.parent().is_none() {
        return Err("Pick a folder, not a whole drive.".into());
    }
    if checked_paths(state).iter().map(|c| store::real(c)).any(|c| store::within_any_case(&real, &c) || store::within_any_case(&c, &real)) {
        return Err("Pick a folder outside your checked folders, so the photos are not read again as photos.".into());
    }
    if store::within_any_case(&real, &store::real(&store::data_dir())) || store::within_any_case(&real, &store::real(&state.thumbs.dir())) {
        return Err("Pick a folder outside the app's own StoredData and cache.".into());
    }
    Ok(())
}

/// Sets the folder a Section's set-aside photos move into, or None for `Not needed` inside the
/// photo folder. A folder inside a checked folder, or holding one, or the app's own is refused.
#[tauri::command(async)]
fn set_move_to(state: State<AppState>, id: String, path: Option<String>) -> Result<MovePlan, String> {
    if let Some(p) = &path {
        check_move_to(&state, Path::new(p))?;
    }
    {
        let mut s = state.settings.lock().unwrap();
        match path {
            Some(p) => s.move_to.insert(id.clone(), p),
            None => s.move_to.remove(&id),
        };
        store::save(&store::settings_file(), &*s)?;
    }
    move_plan(state, id)
}

#[derive(Clone, serde::Serialize)]
struct MoveProgress {
    done: usize,
    total: usize,
}

/// Moves a Section's set-aside photos ticked in Move (`files`, paths inside the photo folder)
/// where `move_plan` says (`decisions/2026/10/09/review-moves-set-aside-photos-itself-with-undo.md`). The Section is read again by the page after.
#[tauri::command]
async fn move_set_aside(app: AppHandle, id: String, files: Vec<String>) -> Result<moves::Outcome, String> {
    let state = app.state::<AppState>();
    let (root, photos, _) = set_aside_of(&state, &id, Some(&files.into_iter().collect()))?;
    if photos.is_empty() {
        return Err("No photo is marked Not needed here.".into());
    }
    let picked = state.settings.lock().unwrap().move_to.get(&id).map(PathBuf::from);
    if let Some(p) = &picked {
        check_move_to(&state, p)?;
    }
    let to = picked.unwrap_or_else(|| root.join(shots::NOT_NEEDED));
    let app2 = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        moves::move_photos(&moves::journal_file(), &id, &root, &to, &photos, &|done, total| {
            let _ = app2.emit("move-progress", MoveProgress { done, total });
        })
    })
    .await
    .map_err(err)?
}

/// Puts back the photos of one move (`decisions/2026/10/09/review-moves-set-aside-photos-itself-with-undo.md`); the ones put back wait for Review again.
#[tauri::command]
async fn undo_move(app: AppHandle, move_id: u64) -> Result<moves::Outcome, String> {
    let app2 = app.clone();
    let out = tauri::async_runtime::spawn_blocking(move || {
        moves::undo(&moves::journal_file(), move_id, &|done, total| {
            let _ = app2.emit("move-progress", MoveProgress { done, total });
        })
    })
    .await
    .map_err(err)??;
    let back = moves::put_back(&moves::journal_file());
    let state = app.state::<AppState>();
    let mut m = state.marks.lock().unwrap();
    // The photos are back by now; a choice not saved is forgotten again on the next start.
    if forget_put_back(&mut m, &back) > 0 && let Err(e) = store::save(&store::marks_file(), &*m) {
        log::warn!("could not save the marks after Undo: {e}");
    }
    Ok(out)
}

/// Sets the gap in hours that starts a new Photos Event; the Photos Sections are read again after.
#[tauri::command]
fn set_photo_gap(state: State<AppState>, hours: u32) -> Result<Settings, String> {
    let mut s = state.settings.lock().unwrap();
    s.photo_gap = hours.clamp(1, 72);
    store::save(&store::settings_file(), &*s)?;
    Ok(s.clone())
}

#[tauri::command]
fn marks(state: State<AppState>) -> Marks {
    state.marks.lock().unwrap().clone()
}

/// Sets or clears a Mark. Setting the same Mark again keeps its date.
#[tauri::command]
fn set_mark(state: State<AppState>, id: String, mark: Option<String>) -> Result<Marks, String> {
    let mut m = state.marks.lock().unwrap();
    match mark {
        Some(v) if v == "next" || v == "done" => {
            if m.marks.get(&id).map(|x| x.m != v).unwrap_or(true) {
                m.marks.insert(id, store::Mark { m: v, at: store::now() });
            }
        }
        _ => {
            m.marks.remove(&id);
        }
    }
    store::save(&store::marks_file(), &*m)?;
    Ok(m.clone())
}

/// Puts many Marks at once, each as given with its date, or none: a Mark on several cards and
/// its Undo, saved once.
#[tauri::command(async)]
fn put_marks(state: State<AppState>, marks: HashMap<String, Option<store::Mark>>) -> Result<Marks, String> {
    if let Some(x) = marks.values().flatten().find(|x| x.m != "next" && x.m != "done") {
        return Err(format!("not a Mark: {}", x.m));
    }
    let mut m = state.marks.lock().unwrap();
    for (id, mark) in marks {
        match mark {
            Some(x) => { m.marks.insert(id, x); }
            None => { m.marks.remove(&id); }
        }
    }
    store::save(&store::marks_file(), &*m)?;
    Ok(m.clone())
}

/// Ticks or unticks items inside a Work, by their Tick keys (`decisions/2026/09/29/ticks-offer-the-mark-never-set-it.md`). Never changes its Mark.
#[tauri::command]
fn set_ticks(state: State<AppState>, id: String, keys: Vec<String>, on: bool) -> Result<Marks, String> {
    let mut m = state.marks.lock().unwrap();
    m.tick(&id, &keys, on);
    store::save(&store::marks_file(), &*m)?;
    Ok(m.clone())
}

/// Moves an Orphaned Mark and its Ticks to the Work the person picked (`decisions/2026/09/30/work-id-is-its-path.md`).
#[tauri::command]
fn move_mark(state: State<AppState>, from: String, to: String) -> Result<Marks, String> {
    let mut m = state.marks.lock().unwrap();
    m.carry(&from, &to);
    log::info!("mark moved from {from} to {to}");
    store::save(&store::marks_file(), &*m)?;
    Ok(m.clone())
}

/// Forgets Orphaned Marks: whole Works, and single Ticks as (Work id, item).
#[tauri::command]
fn forget_marks(state: State<AppState>, ids: Vec<String>, items: Vec<(String, String)>) -> Result<Marks, String> {
    let mut m = state.marks.lock().unwrap();
    m.forget(&ids, &items);
    store::save(&store::marks_file(), &*m)?;
    Ok(m.clone())
}

/// The Marks backup: Marks, Ticks and how the window looks (`decisions/2026/10/01/settings-in-one-versioned-store.md`). Its version is the Marks'
/// own `v`; a second `v` here would write the key twice, and the file would not read back.
#[derive(Serialize, serde::Deserialize)]
struct Backup {
    kind: String,
    #[serde(flatten)]
    marks: Marks,
    #[serde(default)]
    ui: serde_json::Value,
}
const BACKUP_KIND: &str = "filesviewhub-marks";

/// The backup of these Marks and window settings.
fn backup(marks: Marks, ui: serde_json::Value) -> Backup {
    Backup { kind: BACKUP_KIND.into(), marks, ui }
}

/// The backup in a file's text, or why it is refused: it is not JSON, not a FilesViewHub backup
/// (a Hub backup is refused too, `decisions/2026/10/09/the-app-imports-nothing-from-the-hub.md`), or made by a newer app whose Marks this one would
/// read only in part.
fn read_backup(text: &str) -> Result<Backup, String> {
    let not_backup = || "This file is not a marks backup from FilesViewHub.".to_string();
    // Read through a Value first: a backup saved before the fix holds two `v` keys, and the Value
    // keeps only the last one, the Marks' own.
    // A text editor may have saved the file with a byte order mark, which JSON does not allow.
    let text = text.strip_prefix('\u{FEFF}').unwrap_or(text);
    let v: serde_json::Value = serde_json::from_str(text).map_err(|_| not_backup())?;
    if v.get("kind").and_then(|k| k.as_str()) != Some(BACKUP_KIND) {
        return Err(not_backup());
    }
    let b: Backup = serde_json::from_value(v).map_err(|_| not_backup())?;
    if b.marks.v > Marks::default().v {
        return Err("This backup was saved by a newer FilesViewHub. Use that version to add it.".into());
    }
    Ok(b)
}

/// The path is in one of the checked folders, however either is spelled: `..`, trailing dots, a
/// link or junction to it.
fn in_checked(path: &Path, checked: &[PathBuf]) -> bool {
    let real = store::real(path);
    checked.iter().any(|c| store::within_any_case(&real, &store::real(c)))
}

/// Where the backup dialogs start: the folder of the last backup saved or added while it is
/// still there, else Downloads. Saving never starts inside a checked folder (`decisions/2026/10/05/app-data-local-cache-movable.md`). Not
/// Documents: Windows' ransomware protection, when on, stops an app it does not know from writing
/// there, and Save As then says "File not found".
fn backup_start(app: &AppHandle, saving: bool) -> Option<PathBuf> {
    let state = app.state::<AppState>();
    let last = state.settings.lock().unwrap().backup_dir.clone().map(PathBuf::from);
    let usable = |d: &PathBuf| {
        // A network folder goes to the dialog as it is: looking at it would connect to its server
        // first, and could wait a minute for one that is off. The save itself is still checked.
        let t = d.to_string_lossy();
        t.starts_with("\\\\") || t.starts_with("//") || (d.is_dir() && !(saving && in_checked(d, &checked_paths(&state))))
    };
    let outside = |d: &PathBuf| !(saving && in_checked(d, &checked_paths(&state)));
    last.filter(usable).or_else(|| app.path().download_dir().ok().filter(outside))
}

/// Keeps the folder of a backup just saved or added, for the next dialog. The backup itself has
/// worked, so a failure here is only logged.
fn keep_backup_dir(state: &AppState, path: &Path) {
    let Some(dir) = path.parent() else { return };
    let mut s = state.settings.lock().unwrap();
    s.backup_dir = Some(dir.to_string_lossy().into());
    if let Err(e) = store::save(&store::settings_file(), &*s) {
        log::warn!("could not keep the backup folder: {e}");
    }
}

/// A backup file's name and its folder, for the Marks page to show.
#[derive(Serialize)]
struct BackupFile {
    file: String,
    folder: String,
}

fn backup_file(path: &Path) -> BackupFile {
    let part = |p: Option<&std::ffi::OsStr>| p.map(|x| x.to_string_lossy().into_owned()).unwrap_or_default();
    BackupFile { file: part(path.file_name()), folder: part(path.parent().map(Path::as_os_str)) }
}

/// Saves the Marks to a file the person picks, never inside a checked folder (`decisions/2026/10/05/app-data-local-cache-movable.md`). Returns
/// where, or None when they cancelled.
#[tauri::command]
async fn export_marks(app: AppHandle) -> Result<Option<BackupFile>, String> {
    let mut d = app.dialog().file().set_file_name("FilesViewHub marks.json").add_filter("Marks", &["json"]);
    if let Some(dir) = backup_start(&app, true) {
        d = d.set_directory(dir);
    }
    let Some(f) = d.blocking_save_file() else { return Ok(None) };
    let path = f.into_path().map_err(err)?;
    let state = app.state::<AppState>();
    if in_checked(&path, &checked_paths(&state)) {
        return Err("Save the backup outside your checked folders: the app never writes into them.".into());
    }
    let b = backup(state.marks.lock().unwrap().clone(), state.settings.lock().unwrap().ui.clone());
    store::save(&path, &b)?;
    log::info!("marks saved to {}", path.display());
    keep_backup_dir(&state, &path);
    Ok(Some(backup_file(&path)))
}

/// What adding from a backup did, for the Marks page.
#[derive(Serialize)]
struct Imported {
    marks: Marks,
    /// Marks and ticked items from the backup that the app did not have.
    added: usize,
    ticks: usize,
    /// The backup's window settings were applied.
    ui: bool,
    file: String,
}

/// Adds the Marks from a FilesViewHub backup the person picks, and its window settings. None
/// when cancelled.
#[tauri::command]
async fn import_marks(app: AppHandle) -> Result<Option<Imported>, String> {
    let mut d = app.dialog().file().add_filter("Marks", &["json"]);
    if let Some(dir) = backup_start(&app, false) {
        d = d.set_directory(dir);
    }
    let Some(f) = d.blocking_pick_file() else { return Ok(None) };
    let path = f.into_path().map_err(err)?;
    let text = std::fs::read_to_string(&path).map_err(err)?;
    let b = read_backup(&text)?;
    let state = app.state::<AppState>();
    let (incoming, ui) = (b.marks, b.ui);
    let mut m = state.marks.lock().unwrap();
    let all_ticks = |m: &Marks| m.ticks.values().map(|t| t.len()).sum::<usize>();
    let before = all_ticks(&m);
    let added = m.merge(incoming);
    let ticks = all_ticks(&m) - before;
    // A backup made before a move that was undone brings back no choice of the photos put back.
    drop(m);
    let back = moves::put_back(&moves::journal_file());
    let mut m = state.marks.lock().unwrap();
    forget_put_back(&mut m, &back);
    store::save(&store::marks_file(), &*m)?;
    let applied = !ui.is_null();
    if applied {
        // The marks are in by now, so a failure here is only logged.
        let mut s = state.settings.lock().unwrap();
        s.ui = ui;
        if let Err(e) = store::save(&store::settings_file(), &*s) {
            log::warn!("could not save the backup's settings: {e}");
        }
    }
    log::info!("marks added from {}: {added} marks, {ticks} ticks", path.display());
    keep_backup_dir(&state, &path);
    Ok(Some(Imported { marks: m.clone(), added, ticks, ui: applied, file: backup_file(&path).file }))
}

/// Notes that Work `id` was opened from the app now, for Home's Recently opened.
#[tauri::command]
fn set_opened(state: State<AppState>, id: String) -> Result<Marks, String> {
    let mut m = state.marks.lock().unwrap();
    m.open(&id, store::now());
    store::save(&store::marks_file(), &*m)?;
    Ok(m.clone())
}

/// Empties Home's Recently opened: forgets when each Work was opened.
#[tauri::command]
fn clear_opened(state: State<AppState>) -> Result<Marks, String> {
    let mut m = state.marks.lock().unwrap();
    m.opened.clear();
    store::save(&store::marks_file(), &*m)?;
    log::info!("recently opened cleared");
    Ok(m.clone())
}

/// Clears every Mark, Tick, Review choice and time opened.
#[tauri::command]
fn clear_marks(state: State<AppState>) -> Result<Marks, String> {
    let mut m = state.marks.lock().unwrap();
    *m = Marks::default();
    store::save(&store::marks_file(), &*m)?;
    log::info!("all marks cleared");
    Ok(m.clone())
}

/// The program files of `ext` open in; None goes back to the system's own (`decisions/2026/10/05/filesviewhub-is-a-tauri-app-shipped-portable.md`).
#[tauri::command]
fn set_open_with(state: State<AppState>, ext: String, program: Option<String>) -> Result<Settings, String> {
    let mut s = state.settings.lock().unwrap();
    let ext = ext.trim().trim_start_matches('.').to_lowercase();
    if program.as_deref().is_some_and(|p| programs::refused(&ext, p)) {
        return Err(format!("A .{ext} file is a program or script, so it is not opened by a program that runs it."));
    }
    match program {
        Some(p) if !ext.is_empty() => s.open_with.insert(ext, p),
        _ => s.open_with.remove(&ext),
    };
    store::save(&store::settings_file(), &*s)?;
    Ok(s.clone())
}

#[tauri::command]
async fn pick_program(app: AppHandle) -> Option<String> {
    let d = app.dialog().file();
    #[cfg(windows)]
    let d = d.add_filter("Programs", &["exe"]);
    d.blocking_pick_file().and_then(|p| p.into_path().ok()).map(|p| p.to_string_lossy().into())
}

/// Every checked folder's place: as last read, and as last seen in the settings.
fn checked_paths(state: &AppState) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = state.catalog.lock().unwrap().sections.iter().map(|s| PathBuf::from(&s.path)).collect();
    v.extend(state.settings.lock().unwrap().folders.iter().map(|f| PathBuf::from(&f.last_path)));
    v
}

/// The folders of the Sections whose Drive is connected.
fn online_folders(state: &AppState) -> Vec<PathBuf> {
    state.catalog.lock().unwrap().sections.iter().filter(|s| s.online).map(|s| PathBuf::from(&s.path)).collect()
}

/// What opening or showing a path no checked folder holds says.
const OUTSIDE: &str = "This file is not in a checked folder.";

/// Where a path in a checked folder leads, as `store::real` resolves it, for `open` and `reveal`
/// alike; `Err` with what to say when it is not in one (`check`), or is in one whose Drive is not
/// connected.
fn checked_file(state: &AppState, path: &str) -> Result<String, &'static str> {
    check(&online_folders(state), path, link_target, store::real, is_link)
        .map(|r| r.to_string_lossy().to_string())
        .map_err(|e| if e == OUTSIDE && inside_away(state, path) { AWAY } else { e })
}

/// Only files inside a checked folder are opened or shown, as written, as resolved, and with no
/// link on the way out of it (`link_out`, checked before anything is resolved). A path that climbs
/// out with `..` is refused, and so is a name with a stream (`::$DATA`) on Windows.
fn check(folders: &[PathBuf], path: &str, read: impl Fn(&Path) -> Result<Option<PathBuf>, ()>, real: impl Fn(&Path) -> PathBuf, link: impl Fn(&Path) -> bool) -> Result<PathBuf, &'static str> {
    use std::path::Component;
    let p = Path::new(path);
    if !readable(folders, p) {
        return Err(OUTSIDE);
    }
    // A stream exists only on Windows; elsewhere `:` is a letter like any other.
    if cfg!(windows) && p.components().any(|c| matches!(c, Component::Normal(n) if n.to_string_lossy().contains(':'))) {
        return Err("This file name cannot be opened.");
    }
    // `real` reads a name ending in a dot or a space without that end when the path does not exist
    // as named, so the links on both spellings are read.
    let trimmed: PathBuf = p.components().map(|c| match c {
        Component::Normal(n) if cfg!(windows) => PathBuf::from(n.to_string_lossy().trim_end_matches(['.', ' '])),
        c => PathBuf::from(c.as_os_str()),
    }).collect();
    if link_out(folders, p, &read, &real) || (trimmed != p && link_out(folders, &trimmed, &read, &real)) || !inside(folders, path, &real) {
        return Err(OUTSIDE);
    }
    let r = real(p);
    // A name ending in a dot or a space that `real` could not keep is not there as named: without
    // that end it would be the sibling's file.
    if cfg!(windows) && store::ends_oddly(p) && !store::ends_oddly(&r) {
        return Err("This file is not there any more. Choose Read folders again.");
    }
    if resolved_inside(folders, p, &r, &real, link) { Ok(r) } else { Err(OUTSIDE) }
}

/// `path` goes through a link below `folders` that leads out of them by its text, or one that
/// cannot be read. Each link is read with `read` (`read_link`), never followed, and the walk goes
/// on from where it leads, so the system follows no link whose target is not known yet: one out of
/// the folders could lead to a server, asked before the path is refused (`decisions/2026/10/05/nothing-hidden-no-network-by-default.md`). A target
/// with a root but no drive (`\Device\Mup\...`) is a system path and is refused. The folders
/// holding `path` by its text count as where `real` puts them too (a Section's folder that is
/// itself a junction). A path through more than 32 links is refused.
fn link_out(folders: &[PathBuf], path: &Path, read: impl Fn(&Path) -> Result<Option<PathBuf>, ()>, real: impl Fn(&Path) -> PathBuf) -> bool {
    use std::path::Component;
    // Resolved only once a link is found.
    let cell = std::cell::OnceCell::new();
    let homes = || cell.get_or_init(|| folders.iter().cloned().chain(folders.iter().filter(|f| store::within(path, f)).map(|f| real(f))).collect::<Vec<_>>());
    let mut p = path.to_path_buf();
    for _ in 0..=32 {
        let parts: Vec<Component> = p.components().collect();
        let mut at = PathBuf::new();
        let mut next = None;
        for (i, c) in parts.iter().enumerate() {
            at.push(c);
            // A checked folder and the folders above it may be links: the Section is wherever they lead.
            if folders.iter().any(|f| store::within(f, &at)) || cell.get().is_some_and(|h: &Vec<PathBuf>| h.iter().any(|f| store::within(f, &at))) {
                continue;
            }
            let Ok(t) = read(&at) else { return true };
            let Some(t) = t else { continue };
            if cfg!(windows) && t.has_root() && !matches!(t.components().next(), Some(Component::Prefix(_))) {
                return true;
            }
            let t = lexical(&at.parent().unwrap_or(&at).join(t));
            if !homes().iter().any(|f| store::within(&t, f)) {
                return true;
            }
            next = Some(t.join(parts[i + 1..].iter().collect::<PathBuf>()));
            break;
        }
        match next {
            Some(n) => p = n,
            None => return false,
        }
    }
    true
}

/// A path with `.` and `..` worked out by its text.
fn lexical(p: &Path) -> PathBuf {
    use std::path::Component;
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            c => out.push(c),
        }
    }
    out
}

/// `path` may be read to judge it: it is in one of `folders` by its text, or on the drive letter
/// of one, and has no `..`, no `\\?\` or `\\.\` start (`inside`) and no drive letter without a
/// `\` after it (`E:Books`, read from a folder the system keeps per drive).
fn readable(folders: &[PathBuf], p: &Path) -> bool {
    use std::path::{Component, Prefix};
    let letter = drive_letter(p);
    !matches!(p.components().next(), Some(Component::Prefix(x)) if x.kind().is_verbatim() || matches!(x.kind(), Prefix::DeviceNS(_)))
        && !p.components().any(|c| matches!(c, Component::ParentDir))
        && (letter.is_none() || p.has_root())
        && folders.iter().any(|f| store::within(p, f) || (letter.is_some() && drive_letter(f) == letter))
}

/// The drive letter a path starts with, in capitals; `None` for a network path, any other start
/// and every path on macOS and Linux.
fn drive_letter(p: &Path) -> Option<u8> {
    use std::path::{Component, Prefix};
    match p.components().next() {
        Some(Component::Prefix(x)) => match x.kind() {
            Prefix::Disk(l) => Some(l.to_ascii_uppercase()),
            _ => None,
        },
        _ => None,
    }
}

/// `path` is in one of `folders`, by its text, or as `real` resolves both when it is on the drive
/// letter of one of them (another spelling of a folder, as a short 8.3 name). Any other path is
/// judged by its text only: resolving it could connect to a server before the answer is known
/// (a network path, a drive letter mapped elsewhere, a host under macOS' `/net`), and a log line
/// can name any server. A path with a `\\?\` or `\\.\` start is refused: the app never gives one,
/// and in one `/` is no separator, so `..` could hide in a name.
fn inside(folders: &[PathBuf], path: &str, real: impl Fn(&Path) -> PathBuf) -> bool {
    let p = Path::new(path);
    let letter = drive_letter(p);
    readable(folders, p)
        && (folders.iter().any(|f| store::within(p, f)) || {
            let r = real(p);
            folders.iter().filter(|f| drive_letter(f) == letter).any(|f| store::within(&r, &real(f)))
        })
}

fn is_link(p: &Path) -> bool {
    p.symlink_metadata().is_ok_and(|m| m.file_type().is_symlink())
}

/// Where a link or junction leads, read and not followed; `None` for anything else or nothing
/// there, and `Err` for a link, or anything, that cannot be read. A name ending in a dot or a space
/// is read as named; `check` reads the spelling without that end too.
fn link_target(p: &Path) -> Result<Option<PathBuf>, ()> {
    let p = store::named(p);
    match p.symlink_metadata() {
        Ok(m) if m.file_type().is_symlink() => std::fs::read_link(&p).map(Some).map_err(drop),
        Ok(_) => Ok(None),
        Err(e) if matches!(e.kind(), std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory) => Ok(None),
        // What cannot be looked at may be a link the system would follow.
        Err(_) => Err(()),
    }
}

/// `resolved`, what `real` made of `given`, is in one of `folders` as written or as `real`
/// resolves them. On a mapped network drive it is the server's path (`\\?\UNC\server\share\...`),
/// which only the folder resolved the same way can hold. Only the folders on `given`'s drive
/// letter are resolved (every folder on macOS and Linux, which have none), and only when no folder
/// holds it as written: they reach only the server `given` is on, and a folder on another server
/// that does not answer holds up no open. A link left in it below the folder is one the
/// system could not follow (a WSL or server-side link on a share): where it leads is not known, so
/// the path is refused.
fn resolved_inside(folders: &[PathBuf], given: &Path, resolved: &Path, real: impl Fn(&Path) -> PathBuf, link: impl Fn(&Path) -> bool) -> bool {
    let holds = |f: &Path| {
        let mut p = PathBuf::new();
        let depth = f.components().count();
        store::within(resolved, f)
            && !resolved.components().enumerate().any(|(i, c)| {
                p.push(c);
                i >= depth && link(&p)
            })
    };
    let letter = drive_letter(given);
    !resolved.components().any(|c| matches!(c, std::path::Component::ParentDir))
        && (folders.iter().any(|f| holds(f)) || folders.iter().filter(|f| drive_letter(f) == letter).any(|f| holds(&real(f))))
}

/// The path the system is given to open: a server's path without its `\\?\` start, which
/// File Explorer and many programs do not take. A path of 260 characters or more keeps it, since
/// without it Windows cannot reach the file, and so does a name ending in a dot or a space, since
/// without it Windows would drop that end and name another file.
fn plain_unc(resolved: String) -> String {
    match resolved.strip_prefix(r"\\?\UNC\") {
        Some(r) if r.chars().count() + 2 < 260 && !r.split('\\').any(|n| n.ends_with(['.', ' '])) => format!(r"\\{r}"),
        _ => resolved,
    }
}

/// A file's type, read with trailing dots and spaces dropped, as Windows drops them, so `run.vbs.`
/// is a script too where a share keeps that end.
fn type_of(path: &str) -> String {
    let name = Path::new(path).file_name().map(|n| n.to_string_lossy().trim_end_matches(['.', ' ']).to_string()).unwrap_or_default();
    Path::new(&name).extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default()
}

/// What opening a file on a Drive that is not connected says.
const AWAY: &str = "Its drive is not connected. Connect it, then choose Read folders again.";

/// The path is in a Section whose Drive is not connected, judged by its text: nothing on it can be read.
fn inside_away(state: &AppState, path: &str) -> bool {
    !Path::new(path).components().any(|c| matches!(c, std::path::Component::ParentDir))
        && state.catalog.lock().unwrap().sections.iter().any(|s| !s.online && !s.gone && store::within(Path::new(path), Path::new(&s.path)))
}

/// Opens a file. A program the person picked is reached through the shell, which can take a
/// moment, so the command runs off the main thread. A file only in the cloud is not opened
/// unless `download` is set, since opening it makes the cloud app download it: its size comes
/// back instead, for the UI to ask first.
#[tauri::command]
async fn open(app: AppHandle, state: State<'_, AppState>, path: String, download: Option<bool>) -> Result<Option<u64>, String> {
    let path = plain_unc(checked_file(&state, &path)?);
    let ext = type_of(&path);
    // One that runs what it opens is never taken for a program or script type (`programs::refused`).
    let program = state.settings.lock().unwrap().open_with.get(&ext).cloned().filter(|p| !programs::refused(&ext, p));
    // A program or script is shown in its folder, never started from a click on its name,
    // unless the person picked a program for its type (an editor for .js, say).
    if program.is_none() && programs::RUNS.contains(&ext.as_str()) {
        log::info!("show {path} (a program or script is not started)");
        if let Some(why) = unshowable(&path) {
            return Err(why);
        }
        return app.opener().reveal_item_in_dir(&path).map(|_| None).map_err(err);
    }
    if download != Some(true) && let Some(m) = std::fs::metadata(&path).ok().filter(scan::is_cloud) {
        log::info!("not opened yet: {path} is only in the cloud");
        return Ok(Some(m.len()));
    }
    log::info!("open {path}{}", if download == Some(true) { " (downloads it from the cloud first)" } else { "" });
    let r = match program {
        p if path.starts_with(r"\\?\") && path.split('\\').any(|n| n.ends_with(['.', ' '])) => {
            let (e, f) = (ext.clone(), path.clone());
            tauri::async_runtime::spawn_blocking(move || programs::open_as_named(p.as_deref(), &e, &f)).await.unwrap_or_else(|e| Err(err(e)))
        }
        Some(p) => {
            let (e, f) = (ext.clone(), path.clone());
            tauri::async_runtime::spawn_blocking(move || programs::open(&p, &e, &f)).await.unwrap_or_else(|e| Err(err(e)))
        }
        None => open_file(&app, &path),
    };
    if let Err(e) = &r {
        log::warn!("cannot open {path}: {e}");
    }
    r.map(|_| None)
}

/// Settings, Programs: each type in the catalog with the program it opens in (`programs.rs`).
/// Asking the shell takes a moment per type, so it runs off the main thread.
#[tauri::command]
async fn program_list(state: State<'_, AppState>) -> Result<programs::Programs, String> {
    let open_with = state.settings.lock().unwrap().open_with.clone();
    let types = programs::types(&state.catalog.lock().unwrap(), &open_with);
    tauri::async_runtime::spawn_blocking(move || programs::list(types, &open_with)).await.map_err(err)
}

/// On Windows the file opens through Explorer, so the program it opens in sees a signed parent:
/// Foxit PDF Reader warns when an unsigned app (this one) starts it directly.
#[cfg(windows)]
fn open_file(_app: &AppHandle, path: &str) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    let windir = std::env::var_os("WINDIR").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(r"C:\Windows"));
    // Quoted by hand: Explorer splits an unquoted path at commas, and Windows paths hold no '"'.
    std::process::Command::new(windir.join("explorer.exe")).raw_arg(format!("\"{path}\"")).spawn().map(drop).map_err(err)
}

#[cfg(not(windows))]
fn open_file(app: &AppHandle, path: &str) -> Result<(), String> {
    app.opener().open_path(path, None::<&str>).map_err(err)
}

/// Why File Explorer cannot show `path`: on Windows it reads a name ending in a dot or a space
/// without that end, so it would show the file or folder beside it named without it, or nothing
///. `None` for any other path, and on macOS and Linux.
fn unshowable(path: &str) -> Option<String> {
    use std::path::Component;
    let names: Vec<String> = Path::new(path).components().filter_map(|c| match c {
        Component::Normal(n) => Some(n.to_string_lossy().into_owned()),
        _ => None,
    }).collect();
    let k = names.iter().position(|n| n.ends_with(['.', ' '])).filter(|_| cfg!(windows))?;
    // The reason before the name, so a long name cut short in the status bar keeps it.
    let n = &names[k];
    let end = if n.ends_with('.') { "dot" } else { "space" };
    Some(format!("File Explorer cannot show a name ending in a {end}: \u{201c}{n}\u{201d}"))
}

/// Shows a file or a folder selected in its parent folder: File Explorer's `/select`, Finder's
/// selection, or on Linux the file manager's ShowItems where it has one. `open` opens a folder
/// itself instead, and so does a drive's top folder, which has no parent. A file that is not there
/// opens its folder.
#[tauri::command(async)]
fn reveal(app: AppHandle, state: State<'_, AppState>, path: String, open: Option<bool>) -> Result<(), String> {
    let mut mine = vec![store::data_dir(), state.thumbs.dir(), store::default_cache_dir(), state.thumbs.tools.dir.clone()];
    // The folders Review's move puts photos into (`decisions/2026/10/09/review-moves-set-aside-photos-itself-with-undo.md`).
    mine.extend(state.settings.lock().unwrap().move_to.values().map(PathBuf::from));
    let climbs = Path::new(&path).components().any(|c| matches!(c, std::path::Component::ParentDir));
    let ours = |p: &str| !climbs && mine.iter().any(|d| store::within(Path::new(p), d));
    // Any other path is refused as `open` refuses it.
    let allowed = |p: &str| ours(p) || checked_file(&state, p).is_ok();
    if !ours(&path) {
        checked_file(&state, &path)?;
    }
    if let Some(why) = unshowable(&path) {
        return Err(why);
    }
    let p = Path::new(&path);
    if p.is_dir() && (open == Some(true) || p.parent().is_none()) {
        return app.opener().open_path(&path, None::<&str>).map_err(err);
    }
    // A data file not written yet (marks.json before the first Mark): its folder, when that folder
    // may be shown too, so a checked folder gone from its drive does not open the drive.
    if !p.exists() && let Some(dir) = p.parent().filter(|d| d.is_dir() && allowed(&d.to_string_lossy())) {
        return app.opener().open_path(dir.to_string_lossy(), None::<&str>).map_err(err);
    }
    app.opener().reveal_item_in_dir(&path).or_else(|e| match p.parent() {
        // A Linux file manager with no ShowItems and no portal: its folder, with nothing selected.
        Some(dir) if cfg!(target_os = "linux") => {
            log::warn!("cannot select {path} in its folder ({e}); opening the folder");
            app.opener().open_path(dir.to_string_lossy(), None::<&str>)
        }
        _ => Err(e),
    }).map_err(err)
}

/// One place the app stores something, for the "What this app does" screen.
#[derive(Serialize)]
struct Place {
    what: String,
    path: String,
    size: u64,
}

#[derive(Serialize)]
struct Info {
    version: String,
    build: String,
    places: Vec<Place>,
    cache_dir: String,
    cache_default: bool,
    tools_dir: String,
    tools: Vec<tools::Tool>,
    /// Folder names keep their case on this system (`store::CASE_ASIDE`), for a drive the page has
    /// not had listed.
    keeps_case: bool,
}

#[tauri::command]
async fn info(app: AppHandle) -> Info {
    let state = app.state::<AppState>();
    let s = state.settings.lock().unwrap().clone();
    let size = |p: &Path| if p.is_dir() { store::folder_size(p) } else { std::fs::metadata(p).map(|m| m.len()).unwrap_or(0) };
    let mut places = vec![
        ("Checked folders and settings: each Section's name, kind, the view picked at each level and the order of its cards (and of Home's), each folder given a kind or views of its own or split out as a Section of its own, and the Sections whose offer of a kind per folder you answered or put away", store::settings_file()),
        ("The catalog: every Section and Work last read", store::catalog_file()),
        ("Your Next and Done marks, ticked items, Review choices and when you last opened each card", store::marks_file()),
        ("The covers you chose, from a picture of your own or the web, and which card or folder each is for", chosen::list_file(&store::data_dir())),
        ("Those covers' pictures", chosen::pictures_dir(&store::data_dir())),
        ("Thumbnails, file details and photo measures for Review (safe to delete)", state.thumbs.dir()),
        ("The log", store::log_dir()),
        ("The window's own files (WebView2)", store::webview_dir()),
    ];
    // Thumbnails made in the default place while the chosen cache's drive was away.
    let default = store::default_cache_dir();
    if !store::within(&state.thumbs.dir(), &default) && store::folder_size(&default) > 0 {
        places.push(("Thumbnails made while the chosen cache's drive was away (safe to delete)", default));
    }
    // A file the app could not read is set aside, never deleted: it may hold marks.
    for dir in [store::data_dir(), state.thumbs.dir()] {
        for e in std::fs::read_dir(dir).into_iter().flatten().filter_map(Result::ok) {
            if e.file_name().to_string_lossy().ends_with(".damaged") {
                places.push(("A file the app could not read, kept as it was (yours to delete)", e.path()));
            }
        }
    }
    // Review's moves (`decisions/2026/10/09/review-moves-set-aside-photos-itself-with-undo.md`): the journal Undo reads, and each folder picked for a Section.
    let mut places: Vec<(String, PathBuf)> = places.into_iter().map(|(w, p)| (w.to_string(), p)).collect();
    if moves::journal_file().exists() {
        places.push(("Photos moved by Review, kept for Undo".into(), moves::journal_file()));
    }
    let names: std::collections::HashMap<String, String> = state.catalog.lock().unwrap().sections.iter().map(|x| (x.id.clone(), x.name.clone())).collect();
    for (id, p) in &s.move_to {
        places.push((format!("Where Review moves the photos marked Not needed in {} (yours)", names.get(id).map_or(id.as_str(), String::as_str)), PathBuf::from(p)));
    }
    Info {
        version: VERSION.into(),
        build: BUILD.into(),
        places: places.into_iter().map(|(w, p)| Place { what: w, size: size(&p), path: p.to_string_lossy().into() }).collect(),
        cache_dir: state.thumbs.dir().to_string_lossy().into(),
        cache_default: s.cache_dir.is_none(),
        tools_dir: state.thumbs.tools.dir.to_string_lossy().into(),
        tools: state.thumbs.tools.list.clone(),
        keeps_case: !store::CASE_ASIDE,
    }
}

/// Settings, Licences: the app's own licence, built in, and the notices `scripts/notices.mjs`
/// made for this package, next to the tools.
#[derive(Serialize)]
struct Notices {
    licence: &'static str,
    notices: Option<String>,
    file: String,
}

#[tauri::command]
async fn notices(state: State<'_, AppState>) -> Result<Notices, String> {
    let dir = &state.thumbs.tools.dir;
    Ok(Notices {
        licence: include_str!("../../LICENSE"),
        notices: std::fs::read_to_string(dir.join("notices.json")).ok(),
        file: dir.join("THIRD-PARTY-NOTICES.txt").to_string_lossy().into(),
    })
}

/// A link Settings, Licences may open: an https link the notices hold as a whole string.
fn source_link(notices: &str, url: &str) -> bool {
    url.starts_with("https://") && !url.contains(['"', '\\']) && notices.contains(&format!("\"{url}\""))
}

/// Opens a source link from Settings, Licences in the web browser.
#[tauri::command]
fn open_web(app: AppHandle, state: State<AppState>, url: String) -> Result<(), String> {
    let known = std::fs::read_to_string(state.thumbs.tools.dir.join("notices.json")).unwrap_or_default();
    if !source_link(&known, &url) {
        return Err("This link is not one of the app's source links.".into());
    }
    log::info!("opened {url} in the web browser");
    app.opener().open_url(&url, None::<&str>).map_err(err)
}

#[tauri::command]
async fn pick_folder(app: AppHandle) -> Option<String> {
    app.dialog().file().blocking_pick_folder().and_then(|p| p.into_path().ok()).map(|p| p.to_string_lossy().into())
}

/// The names the cache keeps in its folder: the only entries moved with it, so nothing else
/// that shares the folder is ever carried along.
const CACHE_NAMES: [&str; 7] = ["thumbs", "thumbs.json", "details.json", "web.json", "photo-dates.json", "photo-shots.json", "tmp"];
/// The folder the cache gets inside a folder the person picks.
const CACHE_FOLDER: &str = "FilesViewHub Cache";
/// The file that marks a folder as a FilesViewHub cache.
const CACHE_MARK: &str = ".filesviewhub-cache";

/// A folder the app may use as the cache as it is: missing, empty, or marked as one.
fn cache_as_is(p: &Path) -> bool {
    !p.exists() || p.join(CACHE_MARK).exists() || std::fs::read_dir(p).is_ok_and(|mut d| d.next().is_none())
}

fn move_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for e in std::fs::read_dir(from)?.filter_map(Result::ok) {
        let dest = to.join(e.file_name());
        if std::fs::rename(e.path(), &dest).is_err() {
            if e.file_type()?.is_dir() {
                move_dir(&e.path(), &dest)?;
                std::fs::remove_dir(e.path())?;
            } else {
                std::fs::copy(e.path(), &dest)?;
                std::fs::remove_file(e.path())?;
            }
        }
    }
    Ok(())
}

/// Moves the cache's own files from `from` to `to`; anything else in `from` stays where it is.
fn move_cache(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for name in CACHE_NAMES {
        let (a, b) = (from.join(name), to.join(name));
        if !a.exists() {
            continue;
        }
        if a.is_dir() {
            move_dir(&a, &b)?;
            std::fs::remove_dir(&a)?;
        } else if std::fs::rename(&a, &b).is_err() {
            std::fs::copy(&a, &b)?;
            std::fs::remove_file(&a)?;
        }
    }
    Ok(())
}

/// Why the cache cannot go to `new`, if it cannot: inside, or holding, a checked folder or the
/// StoredData folder (`decisions/2026/10/05/app-data-local-cache-movable.md`, `decisions/2026/10/07/app-data-in-storeddata-next-to-the-app.md`: nothing is written into a checked folder).
fn cache_refused(new: &Path, checked: &[PathBuf]) -> Option<&'static str> {
    let (new, data) = (&store::real(new), store::real(&store::data_dir()));
    let checked: Vec<PathBuf> = checked.iter().map(|c| store::real(c)).collect();
    if store::within_any_case(&data, new) || (store::within_any_case(new, &data) && !store::within(new, &store::real(&store::default_cache_dir()))) {
        return Some("The cache cannot go there: that folder holds the app's StoredData folder.");
    }
    if checked.iter().any(|c| store::within_any_case(new, c) || store::within_any_case(c, new)) {
        return Some("The cache cannot go inside a checked folder, or in a folder that holds one: the app never writes into your folders.");
    }
    None
}

/// Moves the cache. `path` None goes back to the default. With `carry`, the files move along;
/// otherwise the new place starts empty and the old one is left as it is.
#[tauri::command]
async fn set_cache_dir(app: AppHandle, path: Option<String>, carry: bool) -> Result<String, String> {
    let state = app.state::<AppState>();
    let old = state.thumbs.dir();
    // A picked folder gets the cache in a folder of its own, so nothing of the person's is ever
    // mixed with it.
    let new = match &path {
        Some(p) => {
            let p = PathBuf::from(p);
            if p.file_name().is_some_and(|n| n.eq_ignore_ascii_case(CACHE_FOLDER)) && cache_as_is(&p) { p } else { p.join(CACHE_FOLDER) }
        }
        None => store::default_cache_dir(),
    };
    let same = store::within(&new, &old) && store::within(&old, &new);
    if store::within(&new, &old) && !same {
        return Err("The new place cannot be inside the current cache.".into());
    }
    if let Some(why) = cache_refused(&new, &checked_paths(&state)) {
        return Err(why.into());
    }
    if path.is_some() && !same && !cache_as_is(&new) {
        return Err("That folder already holds files that are not the app's cache. Choose another folder.".into());
    }
    std::fs::create_dir_all(&new).map_err(err)?;
    let _ = std::fs::write(new.join(CACHE_MARK), b"");
    state.details.save();
    state.shots.save();
    let carried = carry && old.is_dir() && !same;
    {
        let _io = state.thumbs.io.lock().unwrap();
        state.thumbs.write_index();
        if carried && let Err(e) = move_cache(&old, &new) {
            // Whatever moved goes back, so the cache in use stays whole.
            let _ = move_cache(&new, &old);
            return Err(err(e));
        }
        let mut s = state.settings.lock().unwrap();
        s.cache_dir = path.as_ref().map(|_| new.to_string_lossy().to_string());
        store::save(&store::settings_file(), &*s)?;
        state.thumbs.set_dir(new.clone());
        // A picked cache left empty is removed; the default one stays.
        if carried && !store::within(&old, &store::default_cache_dir()) {
            let _ = std::fs::remove_file(old.join(CACHE_MARK));
            let _ = std::fs::remove_dir(&old);
        }
    }
    state.details.reload();
    state.shots.reload();
    state.thumbs.plan(&state.catalog.lock().unwrap());
    state.details.plan(&state.catalog.lock().unwrap());
    state.shots.plan(&state.catalog.lock().unwrap());
    log::info!("cache moved from {} to {} (files carried: {carried})", old.display(), new.display());
    Ok(new.to_string_lossy().into())
}

/// The network log: every request the app made, newest last.
#[tauri::command]
fn read_network_log() -> String {
    std::fs::read_to_string(web::log_file()).unwrap_or_default()
}

/// The data files set aside as damaged since the last ask, by name, for the toast at start.
#[tauri::command]
fn set_aside() -> Vec<String> {
    store::take_set_aside()
}

/// The app log as text, for Copy log; up to about 5 MB, so it is read off the main thread.
#[tauri::command]
async fn read_log() -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(|| String::from_utf8_lossy(&logs::read_app(&store::log_dir())).into_owned()).await.map_err(err)
}

/// A log read into entries for Settings, Log: `app` or `network`, oldest first. Finding paths
/// checks the disk, so it runs off the main thread.
#[tauri::command]
async fn log_entries(state: State<'_, AppState>, which: String) -> Result<Vec<logs::Entry>, String> {
    // Paths become links only under folders Show in File Explorer opens: checked folders that
    // are online, and the app's own.
    let mut roots: Vec<PathBuf> = state.catalog.lock().unwrap().sections.iter().filter(|s| s.online).map(|s| PathBuf::from(&s.path)).collect();
    roots.extend([store::data_dir(), state.thumbs.dir(), store::default_cache_dir(), state.thumbs.tools.dir.clone()]);
    tauri::async_runtime::spawn_blocking(move || {
        let net = which == "network";
        if net { logs::network(&std::fs::read(web::log_file()).unwrap_or_default()) } else { logs::app(&logs::read_app(&store::log_dir()), &roots) }
    })
    .await
    .map_err(err)
}

/// `%20` and the like back to the text they stand for.
fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        let hex = |c: u8| (c as char).to_digit(16);
        match (b[i], b.get(i + 1).copied().and_then(hex), b.get(i + 2).copied().and_then(hex)) {
            (b'%', Some(h), Some(l)) => {
                out.push((h * 16 + l) as u8);
                i += 3;
            }
            (c, ..) => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into()
}

/// Keys kept inside a whole drive's Section in an earlier version as `{serial}:/path` are rewritten as
/// `{serial}:path`, the key its files have now, so Marks, Ticks, Review choices, Folder kinds,
/// move folders and Undo stay with them. Each file changed is saved. Runs before the log starts,
/// so it says nothing.
fn old_drive_keys(s: &mut Settings, cat: &mut Catalog, m: &mut Marks) {
    let drives = s.whole_drives();
    if drives.is_empty() {
        return;
    }
    if s.rekey(&drives) > 0 {
        let _ = store::save(&store::settings_file(), &*s);
    }
    if m.rekey(&drives) > 0 {
        let _ = store::save(&store::marks_file(), &*m);
    }
    let mut n = 0;
    let mut fix = |k: &mut String| {
        if let Some(to) = store::rekey(k, &drives) {
            *k = to;
            n += 1;
        }
    };
    for sec in &mut cat.sections {
        fix(&mut sec.id);
        sec.split_from.iter_mut().for_each(&mut fix);
        sec.links.iter_mut().for_each(|l| fix(&mut l.id));
        sec.works.iter_mut().for_each(|w| fix(&mut w.id));
    }
    if n > 0 {
        let _ = store::save(&store::catalog_file(), &*cat);
    }
    moves::rekey(&moves::journal_file(), &drives);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    logs::EARLY.hold();
    let writable = store::check_data_dir();
    let mut saved: Settings = store::load(&store::settings_file());
    let mut cat: Catalog = store::load(&store::catalog_file());
    // A catalog of an older version is written again in the current form once, only
    // when that gives every Section back exactly; else it stays in its version, at every later save too.
    let was = cat.v;
    let old = (1..packed::VERSION).contains(&was) && !cat.sections.is_empty();
    match (packed::migrate(&mut cat), old) {
        (true, true) => match store::save(&store::catalog_file(), &cat) {
            Ok(()) => log::info!("catalog of version {was} written again as version {}", packed::VERSION),
            Err(e) => log::warn!("catalog of version {was} could not be written again ({e})"),
        },
        (false, true) => log::error!("catalog of version {was} kept as version {was}: the current form would not give it back exactly"),
        _ => {}
    }
    let mut kept = Marks { v: 2, ..store::load(&store::marks_file()) };
    old_drive_keys(&mut saved, &mut cat, &mut kept);
    let made = Thumbs::new(saved.cache_dir());
    let state = AppState {
        settings: Mutex::new(saved),
        catalog: Mutex::new(cat),
        catalog_stamp: Default::default(),
        marks: Mutex::new(kept),
        scanning: Default::default(),
        preview_gen: Default::default(),
        details: details::Details::new(made.clone()),
        shots: shots::Shots::new(made.clone()),
        thumbs: made,
        review: Mutex::new(None),
        review_window: Mutex::new(()),
    };
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .register_asynchronous_uri_scheme_protocol("thumb", |ctx, req, responder| {
            // `thumb://localhost/<key>` (on Windows `http://thumb.localhost/<key>`): one thumbnail
            // from the cache, and nothing else.
            let state = ctx.app_handle().state::<AppState>();
            let file = state.thumbs.file(req.uri().path().trim_start_matches('/'));
            let found = file.and_then(|f| std::fs::read(f).ok());
            let res = tauri::http::Response::builder();
            responder.respond(match found {
                Some(b) => res.header("Content-Type", "image/jpeg").header("Cache-Control", "max-age=31536000, immutable").body(b),
                None => res.status(404).body(vec![]),
            }.unwrap());
        })
        .register_asynchronous_uri_scheme_protocol("photo", |ctx, req, responder| {
            // `photo://localhost/<path>`: one picture the catalog lists, for the viewer, read on a
            // thread of its own. Nothing else is served, and never a file only in the cloud.
            let app = ctx.app_handle().clone();
            let path = percent_decode(req.uri().path().trim_start_matches('/'));
            std::thread::spawn(move || {
                let state = app.state::<AppState>();
                let listed = state.catalog.lock().unwrap().sections.iter().flat_map(|s| &s.works).flat_map(|w| &w.items)
                    .find(|i| i.path == path).map(|i| (i.cloud, i.ext.clone()));
                let res = tauri::http::Response::builder();
                let kind = |e: &str| match e { "jpg" | "jpeg" | "insp" => "image/jpeg", "png" => "image/png", "gif" => "image/gif", "webp" => "image/webp", "bmp" => "image/bmp", _ => "" };
                let found = match listed {
                    Some((false, ext)) if !kind(&ext).is_empty() && !scan::now_cloud(&path) => std::fs::read(store::named(Path::new(&path))).ok().map(|b| (b, kind(&ext))),
                    _ => None,
                };
                responder.respond(match found {
                    Some((b, t)) => res.header("Content-Type", t).body(b),
                    None => res.status(404).body(vec![]),
                }.unwrap());
            });
        })
        .manage(state)
        .setup(move |app| {
            // The log starts here, where it can be handed to `logs::EARLY`, which holds what
            // `run()` logged before (a data file set aside as damaged).
            let (plugin, level, to) = logs::plugin(writable.is_ok().then(store::log_dir))
                .target(tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout))
                .split(app.handle())?;
            app.handle().plugin(plugin)?;
            logs::EARLY.start(level, to);
            if let Err(e) = &writable {
                log::error!("cannot write the data folder: {e}");
                logs::EARLY.release();
                let handle = app.handle().clone();
                // A Mac starts an app from a disk image, or one not yet moved out of Downloads, from a
                // read-only place.
                let place = if cfg!(target_os = "macos") { "for example Applications or Documents, with Finder" } else { "for example Downloads" };
                app.dialog()
                    .message(format!(
                        "FilesViewHub keeps its settings and marks in the StoredData folder next to it, \
                         and cannot write there:\n\n{e}\n\nMove the FilesViewHub folder to a place you can \
                         write, {place}, and start it again."
                    ))
                    .title("FilesViewHub cannot save")
                    .show(move |_| handle.exit(1));
                return Ok(());
            }
            log::info!("FilesViewHub {VERSION} (build {BUILD}) started; data in {}", store::data_dir().display());
            logs::EARLY.release();
            // After the start line, so a damaged list and what was taken over show in this session.
            let thumbs = &app.state::<AppState>().thumbs;
            thumbs.set_chosen(chosen::Covers::load(&store::data_dir()));
            thumbs.adopt_web_covers();
            moves::settle(&moves::journal_file());
            // Photos put back by an Undo in an earlier version still had the choice that moved them.
            {
                let back = moves::put_back(&moves::journal_file());
                let state = app.state::<AppState>();
                let mut m = state.marks.lock().unwrap();
                let n = forget_put_back(&mut m, &back);
                if n > 0 {
                    log::info!("{n} photos put back by Undo wait for Review again");
                    let _ = store::save(&store::marks_file(), &*m);
                }
            }
            // The window is made here so its web view keeps its files in StoredData too (`decisions/2026/10/07/app-data-in-storeddata-next-to-the-app.md`).
            let config = app.config().app.windows[0].clone();
            let window = tauri::WebviewWindowBuilder::from_config(app.handle(), &config)?.data_directory(store::webview_dir());
            #[cfg(windows)]
            let window = window.additional_browser_args(WEBVIEW_ARGS);
            window.build()?;
            let state = app.state::<AppState>();
            state.thumbs.start(app.handle().clone());
            state.thumbs.plan(&state.catalog.lock().unwrap());
            state.details.start(app.handle().clone());
            state.details.plan(&state.catalog.lock().unwrap());
            state.shots.start(app.handle().clone());
            state.shots.plan(&state.catalog.lock().unwrap());
            Ok(())
        })
        // The app ends with its main window: the Review window closes with it.
        .on_window_event(|w, e| {
            if w.label() == "main" && matches!(e, tauri::WindowEvent::Destroyed) && let Some(r) = w.app_handle().get_webview_window("review") {
                let _ = r.destroy();
            }
        })
        .invoke_handler(tauri::generate_handler![
            drives, folders, settings, picked, set_folders, catalog, catalog_part, catalog_unlisted, scan, drives_away, forget_folder, set_section, rename_section, preview_kind, preview_folder, set_folder, split_folder, put_back, suggest_kinds, set_offered, stop_preview, marks, set_mark, put_marks, set_ticks, move_mark, forget_marks, open, reveal, info,
            pick_folder, set_cache_dir, set_aside, read_log, set_ui, thumbs, details, export_marks, import_marks, clear_marks, set_opened, clear_opened, set_open_with, pick_program, want_thumbs, rebuild_thumbs, cover_words, find_covers, cover_round, cover_preview, use_web_cover, pick_cover_file, read_cover, keep_cover, has_cover, remove_cover, restore_cover, read_network_log, log_entries, program_list,
            notices, open_web, photo_sort, set_review, forget_review, restore_review, save_move_script, set_photo_gap, move_plan, set_move_to, move_set_aside, undo_move, last_move, open_review, review_request
        ])
        .build(tauri::generate_context!())
        .expect("error while running FilesViewHub")
        .run(|app, e| {
            if let tauri::RunEvent::Exit = e {
                app.state::<AppState>().thumbs.save();
                app.state::<AppState>().details.save();
                app.state::<AppState>().shots.save();
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A command that takes the catalog's lock runs off the main thread (AGENTS.md,
    /// Runbook): saving the catalog under that lock takes about 1 s on a large one, and a sync
    /// command would hold the window that long. Only a lock taken in the command's own body is
    /// seen, not one taken in a function it calls.
    #[test]
    fn commands_that_lock_the_catalog_are_async() {
        let src = include_str!("lib.rs");
        let commands: Vec<&str> = src.split("\n#[tauri::command").skip(1)
            .map(|c| &c[..c.find("\n}\n").unwrap_or(c.len())]).collect();
        assert!(commands.len() > 50, "found only {} commands in lib.rs; did they move?", commands.len());
        let sync: Vec<&str> = commands.iter()
            .filter(|c| !c.starts_with("(async)") && !c.contains("async fn ") && c.contains("catalog.lock()"))
            .map(|c| c.split("fn ").nth(1).and_then(|f| f.split('(').next()).unwrap_or("?")).collect();
        assert!(sync.is_empty(), "make these #[tauri::command(async)]: {sync:?}");
    }

    /// A ticked folder whose name has a letter of another length in lower case (`İ`, the Kelvin
    /// sign) still gives its unticked folders by their names.
    #[test]
    fn unticked_folders_cut_by_letters() {
        let p = |s: &str| if cfg!(windows) { format!(r"Q:\{}", s.replace('/', "\\")) } else { format!("/media/x/{s}") };
        for top in ["İstanbul", "\u{212A}elvin"] {
            assert_eq!(unticked_in(&p(top), &[p(&format!("{top}/Sub")), p(&format!("{top}/Ünder/Deep"))]), ["Sub", "Ünder/Deep"], "{top}");
        }
    }

    /// `Books` and `BOOKS` ticked are two Sections on Linux, each with only its own unticked
    /// folders; on Windows and macOS they are one folder.
    #[test]
    fn ticks_keep_case_only_on_linux() {
        let p = |s: &str| if cfg!(windows) { format!(r"Q:\x\{}", s.replace('/', "\\")) } else { format!("/media/x/{s}") };
        let checked = [p("Books"), p("BOOKS"), p("BOOKS/In")];
        let tops = top_ticks(&checked, &|_| None);
        let unchecked = [p("BOOKS/sub")];
        if store::CASE_ASIDE {
            assert_eq!(tops, [&checked[0]], "one folder, ticked once");
            assert_eq!(unticked_in(&checked[0], &unchecked), ["sub"]);
        } else {
            assert_eq!(tops, [&checked[1], &checked[0]], "two folders, BOOKS/In part of BOOKS");
            assert!(unticked_in(&checked[0], &unchecked).is_empty(), "BOOKS/sub is not in Books");
            assert_eq!(unticked_in(&checked[1], &unchecked), ["sub"]);
        }
    }

    #[test]
    fn only_the_notices_own_links_open() {
        let n = r#"{"sources":[{"what":"Source","url":"https://crates.io/crates/serde/1.0.0"},{"url":"djvulibre/source/x.tar.zst"}]}"#;
        assert!(source_link(n, "https://crates.io/crates/serde/1.0.0"));
        assert!(!source_link(n, "https://crates.io/crates/serde"), "a start of a link is not the link");
        assert!(!source_link(n, "djvulibre/source/x.tar.zst"), "not a web link");
        assert!(!source_link(n, "https://evil.example/\",\"url\":\"https://crates.io/crates/serde/1.0.0"));
        assert!(!source_link("", "https://crates.io/crates/serde/1.0.0"), "no notices, no links");
    }

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("fvh-lib-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// A rename and a forget made at once, off the main thread, each leave the catalog file as the
    /// app holds it when its lock is let go, and the last one made is the one on disk.
    #[test]
    fn catalog_saves_keep_their_order() {
        let file = temp("save-order").join("catalog.json");
        let sec = |id: &str| scan::Section {
            id: id.into(), name: id.into(), path: format!("Q:\\{id}"), drive: "Q:\\".into(), kind: Kind::Books, guessed: false, online: true, gone: false,
            works: vec![], files: 0, scanned_at: 0, unlisted: vec![], empty: vec![], whole_tree: true, split_from: None, links: vec![],
        };
        let cat = Mutex::new(Catalog { v: packed::VERSION, sections: vec![sec("kept")] });
        let on_disk = || store::load::<Catalog>(&file).sections;
        std::thread::scope(|t| {
            t.spawn(|| {
                for i in 0..40 {
                    let mut c = cat.lock().unwrap();
                    c.sections[0].name = format!("renamed {i}");
                    save_catalog(&file, &c).unwrap();
                    assert_eq!(on_disk(), c.sections, "the file holds this rename");
                }
            });
            t.spawn(|| {
                for i in 0..40 {
                    let mut c = cat.lock().unwrap();
                    if i % 2 == 0 {
                        c.sections.push(sec(&format!("gone {i}")));
                    } else {
                        c.sections.retain(|x| x.id == "kept");
                    }
                    save_catalog(&file, &c).unwrap();
                    assert_eq!(on_disk(), c.sections, "the file holds this forget");
                }
            });
        });
        let c = cat.lock().unwrap();
        assert_eq!(c.sections.len(), 1);
        assert_eq!(c.sections[0].name, "renamed 39");
        assert_eq!(on_disk(), c.sections, "the newer catalog is on disk");
        let _ = std::fs::remove_dir_all(file.parent().unwrap());
    }

    /// Undo of a Move forgets the choices of the photos it put back, made before the move, and no
    /// other; what it forgot stays forgotten after a restart, and so does what it kept.
    #[test]
    fn undo_forgets_only_the_choices_of_the_photos_put_back() {
        let d = temp("undo-review");
        let data = temp("undo-review-data");
        let jf = data.join("moves.json");
        for f in ["a.jpg", "b.jpg", "c.jpg"] {
            std::fs::write(d.join(f), f).unwrap();
        }
        let seen = |f: &str| {
            let md = std::fs::metadata(d.join(f)).unwrap();
            let t = md.modified().unwrap().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
            moves::Photo { rel: f.into(), size: md.len(), mtime: t, cloud: false, preview: false }
        };
        let choice = |keep, at| store::Choice { keep, at, size: Some(5) };
        let before = store::now() - 10;
        let mut m = Marks::default();
        for (k, keep) in [("a.jpg", false), ("b.jpg", false), ("c.jpg", true), ("other.jpg", false)] {
            m.review.insert(format!("0000ABCD:Photos/{k}"), choice(keep, before));
        }
        let out = moves::move_photos(&jf, "0000ABCD:Photos", &d, &d.join(shots::NOT_NEEDED), &[seen("a.jpg"), seen("b.jpg")], &|_, _| {}).unwrap();
        assert_eq!(forget_put_back(&mut m, &moves::put_back(&jf)), 0, "no Undo yet");
        // b is reviewed again after the move, so its new choice stays.
        m.review.insert("0000ABCD:Photos/b.jpg".into(), choice(true, store::now() + 10));
        moves::undo(&jf, out.id, &|_, _| {}).unwrap();
        assert_eq!(forget_put_back(&mut m, &moves::put_back(&jf)), 1);
        let marks = data.join("marks.json");
        store::save(&marks, &m).unwrap();
        let mut m: Marks = store::load(&marks);
        assert!(!m.review.contains_key("0000ABCD:Photos/a.jpg"), "a waits for Review again");
        assert!(m.review["0000ABCD:Photos/b.jpg"].keep && m.review["0000ABCD:Photos/c.jpg"].keep && !m.review["0000ABCD:Photos/other.jpg"].keep);
        assert_eq!(forget_put_back(&mut m, &moves::put_back(&jf)), 0, "a second start forgets nothing more");
        // A backup saved before the move brings a's old choice back; it is forgotten again.
        let mut old = Marks::default();
        old.review.insert("0000ABCD:Photos/a.jpg".into(), choice(false, before));
        m.merge(old);
        assert_eq!(forget_put_back(&mut m, &moves::put_back(&jf)), 1);
        assert!(!m.review.contains_key("0000ABCD:Photos/a.jpg"));
        let _ = std::fs::remove_dir_all(&d);
        let _ = std::fs::remove_dir_all(&data);
    }

    /// Review again forgets one photo's choice, which stays forgotten after a restart; its Undo puts
    /// the choice back as it was, date and size too.
    #[test]
    fn a_review_choice_is_forgotten_and_put_back() {
        let data = temp("forget");
        let file = data.join("marks.json");
        let mut m = Marks::default();
        m.review.insert("0000ABCD:Photos/a.jpg".into(), store::Choice { keep: true, at: 7, size: Some(3) });
        m.review.insert("0000ABCD:Photos/b.jpg".into(), store::Choice { keep: false, at: 8, size: Some(4) });
        let gone = m.forget_review(&["0000ABCD:Photos/a.jpg".into(), "0000ABCD:Photos/none.jpg".into()]);
        assert_eq!(gone.len(), 1);
        store::save(&file, &m).unwrap();
        let mut m: Marks = store::load(&file);
        assert!(!m.review.contains_key("0000ABCD:Photos/a.jpg") && m.review.contains_key("0000ABCD:Photos/b.jpg"));
        m.review.extend(gone);
        store::save(&file, &m).unwrap();
        let m: Marks = store::load(&file);
        let a = m.review["0000ABCD:Photos/a.jpg"];
        assert!(a.keep && a.at == 7 && a.size == Some(3));
        let _ = std::fs::remove_dir_all(&data);
    }

    #[test]
    fn a_saved_backup_reads_back() {
        let mut marks = Marks::default();
        marks.marks.insert("0A1B2C3D:Books/Опасный пациент".into(), store::Mark { m: "done".into(), at: 1_791_000_000 });
        marks.ticks.entry("0A1B2C3D:Courses/Python/".into()).or_default().insert("01 Intro".into(), 1_791_000_100);
        marks.review.insert("0A1B2C3D:Camera/IMG_0001.jpg".into(), store::Choice { keep: false, at: 1_791_000_200, size: Some(12) });
        let ui = serde_json::json!({ "size": "large" });
        let text = serde_json::to_string(&backup(marks.clone(), ui.clone())).unwrap();
        let b = read_backup(&text).expect("the app reads the backup it saved");
        assert_eq!(serde_json::to_value(&b.marks).unwrap(), serde_json::to_value(&marks).unwrap());
        assert_eq!(b.ui, ui);
        assert_eq!(text.matches("\"v\":").count(), 1, "one version key: {text}");
    }

    #[test]
    fn a_backup_saved_with_two_version_keys_still_reads() {
        // What the app wrote before the fix: the Backup's own `v` and the Marks' `v`.
        let text = r#"{"kind":"filesviewhub-marks","v":1,"v":2,"marks":{"0A1B2C3D:Books/Dune":{"m":"next","at":5}},"ticks":{},"review":{},"ui":{"size":"compact"}}"#;
        let b = read_backup(text).expect("a backup saved before the fix reads");
        assert_eq!(b.marks.marks["0A1B2C3D:Books/Dune"].m, "next");
        assert_eq!(b.ui, serde_json::json!({ "size": "compact" }));
    }

    #[test]
    fn a_backup_with_no_review_or_settings_reads() {
        let b = read_backup(r#"{"kind":"filesviewhub-marks","v":2,"marks":{}}"#).unwrap();
        assert!(b.marks.review.is_empty() && b.marks.ticks.is_empty());
        assert!(b.ui.is_null(), "no settings in the backup: the current ones stay");
        assert!(read_backup(r#"{"kind":"filesviewhub-marks","v":2,"marks":{},"ui":null}"#).unwrap().ui.is_null());
    }

    #[test]
    fn a_backup_from_a_newer_app_is_refused() {
        let err = read_backup(r#"{"kind":"filesviewhub-marks","v":3,"marks":{"0A1B2C3D:Books/Dune":{"m":"done","at":5}}}"#).err();
        assert_eq!(err.as_deref(), Some("This backup was saved by a newer FilesViewHub. Use that version to add it."));
        assert!(read_backup(r#"{"kind":"filesviewhub-marks","v":1,"marks":{}}"#).is_ok(), "an older backup still reads");
    }

    #[test]
    fn what_is_not_a_backup_is_refused() {
        let refused = [
            r#"{"kind":"hub-marks","v":3,"marks":{},"ticks":{}}"#,
            r#"{"v":2,"marks":{}}"#,
            r#"{"kind":"filesviewhub-marks""#,
            "",
            "[]",
        ];
        for t in refused {
            assert_eq!(read_backup(t).err().as_deref(), Some("This file is not a marks backup from FilesViewHub."), "{t}");
        }
    }

    #[test]
    fn a_cache_move_carries_only_the_caches_own_files() {
        let root = std::env::temp_dir().join(format!("fvh-move-{}", std::process::id()));
        let (from, to) = (root.join("old"), root.join("new"));
        std::fs::create_dir_all(from.join("thumbs").join("ab")).unwrap();
        std::fs::write(from.join("thumbs").join("ab").join("ab12.jpg"), b"x").unwrap();
        std::fs::write(from.join("thumbs.json"), b"{}").unwrap();
        std::fs::write(from.join("web.json"), b"{}").unwrap();
        std::fs::write(from.join("holiday.jpg"), b"mine").unwrap();
        move_cache(&from, &to).unwrap();
        assert!(to.join("thumbs").join("ab").join("ab12.jpg").exists());
        assert!(to.join("thumbs.json").exists());
        assert!(to.join("web.json").exists(), "an old web.json moves too, so its covers can still be taken over");
        assert!(from.join("holiday.jpg").exists(), "a file the cache does not own stays put");
        assert!(!to.join("holiday.jpg").exists());
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    #[cfg_attr(not(windows), ignore = "Windows paths")]
    fn the_cache_is_refused_in_or_around_a_checked_folder() {
        let books = [PathBuf::from(r"D:\Library\Books")];
        assert!(cache_refused(Path::new(r"d:\library\books\thumbs"), &books).is_some());
        assert!(cache_refused(Path::new(r"D:\Library"), &books).is_some());
        assert!(cache_refused(Path::new(r"D:\Cache"), &books).is_none());
    }

    #[test]
    fn other_spellings_of_a_checked_folder_are_refused_too() {
        let root = std::env::temp_dir().join(format!("fvh-real-{}", std::process::id()));
        let books = root.join("Books");
        std::fs::create_dir_all(books.join("x")).unwrap();
        let checked = [books.clone()];
        assert!(cache_refused(&books.join("x").join("..").join("Cache"), &checked).is_some());
        #[cfg(windows)]
        assert!(cache_refused(&root.join("Books.").join("Cache"), &checked).is_some());
        assert!(cache_refused(&root.join("Other"), &checked).is_none());
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_backup_is_refused_in_any_spelling_of_a_checked_folder() {
        let root = std::env::temp_dir().join(format!("fvh-backup-{}", std::process::id()));
        let books = root.join("Books");
        std::fs::create_dir_all(books.join("x")).unwrap();
        std::fs::create_dir_all(root.join("Other")).unwrap();
        let checked = [books.clone()];
        let file = "FilesViewHub marks.json";
        assert!(in_checked(&books.join(file), &checked));
        assert!(in_checked(&books.join("x").join("..").join(file), &checked));
        assert!(!in_checked(&root.join("Other").join(file), &checked));
        #[cfg(windows)]
        {
            assert!(in_checked(&root.join("Books.").join(file), &checked));
            let link = root.join("Link");
            let made = std::process::Command::new("cmd").args(["/c", "mklink", "/J"]).arg(&link).arg(&books).output().unwrap();
            assert!(made.status.success(), "{}", String::from_utf8_lossy(&made.stderr));
            assert!(in_checked(&link.join(file), &checked), "a junction to a checked folder");
            std::fs::remove_dir(&link).unwrap();
        }
        std::fs::remove_dir_all(&root).unwrap();
    }

    /// How the system resolves two mapped network drives: Q: to the share the Section is on, R:
    /// to another server; every other path is left as it is.
    fn mapped(p: &Path) -> PathBuf {
        let t = p.to_string_lossy();
        match (t.strip_prefix("Q:"), t.strip_prefix("R:")) {
            (Some(r), _) => PathBuf::from(format!(r"\\?\UNC\wsl$\Ubuntu-24.04{r}")),
            (_, Some(r)) => PathBuf::from(format!(r"\\?\UNC\other\share{r}")),
            _ => p.to_path_buf(),
        }
    }

    /// `open`'s and `reveal`'s check, on paths with no link the walk can see.
    fn opens(folders: &[PathBuf], path: &str, real: impl Fn(&Path) -> PathBuf, link: impl Fn(&Path) -> bool) -> bool {
        check(folders, path, |_| Ok(None), real, link).is_ok()
    }

    /// A `real` that records every path it is asked to resolve.
    fn recording(asked: &std::cell::RefCell<Vec<PathBuf>>) -> impl Fn(&Path) -> PathBuf + '_ {
        move |p: &Path| {
            asked.borrow_mut().push(p.to_path_buf());
            mapped(p)
        }
    }

    /// A file on a mapped network drive opens, though the system resolves it to the server's path
    ///; one on another drive letter, server or share does not. A network path, in any
    /// spelling, and a drive letter no folder is on are never resolved.
    #[test]
    #[cfg_attr(not(windows), ignore = "Windows paths")]
    fn a_mapped_drive_opens_only_its_own_folder() {
        let q = [PathBuf::from(r"Q:\tmp\fvh\Books")];
        let given = Path::new(r"Q:\tmp\fvh\Books\a.pdf");
        let none = |_: &Path| false;
        assert!(opens(&q, r"Q:\tmp\fvh\Books\a.pdf", mapped, none));
        assert!(!opens(&q, r"R:\tmp\fvh\Books\a.pdf", mapped, none), "a drive letter mapped elsewhere");
        for (resolved, what) in [
            (r"\\?\UNC\other\share\tmp\fvh\Books\a.pdf", "the same folders on another server"),
            (r"\\?\UNC\wsl$\Other\tmp\fvh\Books\a.pdf", "another share on the same server"),
            (r"\\?\UNC\wsl$\Ubuntu-24.04\tmp\fvh\Books2\a.pdf", "a folder next to it"),
        ] {
            assert!(!resolved_inside(&q, given, Path::new(resolved), mapped, none), "{what}");
        }
        let unfollowed = |p: &Path| p.ends_with("link");
        assert!(!resolved_inside(&q, given, Path::new(r"\\?\UNC\wsl$\Ubuntu-24.04\tmp\fvh\Books\link\a.pdf"), mapped, unfollowed), "a link on the share");
        for path in [
            r"\\other\share\tmp\fvh\Books\a.pdf",
            r"\\wsl$\Other\tmp\fvh\Books\a.pdf",
            r"\\?\UNC\other\share\a.pdf",
            "//other/share/a.pdf",
            r"\/other/share/a.pdf",
            r"/\other\share\a.pdf",
            r"\??\UNC\other\share\a.pdf",
            r"\\?\Q:\tmp\fvh\Books2\a.pdf",
            r"\\?\Q:\tmp\fvh\Books\a.pdf",
            r"\\?\Q:\tmp\fvh\Books\x/../../a.pdf",
            r"\\.\Q:\tmp\fvh\Books2\a.pdf",
            r"\\?\GLOBALROOT\Device\Mup\other\share\a.pdf",
            r"R:\tmp\fvh\Books\a.pdf",
            r"Q:tmp\fvh\Books\a.pdf",
        ] {
            let asked = std::cell::RefCell::new(vec![]);
            assert!(!inside(&q, path, recording(&asked)), "{path}");
            assert!(asked.borrow().is_empty(), "{path} is judged by its text, never resolved");
        }
    }

    /// A link in a checked folder is read, never followed, and one leading out of the folder is
    /// refused before anything is resolved, so a server it leads to is never asked. No
    /// path is read through a link whose target was not read first; a link to a link is read on.
    #[test]
    #[cfg_attr(not(windows), ignore = "Windows paths")]
    fn a_link_out_is_refused_unfollowed() {
        let folders = [PathBuf::from(r"E:\Books"), PathBuf::from(r"Q:\tmp\fvh\Books")];
        let links: HashMap<&str, &str> = HashMap::from([
            (r"E:\Books\unc", r"\\server\share"),
            (r"E:\Books\verbatim", r"\\?\UNC\server\share"),
            (r"E:\Books\mup", r"\\?\GLOBALROOT\Device\Mup\server\share"),
            (r"E:\Books\nt", r"\Device\Mup\server\share"),
            (r"E:\Books\global", r"\GLOBAL??\UNC\server\share"),
            (r"E:\Books\mapped", r"R:\share"),
            (r"E:\Books\local", r"\\?\C:\Private"),
            (r"E:\Books\climbs", r"..\Secret"),
            (r"E:\Books\chain", r"E:\Books\hop"),
            (r"E:\Books\hop", r"\\server\share"),
            (r"E:\Books\loop", r"E:\Books\loop"),
            (r"E:\Books\sub", r"Novels"),
            (r"E:\Books\own", r"\\?\E:\Books\Novels"),
            (r"E:\Books\around", r"..\Books\Novels"),
            (r"Q:\tmp\fvh\Books\other", r"Q:\tmp\other"),
            (r"Q:\tmp", r"\\?\UNC\wsl$\Ubuntu-24.04\tmp"),
        ]);
        let asked = std::cell::RefCell::new(vec![]);
        let read = |p: &Path| {
            asked.borrow_mut().push(p.to_path_buf());
            match p.to_string_lossy().as_ref() {
                r"E:\Books\unreadable" => Err(()),
                t => Ok(links.get(t).map(PathBuf::from)),
            }
        };
        let through_a_link = |asked: &[PathBuf]| asked.iter().any(|p| p.ancestors().skip(1).any(|a| links.contains_key(a.to_string_lossy().as_ref()) && !folders.iter().any(|f| store::within(f, a))));
        for link in ["unc", "verbatim", "mup", "nt", "global", "mapped", "local", "climbs", "chain", "loop", "unreadable", r"..\Books\unc", "other"] {
            let path = if link == "other" { r"Q:\tmp\fvh\Books\other\a.pdf".to_string() } else { format!(r"E:\Books\{link}\a.pdf") };
            asked.borrow_mut().clear();
            let resolved = std::cell::RefCell::new(vec![]);
            assert_eq!(check(&folders, &path, read, recording(&resolved), |_| false), Err(OUTSIDE), "{path}");
            assert!(resolved.borrow().iter().all(|p| folders.contains(p)), "{path} is never resolved, only its folder: {:?}", resolved.borrow());
            assert!(!through_a_link(&asked.borrow()), "{path}: read through a link: {:?}", asked.borrow());
        }
        asked.borrow_mut().clear();
        assert_eq!(check(&folders, r"E:\Books\unc::$INDEX_ALLOCATION\a.pdf", read, mapped, |_| false), Err("This file name cannot be opened."));
        assert!(asked.borrow().is_empty(), "a stream name is refused before anything is read");
        for path in [r"E:\Books\a.pdf", r"E:\Books\sub\a.pdf", r"E:\Books\own\a.pdf", r"E:\Books\around\a.pdf", r"Q:\tmp\fvh\Books\a.pdf"] {
            asked.borrow_mut().clear();
            assert!(!link_out(&folders, Path::new(path), read, mapped), "{path}");
            assert!(!through_a_link(&asked.borrow()), "{path}: read through a link: {:?}", asked.borrow());
        }
    }

    /// A path on macOS or Linux is judged by its text only: under macOS' `/net` resolving it would
    /// reach the host it names.
    #[test]
    fn a_path_with_no_drive_letter_is_never_resolved() {
        let folders = [PathBuf::from("/Volumes/Books")];
        for path in ["/net/evil.example/x/a.pdf", "/Volumes/Other/a.pdf"] {
            let asked = std::cell::RefCell::new(vec![]);
            assert!(!inside(&folders, path, recording(&asked)), "{path}");
            assert!(asked.borrow().is_empty(), "{path} is never resolved");
        }
    }

    /// A file a folder holds as written is let through without resolving any folder, and one on a
    /// mapped drive resolves only the folders on its letter, so a folder whose server does not
    /// answer holds up no other open.
    #[test]
    #[cfg_attr(not(windows), ignore = "Windows paths")]
    fn only_the_folders_on_the_paths_drive_are_resolved() {
        let folders = [PathBuf::from(r"R:\Kids"), PathBuf::from(r"Q:\tmp\fvh\Books"), PathBuf::from(r"E:\Books")];
        let none = |_: &Path| false;
        let asked = std::cell::RefCell::new(vec![]);
        assert!(opens(&folders, r"E:\Books\a.pdf", recording(&asked), none));
        assert_eq!(*asked.borrow(), [PathBuf::from(r"E:\Books\a.pdf")], "only the path itself, as open does");
        let asked = std::cell::RefCell::new(vec![]);
        assert!(opens(&folders, r"Q:\tmp\fvh\Books\a.pdf", recording(&asked), none));
        assert!(!asked.borrow().iter().any(|p| p.starts_with(r"R:\")), "R: is never asked");
    }

    /// File Explorer is never asked to show a name ending in a dot or a space, or what is inside
    /// one: it would show the sibling without that end.
    #[test]
    fn explorer_is_not_asked_to_show_a_name_ending_in_a_dot() {
        for p in [r"Q:\tmp\Books\a.pdf", r"Q:\tmp\Books\a.b.pdf"] {
            assert_eq!(unshowable(p), None);
        }
        if cfg!(windows) {
            assert_eq!(unshowable(r"Q:\tmp\Books.\a.pdf").unwrap(), "File Explorer cannot show a name ending in a dot: \u{201c}Books.\u{201d}");
            assert_eq!(unshowable(r"\\?\Q:\tmp\Books\a.pdf ").unwrap(), "File Explorer cannot show a name ending in a space: \u{201c}a.pdf \u{201d}");
        } else {
            assert_eq!(unshowable("/tmp/Books./a.pdf"), None);
        }
    }

    /// The path the system is given has no `\\?\` start, unless a name ends in a dot or a space or
    /// the path is too long without it.
    #[test]
    fn a_server_path_is_opened_without_its_verbatim_start() {
        assert_eq!(plain_unc(r"\\?\UNC\wsl$\Ubuntu-24.04\tmp\a.pdf".into()), r"\\wsl$\Ubuntu-24.04\tmp\a.pdf");
        assert_eq!(plain_unc(r"\\?\UNC\srv\share\x.\a.pdf".into()), r"\\?\UNC\srv\share\x.\a.pdf");
        let long = format!(r"\\?\UNC\srv\share\{}\a.pdf", "x".repeat(250));
        assert_eq!(plain_unc(long.clone()), long);
        assert_eq!(plain_unc(r"E:\Books\a.pdf".into()), r"E:\Books\a.pdf");
    }

    /// A path that climbs out with `..`, or through a link or junction inside the folder, followed
    /// or not, is not opened; a file of a local Section is.
    #[test]
    fn open_refuses_a_path_out_of_the_folder() {
        let d = temp("open-out");
        let (books, out) = (d.join("Books"), d.join("Secret"));
        std::fs::create_dir_all(&books).unwrap();
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(books.join("a.pdf"), "a").unwrap();
        std::fs::write(out.join("s.pdf"), "s").unwrap();
        let folders = [books.clone()];
        let s = |p: PathBuf| p.to_string_lossy().to_string();
        assert!(opens(&folders, &s(books.join("a.pdf")), store::real, is_link), "a local Section's file");
        assert!(!opens(&folders, &s(books.join("..").join("Secret").join("s.pdf")), store::real, is_link));
        assert!(!opens(&folders, &s(books.join("x").join("..").join("a.pdf")), store::real, is_link), "any `..` is refused");
        #[cfg(windows)]
        let made = std::process::Command::new("cmd").args(["/c", "mklink", "/J"]).arg(books.join("link")).arg(&out).output().unwrap().status.success();
        #[cfg(unix)]
        let made = std::os::unix::fs::symlink(&out, books.join("link")).is_ok();
        assert!(made, "the link is made");
        let through = s(books.join("link").join("s.pdf"));
        assert!(inside(&folders, &through, store::real), "its text is inside");
        assert!(!opens(&folders, &through, store::real, is_link), "where it leads is not");
        let through = Path::new(&through);
        assert!(!resolved_inside(&folders, through, through, store::real, is_link), "a link the system did not follow");
        let a = books.join("a.pdf");
        assert!(resolved_inside(&folders, &a, &store::real(&a), store::real, is_link));
        let checks = |p: &Path| check(&folders, &s(p.to_path_buf()), link_target, store::real, is_link);
        assert_eq!(checks(through), Err(OUTSIDE), "open and reveal refuse it alike");
        assert!(checks(&a).is_ok());
        // A name ending in a dot or a space that is not there as named is read without that end,
        // as `real` would resolve it, and the link there is not followed.
        #[cfg(windows)]
        for name in ["link.", "link "] {
            let p = s(books.join(name).join("s.pdf"));
            let asked = std::cell::RefCell::new(vec![]);
            let real = |q: &Path| {
                asked.borrow_mut().push(q.to_path_buf());
                store::real(q)
            };
            assert_eq!(check(&folders, &p, link_target, real, is_link), Err(OUTSIDE), "{p}");
            assert!(asked.borrow().iter().all(|q| folders.contains(q)), "{p} is not resolved: {:?}", asked.borrow());
        }
        // A link whose name ends in a dot is read as named, not as its name without the dot.
        #[cfg(windows)]
        {
            let v = |p: PathBuf| PathBuf::from(format!(r"\\?\{}", p.display()));
            std::fs::rename(v(books.join("link")), v(books.join("out."))).unwrap();
            assert!(link_out(&folders, &books.join("out.").join("s.pdf"), link_target, store::real), "the link out. leads out");
            assert_eq!(checks(&books.join("out.").join("s.pdf")), Err(OUTSIDE));
            std::fs::remove_dir(v(books.join("out."))).unwrap();
        }
        let _ = std::fs::remove_dir_all(&d);
    }

    /// A folder named `Books.`, as a share can hold, opens its own file, never the one of its
    /// sibling `Books`, and a file of that sibling is refused; a path whose dot Windows drops
    /// still finds the folder without it. NTFS holds such names too, made with `\\?\`.
    #[test]
    #[cfg(windows)]
    fn a_name_ending_in_a_dot_opens_its_own_file() {
        // The long form of the temp folder: TEMP may hold a short name (RUNNER~1), which the app resolves.
        let d = PathBuf::from(std::fs::canonicalize(temp("dot")).unwrap().to_string_lossy().trim_start_matches(r"\\?\").to_string());
        let v = |p: PathBuf| PathBuf::from(format!(r"\\?\{}", p.display()));
        let (dotted, plain, spaced) = (d.join("Books."), d.join("Books"), d.join("Notes "));
        for f in [&dotted, &plain, &spaced] {
            std::fs::create_dir_all(v(f.clone())).unwrap();
            std::fs::write(v(f.join("a.pdf")), f.to_string_lossy().as_bytes()).unwrap();
        }
        let s = |p: PathBuf| p.to_string_lossy().to_string();
        let a = dotted.join("a.pdf");
        assert_eq!(store::real(&a), v(a.clone()), "the dot is kept, with the start that keeps it");
        assert_eq!(std::fs::read_to_string(plain_unc(s(store::real(&a)))).unwrap(), s(dotted.clone()), "its own file opens");
        let folders = [dotted.clone()];
        assert!(opens(&folders, &s(a.clone()), store::real, is_link), "a file of the checked Books.");
        assert!(!opens(&folders, &s(plain.join("a.pdf")), store::real, is_link), "a file of the unchecked Books");
        assert!(!opens(std::slice::from_ref(&plain), &s(a.clone()), store::real, is_link), "and the other way round");
        assert!(opens(std::slice::from_ref(&d), &s(spaced.join("a.pdf")), store::real, is_link), "a folder ending in a space, inside a checked one");
        assert_eq!(store::real(&d.join("Books..").join("a.pdf")), plain.join("a.pdf"), "no such name: the dots go, as Windows drops them");
        assert_eq!(store::real(&plain.join("a.pdf")), plain.join("a.pdf"), "a plain name is as before");
        let gone = d.join("Gone.");
        std::fs::create_dir_all(d.join("Gone")).unwrap();
        std::fs::write(d.join("Gone").join("a.pdf"), "sibling").unwrap();
        assert!(!opens(std::slice::from_ref(&gone), &s(gone.join("a.pdf")), store::real, is_link), "Gone. is not there: never the sibling's file");
        let _ = std::fs::remove_dir_all(v(d));
    }

    #[test]
    fn a_type_is_read_without_a_trailing_dot_or_space() {
        for (path, ext) in [(r"\\?\UNC\srv\share\run.VBS.", "vbs"), (r"\\?\UNC\srv\share\run.vbs ", "vbs"), (r"\\?\UNC\srv\Books.\a.pdf", "pdf"), (r"E:\Books\notes", "")] {
            assert_eq!(type_of(path), ext, "{path}");
        }
    }

    #[test]
    #[cfg_attr(not(windows), ignore = "Windows paths")]
    fn within_compares_whole_names_with_case_aside() {
        assert!(store::within(Path::new(r"E:\Books\A"), Path::new(r"e:\books")));
        assert!(!store::within(Path::new(r"E:\Books2"), Path::new(r"E:\Books")));
        assert!(store::within(Path::new(r"\\?\E:\Books\x.\a.pdf"), Path::new(r"E:\Books")), "the verbatim start names the same drive");
        assert!(!store::within(Path::new(r"\\?\E:\Books.\a.pdf"), Path::new(r"E:\Books")));
    }

    /// On Linux a file in `BOOKS` next to the checked `Books` is refused, and the one in `Books`
    /// opens; nothing is written in `BOOKS`, which may be `Books` on a FAT stick.
    #[test]
    #[cfg_attr(not(target_os = "linux"), ignore = "a folder name in other case is the same folder")]
    fn a_folder_in_other_case_is_another_folder() {
        let d = temp("case");
        for n in ["Books", "BOOKS"] {
            std::fs::create_dir_all(d.join(n)).unwrap();
            std::fs::write(d.join(n).join("a.pdf"), n).unwrap();
        }
        let at = |n: &str| d.join(n).join("a.pdf");
        let open = |n: &str| check(&[d.join("Books")], &at(n).to_string_lossy(), link_target, store::real, is_link);
        assert_eq!(open("Books"), Ok(at("Books")));
        assert_eq!(open("BOOKS"), Err(OUTSIDE));
        assert!(in_checked(&d.join("BOOKS").join("marks.json"), &[d.join("Books")]));
        let _ = std::fs::remove_dir_all(&d);
    }
}
