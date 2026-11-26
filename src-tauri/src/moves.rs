//! Review's move (`decisions/2026/10/09/review-moves-set-aside-photos-itself-with-undo.md`): the photos set aside go into `Not needed` inside the photo folder, or
//! a folder the person picks outside every checked folder. On one drive a photo is renamed; on
//! another it is copied, the copy flushed to disk and checked by its hash, and only then the
//! original removed. Each move is written to `moves.json` in StoredData before any file moves, so
//! Undo can put photos back and a move cut short is settled on a later start. Nothing is ever
//! deleted outright. One move, Undo or settling runs at a time, across every running copy of the
//! app, held by a lock on `moves.lock` that the system lets go of if the app dies.

use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

/// The moves kept for Undo, newest last.
const KEEP: usize = 30;
/// The name a copy has until it is checked, so a copy cut short never looks like a photo.
const PART: &str = ".filesviewhub-part";
/// Seconds two dates of one file may differ by: FAT keeps them to 2 s.
const SLACK: i64 = 2;

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Journal {
    pub moves: Vec<Move>,
}

/// One press of Move: every photo of one Section it took.
#[derive(Serialize, Deserialize, Clone)]
pub struct Move {
    pub id: u64,
    /// When, as seconds since 1970.
    pub at: i64,
    pub section: String,
    /// The folder the photos went into.
    pub to: String,
    /// The volume serials of the photo folder's drive and of the drive moved to, as found then,
    /// so Undo never writes to another drive that now has the same letter.
    #[serde(default)]
    pub from_drive: Option<u32>,
    #[serde(default)]
    pub to_drive: Option<u32>,
    pub files: Vec<Entry>,
    /// Undo put back every photo it could; nothing of this move is left to put back.
    #[serde(default)]
    pub undone: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Entry {
    pub from: String,
    pub to: String,
    pub size: u64,
    /// The date of the photo in its new place, as found right after the move: Undo puts it back
    /// only while it still has this size and date.
    pub mtime: i64,
    pub state: State,
    /// An Insta360 preview moved with its video: moved and put back, never counted as a photo.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub preview: bool,
    /// Why it was skipped or not put back, in words for the person.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub why: Option<String>,
    /// The key of the photo's Review choice (`store::key_in` of the Section and its path); none
    /// for a preview, and for a move made in an earlier version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum State {
    /// Written down, not moved yet; one still in this state after a crash is settled later.
    Planned,
    Moved,
    Skipped,
    /// Put back by Undo.
    Back,
}

/// A photo to move as Review saw it: its path from the Section's folder, size and date, and
/// whether it was only in the cloud.
pub struct Photo {
    pub rel: String,
    pub size: u64,
    pub mtime: u64,
    pub cloud: bool,
    /// An Insta360 preview that goes with a set-aside video (`shots::to_move`).
    pub preview: bool,
}

pub fn journal_file() -> PathBuf {
    crate::store::data_dir().join("moves.json")
}

/// What a move or an Undo did, for the person.
#[derive(Serialize, Clone, Default, Debug)]
pub struct Outcome {
    pub id: u64,
    pub moved: usize,
    /// Each skipped photo's name and why.
    pub skipped: Vec<(String, String)>,
}

/// The lock that lets one move, Undo or settling run at a time; dropped, it lets go.
struct Lock(#[allow(dead_code)] std::fs::File);

fn lock(journal: &Path) -> Result<Lock, String> {
    if let Some(dir) = journal.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let f = std::fs::File::options().create(true).truncate(false).write(true).open(journal.with_extension("lock")).map_err(|e| e.to_string())?;
    match f.try_lock() {
        Ok(()) => Ok(Lock(f)),
        Err(std::fs::TryLockError::WouldBlock) => Err("Photos are being moved already, in this window or another FilesViewHub. Wait for that to finish.".into()),
        Err(std::fs::TryLockError::Error(e)) => Err(e.to_string()),
    }
}

fn load(journal: &Path) -> Journal {
    crate::store::load(journal)
}

/// Saves the journal flushed to disk, so a power cut never leaves it half written.
/// Moves kept for a whole drive's Section or folder as `{serial}:/path` in an earlier version are rewritten
/// as `{serial}:path` (`store::rekey`), so Undo still finds them.
pub fn rekey(journal: &Path, drives: &[String]) {
    if !journal.exists() {
        return;
    }
    let mut j = load(journal);
    let mut n = 0;
    for m in &mut j.moves {
        if let Some(k) = crate::store::rekey(&m.section, drives) {
            m.section = k;
            n += 1;
        }
    }
    if n > 0 {
        let _ = save(journal, &j);
    }
}

fn save(journal: &Path, j: &Journal) -> Result<(), String> {
    let tmp = journal.with_extension("json.tmp");
    let text = serde_json::to_string(j).map_err(|e| e.to_string())?;
    let mut f = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
    f.write_all(text.as_bytes()).and_then(|_| f.sync_all()).map_err(|e| e.to_string())?;
    drop(f);
    std::fs::rename(&tmp, journal).map_err(|e| e.to_string())
}

fn mtime(md: &std::fs::Metadata) -> i64 {
    md.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs() as i64)
}

fn name_of(p: &str) -> String {
    Path::new(p).file_name().map_or_else(|| p.to_string(), |n| n.to_string_lossy().into())
}

/// A path from a Section's folder made of plain names only, never `..` or a drive.
fn plain(rel: &str) -> Option<PathBuf> {
    let p: PathBuf = rel.replace('\\', "/").split('/').filter(|s| !s.is_empty()).collect();
    (p.components().count() > 0 && p.components().all(|c| matches!(c, Component::Normal(_)))).then_some(p)
}

/// The serial of the drive holding `p`, as the system finds it now.
fn drive_serial(drives: &[crate::drives::Drive], p: &Path) -> Option<u32> {
    crate::drives::drive_of(drives, p).map(|d| d.serial)
}

/// Moves the photos from `root` into `dest`, keeping their folders under it, and returns what it
/// did. A photo moves only while it is the one Review saw (same size and date) and is on disk. The
/// journal is saved before the first file moves and again at the end. A press that would move
/// nothing is not kept, so it never hides the move before it from Undo.
pub fn move_photos(journal: &Path, section: &str, root: &Path, dest: &Path, photos: &[Photo], progress: &dyn Fn(usize, usize)) -> Result<Outcome, String> {
    let _lock = lock(journal)?;
    let mut j = load(journal);
    let id = j.moves.iter().map(|m| m.id).max().unwrap_or(0).max(now_ms()) + 1;
    let mut files = vec![];
    for p in photos {
        let Some(rel) = plain(&p.rel) else { continue };
        let (from, to) = (root.join(&rel), dest.join(&rel));
        let mut e = Entry { from: from.to_string_lossy().into(), to: to.to_string_lossy().into(), size: p.size, mtime: 0, state: State::Planned, preview: p.preview, why: None,
            key: (!p.preview).then(|| crate::store::key_in(section, &p.rel)) };
        // Read as named: on Windows `Photos.\a.jpg` without the `\\?\` start is `Photos\a.jpg`.
        let (from, to) = (crate::store::named(&from), crate::store::named(&to));
        match std::fs::metadata(&from) {
            Err(_) => skip(&mut e, "not there any more"),
            Ok(md) if p.cloud || crate::scan::is_cloud(&md) => skip(&mut e, "only in the cloud: moving it could download it"),
            Ok(md) if md.len() != p.size || (mtime(&md) - p.mtime as i64).abs() > SLACK => skip(&mut e, "changed since it was read; read the folder again and review it"),
            Ok(_) if to.symlink_metadata().is_ok() => skip(&mut e, "already in that folder"),
            Ok(_) => {}
        }
        files.push(e);
    }
    let drives = crate::drives::list();
    let mut m = Move { id, at: now_ms() as i64 / 1000, section: section.into(), to: dest.to_string_lossy().into(), from_drive: drive_serial(&drives, root), to_drive: drive_serial(&drives, dest), files, undone: false };
    if !m.files.iter().any(|e| e.state == State::Planned) {
        return Ok(outcome(&m, State::Moved));
    }
    j.moves.push(m.clone());
    save(journal, &j)?;
    let total = m.files.len();
    for (k, e) in m.files.iter_mut().enumerate() {
        if e.state == State::Planned {
            let to = PathBuf::from(&e.to);
            match move_one(Path::new(&e.from), &to) {
                Ok(()) => {
                    e.state = State::Moved;
                    e.mtime = std::fs::metadata(crate::store::named(&to)).map_or(0, |md| mtime(&md));
                }
                Err(why) => skip(e, &why),
            }
        }
        progress(k + 1, total);
    }
    let out = outcome(&m, State::Moved);
    log::info!("moved {} photos of {section} into {}, {} skipped", out.moved, dest.display(), out.skipped.len());
    *j.moves.last_mut().unwrap() = m;
    // Older moves go, but never one with photos Undo could still put back.
    while j.moves.len() > KEEP {
        let Some(k) = j.moves.iter().position(|m| m.undone || !m.files.iter().any(|e| e.state == State::Moved)) else { break };
        j.moves.remove(k);
    }
    save(journal, &j)?;
    Ok(out)
}

/// Puts back the photos of one move that are still where it put them, unchanged, and whose old
/// place is free; the folders it made and left empty go too. A photo it cannot put back yet (its
/// drive away, the file open) keeps the move open, so Undo can be pressed again later.
pub fn undo(journal: &Path, id: u64, progress: &dyn Fn(usize, usize)) -> Result<Outcome, String> {
    let _lock = lock(journal)?;
    let mut j = load(journal);
    let m = j.moves.iter_mut().find(|m| m.id == id).ok_or("That move is no longer kept.")?;
    if m.undone {
        return Err("That move was undone already.".into());
    }
    let dest = PathBuf::from(&m.to);
    let drives = crate::drives::list();
    let root = m.files.first().map(|e| PathBuf::from(&e.from)).unwrap_or_default();
    if m.to_drive.is_some() && drive_serial(&drives, &dest) != m.to_drive || m.from_drive.is_some() && drive_serial(&drives, &root) != m.from_drive {
        return Err("A drive of this move is not connected, or now has another letter. Connect it as it was, then undo.".into());
    }
    let total = m.files.len();
    let mut back = 0;
    for (k, e) in m.files.iter_mut().enumerate() {
        progress(k + 1, total);
        if e.state != State::Moved {
            continue;
        }
        let (from, to) = (PathBuf::from(&e.from), PathBuf::from(&e.to));
        let (from, to) = (crate::store::named(&from), crate::store::named(&to));
        let same = std::fs::metadata(&to).is_ok_and(|md| md.len() == e.size && (mtime(&md) - e.mtime).abs() <= SLACK);
        let why = if !to.exists() {
            Some("no longer in the folder it was moved to")
        } else if !same {
            Some("changed since it was moved")
        } else if from.symlink_metadata().is_ok() {
            Some("a file of that name is in its old place")
        } else {
            None
        };
        match why.map_or_else(|| move_one(&to, &from), |w| Err(w.to_string())) {
            Ok(()) => {
                e.state = State::Back;
                e.why = None;
                back += usize::from(!e.preview);
                prune(Path::new(&e.to).parent(), &dest);
            }
            Err(w) => e.why = Some(w),
        }
    }
    m.undone = !m.files.iter().any(|e| e.state == State::Moved);
    // `Not needed` the move made inside the photo folder goes too once it is empty.
    if m.undone && dest.file_name().is_some_and(|n| n == crate::shots::NOT_NEEDED) {
        let _ = std::fs::remove_dir(crate::store::named(&dest));
    }
    let out = Outcome { id, moved: back, skipped: m.files.iter().filter(|e| e.state == State::Moved).map(|e| (name_of(&e.from), e.why.clone().unwrap_or_default())).collect() };
    log::info!("undid move {id}: {} put back, {} left", out.moved, out.skipped.len());
    save(journal, &j)?;
    Ok(out)
}

/// Settles a move cut short: a photo found only in its new place counts as moved, one found only
/// in its old place as not moved, and a checked copy beside its original finishes the move. A copy
/// left half made is removed. A photo whose drive is away stays as it is until a later start. Run
/// as the app starts; if another copy of the app is moving photos, it waits for a later start.
pub fn settle(journal: &Path) {
    let Ok(_lock) = lock(journal) else { return };
    let mut j = load(journal);
    let drives = crate::drives::list();
    let mut changed = false;
    for m in &mut j.moves {
        let root = m.files.first().map(|e| PathBuf::from(&e.from)).unwrap_or_default();
        let here = (m.from_drive.is_none() || drive_serial(&drives, &root) == m.from_drive) && (m.to_drive.is_none() || drive_serial(&drives, Path::new(&m.to)) == m.to_drive);
        if !here {
            continue;
        }
        for e in m.files.iter_mut().filter(|e| e.state == State::Planned) {
            changed = true;
            let (from, to) = (crate::store::named(Path::new(&e.from)), crate::store::named(Path::new(&e.to)));
            let (from, to) = (&*from, &*to);
            let _ = std::fs::remove_file(part_of(to));
            match (from.exists(), to.exists()) {
                (false, true) => {
                    e.state = State::Moved;
                    e.mtime = std::fs::metadata(to).map_or(0, |md| mtime(&md));
                }
                (true, true) if same_content(from, to).unwrap_or(false) && std::fs::remove_file(from).is_ok() => {
                    e.state = State::Moved;
                    e.mtime = std::fs::metadata(to).map_or(0, |md| mtime(&md));
                }
                (true, _) => skip(e, "the move was cut short; it stayed in place"),
                (false, false) => skip(e, "the move was cut short, and it is in neither place"),
            }
        }
    }
    if changed {
        log::warn!("a move was cut short last time; settled from what is on disk");
        let _ = save(journal, &j);
    }
}

/// The newest move that Undo can still take back of a Photos Section or Photos folder, by its id
/// (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`), or of a Photos folder in it or around it: the Photos kind may have moved to
/// another folder level since, and Undo puts each file back where it was all the same.
pub fn last(journal: &Path, place: &str) -> Option<Move> {
    let near = |m: &Move| m.section == place || m.section.starts_with(&crate::store::inside(place)) || place.starts_with(&crate::store::inside(&m.section));
    load(journal).moves.into_iter().rev().find(|m| near(m) && !m.undone && m.files.iter().any(|e| e.state == State::Moved))
}

/// The Review choices of the photos Undo put back, each with when its move was made: a
/// choice made before then is the one that moved the photo, so it waits for Review again; one
/// made after stays. A move made in an earlier version has no keys, so they come from the drive the photo
/// is on now; one whose drive is away is passed over until a later start.
pub fn put_back(journal: &Path) -> Vec<(String, i64)> {
    let j = load(journal);
    let mut drives = None;
    let mut out = vec![];
    for m in &j.moves {
        for e in m.files.iter().filter(|e| e.state == State::Back && !e.preview) {
            let key = e.key.clone().or_else(|| old_key(&m.section, &e.from, drives.get_or_insert_with(crate::drives::list)));
            out.extend(key.map(|k| (k, m.at)));
        }
    }
    out
}

/// The Review key of `from`, a photo of Section or Photos folder `section` (`{serial}:path`),
/// from the drive with that serial it sits on now.
fn old_key(section: &str, from: &str, drives: &[crate::drives::Drive]) -> Option<String> {
    let (serial, rel) = section.split_once(':')?;
    let serial = u32::from_str_radix(serial, 16).ok()?;
    let p = Path::new(from);
    let d = [crate::drives::drive_of(drives, p), crate::drives::mount_of(drives, p)].into_iter().flatten().find(|d| d.serial == serial)?;
    let on = crate::drives::rel_on_drive(d, p);
    let inner = if rel.is_empty() { Some(on.as_str()) } else { on.strip_prefix(rel).and_then(|r| r.strip_prefix('/')) }?;
    (!inner.is_empty()).then(|| crate::store::key_in(section, inner))
}

fn outcome(m: &Move, done: State) -> Outcome {
    Outcome {
        id: m.id,
        moved: m.files.iter().filter(|e| e.state == done && !e.preview).count(),
        skipped: m.files.iter().filter(|e| e.state == State::Skipped).map(|e| (name_of(&e.from), e.why.clone().unwrap_or_default())).collect(),
    }
}

fn skip(e: &mut Entry, why: &str) {
    e.state = State::Skipped;
    e.why = Some(why.into());
}

fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

/// Removes the folders under `dest` that a move made and left empty, from `dir` up.
fn prune(mut dir: Option<&Path>, dest: &Path) {
    while let Some(d) = dir {
        if d == dest || !d.starts_with(dest) || std::fs::remove_dir(crate::store::named(d)).is_err() {
            break;
        }
        dir = d.parent();
    }
}

fn part_of(to: &Path) -> PathBuf {
    let mut s = to.as_os_str().to_owned();
    s.push(PART);
    PathBuf::from(s)
}

/// Moves one file without ever replacing another: a rename on one drive, else a copy flushed to
/// disk and checked by its hash, then the original removed.
pub fn move_one(from: &Path, to: &Path) -> Result<(), String> {
    let (from, to) = (crate::store::named(from), crate::store::named(to));
    let (from, to) = (&*from, &*to);
    if let Some(dir) = to.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("could not make its folder: {e}"))?;
    }
    match rename_new(from, to) {
        Ok(()) => return Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::CrossesDevices => {}
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => return Err("already in that folder".into()),
        Err(e) => return Err(format!("could not move it: {e}")),
    }
    let part = part_of(to);
    let undo_part = |why: String| {
        let _ = std::fs::remove_file(&part);
        Err(why)
    };
    if let Err(e) = std::fs::copy(from, &part) {
        return undo_part(format!("could not copy it: {e}"));
    }
    // The copy keeps the photo's date, as a rename does, and is on the disk before it is checked.
    let flushed = std::fs::File::options().write(true).open(&part).and_then(|f| {
        if let Ok(t) = std::fs::metadata(from).and_then(|m| m.modified()) {
            let _ = f.set_modified(t);
        }
        f.sync_all()
    });
    if let Err(e) = flushed {
        return undo_part(format!("could not write the copy to disk: {e}"));
    }
    match same_content(from, &part) {
        Ok(true) => {}
        Ok(false) => return undo_part("the copy did not match the photo".into()),
        Err(e) => return undo_part(format!("could not check the copy: {e}")),
    }
    if let Err(e) = rename_new(&part, to) {
        return undo_part(format!("could not name the copy: {e}"));
    }
    #[cfg(not(windows))]
    if let Some(dir) = to.parent() {
        let _ = std::fs::File::open(dir).and_then(|d| d.sync_all());
    }
    if let Err(e) = std::fs::remove_file(from) {
        // The original is still there, so its checked copy goes and the photo stays where it was.
        // If the original went after all (a network drive can answer late), the copy is kept.
        if from.exists() {
            let _ = std::fs::remove_file(to);
            return Err(format!("could not remove it from its old place: {e}"));
        }
    }
    Ok(())
}

/// Renames `from` to `to`, failing rather than replacing a file already at `to`.
#[cfg(windows)]
fn rename_new(from: &Path, to: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{MOVEFILE_WRITE_THROUGH, MoveFileExW};
    // The `\\?\` form lets a path run past 260 characters.
    let wide = |p: &Path| {
        let s = p.as_os_str().to_string_lossy();
        let long = if s.starts_with(r"\\?\") {
            s.to_string()
        } else if let Some(unc) = s.strip_prefix(r"\\") {
            format!(r"\\?\UNC\{unc}")
        } else {
            format!(r"\\?\{s}")
        };
        std::ffi::OsStr::new(&long).encode_wide().chain([0]).collect::<Vec<u16>>()
    };
    // Without MOVEFILE_REPLACE_EXISTING it fails on a file at `to`; without MOVEFILE_COPY_ALLOWED
    // it fails across drives (ERROR_NOT_SAME_DEVICE), which the caller turns into a checked copy.
    if unsafe { MoveFileExW(wide(from).as_ptr(), wide(to).as_ptr(), MOVEFILE_WRITE_THROUGH) } != 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

/// On Linux `renameat2(RENAME_NOREPLACE)` fails on a file at `to` in the same step as the rename.
/// A file system without it (some network and FUSE ones) gets the check, then the rename.
#[cfg(target_os = "linux")]
fn rename_new(from: &Path, to: &Path) -> std::io::Result<()> {
    use std::os::unix::ffi::OsStrExt;
    let c = |p: &Path| std::ffi::CString::new(p.as_os_str().as_bytes()).map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidInput));
    let (f, t) = (c(from)?, c(to)?);
    if unsafe { libc::renameat2(libc::AT_FDCWD, f.as_ptr(), libc::AT_FDCWD, t.as_ptr(), libc::RENAME_NOREPLACE) } == 0 {
        return Ok(());
    }
    let e = std::io::Error::last_os_error();
    if !matches!(e.raw_os_error(), Some(libc::EINVAL | libc::ENOSYS)) {
        return Err(e);
    }
    if to.symlink_metadata().is_ok() {
        return Err(std::io::ErrorKind::AlreadyExists.into());
    }
    std::fs::rename(from, to)
}

/// On macOS a file made at `to` between the check and the rename would be replaced;
/// `renamex_np(RENAME_EXCL)` closes that gap.
#[cfg(not(any(windows, target_os = "linux")))]
fn rename_new(from: &Path, to: &Path) -> std::io::Result<()> {
    if to.symlink_metadata().is_ok() {
        return Err(std::io::ErrorKind::AlreadyExists.into());
    }
    std::fs::rename(from, to)
}

/// The two files hold the same bytes, by size and SHA-1.
fn same_content(a: &Path, b: &Path) -> std::io::Result<bool> {
    if std::fs::metadata(a)?.len() != std::fs::metadata(b)?.len() {
        return Ok(false);
    }
    Ok(sha1(a)? == sha1(b)?)
}

fn sha1(p: &Path) -> std::io::Result<String> {
    let mut f = std::fs::File::open(p)?;
    let mut h = sha1_smol::Sha1::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(h.digest().to_string())
}

/// The Dropbox, OneDrive or other synced folder holding `p`, if any: its name and its folder.
pub fn sync_root(p: &Path) -> Option<(String, PathBuf)> {
    let p = crate::store::real(p);
    sync_roots().into_iter().find(|(_, r)| crate::store::within(&p, &crate::store::real(r)))
}

fn sync_roots() -> Vec<(String, PathBuf)> {
    let mut out = vec![];
    // Dropbox writes where each of its folders is in info.json.
    let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(PathBuf::from);
    let mut infos = vec![];
    for var in ["LOCALAPPDATA", "APPDATA"] {
        if let Some(d) = std::env::var_os(var) {
            infos.push(PathBuf::from(d).join("Dropbox").join("info.json"));
        }
    }
    if let Some(h) = &home {
        infos.push(h.join(".dropbox").join("info.json"));
    }
    for f in infos {
        let Ok(text) = std::fs::read_to_string(&f) else { continue };
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else { continue };
        for (_, acc) in v.as_object().into_iter().flatten() {
            if let Some(path) = acc.get("path").and_then(|p| p.as_str()) {
                out.push(("Dropbox".to_string(), PathBuf::from(path)));
            }
        }
    }
    for var in ["OneDrive", "OneDriveConsumer", "OneDriveCommercial"] {
        if let Some(d) = std::env::var_os(var) {
            out.push(("OneDrive".to_string(), PathBuf::from(d)));
        }
    }
    // macOS keeps every synced folder (Dropbox, OneDrive, Google Drive) under CloudStorage.
    if let Some(h) = &home {
        if let Ok(rd) = std::fs::read_dir(h.join("Library").join("CloudStorage")) {
            for d in rd.flatten() {
                let n = d.file_name().to_string_lossy().to_string();
                out.push((n.split('-').next().unwrap_or(&n).to_string(), d.path()));
            }
        }
        out.push(("iCloud Drive".to_string(), h.join("Library").join("Mobile Documents")));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("fvh-moves-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// A photo as Review saw it: its size and date read from the disk now.
    fn seen(root: &Path, rel: &str) -> Photo {
        let md = std::fs::metadata(root.join(rel)).ok();
        Photo { rel: rel.into(), size: md.as_ref().map_or(0, |m| m.len()), mtime: md.as_ref().map_or(0, |m| mtime(m) as u64), cloud: false, preview: false }
    }

    /// A move never replaces a file, and a copy is checked before the original goes.
    #[test]
    fn move_one_never_replaces() {
        let d = temp("one");
        std::fs::write(d.join("a.jpg"), "a").unwrap();
        std::fs::write(d.join("b.jpg"), "b").unwrap();
        assert!(move_one(&d.join("a.jpg"), &d.join("b.jpg")).is_err());
        assert_eq!(std::fs::read_to_string(d.join("b.jpg")).unwrap(), "b");
        assert!(d.join("a.jpg").exists());
        move_one(&d.join("a.jpg"), &d.join("Not needed").join("2018").join("a.jpg")).unwrap();
        assert!(!d.join("a.jpg").exists());
        assert_eq!(std::fs::read_to_string(d.join("Not needed/2018/a.jpg")).unwrap(), "a");
        let _ = std::fs::remove_dir_all(&d);
    }

    /// On Windows `Photos.` beside `Photos`, both holding `a.jpg`: the move, its Undo and the
    /// settling after a crash take only `Photos.`'s files, and `Photos` is never touched.
    #[test]
    #[cfg(windows)]
    fn a_folder_ending_in_a_dot_moves_as_named() {
        let n = |p: &Path| crate::store::named(p).into_owned();
        let base = temp("dot");
        let (d, sib) = (base.join("Photos."), base.join("Photos"));
        let jf = temp("dot-data").join("moves.json");
        for (dir, text, only) in [(&d, "dot", "b.jpg"), (&sib, "sibling", "s.jpg")] {
            std::fs::create_dir_all(n(&dir.join("2018"))).unwrap();
            for f in ["a.jpg", "2018/c.jpg", only] {
                std::fs::write(n(&dir.join(f)), text).unwrap();
            }
        }
        let seen_in = |dir: &Path, rel: &str| {
            let md = std::fs::metadata(n(&dir.join(rel))).unwrap();
            Photo { rel: rel.into(), size: md.len(), mtime: mtime(&md) as u64, cloud: false, preview: false }
        };
        let untouched = || {
            for f in ["a.jpg", "2018/c.jpg", "s.jpg"] {
                assert_eq!(std::fs::read_to_string(sib.join(f)).unwrap(), "sibling", "{f} of the sibling");
            }
            assert!(!sib.join(crate::shots::NOT_NEEDED).exists() && !sib.join("b.jpg").exists());
        };
        let dest = d.join(crate::shots::NOT_NEEDED);
        // `s.jpg` is only in the sibling, as Review would have seen it there.
        let photos = [seen_in(&d, "a.jpg"), seen_in(&d, "2018/c.jpg"), seen_in(&d, "b.jpg"), seen_in(&sib, "s.jpg")];
        let out = move_photos(&jf, "S", &d, &dest, &photos, &|_, _| {}).unwrap();
        assert_eq!(out.moved, 3, "{:?}", out.skipped);
        assert_eq!(out.skipped, [("s.jpg".to_string(), "not there any more".to_string())]);
        for f in ["a.jpg", "2018/c.jpg", "b.jpg"] {
            assert_eq!(std::fs::read_to_string(n(&dest.join(f))).unwrap(), "dot");
            assert!(!n(&d.join(f)).exists());
        }
        untouched();
        let back = undo(&jf, out.id, &|_, _| {}).unwrap();
        assert_eq!(back.moved, 3, "{:?}", back.skipped);
        for f in ["a.jpg", "2018/c.jpg", "b.jpg"] {
            assert_eq!(std::fs::read_to_string(n(&d.join(f))).unwrap(), "dot");
        }
        assert!(!n(&dest).exists(), "Not needed goes once empty");
        untouched();
        // Cut short after `a.jpg` reached `Not needed`: it counts as moved, though the sibling
        // still holds an `a.jpg` in the old place.
        std::fs::create_dir_all(n(&dest)).unwrap();
        move_one(&d.join("a.jpg"), &dest.join("a.jpg")).unwrap();
        let e = Entry { from: d.join("a.jpg").to_string_lossy().into(), to: dest.join("a.jpg").to_string_lossy().into(), size: 3, mtime: 0, state: State::Planned, preview: false, why: None, key: None };
        save(&jf, &Journal { moves: vec![Move { id: 1, at: 0, section: "S".into(), to: dest.to_string_lossy().into(), from_drive: None, to_drive: None, files: vec![e], undone: false }] }).unwrap();
        settle(&jf);
        assert_eq!(load(&jf).moves[0].files[0].state, State::Moved);
        untouched();
        let _ = std::fs::remove_dir_all(n(&d));
        let _ = std::fs::remove_dir_all(&base);
        let _ = std::fs::remove_dir_all(jf.parent().unwrap());
    }

    /// Move, then Undo: every photo is back, the folders made for them and `Not needed` are gone,
    /// and what was skipped says why. A second Undo is refused.
    #[test]
    fn move_and_undo() {
        let d = temp("undo");
        let jf = temp("undo-data").join("moves.json");
        std::fs::create_dir_all(d.join("2018")).unwrap();
        for f in ["a.jpg", "2018/c.jpg", "Фото it's.jpg", "edited.jpg"] {
            std::fs::write(d.join(f), f).unwrap();
        }
        let dest = d.join(crate::shots::NOT_NEEDED);
        let mut cloud = seen(&d, "a.jpg");
        cloud.rel = "cloud.jpg".into();
        cloud.cloud = true;
        let mut edited = seen(&d, "edited.jpg");
        edited.size += 1;
        let photos = [seen(&d, "a.jpg"), seen(&d, "2018/c.jpg"), seen(&d, "Фото it's.jpg"), seen(&d, "gone.jpg"), cloud, edited,
            Photo { rel: "../escape.jpg".into(), size: 0, mtime: 0, cloud: false, preview: false }];
        let out = move_photos(&jf, "S", &d, &dest, &photos, &|_, _| {}).unwrap();
        assert_eq!(out.moved, 3);
        assert_eq!(out.skipped.len(), 3, "{:?}", out.skipped);
        assert!(d.join("edited.jpg").exists());
        assert!(dest.join("2018/c.jpg").exists() && !d.join("2018/c.jpg").exists());
        assert_eq!(last(&jf, "S").unwrap().id, out.id);
        let back = undo(&jf, out.id, &|_, _| {}).unwrap();
        assert_eq!(back.moved, 3);
        for f in ["a.jpg", "2018/c.jpg", "Фото it's.jpg"] {
            assert_eq!(std::fs::read_to_string(d.join(f)).unwrap(), f);
        }
        assert!(!dest.exists());
        assert!(undo(&jf, out.id, &|_, _| {}).is_err());
        assert!(last(&jf, "S").is_none());
        let _ = std::fs::remove_dir_all(&d);
        let _ = std::fs::remove_dir_all(jf.parent().unwrap());
    }

    /// Undo names the Review choices of the photos it put back, and only those: not a photo it
    /// could not put back, nor a preview, which has no choice.
    #[test]
    fn undo_names_the_photos_it_put_back() {
        let d = temp("back");
        let jf = temp("back-data").join("moves.json");
        std::fs::create_dir_all(d.join("2018")).unwrap();
        for f in ["a.jpg", "2018/c.jpg", "b.jpg", "VID_1.lrv"] {
            std::fs::write(d.join(f), f).unwrap();
        }
        let dest = d.join(crate::shots::NOT_NEEDED);
        let mut lrv = seen(&d, "VID_1.lrv");
        lrv.preview = true;
        let out = move_photos(&jf, "0000ABCD:Photos", &d, &dest, &[seen(&d, "a.jpg"), seen(&d, "2018/c.jpg"), seen(&d, "b.jpg"), lrv], &|_, _| {}).unwrap();
        assert!(put_back(&jf).is_empty(), "nothing is put back before Undo");
        // b's old place is taken, so it stays moved.
        std::fs::write(d.join("b.jpg"), "other").unwrap();
        undo(&jf, out.id, &|_, _| {}).unwrap();
        let at = load(&jf).moves[0].at;
        let mut keys = put_back(&jf);
        keys.sort();
        assert_eq!(keys, [("0000ABCD:Photos/2018/c.jpg".to_string(), at), ("0000ABCD:Photos/a.jpg".to_string(), at)]);
        let _ = std::fs::remove_dir_all(&d);
        let _ = std::fs::remove_dir_all(jf.parent().unwrap());
    }

    /// A move made in an earlier version kept no keys: each comes from the drive with the Section's serial
    /// that the photo sits on now, for a Section, a Photos folder in one, a whole drive and a mount.
    #[test]
    fn old_moves_find_their_keys_on_the_drive() {
        let drive = |path: &str, serial| crate::drives::Drive { path: path.into(), label: String::new(), serial, kind: "fixed".into(), free: 0, total: 0, system: false };
        let ds = [drive(r"D:\", 0xABCD), drive(r"E:\", 0x1234), drive("/media/usb", 0x77)];
        let key = |sec: &str, from: &str| old_key(sec, from, &ds);
        assert_eq!(key("0000ABCD:Dropbox/Photos", r"D:\Dropbox\Photos\2018\a.jpg").as_deref(), Some("0000ABCD:Dropbox/Photos/2018/a.jpg"));
        assert_eq!(key("0000ABCD:Data/Phone", r"D:\Data\Phone\Фото it's.jpg").as_deref(), Some("0000ABCD:Data/Phone/Фото it's.jpg"));
        assert_eq!(key("00001234:", r"E:\Camera\b.jpg").as_deref(), Some("00001234:Camera/b.jpg"));
        assert_eq!(key("00000077:Photos", "/media/usb/Photos/c.jpg").as_deref(), Some("00000077:Photos/c.jpg"));
        assert_eq!(key("0000ABCD:Dropbox/Photos", r"E:\Dropbox\Photos\a.jpg"), None, "on a drive with another serial");
        assert_eq!(key("0000ABCD:Dropbox/Photos", r"D:\Dropbox\Photos2\a.jpg"), None, "outside the Section");
        assert_eq!(key("0000FFFF:Photos", r"Z:\Photos\a.jpg"), None, "its drive away");
    }

    /// A press that moves nothing is not kept, so Undo still finds the move before it; an Undo that
    /// cannot put every photo back stays open and can be pressed again.
    #[test]
    fn undo_stays_open_and_empty_moves_are_not_kept() {
        let d = temp("open");
        let jf = temp("open-data").join("moves.json");
        for f in ["a.jpg", "b.jpg"] {
            std::fs::write(d.join(f), f).unwrap();
        }
        let dest = d.join(crate::shots::NOT_NEEDED);
        let out = move_photos(&jf, "S", &d, &dest, &[seen(&d, "a.jpg"), seen(&d, "b.jpg")], &|_, _| {}).unwrap();
        // Pressed again with what Review saw before: both are gone from the photo folder.
        let again = move_photos(&jf, "S", &d, &dest, &[Photo { rel: "a.jpg".into(), size: 5, mtime: 0, cloud: false, preview: false }], &|_, _| {}).unwrap();
        assert_eq!(again.moved, 0);
        assert_eq!(last(&jf, "S").unwrap().id, out.id);
        // b's old place is taken, so it cannot go back yet.
        std::fs::write(d.join("b.jpg"), "other").unwrap();
        let first = undo(&jf, out.id, &|_, _| {}).unwrap();
        assert_eq!((first.moved, first.skipped.len()), (1, 1));
        assert_eq!(last(&jf, "S").unwrap().id, out.id);
        std::fs::remove_file(d.join("b.jpg")).unwrap();
        assert_eq!(undo(&jf, out.id, &|_, _| {}).unwrap().moved, 1);
        assert_eq!(std::fs::read_to_string(d.join("b.jpg")).unwrap(), "b.jpg");
        let _ = std::fs::remove_dir_all(&d);
        let _ = std::fs::remove_dir_all(jf.parent().unwrap());
    }

    /// While one move holds the lock, another is refused and settling waits.
    /// Undo stays offered when the Photos kind moved to the folder above or below (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`).
    #[test]
    fn the_last_move_is_found_from_a_folder_around_it() {
        let d = temp("last-near");
        let jf = d.join("moves.json");
        let m = |id, section: &str| Move { id, at: 0, section: section.into(), to: String::new(), from_drive: None, to_drive: None,
            files: vec![Entry { from: "a.jpg".into(), to: "a.jpg".into(), size: 1, mtime: 0, state: State::Moved, preview: false, why: None, key: None }], undone: false };
        save(&jf, &Journal { moves: vec![m(1, "S/Data/Phone"), m(2, "S/Other")] }).unwrap();
        assert_eq!(last(&jf, "S/Data/Phone").map(|m| m.id), Some(1));
        assert_eq!(last(&jf, "S/Data").map(|m| m.id), Some(1), "the folder above");
        assert_eq!(last(&jf, "S/Data/Phone/Camera").map(|m| m.id), Some(1), "a folder below");
        assert_eq!(last(&jf, "S/Dat").map(|m| m.id), None, "not a folder around it");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn one_move_at_a_time() {
        let d = temp("lock");
        let jf = d.join("moves.json");
        let held = lock(&jf).unwrap();
        assert!(move_photos(&jf, "S", &d, &d.join("x"), &[], &|_, _| {}).is_err());
        assert!(undo(&jf, 1, &|_, _| {}).is_err());
        drop(held);
        assert!(lock(&jf).is_ok());
        let _ = std::fs::remove_dir_all(&d);
    }

    /// A move cut short is settled from the disk: moved, kept in place, or a checked copy beside
    /// its original finished; a half-made copy goes.
    #[test]
    fn settle_after_a_crash() {
        let d = temp("settle");
        let jf = d.join("moves.json");
        let dest = d.join("Not needed");
        std::fs::create_dir_all(&dest).unwrap();
        // a: moved before the crash; b: not reached; c: copied and checked, original not removed yet.
        std::fs::write(dest.join("a.jpg"), "a").unwrap();
        std::fs::write(d.join("b.jpg"), "b").unwrap();
        std::fs::write(dest.join("b.jpg.filesviewhub-part"), "b-half").unwrap();
        std::fs::write(d.join("c.jpg"), "c").unwrap();
        std::fs::write(dest.join("c.jpg"), "c").unwrap();
        let e = |n: &str| Entry { from: d.join(n).to_string_lossy().into(), to: dest.join(n).to_string_lossy().into(), size: 1, mtime: 0, state: State::Planned, preview: false, why: None, key: None };
        let j = Journal { moves: vec![Move { id: 1, at: 0, section: "S".into(), to: dest.to_string_lossy().into(), from_drive: None, to_drive: None, files: vec![e("a.jpg"), e("b.jpg"), e("c.jpg")], undone: false }] };
        save(&jf, &j).unwrap();
        settle(&jf);
        let j = load(&jf);
        let st: Vec<State> = j.moves[0].files.iter().map(|e| e.state).collect();
        assert_eq!(st, [State::Moved, State::Skipped, State::Moved]);
        assert!(d.join("b.jpg").exists() && !dest.join("b.jpg.filesviewhub-part").exists());
        assert!(!d.join("c.jpg").exists() && dest.join("c.jpg").exists());
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Across drives: copied, checked, the original removed, its date kept; Undo copies it back.
    /// Set `FVH_OTHER_DRIVE` to a folder on a drive other than the temp folder's.
    #[test]
    #[ignore]
    fn across_drives() {
        let other = PathBuf::from(std::env::var("FVH_OTHER_DRIVE").expect("FVH_OTHER_DRIVE"));
        let d = temp("across");
        let dest = other.join(format!("fvh-across-{}", std::process::id()));
        let jf = d.join("moves.json");
        let body = vec![7u8; 3 << 20];
        std::fs::write(d.join("big.jpg"), &body).unwrap();
        let t0 = std::fs::metadata(d.join("big.jpg")).unwrap().modified().unwrap();
        let out = move_photos(&jf, "S", &d, &dest, &[seen(&d, "big.jpg")], &|_, _| {}).unwrap();
        assert_eq!(out.moved, 1, "{:?}", out.skipped);
        assert!(!d.join("big.jpg").exists());
        assert_eq!(std::fs::read(dest.join("big.jpg")).unwrap(), body);
        let t1 = std::fs::metadata(dest.join("big.jpg")).unwrap().modified().unwrap();
        assert!(t0.duration_since(t1).or(t1.duration_since(t0)).unwrap().as_secs() <= 2);
        assert_eq!(undo(&jf, out.id, &|_, _| {}).unwrap().moved, 1);
        assert_eq!(std::fs::read(d.join("big.jpg")).unwrap(), body);
        assert!(!dest.join("big.jpg").exists());
        let _ = std::fs::remove_dir_all(&d);
        let _ = std::fs::remove_dir_all(&dest);
    }
}
