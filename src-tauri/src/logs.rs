//! The app log and the network log read into entries for Settings, Log: time, level, a topic
//! and the message with its paths marked, so the UI can show a path as a link.

use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// The app log's name in the log folder: `filesviewhub.log` is written now, and each one put
/// aside is named for when (`filesviewhub_2026-10-28_14-39-00.log`).
const APP_LOG: &str = "filesviewhub";
/// The app log file's size, in bytes, at which it is put aside and a new one started.
const FILE_MAX: u128 = 1_000_000;
/// How many put aside are kept: with the one written now, the app log takes about 5 MB.
const OLD_KEPT: usize = 4;

/// The log plugin as the app sets it up, the app log in `dir` (none when the data folder cannot
/// be written). The plugin removes the oldest file past `OLD_KEPT`; a file it set aside as
/// `.bak`, when two were put aside in one second, it never removes, so those go here. Such a
/// file needs more than 1 MB logged in one second, which the app does not do (one line a file,
/// at the speed thumbnails are made), so the bound holds within a session too.
pub fn plugin(dir: Option<PathBuf>) -> tauri_plugin_log::Builder {
    use tauri_plugin_log::{RotationStrategy, Target, TargetKind, TimezoneStrategy};
    for f in dir.iter().flat_map(std::fs::read_dir).flatten().flatten() {
        let name = f.file_name().to_string_lossy().into_owned();
        if name.starts_with(&format!("{APP_LOG}_")) && name.ends_with(".log.bak") {
            let _ = std::fs::remove_file(f.path());
        }
    }
    tauri_plugin_log::Builder::default()
        .clear_targets()
        .targets(dir.map(|path| Target::new(TargetKind::Folder { path, file_name: Some(APP_LOG.into()) })))
        .max_file_size(FILE_MAX)
        .rotation_strategy(RotationStrategy::KeepSome(OLD_KEPT))
        .timezone_strategy(TimezoneStrategy::UseLocal)
        .level(log::LevelFilter::Info)
}

/// The app log's files in `dir`, oldest first: those put aside, by the time in their name (as
/// the plugin sorts them), then the one written now.
fn app_files(dir: &Path) -> Vec<PathBuf> {
    let mut old: Vec<(String, PathBuf)> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|f| {
            let name = f.file_name().to_string_lossy().into_owned();
            Some((name.strip_prefix(APP_LOG)?.strip_prefix('_')?.strip_suffix(".log")?.to_string(), f.path()))
        })
        .collect();
    old.sort();
    old.into_iter().map(|(_, f)| f).chain([dir.join(format!("{APP_LOG}.log"))]).collect()
}

/// The app log, its files in `dir` read into one, oldest first. A file that cannot be read
/// (removed as it was read) is left out.
pub fn read_app(dir: &Path) -> Vec<u8> {
    let mut raw = vec![];
    for f in app_files(dir) {
        let Ok(b) = std::fs::read(f) else { continue };
        raw.extend_from_slice(&b);
        if !raw.is_empty() && !raw.ends_with(b"\n") {
            raw.push(b'\n');
        }
    }
    raw
}

/// The logger from the first line of `run()`: the data files are read before the log file is
/// open, so what it is told then is held and written once `setup` hands it the log.
pub static EARLY: Early = Early::new();

pub struct Early {
    to: OnceLock<Box<dyn log::Log>>,
    held: Mutex<Vec<(log::Level, String, String)>>,
}

impl Early {
    const fn new() -> Early {
        Early { to: OnceLock::new(), held: Mutex::new(Vec::new()) }
    }

    /// Makes this the app's logger, holding info and above until `start`.
    pub fn hold(&'static self) {
        if log::set_logger(self).is_ok() {
            log::set_max_level(log::LevelFilter::Info);
        }
    }

    /// From now on every line goes to `to`; what was held waits for `release`, so it can follow
    /// the line that opens this session.
    pub fn start(&self, level: log::LevelFilter, to: Box<dyn log::Log>) {
        let _held = self.held.lock().unwrap();
        if self.to.set(to).is_ok() {
            log::set_max_level(level);
        }
    }

    /// Writes the held lines, in the order they came.
    pub fn release(&self) {
        let Some(to) = self.to.get() else { return };
        for (level, target, m) in self.held.lock().unwrap().drain(..) {
            to.log(&log::Record::builder().level(level).target(&target).args(format_args!("{m}")).build());
        }
        to.flush();
    }
}

impl log::Log for Early {
    fn enabled(&self, m: &log::Metadata) -> bool {
        self.to.get().map_or(m.level() <= log::Level::Info, |to| to.enabled(m))
    }

    fn log(&self, r: &log::Record) {
        if let Some(to) = self.to.get() {
            return to.log(r);
        }
        let mut held = self.held.lock().unwrap();
        match self.to.get() {
            Some(to) => to.log(r),
            None if r.level() <= log::Level::Info => held.push((r.level(), r.target().to_string(), r.args().to_string())),
            None => {}
        }
    }

    fn flush(&self) {
        if let Some(to) = self.to.get() {
            to.flush();
        }
    }
}

#[derive(Serialize, Debug, PartialEq)]
pub struct Part {
    pub text: String,
    pub path: bool,
}

#[derive(Serialize, Debug)]
pub struct Entry {
    /// `2026-10-08`; empty for a line in no known form.
    pub date: String,
    /// `09:01:44`, local time in the app log, UTC in the network log.
    pub time: String,
    pub utc: bool,
    /// `INFO`, `WARN`, `ERROR` (or `DEBUG`, `TRACE`); empty for a line in no known form.
    pub level: String,
    /// What the entry is about: start, scan, open, thumbs, net, save or other.
    pub topic: &'static str,
    pub parts: Vec<Part>,
}

/// The app log, oldest first. A line that does not start a new entry (a panic's next lines)
/// belongs to the entry before it; blank lines are dropped. Paths are found only under `roots`
/// (the checked folders that are online, and the app's own folders): those are the ones Show in
/// File Explorer opens, and the only ones worth asking the disk about.
pub fn app(raw: &[u8], roots: &[PathBuf]) -> Vec<Entry> {
    let text = String::from_utf8_lossy(raw);
    let mut heads: Vec<(String, String, String, String)> = vec![];
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        match head(line) {
            Some(h) => heads.push(h),
            None if !heads.is_empty() => {
                let m = &mut heads.last_mut().unwrap().3;
                m.push('\n');
                m.push_str(line);
            }
            None => heads.push((String::new(), String::new(), String::new(), line.to_string())),
        }
    }
    let roots: Vec<String> = roots.iter().map(|r| r.to_string_lossy().trim_end_matches(['\\', '/']).to_string()).filter(|r| r.len() > 1).collect();
    let mut dirs = Dirs::default();
    heads
        .into_iter()
        .map(|(date, time, level, msg)| Entry { topic: topic(&msg), parts: parts(&msg, &roots, &mut dirs), date, time, utc: false, level })
        .collect()
}

fn digits(s: &str, form: &str) -> bool {
    s.len() == form.len() && s.bytes().zip(form.bytes()).all(|(c, f)| if f == b'0' { c.is_ascii_digit() } else { c == f })
}

/// The network log (`web.rs`): `2026-10-08 13:02:40Z<tab>GET <address><tab><why>`, oldest first.
pub fn network(raw: &[u8]) -> Vec<Entry> {
    String::from_utf8_lossy(raw)
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let mut f = l.splitn(3, '\t');
            let (when, req, why) = (f.next().unwrap_or(""), f.next().filter(|r| !r.trim().is_empty()), f.next());
            let at = when.strip_suffix('Z').and_then(|w| w.split_once(' ')).filter(|(d, t)| digits(d, "0000-00-00") && digits(t, "00:00:00"));
            match (at, req) {
                (Some((d, t)), Some(req)) => {
                    let text = match why {
                        Some(w) if !w.is_empty() => format!("{req} ({w})"),
                        _ => req.to_string(),
                    };
                    Entry { date: d.into(), time: t.into(), utc: true, level: "INFO".into(), topic: "net", parts: vec![Part { text, path: false }] }
                }
                _ => Entry { date: String::new(), time: String::new(), utc: true, level: String::new(), topic: "net", parts: vec![Part { text: l.into(), path: false }] },
            }
        })
        .collect()
}

/// `[2026-10-08][09:01:44][INFO][filesviewhub_lib] message` into its date, time, level and message.
fn head(line: &str) -> Option<(String, String, String, String)> {
    let mut rest = line;
    let mut take = || -> Option<&str> {
        let r = rest.strip_prefix('[')?;
        let end = r.find(']')?;
        rest = &r[end + 1..];
        Some(&r[..end])
    };
    let (date, time, a, b) = (take()?, take()?, take()?, take()?);
    let ok = digits(date, "0000-00-00") && digits(time, "00:00:00");
    // The plugin writes the level before the target; its docs show it after. Either is read.
    let is_level = |l: &str| matches!(l, "ERROR" | "WARN" | "INFO" | "DEBUG" | "TRACE");
    let level = if is_level(a) { a } else { b };
    if !ok || !is_level(level) {
        return None;
    }
    Some((date.into(), time.into(), level.into(), rest.strip_prefix(' ').unwrap_or(rest).into()))
}

/// The topic, from how the app words each message (`log::info!` calls in `lib.rs` and the rest).
fn topic(m: &str) -> &'static str {
    let starts = |p: &[&str]| p.iter().any(|p| m.starts_with(p));
    if m.starts_with("FilesViewHub ") && m.contains(" started;") {
        "start"
    } else if starts(&["open ", "show ", "cannot open "]) {
        "open"
    } else if starts(&["network: "]) {
        "net"
    } else if starts(&["scanned ", "read "]) || m.ends_with(" is not connected") || m.ends_with(" is not there any more") {
        "scan"
    } else if m.contains("thumbnail") || starts(&["file details:", "photos for Review:", "cover lookup", "cannot read "]) {
        "thumbs"
    } else if starts(&["marks ", "mark moved", "all marks", "move script", "cache moved", "stopped showing", "web covers:"]) {
        "save"
    } else {
        "other"
    }
}

/// The message split into text and paths. A path is one under `roots`; it runs to the longest
/// stretch that exists on disk, so a name with spaces or brackets stays one path. A path that
/// no longer exists ends where the app's wording resumes (" (", ": ", " for ", " is ", ...).
fn parts(m: &str, roots: &[String], seen: &mut Dirs) -> Vec<Part> {
    let mut out: Vec<Part> = vec![];
    let mut text = String::new();
    let mut i = 0;
    let mut prev = ' ';
    while i < m.len() {
        let free = !prev.is_alphanumeric() && prev != '\\' && prev != '/';
        if let Some(len) = free.then(|| path_at(&m[i..], roots, seen)).flatten() {
            if !text.is_empty() {
                out.push(Part { text: std::mem::take(&mut text), path: false });
            }
            out.push(Part { text: m[i..i + len].into(), path: true });
            i += len;
            prev = ' ';
            continue;
        }
        let c = m[i..].chars().next().unwrap();
        prev = c;
        text.push(c);
        i += c.len_utf8();
    }
    if !text.is_empty() {
        out.push(Part { text, path: false });
    }
    out
}

/// Folder listings, by folder, names in lower case: a path is looked up in its folder's list,
/// so each folder is read once however many lines name it. A root that is not there is noted,
/// so its paths skip the lookups.
#[derive(Default)]
struct Dirs {
    lists: HashMap<String, HashSet<String>>,
    /// Whether each root is there, by its place in the roots list.
    roots: HashMap<usize, bool>,
}

fn exists(p: &str, dirs: &mut Dirs) -> bool {
    let Some(k) = p.rfind(['\\', '/']) else { return false };
    let (dir, name) = (&p[..k], &p[k + 1..]);
    if name.is_empty() {
        return false;
    }
    // `C:` alone is the current folder on C:, not its root.
    let dir = if dir.len() == 2 && dir.ends_with(':') || dir.is_empty() { format!("{dir}{}", &p[k..k + 1]) } else { dir.to_string() };
    dirs.lists
        .entry(dir.to_lowercase())
        .or_insert_with(|| std::fs::read_dir(&dir).map(|r| r.flatten().map(|e| e.file_name().to_string_lossy().to_lowercase()).collect()).unwrap_or_default())
        .contains(&name.to_lowercase())
}

/// Words that follow a path in the app's messages.
const STOPS: [&str; 10] = [" (", ": ", " for ", " is ", " in ", " again", " from ", " to ", ", ", " could not"];
/// The most ends of one path that are tried, so a long line stays quick.
const TRIES: usize = 24;
/// The longest path looked for past its root, in bytes.
const PATH_MAX: usize = 400;

/// Where the wording resumes in `s` from `from` on: the first stop word, in one pass.
fn first_stop(s: &str, from: usize) -> Option<usize> {
    s[from..].match_indices([' ', ',']).map(|(k, _)| k + from).find(|&k| STOPS.iter().any(|w| s[k..].starts_with(w)))
}

/// The length in bytes of a path under one of `roots` at the start of `s`.
fn path_at(s: &str, roots: &[String], dirs: &mut Dirs) -> Option<usize> {
    // A root ends at a separator or the end of the path; a bare drive (`C:`) only at `\`.
    let after = |r: &str, c: Option<char>| match c {
        _ if r.len() == 2 && r.ends_with(':') => c == Some('\\'),
        None => true,
        Some(c) => c == '\\' || c == '/' || !c.is_alphanumeric(),
    };
    let (at, root) = roots
        .iter()
        .enumerate()
        .filter(|(_, r)| s.len() >= r.len() && s.is_char_boundary(r.len()) && s[..r.len()].eq_ignore_ascii_case(r) && after(r, s[r.len()..].chars().next()))
        .map(|(i, r)| (i, r.len()))
        .max_by_key(|&(_, n)| n)?;
    // Only the next stretch of the line is looked at, so a long line stays linear.
    let mut cap = s.len().min(root + PATH_MAX);
    while !s.is_char_boundary(cap) {
        cap -= 1;
    }
    let s = &s[..cap];
    // The longest stretch it can be: up to a line end, a tab or a quote, and on Windows a `:`
    // past the drive (a name holds none; `a.mkv: why` would name a stream of `a.mkv`).
    let mut max = s.find(['\n', '\t', '"']).unwrap_or(s.len());
    if cfg!(windows)
        && let Some(k) = s[..max].char_indices().skip(2).find(|&(_, c)| c == ':').map(|(k, _)| k)
    {
        max = k;
    }
    let max = s[..max].trim_end().len();
    if max < root {
        return None;
    }
    let trim = |e: usize| s[..e].trim_end().len();
    let clean = |e: usize| s[..e].trim_end().trim_end_matches(['.', ',', ';', ':', ')']).trim_end().len();
    // A path ends where the wording resumes, at the first stop word or a later one (a name may
    // hold " for " or " ("): the ends tried are those from the first stop on, longest first.
    let stop = first_stop(&s[..max], root).unwrap_or(max);
    let there = *dirs.roots.entry(at).or_insert_with(|| crate::store::named(std::path::Path::new(&roots[at])).exists());
    if there {
        let mut ends: Vec<usize> = s[stop..max].match_indices(' ').map(|(k, _)| k + stop).take(TRIES).collect();
        if ends.len() < TRIES {
            ends.push(max);
        }
        // Longest first: an end that exists, or a file since gone, whose folder exists and runs
        // past the first stop word (so that stop word was part of a name). A gone file's name
        // ends at the first stop word after something shaped like an extension.
        for e in ends.into_iter().rev().flat_map(|e| [trim(e), clean(e)]) {
            if e <= root {
                continue;
            }
            if exists(&s[..e], dirs) {
                return Some(e);
            }
            if let Some(k) = s[..e].rfind(['\\', '/']).filter(|&k| k > stop && k > root)
                && exists(&s[..k], dirs)
            {
                let ends_ext = |at: usize| s[k..at].rfind('.').is_some_and(|d| at - (k + d) <= 13 && s[k + d + 1..at].chars().all(char::is_alphanumeric) && at > k + d + 1);
                let mut from = k;
                let named = loop {
                    match first_stop(&s[..max], from) {
                        Some(w) if ends_ext(w) => break Some(w),
                        Some(w) => from = w + 1,
                        None => break None,
                    }
                };
                let whole = ends_ext(clean(max)).then_some(max);
                return Some(clean(named.or(whole).or_else(|| first_stop(&s[..max], k)).unwrap_or(max)));
            }
        }
    }
    let e = clean(stop).max(root);
    Some(e)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The folders paths are found under in these tests: a drive that is not there, a share
    /// and the temp folder.
    fn roots() -> Vec<PathBuf> {
        vec![PathBuf::from("Q:\\"), PathBuf::from("\\\\localhost\\nothere"), std::env::temp_dir()]
    }

    fn app(raw: &[u8]) -> Vec<Entry> {
        super::app(raw, &roots())
    }

    fn texts(e: &Entry) -> Vec<(&str, bool)> {
        e.parts.iter().map(|p| (p.text.as_str(), p.path)).collect()
    }

    #[test]
    fn every_level_and_its_fields() {
        let raw = b"[2026-10-08][09:01:44][filesviewhub_lib][INFO] FilesViewHub 0.4.0 (build 412) started; data in Q:\\nowhere\\StoredData\n\
[2026-10-08][09:01:45][filesviewhub_lib::tools][WARN] ffprobe is missing from Q:\\nowhere\\tools\n\
[2026-10-08][09:04:40][filesviewhub_lib][ERROR] cannot write the data folder: Access is denied. (os error 5)\n\
[2026-10-08][09:04:41][filesviewhub_lib][DEBUG] x\n[2026-10-08][09:04:42][filesviewhub_lib][TRACE] y\n";
        let e = app(raw);
        assert_eq!(e.iter().map(|e| e.level.as_str()).collect::<Vec<_>>(), ["INFO", "WARN", "ERROR", "DEBUG", "TRACE"]);
        assert_eq!((e[0].date.as_str(), e[0].time.as_str(), e[0].utc, e[0].topic), ("2026-10-08", "09:01:44", false, "start"));
        assert_eq!(texts(&e[1]), [("ffprobe is missing from ", false), ("Q:\\nowhere\\tools", true)]);
        assert_eq!(texts(&e[2]), [("cannot write the data folder: Access is denied. (os error 5)", false)]);
    }

    /// Lines as the running app writes them: the level before the target.
    #[test]
    #[cfg_attr(not(windows), ignore = "Windows paths")]
    fn real_lines_level_first() {
        let raw = "[2026-10-08][20:05:19][INFO][filesviewhub_lib] FilesViewHub 0.0.1 (build 180) started; data in Q:\\x\\data\n\
[2026-10-08][20:05:20][INFO][filesviewhub_lib::thumbs] no thumbnail from Q:\\x\\Library\\Docs\\song.flac: no cover picture in the file\n\
[2026-10-08][20:05:21][WARN][filesviewhub_lib::tools] ffprobe is missing from Q:\\x\\tools\n";
        let e = app(raw.as_bytes());
        assert_eq!(e.iter().map(|e| (e.level.as_str(), e.topic)).collect::<Vec<_>>(), [("INFO", "start"), ("INFO", "thumbs"), ("WARN", "other")]);
        assert_eq!(texts(&e[1])[1], ("Q:\\x\\Library\\Docs\\song.flac", true));
    }

    #[test]
    fn brackets_in_a_message_stay_in_it() {
        let e = app(b"[2026-10-08][09:00:00][filesviewhub_lib][INFO] cover lookup for \"[Draft] Notes\" stopped, tried again later: [503]\n");
        assert_eq!(e.len(), 1);
        assert_eq!(texts(&e[0]), [("cover lookup for \"[Draft] Notes\" stopped, tried again later: [503]", false)]);
    }

    #[test]
    fn a_panic_with_more_lines_is_one_entry() {
        let raw = b"[2026-10-08][09:00:00][filesviewhub_lib][ERROR] thread 'main' panicked at src/lib.rs:10:5:\nindex out of bounds\nstack backtrace:\n[2026-10-08][09:00:01][filesviewhub_lib][INFO] next\n";
        let e = app(raw);
        assert_eq!(e.len(), 2);
        assert_eq!(e[0].parts.iter().map(|p| p.text.as_str()).collect::<String>(), "thread 'main' panicked at src/lib.rs:10:5:\nindex out of bounds\nstack backtrace:");
    }

    #[test]
    fn empty_cut_and_broken_files() {
        assert!(app(b"").is_empty());
        assert!(app(b"\n\n").is_empty());
        // Cut mid-line: the head is not whole, so it is plain text.
        let e = app(b"[2026-10-08][09:0");
        assert_eq!((e.len(), e[0].level.as_str(), e[0].date.as_str()), (1, "", ""));
        // Bytes that are not UTF-8 show as the replacement character.
        let e = app(b"[2026-10-08][09:00:00][filesviewhub_lib][INFO] open Q:\\a\xff\xfeb.txt\n");
        assert!(e[0].parts.iter().any(|p| p.text.contains('\u{FFFD}')));
        // Lines before the first entry (the tail of one cut off) are one block.
        let e = app(b"tail of a panic\nmore\n[2026-10-08][09:00:00][filesviewhub_lib][INFO] x\n");
        assert_eq!((e.len(), e[0].parts[0].text.as_str()), (2, "tail of a panic\nmore"));
    }

    #[test]
    fn paths_with_spaces_and_cyrillic_that_exist_are_whole() {
        let dir = std::env::temp_dir().join("fvh-logs-test").join("Фото 2024 🌊");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("FilesViewHub marks 2026-10-07.json");
        std::fs::write(&file, b"{}").unwrap();
        let f = file.display().to_string();
        let msg = format!("[2026-10-08][09:00:00][filesviewhub_lib][INFO] marks saved to {f}\n");
        let e = app(msg.as_bytes());
        assert_eq!(texts(&e[0]), [("marks saved to ", false), (f.as_str(), true)]);
        let msg = format!("[2026-10-08][09:00:00][filesviewhub_lib][INFO] scanned {} (3 files, 1 Works) in 0.1 s\n", dir.display());
        let e = app(msg.as_bytes());
        assert_eq!(texts(&e[0])[1], (dir.display().to_string().as_str(), true));
        assert_eq!(e[0].topic, "scan");
    }

    #[test]
    #[cfg_attr(not(windows), ignore = "Windows paths")]
    fn a_gone_path_ends_where_the_wording_resumes() {
        let e = app(b"[2026-10-08][09:00:00][filesviewhub_lib][INFO] no thumbnail from Q:\\gone\\Lesson 04.mkv: ffmpeg found no frame\n");
        assert_eq!(texts(&e[0]), [("no thumbnail from ", false), ("Q:\\gone\\Lesson 04.mkv", true), (": ffmpeg found no frame", false)]);
        let e = app(b"[2026-10-08][09:00:00][filesviewhub_lib][INFO] Q:\\Photos is not connected\n");
        assert_eq!(texts(&e[0]), [("Q:\\Photos", true), (" is not connected", false)]);
        let e = app(b"[2026-10-08][09:00:00][filesviewhub_lib::store][WARN] Q:\\x\\marks.json could not be read (expected value); set aside as Q:\\x\\marks.json.damaged\n");
        assert_eq!(texts(&e[0])[..2], [("Q:\\x\\marks.json", true), (" could not be read (expected value); set aside as ", false)]);
        let e = app(b"[2026-10-08][09:00:00][filesviewhub_lib][INFO] show Q:\\x\\setup.ps1 (a program or script is not started)\n");
        assert_eq!(texts(&e[0])[1], ("Q:\\x\\setup.ps1", true));
        assert_eq!(e[0].topic, "open");
    }

    #[test]
    fn odd_paths() {
        let e = app(b"[2026-10-08][09:00:00][filesviewhub_lib][INFO] open \\\\localhost\\nothere\\a b.mp4\n");
        assert!(e[0].parts.iter().any(|p| p.path && p.text.starts_with("\\\\localhost\\nothere\\")));
        // A drive letter inside a word is not a path start when it lacks the backslash.
        let e = app(b"[2026-10-08][09:00:00][filesviewhub_lib][INFO] ratio 4:3 and C: alone\n");
        assert!(e[0].parts.iter().all(|p| !p.path));
        // A path inside quotes ends at the quote.
        let e = app(b"[2026-10-08][09:00:00][filesviewhub_lib][INFO] open \"Q:\\a\\b.txt\" now\n");
        assert_eq!(texts(&e[0])[1], ("Q:\\a\\b.txt", true));
    }

    #[test]
    fn topics() {
        for (m, t) in [
            ("open D:\\a.mp4", "open"),
            ("network: GET https://x (why)", "net"),
            ("scanned D:\\a (1 files, 1 Works) in 0.1 s", "scan"),
            ("scanned D:\\a (3 files, 1 Works) in 0.1 s; 2 files only in the cloud were not read or downloaded", "scan"),
            ("read a / b again in D:\\a: 3 Works in 0.2 s", "scan"),
            ("D:\\x is not there any more", "scan"),
            ("thumbnails: 1 ready, 0 to make, 0 removed, 0 books to look up on the web", "thumbs"),
            ("no thumbnail yet from D:\\a (tried again later): x", "thumbs"),
            ("cannot read D:\\a.heic for Review: x", "thumbs"),
            ("marks saved to D:\\m.json", "save"),
            ("move script for 4 files saved to D:\\m.cmd", "save"),
            ("cache moved from a to b (files carried: true)", "save"),
            ("cannot write the network log: x", "other"),
            ("cannot open D:\\a.ini: gone is not installed any more.", "open"),
        ] {
            assert_eq!(topic(m), t, "{m}");
        }
    }

    #[test]
    fn network_lines() {
        let raw = "2026-10-07 23:12:14Z\tGET https://openlibrary.org/search.json?title=The+Martian\tcover for \"The Martian\"\n\
2026-10-07 23:12:16Z\tGET https://api.fantlab.ru/search-works?q=Пикник\tcover for \"Пикник\"\n\
garbage line\n2026-10-07 23:12:17Z\tGET https://x\n\n";
        let e = network(raw.as_bytes());
        assert_eq!(e.len(), 4);
        assert_eq!((e[0].date.as_str(), e[0].time.as_str(), e[0].utc, e[0].level.as_str()), ("2026-10-07", "23:12:14", true, "INFO"));
        assert_eq!(texts(&e[0]), [("GET https://openlibrary.org/search.json?title=The+Martian (cover for \"The Martian\")", false)]);
        assert!(texts(&e[1])[0].0.contains("Пикник"));
        assert_eq!((e[2].level.as_str(), texts(&e[2])[0].0), ("", "garbage line"));
        assert_eq!(texts(&e[3]), [("GET https://x", false)]);
    }

    #[test]
    fn only_paths_under_the_roots_are_paths() {
        let e = super::app(b"[2026-10-08][09:00:00][INFO][t] marks saved to C:\\Users\\me\\Documents\\m.json and Q:\\a\\b.txt\n", &[PathBuf::from("Q:\\a")]);
        assert_eq!(texts(&e[0]), [("marks saved to C:\\Users\\me\\Documents\\m.json and ", false), ("Q:\\a\\b.txt", true)]);
        // A path starting with a root's name is not under it.
        let e = super::app(b"[2026-10-08][09:00:00][INFO][t] open Q:\\ab\\c.txt\n", &[PathBuf::from("Q:\\a")]);
        assert!(e[0].parts.iter().all(|p| !p.path));
    }

    #[test]
    fn a_name_ending_in_a_bracket_is_whole() {
        let dir = std::env::temp_dir().join("fvh-logs-test").join("Film (2010)");
        std::fs::create_dir_all(&dir).unwrap();
        let d = dir.display().to_string();
        let e = app(format!("[2026-10-08][09:00:00][INFO][t] open {d}\n").as_bytes());
        assert_eq!(texts(&e[0])[1], (d.as_str(), true));
        let e = app(format!("[2026-10-08][09:00:00][INFO][t] scanned {d} (3 files, 1 Works) in 0.1 s\n").as_bytes());
        assert_eq!(texts(&e[0])[1], (d.as_str(), true));
        assert_eq!(texts(&e[0])[2], (" (3 files, 1 Works) in 0.1 s", false));
    }

    #[test]
    fn blank_lines_are_dropped_and_dates_need_digits() {
        let e = app(b"[2026-10-08][09:00:00][INFO][t] one\n\n   \n[2026-10-08][09:00:01][INFO][t] two\n\n");
        assert_eq!(e.iter().map(|e| e.parts[0].text.as_str()).collect::<Vec<_>>(), ["one", "two"]);
        let e = app(b"[abcd-ef-gh][ij:kl:mn][INFO][t] fake\n");
        assert_eq!(e[0].level, "");
    }

    #[test]
    fn a_full_log_under_a_real_folder_reads_fast() {
        let root = std::env::temp_dir();
        let line = format!("[2026-10-08][09:00:00][INFO][t] no thumbnail from {}\\gone\\Курс\\Lesson 04 part two of the long course.mkv: ffmpeg found no frame in it at all\n", root.display());
        let raw = line.repeat(1_000_000 / line.len() + 1);
        let t = std::time::Instant::now();
        let e = super::app(raw.as_bytes(), &[root]);
        let ms = t.elapsed().as_millis();
        assert!(e[0].parts.iter().any(|p| p.path));
        assert!(ms < if cfg!(debug_assertions) { 5000 } else { 1000 }, "{ms} ms for {} entries", e.len());
        eprintln!("1 MB log under a real folder: {} entries in {ms} ms", e.len());
    }

    /// All the app log keeps (`OLD_KEPT` files put aside and the one written now).
    #[test]
    fn a_full_log_reads_fast() {
        let line = "[2026-10-08][09:00:00][filesviewhub_lib][INFO] no thumbnail from Q:\\gone\\Курс\\Lesson 04 part two.mkv: ffmpeg found no frame\n";
        let raw = line.repeat((OLD_KEPT + 1) * FILE_MAX as usize / line.len() + 1);
        let t = std::time::Instant::now();
        let e = app(raw.as_bytes());
        let ms = t.elapsed().as_millis();
        assert!(e.len() > 35_000);
        assert!(ms < if cfg!(debug_assertions) { 10_000 } else { 2000 }, "{ms} ms for {} entries", e.len());
        eprintln!("5 MB log: {} entries in {ms} ms", e.len());
    }

    /// The log folder after rollovers, as the plugin leaves it: the files put aside read first,
    /// by the time in their name, then the one written now, as one log with a session that runs
    /// across files whole. A `.bak` file goes when the log starts, and the network log next to
    /// them is neither read as the app log nor touched.
    #[test]
    fn the_files_put_aside_read_first_by_their_time() {
        let dir = std::env::temp_dir().join("fvh-logs-rollover");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let line = |t: &str, m: &str| format!("[2026-10-08][{t}][INFO][t] {m}\n");
        let start = "FilesViewHub 0.0.9 (build 400) started; data in Q:\\x";
        let put_aside = |at: &str, text: &str| std::fs::write(dir.join(format!("{APP_LOG}_2026-10-08_{at}.log")), text).unwrap();
        put_aside("10-00-00", &line("09:30:00", "second"));
        // Cut before its line end: the next file still starts an entry of its own.
        put_aside("09-00-00", line("08:00:00", start).trim_end());
        put_aside("11-00-00", &line("10:30:00", start));
        std::fs::write(dir.join(format!("{APP_LOG}.log")), line("11:30:00", "fourth")).unwrap();
        let bak = dir.join(format!("{APP_LOG}_2026-10-08_11-00-00.log.bak"));
        std::fs::write(&bak, line("10:59:00", "bak")).unwrap();
        std::fs::write(dir.join("network.log"), b"2026-10-07 23:12:14Z\tGET https://x\twhy\n").unwrap();
        let e = app(&read_app(&dir));
        let m: Vec<String> = e.iter().map(|e| e.parts.iter().map(|p| p.text.as_str()).collect()).collect();
        assert_eq!(m, [start, "second", start, "fourth"]);
        assert_eq!(e.iter().filter(|e| e.topic == "start").count(), 2);
        let _ = plugin(Some(dir.clone()));
        assert!(!bak.exists(), "a .bak file goes at start");
        assert_eq!(app_files(&dir).len(), 4);
        assert_eq!(std::fs::read(dir.join("network.log")).unwrap(), b"2026-10-07 23:12:14Z\tGET https://x\twhy\n");
        assert!(read_app(&std::env::temp_dir().join("fvh-logs-none")).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Lines logged before the log starts reach it, in order and after the line that opens the
    /// session; what is logged once it runs goes straight to it.
    #[test]
    fn warnings_raised_before_the_log_starts_reach_it() {
        struct Lines(std::sync::Arc<Mutex<Vec<String>>>);
        impl log::Log for Lines {
            fn enabled(&self, _: &log::Metadata) -> bool {
                true
            }
            fn log(&self, r: &log::Record) {
                self.0.lock().unwrap().push(format!("{} {} {}", r.level(), r.target(), r.args()));
            }
            fn flush(&self) {}
        }
        let say = |e: &Early, level, m: &str| {
            log::Log::log(e, &log::Record::builder().level(level).target("filesviewhub_lib::store").args(format_args!("{m}")).build())
        };
        let early = Early::new();
        say(&early, log::Level::Warn, "settings.json could not be read; set aside as settings.json.damaged");
        say(&early, log::Level::Debug, "too fine to keep");
        say(&early, log::Level::Warn, "marks.json could not be read; set aside as marks.json.damaged");
        say(&early, log::Level::Info, "taken over");
        let lines = std::sync::Arc::new(Mutex::new(Vec::new()));
        early.start(log::LevelFilter::Info, Box::new(Lines(lines.clone())));
        assert!(lines.lock().unwrap().is_empty(), "held until released");
        say(&early, log::Level::Info, "started");
        early.release();
        say(&early, log::Level::Warn, "later");
        early.release();
        assert_eq!(*lines.lock().unwrap(), [
            "INFO filesviewhub_lib::store started",
            "WARN filesviewhub_lib::store settings.json could not be read; set aside as settings.json.damaged",
            "WARN filesviewhub_lib::store marks.json could not be read; set aside as marks.json.damaged",
            "INFO filesviewhub_lib::store taken over",
            "WARN filesviewhub_lib::store later",
        ]);
    }
}
