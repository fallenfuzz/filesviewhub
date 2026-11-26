//! QA: an independent suite for `logs.rs` and `programs.rs`, written against the spec, not
//! against the modules' own tests. Fixtures go under `.scratch/qa/`.

use crate::logs::{self, Entry};
use crate::programs;
use crate::scan::Catalog;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Instant;

// ---------- helpers ----------

/// Broad roots, so these tests see paths on every drive and share form they use (logs::app
/// finds paths only under roots).
/// The speed limit: 1 s per MB in a release build; an unoptimized test build is given 5 s.
const LIMIT_MS: u128 = if cfg!(debug_assertions) { 5000 } else { 1000 };

fn logs_app(raw: &[u8]) -> Vec<Entry> {
    let roots: Vec<PathBuf> = ["C:\\", "D:\\", "E:\\", "Q:\\", "\\\\localhost\\nas", "\\\\localhost\\E$", "/Users", "/Volumes", "/Applications", "/home", "/mnt", "/tmp"].iter().map(PathBuf::from).collect();
    logs::app(raw, &roots)
}

fn scratch(sub: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().join(".scratch").join("qa").join(sub);
    std::fs::create_dir_all(&root).unwrap();
    root
}

fn texts(e: &Entry) -> Vec<(String, bool)> {
    e.parts.iter().map(|p| (p.text.clone(), p.path)).collect()
}

fn joined(e: &Entry) -> String {
    e.parts.iter().map(|p| p.text.as_str()).collect()
}

fn paths(e: &Entry) -> Vec<String> {
    e.parts.iter().filter(|p| p.path).map(|p| p.text.clone()).collect()
}

/// One app log line, level first, as the plugin writes it.
fn line(msg: &str) -> String {
    format!("[2026-10-08][09:00:00][INFO][filesviewhub_lib] {msg}\n")
}

fn one(msg: &str) -> Entry {
    let mut e = logs_app(line(msg).as_bytes());
    assert_eq!(e.len(), 1, "one entry for {msg:?}");
    let e = e.remove(0);
    assert_eq!(joined(&e), msg, "parts must give back the message");
    e
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn pick<'a, T>(&mut self, v: &'a [T]) -> &'a T {
        &v[(self.next() % v.len() as u64) as usize]
    }
}

// ---------- logs: heads ----------

#[test]
fn qa_logs_both_head_orders_and_all_levels() {
    let mut raw = String::new();
    for l in ["ERROR", "WARN", "INFO", "DEBUG", "TRACE"] {
        raw += &format!("[2026-01-02][03:04:05][{l}][t::x] a {l}\n");
        raw += &format!("[2026-01-02][03:04:06][t::x][{l}] b {l}\n");
    }
    let e = logs_app(raw.as_bytes());
    assert_eq!(e.len(), 10);
    for (i, l) in ["ERROR", "WARN", "INFO", "DEBUG", "TRACE"].iter().enumerate() {
        assert_eq!(e[2 * i].level, *l);
        assert_eq!(e[2 * i + 1].level, *l);
        assert_eq!(joined(&e[2 * i]), format!("a {l}"));
        assert_eq!(joined(&e[2 * i + 1]), format!("b {l}"));
        assert_eq!((e[2 * i].date.as_str(), e[2 * i].time.as_str(), e[2 * i].utc), ("2026-01-02", "03:04:05", false));
    }
}

#[test]
fn qa_logs_an_unknown_level_is_not_a_head() {
    // Lower case, a made-up level, and three brackets only: each joins the entry before.
    let raw = "[2026-10-08][09:00:00][INFO][t] first\n[2026-10-08][09:00:01][info][t] lower\n[2026-10-08][09:00:02][FATAL][t] fatal\n[2026-10-08][09:00:03][INFO] three\n";
    let e = logs_app(raw.as_bytes());
    assert_eq!(e.len(), 1, "{:?}", e.iter().map(joined).collect::<Vec<_>>());
    assert_eq!(joined(&e[0]), "first\n[2026-10-08][09:00:01][info][t] lower\n[2026-10-08][09:00:02][FATAL][t] fatal\n[2026-10-08][09:00:03][INFO] three");
}

#[test]
fn qa_logs_a_date_or_time_not_in_digits_is_not_a_head() {
    // Spec: `[YYYY-MM-DD][HH:MM:SS]`. These have the right lengths and separators but no digits.
    let raw = "[2026-10-08][09:00:00][INFO][t] first\n[abcd-ef-gh][ij:kl:mn][INFO][t] fake\n";
    let e = logs_app(raw.as_bytes());
    assert_eq!(e.len(), 1, "a non-digit date became its own entry: {:?}", e.iter().map(|e| (&e.date, &e.time)).collect::<Vec<_>>());
}

#[test]
fn qa_logs_no_space_after_head_and_empty_message() {
    let e = logs_app(b"[2026-10-08][09:00:00][INFO][t]x\n[2026-10-08][09:00:01][INFO][t]\n[2026-10-08][09:00:02][INFO][t]   two spaces\n");
    assert_eq!(e.len(), 3);
    assert_eq!(joined(&e[0]), "x");
    assert_eq!(joined(&e[1]), "");
    assert_eq!(joined(&e[2]), "  two spaces");
}

#[test]
fn qa_logs_continuations_and_leading_lines() {
    let raw = "lost 1\n\n  lost 2\n[2026-10-08][09:00:00][ERROR][t] thread 'main' panicked at src\\lib.rs:1:1:\n  boom\n\tat x\n[2026-10-08][09:00:01][INFO][t] next\n";
    let e = logs_app(raw.as_bytes());
    assert_eq!(e.len(), 3, "{:?}", e.iter().map(joined).collect::<Vec<_>>());
    assert_eq!((e[0].level.as_str(), e[0].date.as_str(), e[0].time.as_str()), ("", "", ""));
    // Lines before any entry form ONE plain entry. Whether the blank line between them is kept
    // inside it is not stated; both readings are accepted, the two lines must be together.
    let j = joined(&e[0]);
    assert!(j == "lost 1\n  lost 2" || j == "lost 1\n\n  lost 2", "{j:?}");
    assert_eq!(joined(&e[1]), "thread 'main' panicked at src\\lib.rs:1:1:\n  boom\n\tat x");
    assert_eq!(joined(&e[2]), "next");
}

#[test]
fn qa_logs_blank_lines_between_entries_are_skipped() {
    // Spec: blank lines alone are skipped. A blank line between two entries is not panic text.
    let raw = "[2026-10-08][09:00:00][INFO][t] one\n\n   \n[2026-10-08][09:00:01][INFO][t] two\n\n";
    let e = logs_app(raw.as_bytes());
    assert_eq!(e.len(), 2);
    assert_eq!(joined(&e[0]), "one", "blank lines were added to the entry before");
    assert_eq!(joined(&e[1]), "two", "trailing blank line was added to the last entry");
}

#[test]
fn qa_logs_crlf_lines() {
    let e = logs_app(b"[2026-10-08][09:00:00][INFO][t] open Q:\\a\\b.mp4\r\n[2026-10-08][09:00:01][INFO][t] two\r\n");
    assert_eq!(e.len(), 2);
    assert_eq!(texts(&e[0]), [("open ".into(), false), ("Q:\\a\\b.mp4".into(), true)]);
}

#[test]
fn qa_logs_not_utf8_never_panics() {
    let mut r = Rng(0x9e3779b97f4a7c15);
    let seeds: [&[u8]; 6] = [b"[2026-10-08][09:00:00][INFO][t] ", b"C:\\", b"\\\\", b"/tmp/", b" (", b"\n"];
    for _ in 0..3000 {
        let mut raw = vec![];
        for _ in 0..(r.next() % 40) {
            if r.next().is_multiple_of(3) {
                raw.extend_from_slice(r.pick(&seeds));
            } else {
                raw.push((r.next() & 0xff) as u8);
            }
        }
        let _ = logs_app(&raw);
        let _ = logs::network(&raw);
    }
    // Invalid bytes inside the head: the line is not a head, no panic.
    let e = logs_app(b"[2026-10-0\xff][09:00:00][INFO][t] x\n");
    assert_eq!((e.len(), e[0].level.as_str()), (1, ""));
    // A date of 10 bytes with multi-byte characters.
    let e = logs_app("[2026-10-ё][09:00:00][INFO][t] x\n[2026-1ё-08][0ё:00:0][INFO][t] y\n".as_bytes());
    assert!(e.iter().all(|e| e.level.is_empty()));
}

// ---------- logs: topics ----------

#[test]
fn qa_logs_topics_per_spec() {
    for (m, t) in [
        ("FilesViewHub 0.4.0 (build 1) started; data in Q:\\d", "start"),
        ("FilesViewHub 1 starting; x", "other"),
        ("open Q:\\a.mp4", "open"),
        ("show Q:\\a.ps1 (a program)", "open"),
        ("cannot open Q:\\a.ini: x", "open"),
        ("network: GET https://x", "net"),
        ("scanned Q:\\a (1 files) in 0.1 s", "scan"),
        ("read a again", "scan"),
        ("Q:\\a is not connected", "scan"),
        ("Q:\\a is not there any more", "scan"),
        ("no thumbnail from Q:\\a: x", "thumbs"),
        ("file details: 3 to read", "thumbs"),
        ("photos for Review: 10 measured", "thumbs"),
        ("cover lookup for \"X\" failed", "thumbs"),
        ("cannot read Q:\\a.heic: x", "thumbs"),
        ("marks saved to Q:\\m.json", "save"),
        ("mark moved from a to b", "save"),
        ("all marks cleared", "save"),
        ("move script saved to Q:\\m.cmd", "save"),
        ("cache moved from a to b", "save"),
        ("stopped showing Q:\\a", "save"),
        ("web covers: switched on", "save"),
        ("something else", "other"),
        ("", "other"),
        ("ОТКРЫТЬ 🌊", "other"),
    ] {
        let e = logs_app(line(m).as_bytes());
        assert_eq!(e[0].topic, t, "{m:?}");
    }
}

// ---------- logs: paths ----------

#[test]
fn qa_paths_each_start_form() {
    for (m, p) in [
        ("open Q:\\a\\b.mp4", "Q:\\a\\b.mp4"),
        ("open q:\\lower.mp4", "q:\\lower.mp4"),
        ("open \\\\localhost\\nas\\share\\x.mp4", "\\\\localhost\\nas\\share\\x.mp4"),
        ("open /Users/ann/x.mp4", "/Users/ann/x.mp4"),
        ("open /Volumes/Disk/x.mp4", "/Volumes/Disk/x.mp4"),
        ("open /Applications/VLC.app", "/Applications/VLC.app"),
        ("open /home/ann/qa-none/x.mp4", "/home/ann/qa-none/x.mp4"),
        ("open /mnt/qa-none/x.mp4", "/mnt/qa-none/x.mp4"),
        ("open /tmp/qa-none/x.mp4", "/tmp/qa-none/x.mp4"),
    ] {
        let e = one(m);
        assert_eq!(paths(&e), [p], "{m}");
    }
}

#[test]
fn qa_paths_not_after_a_word_or_backslash() {
    for m in ["xQ:\\a\\b", "9Q:\\a\\b", "ЖQ:\\a\\b", "a\\Q:\\a\\b", "a/Users/x/y", "usr/tmp/x", "/opt/x", "/usr/x", "Q:/a/b", "Q:", "ratio 4:3", "\\\\ server", "\\\\\\\\"] {
        let e = one(m);
        assert!(paths(&e).is_empty(), "{m:?} gave {:?}", paths(&e));
    }
    // A non-word, non-backslash char before is fine.
    for (m, p) in [("(Q:\\a\\b)", "Q:\\a\\b"), ("_Q:\\a", "Q:\\a"), ("'Q:\\x\\y'", "Q:\\x\\y'"), ("=/tmp/qa-none/x", "/tmp/qa-none/x")] {
        let e = one(m);
        assert_eq!(paths(&e), [p], "{m:?}");
    }
}

#[test]
#[cfg_attr(not(windows), ignore = "Windows paths")]
fn qa_paths_gone_end_at_each_stop() {
    for stop in [" (x)", ": x", " for x", " is x", " in x", " again", " from x", " to x", ", x", "\tx", "\"x"] {
        let m = format!("from Q:\\gone dir\\a b.mkv{stop}");
        let e = one(&m);
        assert_eq!(paths(&e), ["Q:\\gone dir\\a b.mkv"], "stop {stop:?}");
    }
    // The earliest stop wins.
    let e = one("Q:\\a b is c for d");
    assert_eq!(paths(&e), ["Q:\\a b"]);
}

#[test]
fn qa_paths_trailing_punctuation_dropped() {
    for (m, p) in [
        ("saved to Q:\\x\\m.json.", "Q:\\x\\m.json"),
        ("saved to Q:\\x\\m.json...", "Q:\\x\\m.json"),
        ("saved to Q:\\x\\m.json;", "Q:\\x\\m.json"),
        ("saved to Q:\\x\\m.json:", "Q:\\x\\m.json"),
        ("saved (Q:\\x\\m.json)", "Q:\\x\\m.json"),
        ("saved to Q:\\x\\m.json,", "Q:\\x\\m.json"),
        ("saved to Q:\\x\\m.json).;", "Q:\\x\\m.json"),
    ] {
        assert_eq!(paths(&one(m)), [p], "{m:?}");
    }
}

#[test]
fn qa_paths_at_start_end_and_alone() {
    assert_eq!(texts(&one("Q:\\a\\b")), [("Q:\\a\\b".into(), true)]);
    assert_eq!(texts(&one("Q:\\a\\b is gone")), [("Q:\\a\\b".into(), true), (" is gone".into(), false)]);
    assert_eq!(texts(&one("gone: Q:\\a\\b")), [("gone: ".into(), false), ("Q:\\a\\b".into(), true)]);
    // Just a root, gone: not a path (too short to be useful) or a path; either way the text holds.
    let _ = one("Q:\\");
    let _ = one("Q:\\ is gone");
}

#[test]
fn qa_paths_adjacent() {
    let e = one("moved Q:\\a\\x.mp4 to Q:\\b\\x.mp4");
    assert_eq!(paths(&e), ["Q:\\a\\x.mp4", "Q:\\b\\x.mp4"]);
    let e = one("Q:\\a, Q:\\b, \\\\localhost\\nas\\c");
    assert_eq!(paths(&e), ["Q:\\a", "Q:\\b", "\\\\localhost\\nas\\c"]);
    let e = one("\"Q:\\a\"\"Q:\\b\"");
    assert_eq!(paths(&e), ["Q:\\a", "Q:\\b"]);
    let e = one("Q:\\a\tQ:\\b");
    assert_eq!(paths(&e), ["Q:\\a", "Q:\\b"]);
}

#[test]
#[cfg_attr(not(windows), ignore = "Windows paths")]
fn qa_paths_unicode_and_emoji_gone() {
    let e = one("no thumbnail from Q:\\Фото 2024 🌊\\Пикник на обочине.fb2: broken");
    assert_eq!(paths(&e), ["Q:\\Фото 2024 🌊\\Пикник на обочине.fb2"]);
    let e = one("Q:\\日本語\\ファイル.mp4 is gone");
    assert_eq!(paths(&e), ["Q:\\日本語\\ファイル.mp4"]);
    // Combining marks and a zero-width joiner in names.
    let e = one("open Q:\\cafe\u{301}\\👨\u{200d}👩\u{200d}👧.jpg");
    assert_eq!(paths(&e), ["Q:\\cafe\u{301}\\👨\u{200d}👩\u{200d}👧.jpg"]);
}

#[cfg(windows)]
#[test]
fn qa_paths_existing_taken_whole() {
    let dir = scratch("logs").join("Music for kids is fun, again (2024)");
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("Track 01 - in the end.mp3");
    std::fs::write(&file, b"x").unwrap();
    let d = dir.display().to_string();
    let f = file.display().to_string();
    for (m, p) in [
        (format!("open {f}"), f.clone()),
        (format!("no thumbnail from {f}: no cover"), f.clone()),
        (format!("{f} is not there any more"), f.clone()),
        (format!("scanned {d} (3 files) in 0.1 s"), d.clone()),
        (format!("moved {f} to {d}"), f.clone()),
        (format!("saved to {f}."), f.clone()),
    ] {
        let e = one(&m);
        assert_eq!(paths(&e).first(), Some(&p), "{m}");
    }
    // Two existing paths side by side.
    let e = one(&format!("moved {f} to {d}"));
    assert_eq!(paths(&e), [f.clone(), d.clone()]);
    // An existing path with a gone tail: the existing part is not cut early at " is ".
    let e = one(&format!("{d}\\gone.mp3 is gone"));
    assert_eq!(paths(&e), [format!("{d}\\gone.mp3")]);
}

#[cfg(windows)]
#[test]
fn qa_paths_existing_folder_then_space_and_dot() {
    // Windows drops a trailing space when it looks a name up, so "C:\Windows " exists.
    let e = one("saved to C:\\Windows .");
    for p in paths(&e) {
        assert!(!p.ends_with(' '), "a path part ends in a space: {p:?}");
    }
}

#[test]
fn qa_parts_concat_invariant_fuzz() {
    let toks = [
        "Q:\\", "C:\\", "C:\\Windows", "\\\\", "\\\\localhost\\nas\\", "/tmp/", "/Users/", "/mnt/", "a", "Ж", "🌊", " ", " (", ": ", " for ", " is ", " in ", " again", " from ",
        " to ", ", ", "\t", "\"", ".", ",", ";", ":", ")", "\\", "/", "x.mp4", "Фото", "\u{301}", "\u{200d}", "é", "1",
    ];
    let mut r = Rng(12345);
    for _ in 0..20000 {
        let n = r.next() % 14;
        let m: String = (0..n).map(|_| *r.pick(&toks)).collect();
        let e = logs_app(line(&m).as_bytes());
        let all: String = e.iter().map(joined).collect::<Vec<_>>().join("\n");
        assert_eq!(all, m.trim_end_matches(['\n']), "{m:?}");
        for en in &e {
            for w in en.parts.windows(2) {
                assert!(w[0].path || w[1].path, "two text parts side by side in {m:?}");
            }
            assert!(en.parts.iter().all(|p| !p.text.is_empty()), "an empty part in {m:?}");
        }
    }
}

// ---------- logs: network ----------

#[test]
fn qa_network_lines() {
    let raw = "2026-10-07 23:12:14Z\tGET https://a/b?q=Пикник\tcover for \"X\"\r\n\
2026-10-07 23:12:15Z\tGET https://a\t\n\
2026-10-07 23:12:16Z\tGET https://b\n\
2026-10-07T23:12:17Z\tGET https://c\twhy\n\
2026-10-07 23:12:18\tGET https://d\twhy\n\
2026-10-07 23:12:19Z GET https://e why\n\
\n\
just text\n";
    let e = logs::network(raw.as_bytes());
    assert_eq!(e.len(), 7);
    assert!(e.iter().all(|e| e.utc && e.topic == "net"));
    assert_eq!(joined(&e[0]), "GET https://a/b?q=Пикник (cover for \"X\")");
    assert_eq!((e[0].date.as_str(), e[0].time.as_str()), ("2026-10-07", "23:12:14"));
    assert_eq!(joined(&e[1]), "GET https://a");
    assert_eq!(joined(&e[2]), "GET https://b");
    for (i, line) in e.iter().enumerate().skip(3) {
        assert_eq!((line.date.as_str(), line.level.as_str()), ("", ""), "line {i} is plain");
    }
    assert_eq!(joined(&e[3]), "2026-10-07T23:12:17Z\tGET https://c\twhy");
    assert_eq!(joined(&e[6]), "just text");
    assert!(e.iter().all(|e| e.parts.iter().all(|p| !p.path)));
}

#[test]
fn qa_network_a_why_with_a_tab_is_kept_whole() {
    let e = logs::network(b"2026-10-07 23:12:14Z\tGET https://a\tcover for \"A\tB\"\n");
    assert_eq!(joined(&e[0]), "GET https://a (cover for \"A\tB\")", "text after a second tab is lost");
}

#[test]
fn qa_network_a_head_with_no_request_is_plain() {
    // Spec: `date time Z<TAB>GET url<TAB>why`; a line with an empty request is not that form.
    let e = logs::network(b"2026-10-07 23:12:14Z\t\n2026-10-07 23:12:15Z\t\twhy\n");
    assert_eq!(e.len(), 2);
    for x in &e {
        assert_eq!(x.date, "", "an empty request read as a dated entry: {:?}", joined(x));
    }
}

#[test]
fn qa_network_a_date_not_in_digits_is_plain() {
    let e = logs::network(b"abcd-ef-gh ij:kl:mnZ\tGET https://a\twhy\n");
    assert_eq!(e[0].date, "", "a non-digit date read as a dated entry");
}

// ---------- logs: speed ----------

fn timed(label: &str, raw: &[u8], f: fn(&[u8]) -> Vec<Entry>) -> u128 {
    let t = Instant::now();
    let e = f(raw);
    let ms = t.elapsed().as_millis();
    eprintln!("QA speed: {label}: {} bytes, {} entries, {ms} ms", raw.len(), e.len());
    ms
}

#[test]
fn qa_speed_gone_paths_on_a_drive_that_exists() {
    // The real case: paths on C:\, a drive that is there, each file gone, words after the path.
    let mut raw = String::new();
    let mut i = 0;
    while raw.len() < 1_000_000 {
        raw += &line(&format!("no thumbnail from C:\\Users\\qa-none\\Курс {i}\\Lesson {i} part two.mkv: ffmpeg found no frame in the file at all so it is skipped for now"));
        i += 1;
    }
    let ms = timed("1 MB, gone paths on C:", raw.as_bytes(), logs_app);
    assert!(ms < LIMIT_MS, "{ms} ms");
}

#[test]
fn qa_speed_odd_shapes() {
    // One 1 MB line with no newline, full of path starts.
    let big = "Q:\\a b ".repeat(1_000_000 / 7);
    let ms = timed("1 MB one line of Q: paths", line(&big).as_bytes(), logs_app);
    assert!(ms < LIMIT_MS, "one line: {ms} ms");
    // One head and 1 MB of continuation lines.
    let raw = format!("{}{}", line("panic"), "  at frame\n".repeat(1_000_000 / 11));
    let ms = timed("1 MB of continuation lines", raw.as_bytes(), logs_app);
    assert!(ms < LIMIT_MS, "continuations: {ms} ms");
    // 1 MB of network lines.
    let raw = "2026-10-07 23:12:14Z\tGET https://openlibrary.org/search.json?title=x\tcover for \"x\"\n".repeat(1_000_000 / 80);
    let ms = timed("1 MB network log", raw.as_bytes(), logs::network);
    assert!(ms < LIMIT_MS, "network: {ms} ms");
}

#[test]
fn qa_speed_one_line_of_c_paths() {
    // One 200 KB line of distinct gone paths on C:, each with many spaces after it.
    let mut m = String::new();
    let mut i = 0;
    while m.len() < 200_000 {
        m += &format!("C:\\qa-none\\{i} a b c d e f g h ");
        i += 1;
    }
    let ms = timed("200 KB one line of C: paths", line(&m).as_bytes(), logs_app);
    assert!(ms < LIMIT_MS, "{ms} ms");
}

// ---------- programs: types ----------

fn catalog(sections: &[(&[&str], &[&str])]) -> Catalog {
    let item = |p: &&str| {
        let name = p.rsplit('\\').next().unwrap();
        let ext = name.rsplit_once('.').map(|(s, e)| if s.is_empty() { "" } else { e }).unwrap_or("");
        serde_json::json!({ "name": name, "rel": name, "path": p, "ext": ext, "size": 1, "mtime": 0, "cloud": false })
    };
    let un = |p: &&str| serde_json::json!({ "name": p.rsplit('\\').next().unwrap(), "rel": "", "path": p, "folder": [], "reason": "unknown", "size": 1, "cloud": false });
    let secs: Vec<_> = sections
        .iter()
        .enumerate()
        .map(|(i, (items, loose))| {
            serde_json::json!({
                "id": format!("S{i}"), "name": "S", "path": "D:\\S", "drive": "D:", "kind": "Files", "guessed": false, "online": true,
                "works": [{ "id": format!("S{i}:w"), "title": "w", "path": "D:\\S\\w", "folder": [], "is_folder": true,
                    "items": items.iter().map(item).collect::<Vec<_>>(), "media": [], "size": 0, "added": 0, "cloud": 0 }],
                "files": 0, "scanned_at": 0, "unlisted": loose.iter().map(un).collect::<Vec<_>>() })
        })
        .collect();
    serde_json::from_value(serde_json::json!({ "v": 1, "sections": secs })).unwrap()
}

fn tv(t: &[(String, usize, Option<String>)]) -> Vec<(&str, usize)> {
    t.iter().map(|(e, n, _)| (e.as_str(), *n)).collect()
}

#[test]
fn qa_types_order_group_count_name() {
    let cat = catalog(&[
        (&["D:\\a.txt", "D:\\b.TXT", "D:\\c.pdf", "D:\\d.flac", "D:\\e.png", "D:\\f.webm", "D:\\g.zip"], &["D:\\h.ZIP", "D:\\i.nfo", "D:\\j.aaa"]),
        (&["D:\\k.pdf", "D:\\l.pdf", "D:\\m.mp3"], &["D:\\n.mp4"]),
    ]);
    let t = types(&cat);
    assert_eq!(tv(&t), [("mp4", 1), ("webm", 1), ("flac", 1), ("mp3", 1), ("png", 1), ("pdf", 3), ("txt", 2), ("zip", 2), ("aaa", 1), ("nfo", 1)]);
    // The sample is the first file of the type, in catalog order.
    assert_eq!(t.iter().find(|x| x.0 == "pdf").unwrap().2.as_deref(), Some("D:\\c.pdf"));
    assert_eq!(t.iter().find(|x| x.0 == "zip").unwrap().2.as_deref(), Some("D:\\g.zip"));
}

fn types(cat: &Catalog) -> Vec<(String, usize, Option<String>)> {
    programs::types(cat, &BTreeMap::new())
}

#[test]
fn qa_types_odd_names() {
    let loose: &[&str] = &[
        "D:\\archive.tar.gz", "D:\\noext", "D:\\.gitignore", "D:\\.hidden.txt", "D:\\trailing.", "D:\\two..", "D:\\ПРИМЕР.ПДФ", "D:\\x.İ", "D:\\a.b c",
        "D:\\exactly.abcdefghijkl", "D:\\long.abcdefghijklm", "D:\\..", "D:\\.",
    ];
    let t = types(&catalog(&[(&[], loose)]));
    let got: BTreeMap<&str, usize> = t.iter().map(|(e, n, _)| (e.as_str(), *n)).collect();
    eprintln!("QA types from odd names: {got:?}");
    assert_eq!(got.get("gz"), Some(&1), ".tar.gz counts as gz");
    assert_eq!(got.get("txt"), Some(&1), ".hidden.txt has the extension txt");
    assert_eq!(got.get("пдф"), Some(&1), "upper-case Cyrillic is lower-cased");
    assert_eq!(got.get("abcdefghijkl"), Some(&1));
    assert!(!got.contains_key("gitignore"), "a dot-file is skipped");
    assert!(!got.contains_key(""), "no empty extension");
    assert!(!got.keys().any(|k| k.contains('.')), "no extension with a dot: {got:?}");
    // Decided: an unlisted file's "extension" over 12 characters is part of its name, not a type.
    assert_eq!(got.get("abcdefghijklm"), None, "a 13-character extension is not a type");
}

#[test]
fn qa_types_items_with_odd_ext_fields() {
    // A Work item's ext as the catalog stores it: empty, upper-case.
    let t = types(&catalog(&[(&["D:\\noext", "D:\\A.MKV", "D:\\b.Mkv"], &[])]));
    assert_eq!(tv(&t), [("mkv", 2)]);
}

#[test]
fn qa_types_open_with_keys_appear_with_zero() {
    let mut ow = BTreeMap::new();
    for k in ["cbz", "mp4", "zzz", "epub"] {
        ow.insert(k.to_string(), "C:\\x.exe".to_string());
    }
    let t = programs::types(&catalog(&[(&["D:\\a.mp4"], &[])]), &ow);
    assert_eq!(tv(&t), [("mp4", 1), ("cbz", 0), ("epub", 0), ("zzz", 0)]);
    assert!(t.iter().filter(|x| x.1 == 0).all(|x| x.2.is_none()));
    // Empty catalog, empty choices.
    assert!(programs::types(&catalog(&[]), &BTreeMap::new()).is_empty());
}

#[test]
fn qa_types_open_with_key_in_upper_case() {
    // The spec does not say open_with keys are lower case; a stored "MP4" must not make a
    // second row next to the catalog's "mp4".
    let mut ow = BTreeMap::new();
    ow.insert("MP4".to_string(), "C:\\x.exe".to_string());
    let t = programs::types(&catalog(&[(&["D:\\a.mp4"], &[])]), &ow);
    assert_eq!(tv(&t), [("mp4", 1)], "an upper-case choice key gave a second row");
}

#[test]
fn qa_types_many_fast() {
    let names: Vec<String> = (0..20000).map(|i| format!("D:\\f{i}.e{}", i % 5000)).collect();
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    let t0 = Instant::now();
    let t = types(&catalog(&[(&refs[..10000], &refs[10000..])]));
    eprintln!("QA speed: types of 20000 files, 5000 extensions: {} ms", t0.elapsed().as_millis());
    assert_eq!(t.len(), 5000);
    assert!(t.iter().all(|x| x.1 == 4));
    assert!(t.windows(2).all(|w| w[0].0 < w[1].0), "same group and count: sorted by name");
}

// ---------- programs: list ----------

#[test]
fn qa_list_runs_for_every_runs_type() {
    let t: Vec<_> = programs::RUNS.iter().map(|e| (e.to_string(), 1, None)).chain([("mp4".to_string(), 1, None), ("txt".into(), 1, None)]).collect();
    let r = programs::list(t, &BTreeMap::new());
    for row in &r.rows {
        assert_eq!(row.runs, programs::RUNS.contains(&row.ext.as_str()), ".{}", row.ext);
        assert!(!row.gone && row.yours.is_none());
    }
}

#[test]
fn qa_list_yours_from_file_and_gone() {
    let fake = scratch("programs").join("Fake Player.exe");
    std::fs::write(&fake, b"not a program").unwrap();
    let fake_dir = scratch("programs").join("Some Folder");
    std::fs::create_dir_all(&fake_dir).unwrap();
    let mut ow = BTreeMap::new();
    ow.insert("qa1".to_string(), fake.display().to_string());
    ow.insert("qa2".to_string(), "C:\\Nowhere\\Мой Плеер.exe".to_string());
    ow.insert("qa3".to_string(), "C:\\Nowhere\\Foo.Exe".to_string());
    ow.insert("qa4".to_string(), "Some Store App".to_string());
    ow.insert("qa5".to_string(), String::new());
    ow.insert("qa6".to_string(), fake_dir.display().to_string());
    let t = programs::types(&catalog(&[]), &ow);
    let r = programs::list(t, &ow);
    let row = |e: &str| r.rows.iter().find(|x| x.ext == e).unwrap();
    let y = |e: &str| row(e).yours.clone().unwrap();
    assert_eq!((y("qa1").name.as_str(), row("qa1").gone), ("Fake Player", false));
    assert_eq!((y("qa2").name.as_str(), row("qa2").gone), ("Мой Плеер", true));
    assert_eq!((y("qa4").name.as_str(), row("qa4").gone), ("Some Store App", true));
    assert!(row("qa5").gone, "an empty choice is not a program on this PC");
    // Spec: gone = neither offered nor an existing file. A folder is not a file.
    let dir_gone = row("qa6").gone;
    // Spec: named from the file name; the module's own rule is the name without `.exe`.
    let foo = y("qa3").name;
    eprintln!("QA list: a folder as the choice gone={dir_gone}; Foo.Exe named {foo:?}");
    assert_eq!(foo, "Foo", "an .Exe in mixed case keeps its extension in the name");
}

#[test]
fn qa_list_folder_choice_is_gone() {
    let fake_dir = scratch("programs").join("Some Folder");
    std::fs::create_dir_all(&fake_dir).unwrap();
    let mut ow = BTreeMap::new();
    ow.insert("qa6".to_string(), fake_dir.display().to_string());
    let r = programs::list(vec![("qa6".into(), 0, None)], &ow);
    assert!(r.rows[0].gone, "a folder counts as an existing program file");
}

#[test]
fn qa_list_odd_exts_do_not_panic() {
    let long = "x".repeat(300);
    let exts = ["", "tar.gz", "a b", "x\\y", "abcdefghijklm", "пдф", "🌊", "a\0b", ".", "..", "*", "?", long.as_str(), "MP4"];
    let r = programs::list(exts.iter().map(|e| (e.to_string(), 1, Some("D:\\nowhere\\x".into()))).collect(), &BTreeMap::new());
    assert_eq!(r.rows.len(), exts.len());
    for row in &r.rows {
        eprintln!("QA odd ext {:?}: default {:?}, {} choices", row.ext, row.default.as_ref().map(|p| &p.name), row.choices.len());
    }
}

#[cfg(windows)]
#[test]
fn qa_list_live_shell() {
    let exts = ["jpg", "png", "mp4", "mkv", "mp3", "flac", "pdf", "epub", "txt", "docx", "xlsx", "zip", "html", "srt", "nfo", "ps1", "exe", "lnk"];
    let r = programs::list(exts.iter().map(|e| (e.to_string(), 1, None)).collect(), &BTreeMap::new());
    let mut missing = vec![];
    for row in &r.rows {
        eprintln!("QA live .{}: default {:?}", row.ext, row.default.as_ref().map(|p| (&p.name, &p.id)));
        for c in &row.choices {
            let icon = r.icons.get(&c.id).map(|i| i.len()).unwrap_or(0);
            eprintln!("    {} [{}] icon {}", c.name, c.id, icon);
            if icon == 0 {
                missing.push(format!(".{}: {} [{}]", row.ext, c.name, c.id));
            }
            assert!(!c.id.is_empty(), "a choice with no id");
            assert!(icon == 0 || r.icons[&c.id].starts_with("data:image/png;base64,"));
        }
        // No program offered twice.
        let mut ids: Vec<String> = row.choices.iter().map(|c| c.id.to_lowercase()).collect();
        ids.sort();
        let n = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), n, ".{} offers a program twice", row.ext);
        if let Some(d) = &row.default
            && !d.id.is_empty()
        {
            assert_eq!(row.choices.first(), Some(d), ".{}: the default leads the choices", row.ext);
        }
    }
    assert!(missing.is_empty(), "offered programs with no icon: {missing:#?}");
}

#[cfg(windows)]
#[test]
fn qa_list_live_yours_is_an_offered_program() {
    let base = programs::list(vec![("jpg".into(), 1, None), ("txt".into(), 1, None)], &BTreeMap::new());
    let mut ow = BTreeMap::new();
    for row in &base.rows {
        let c = row.choices.last().expect("jpg and txt have programs");
        // Stored in another case: still the offered program, named as the system names it.
        let id = if c.id.chars().any(|ch| ch.is_ascii_lowercase()) { c.id.to_uppercase() } else { c.id.to_lowercase() };
        ow.insert(row.ext.clone(), id);
    }
    let r = programs::list(vec![("jpg".into(), 1, None), ("txt".into(), 1, None)], &ow);
    for (row, b) in r.rows.iter().zip(&base.rows) {
        let y = row.yours.as_ref().unwrap();
        assert_eq!(y, b.choices.last().unwrap(), ".{}", row.ext);
        assert!(!row.gone);
    }
}

// ---------- programs: open ----------

#[test]
fn qa_open_unknown_program_errors_with_name_and_ext() {
    for (id, ext, name) in [
        ("C:\\Nowhere\\Плеер 2.exe", "xyzqa", "Плеер 2"),
        ("C:\\Nowhere\\gone.exe", "jpg", "gone"),
        ("Not A Real Store App", "mp4", "Not A Real Store App"),
        ("/Applications/Gone.app", "pdf", "Gone"),
    ] {
        let e = programs::open(id, ext, "C:\\Nowhere\\file.x").unwrap_err();
        assert!(e.contains(name) && e.contains(&format!(".{ext}")), "{id}: {e}");
    }
    let long = format!("C:\\Nowhere\\{}.exe", "a".repeat(5000));
    assert!(programs::open(&long, "xyzqa", "x").is_err());
    assert!(programs::open("C:\\Nowhere\\a\0b.exe", "xyzqa", "x").is_err());
}

#[test]
fn qa_open_empty_id_names_something() {
    let e = programs::open("", "xyzqa", "C:\\Nowhere\\file.x").unwrap_err();
    assert!(!e.starts_with(' '), "the error names no program: {e:?}");
}

#[test]
fn qa_open_a_folder_is_not_a_program() {
    // Spec: an id that is neither an offered handler nor an existing FILE is the named error.
    let dir = scratch("programs").join("Some Folder");
    std::fs::create_dir_all(&dir).unwrap();
    let r = programs::open(&dir.display().to_string(), "xyzqa", "C:\\Nowhere\\file.x");
    let e = r.expect_err("opening in a folder succeeded");
    assert!(e.contains("Some Folder") && e.contains(".xyzqa"), "the error does not name the program and type: {e:?}");
}

#[cfg(windows)]
#[test]
fn qa_paths_existing_name_ending_in_a_bracket() {
    let dir = scratch("logs").join("Film (2010)");
    std::fs::create_dir_all(&dir).unwrap();
    let d = dir.display().to_string();
    for m in [format!("open {d}"), format!("scanned {d} (3 files) in 0.1 s"), format!("{d} is not connected")] {
        assert_eq!(paths(&one(&m)), [d.as_str()], "{m}");
    }
}

#[cfg(windows)]
#[test]
fn qa_paths_existing_unc_with_spaces() {
    let dir = scratch("logs").join("Music for kids");
    std::fs::create_dir_all(&dir).unwrap();
    let unc = format!("\\\\localhost\\E$\\{}", &dir.display().to_string()[3..]);
    if !std::path::Path::new(&unc).exists() {
        eprintln!("QA: NOT VERIFIED, {unc} is not reachable");
        return;
    }
    let m = format!("scanned {unc} (3 files) in 0.1 s");
    assert_eq!(paths(&one(&m)), [unc.as_str()], "an existing UNC path with spaces is cut");
}

#[test]
fn qa_speed_realistic_lines_on_c() {
    // The module's own speed line, on a drive that exists (C:) instead of Q:.
    let mut raw = String::new();
    let mut i = 0;
    while raw.len() < 1_000_000 {
        raw += &line(&format!("no thumbnail from C:\\gone\\Курс {i}\\Lesson 04 part two.mkv: ffmpeg found no frame"));
        i += 1;
    }
    let ms = timed("1 MB, the module's line on C:", raw.as_bytes(), logs_app);
    let raw_q = raw.replace("C:\\", "Q:\\");
    let ms_q = timed("1 MB, the module's line on Q:", raw_q.as_bytes(), logs_app);
    eprintln!("QA speed: C: {ms} ms vs Q: {ms_q} ms");
    assert!(ms < LIMIT_MS, "{ms} ms");
}

// ======================================================================================
// QA: the Marks backup (`backup`, `read_backup` in lib.rs, `Marks::merge` in store.rs),
// against the spec in `decisions/2026/10/09/the-app-imports-nothing-from-the-hub.md` and `decisions/2026/10/09/records-matched-to-the-built-app.md` "Stores", not against lib.rs's own tests.
// ======================================================================================

mod backup_qa {
    use crate::store::{Choice, Mark, Marks};
    use crate::{backup, read_backup, BACKUP_KIND};
    use serde_json::{json, Value};
    use std::time::Instant;

    const REFUSED: &str = "This file is not a marks backup from FilesViewHub.";

    fn mark(m: &str, at: u64) -> Mark {
        Mark { m: m.into(), at }
    }

    fn choice(keep: bool, at: u64) -> Choice {
        Choice { keep, at, size: Some(100) }
    }

    fn sample() -> Marks {
        let mut s = Marks::default();
        s.marks.insert("0A1B2C3D:Книги/Мастер и Маргарита".into(), mark("done", 1_700_000_000));
        s.marks.insert("0A1B2C3D:Courses/Rust".into(), mark("next", 1_700_000_001));
        s.ticks.entry("0A1B2C3D:Courses/Rust".into()).or_default().insert("01 Intro.mp4".into(), 1_700_000_002);
        s.ticks.entry("0A1B2C3D:Courses/Rust".into()).or_default().insert("02 Борроу.mp4".into(), 1_700_000_003);
        s.review.insert("0A1B2C3D:Camera/IMG_1.jpg".into(), choice(false, 1_700_000_004));
        s.review.insert("0A1B2C3D:Camera/IMG_2.jpg".into(), Choice { keep: true, at: 1_700_000_005, size: None });
        s
    }

    fn as_value(m: &Marks) -> Value {
        serde_json::to_value(m).unwrap()
    }

    fn refused(text: &str) {
        match read_backup(text) {
            Ok(_) => panic!("read, should be refused: {text:?}"),
            Err(e) => assert_eq!(e, REFUSED, "for {text:?}"),
        }
    }

    /// The text the app writes, through the same `store::save` path `export_marks` uses.
    fn saved_text(m: Marks, ui: Value, name: &str) -> String {
        let p = super::scratch("backup").join(name);
        crate::store::save(&p, &backup(m, ui)).unwrap();
        std::fs::read_to_string(&p).unwrap()
    }

    fn count_v_keys(text: &str) -> usize {
        let v: Value = serde_json::from_str(text).unwrap();
        // A JSON object in serde_json keeps one value per key, so count in the raw text, top level.
        let mut depth = 0i32;
        let (mut n, mut in_str, mut esc) = (0, false, false);
        let b = text.as_bytes();
        for (i, &c) in b.iter().enumerate() {
            if in_str {
                if esc { esc = false } else if c == b'\\' { esc = true } else if c == b'"' { in_str = false }
                continue;
            }
            match c {
                b'{' | b'[' => depth += 1,
                b'}' | b']' => depth -= 1,
                b'"' => {
                    if depth == 1 && text[i..].starts_with("\"v\"") {
                        let rest = text[i + 3..].trim_start();
                        if rest.starts_with(':') { n += 1 }
                    }
                    in_str = true;
                }
                _ => {}
            }
        }
        assert!(v.is_object());
        n
    }

    // ---------- round trip ----------

    #[test]
    fn qa_backup_saved_by_the_app_reads_back_identical() {
        let ui = json!({"size": "large", "view": "grid", "colw": [220, 300], "folders": true});
        let text = saved_text(sample(), ui.clone(), "roundtrip.json");
        let b = read_backup(&text).expect("the app's own backup reads");
        assert_eq!(b.kind, BACKUP_KIND);
        assert_eq!(as_value(&b.marks), as_value(&sample()));
        assert_eq!(b.ui, ui);
        assert_eq!(count_v_keys(&text), 1, "exactly one top-level v: {text}");
        let top: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(top["kind"], "filesviewhub-marks");
        assert_eq!(top["v"], 2, "the Marks' own version");
    }

    #[test]
    fn qa_backup_with_null_ui_round_trips_as_null() {
        let text = saved_text(sample(), Value::Null, "null-ui.json");
        assert!(read_backup(&text).unwrap().ui.is_null());
        assert_eq!(count_v_keys(&text), 1);
    }

    #[test]
    fn qa_empty_backup_round_trips() {
        let text = saved_text(Marks::default(), json!({}), "empty.json");
        let b = read_backup(&text).unwrap();
        assert!(b.marks.marks.is_empty() && b.marks.ticks.is_empty() && b.marks.review.is_empty());
        assert_eq!(b.ui, json!({}));
        assert_eq!(count_v_keys(&text), 1);
    }

    #[test]
    fn qa_backup_read_twice_is_the_same() {
        let text = saved_text(sample(), json!({"a": 1}), "twice.json");
        let once = read_backup(&text).unwrap();
        let again = serde_json::to_string(&backup(once.marks, once.ui)).unwrap();
        assert_eq!(again.len(), text.len(), "save, read, save keeps the size");
        let a: Value = serde_json::from_str(&again).unwrap();
        let b: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn qa_awkward_work_ids_round_trip() {
        let mut m = Marks::default();
        let ids = [
            "0A1B2C3D:Книги/Достоевский - Преступление и наказание (1866)".to_string(),
            "0A1B2C3D:quote\"in\"name".to_string(),
            "0A1B2C3D:back\\slash\\path\\".to_string(),
            "0A1B2C3D:emoji 📚🎬 and ZWJ 👨‍👩‍👧".to_string(),
            "0A1B2C3D:control\u{0001}\t\nchars".to_string(),
            "0A1B2C3D:\u{FEFF}starts with a BOM".to_string(),
            "".to_string(),
            format!("0A1B2C3D:{}", "Очень длинное имя/".repeat(2_000)),
        ];
        for (i, id) in ids.iter().enumerate() {
            m.marks.insert(id.clone(), mark("done", i as u64));
            m.ticks.entry(id.clone()).or_default().insert(format!("{id}/item \"x\""), i as u64);
            m.review.insert(id.clone(), choice(i % 2 == 0, i as u64));
        }
        let text = saved_text(m.clone(), json!({"q": "\"\\"}), "awkward.json");
        let b = read_backup(&text).unwrap();
        assert_eq!(as_value(&b.marks), as_value(&m));
        for id in &ids {
            assert!(b.marks.marks.contains_key(id), "id kept: {:.60}", id);
        }
        assert_eq!(count_v_keys(&text), 1);
    }

    #[test]
    fn qa_large_backup_reads_fast() {
        let mut m = Marks::default();
        for i in 0..50_000u64 {
            m.marks.insert(format!("0A1B2C3D:Section {}/Работа номер {i}", i % 40), mark(if i % 2 == 0 { "done" } else { "next" }, 1_700_000_000 + i));
        }
        for i in 0..2_000u64 {
            let t = m.ticks.entry(format!("0A1B2C3D:Courses/{i}")).or_default();
            for j in 0..10 {
                t.insert(format!("{j:02} lesson.mp4"), i);
            }
        }
        for i in 0..20_000u64 {
            m.review.insert(format!("0A1B2C3D:Camera/IMG_{i:05}.jpg"), choice(i % 3 == 0, i));
        }
        let text = saved_text(m.clone(), json!({"size": "small"}), "large.json");
        let t = Instant::now();
        let b = read_backup(&text).unwrap();
        let ms = t.elapsed().as_millis();
        eprintln!("QA backup: {} bytes read in {ms} ms", text.len());
        assert_eq!(b.marks.marks.len(), 50_000);
        assert_eq!(b.marks.ticks.values().map(|t| t.len()).sum::<usize>(), 20_000);
        assert_eq!(b.marks.review.len(), 20_000);
        assert_eq!(as_value(&b.marks), as_value(&m));
        assert!(ms < 1000, "50,000 marks read in {ms} ms");
    }

    // ---------- older files ----------

    #[test]
    fn qa_pre_fix_backup_with_two_v_keys_reads() {
        // Exactly the shape the old Backup struct wrote: kind, its own v:1, then the flattened Marks.
        let text = r#"{"kind":"filesviewhub-marks","v":1,"v":2,"marks":{"0A1B2C3D:Книги/Дюна":{"m":"done","at":10}},"ticks":{"0A1B2C3D:C/":{"01":11}},"review":{"0A1B2C3D:Camera/a.jpg":{"keep":false,"at":12,"size":5}},"ui":{"size":"compact"}}"#;
        let b = read_backup(text).expect("a pre-fix backup reads");
        assert_eq!(b.marks.marks["0A1B2C3D:Книги/Дюна"].m, "done");
        assert_eq!(b.marks.marks["0A1B2C3D:Книги/Дюна"].at, 10);
        assert_eq!(b.marks.ticks["0A1B2C3D:C/"]["01"], 11);
        assert!(!b.marks.review["0A1B2C3D:Camera/a.jpg"].keep);
        assert_eq!(b.ui, json!({"size": "compact"}));
    }

    #[test]
    fn qa_pre_fix_backup_written_through_old_struct_reads() {
        // Rebuild the old struct here and let serde write it, so the test does not rely on a
        // hand-typed guess of the old output.
        #[derive(serde::Serialize)]
        struct OldBackup {
            kind: String,
            v: u32,
            #[serde(flatten)]
            marks: Marks,
            ui: Value,
        }
        let old = serde_json::to_string(&OldBackup { kind: BACKUP_KIND.into(), v: 1, marks: sample(), ui: json!({"z": 1}) }).unwrap();
        assert_eq!(count_v_keys(&old), 2, "the old struct wrote two v keys: {old}");
        let b = read_backup(&old).expect("the old app's backup reads");
        assert_eq!(as_value(&b.marks), as_value(&sample()));
        assert_eq!(b.ui, json!({"z": 1}));
    }

    #[test]
    fn qa_missing_review_ticks_ui_read_as_empty() {
        let b = read_backup(r#"{"kind":"filesviewhub-marks","v":2,"marks":{"a":{"m":"next","at":1}}}"#).unwrap();
        assert_eq!(b.marks.marks.len(), 1);
        assert!(b.marks.ticks.is_empty());
        assert!(b.marks.review.is_empty());
        assert!(b.ui.is_null(), "no ui: the current settings stay");
    }

    #[test]
    fn qa_v1_backup_with_no_ticks_reads() {
        // store.rs: "Version 1 had no Ticks and reads as version 2 with none."
        let b = read_backup(r#"{"kind":"filesviewhub-marks","v":1,"marks":{"a":{"m":"done","at":3}}}"#).unwrap();
        assert_eq!(b.marks.marks["a"].at, 3);
        assert!(b.marks.ticks.is_empty());
    }

    #[test]
    fn qa_extra_unknown_keys_are_ignored() {
        let b = read_backup(r#"{"kind":"filesviewhub-marks","v":2,"marks":{"a":{"m":"done","at":3,"extra":true}},"future":[1,2],"note":"hi","ui":{"x":1}}"#)
            .expect("unknown keys do not refuse a backup");
        assert_eq!(b.marks.marks["a"].m, "done");
        assert_eq!(b.ui, json!({"x": 1}));
    }

    // ---------- refused ----------

    #[test]
    fn qa_not_a_backup_is_refused_with_the_exact_message() {
        let cases = [
            // A Hub backup (`decisions/2026/10/09/the-app-imports-nothing-from-the-hub.md`).
            r#"{"kind":"hub-marks","v":3,"marks":{"x":{"m":"done","at":null}},"ticks":{},"settings":{}}"#,
            r#"{"kind":"hub-marks","v":2,"marks":{}}"#,
            // No kind, a wrong kind, a kind of the wrong type or case.
            r#"{"v":2,"marks":{},"ticks":{},"review":{}}"#,
            r#"{"kind":"filesviewhub-settings","v":2,"marks":{}}"#,
            r#"{"kind":"FilesViewHub-marks","v":2,"marks":{}}"#,
            r#"{"kind":" filesviewhub-marks","v":2,"marks":{}}"#,
            r#"{"kind":null,"v":2,"marks":{}}"#,
            r#"{"kind":1,"v":2,"marks":{}}"#,
            r#"{"kind":["filesviewhub-marks"],"v":2,"marks":{}}"#,
            // Broken JSON, empty, whitespace, an array, non-objects.
            r#"{"kind":"filesviewhub-marks","v":2,"marks":{}"#,
            r#"{"kind":"filesviewhub-marks","v":2,"marks":{},}"#,
            r#"{"kind":"filesviewhub-marks","v":2,"marks":{}} trailing"#,
            "",
            "   \r\n",
            "[]",
            r#"[{"kind":"filesviewhub-marks","v":2,"marks":{}}]"#,
            "null",
            "true",
            "42",
            r#""filesviewhub-marks""#,
        ];
        for t in cases {
            refused(t);
        }
    }

    #[test]
    fn qa_right_kind_but_wrong_shape_is_refused() {
        let cases = [
            // marks missing entirely (Marks.marks has no default).
            r#"{"kind":"filesviewhub-marks","v":2}"#,
            // v missing, a string, negative.
            r#"{"kind":"filesviewhub-marks","marks":{}}"#,
            r#"{"kind":"filesviewhub-marks","v":"2","marks":{}}"#,
            r#"{"kind":"filesviewhub-marks","v":-1,"marks":{}}"#,
            // at negative, a float, null, a string.
            r#"{"kind":"filesviewhub-marks","v":2,"marks":{"a":{"m":"done","at":-5}}}"#,
            r#"{"kind":"filesviewhub-marks","v":2,"marks":{"a":{"m":"done","at":1.5}}}"#,
            r#"{"kind":"filesviewhub-marks","v":2,"marks":{"a":{"m":"done","at":null}}}"#,
            r#"{"kind":"filesviewhub-marks","v":2,"marks":{"a":{"m":"done","at":"2026-10-09"}}}"#,
            // marks not an object.
            r#"{"kind":"filesviewhub-marks","v":2,"marks":[]}"#,
            r#"{"kind":"filesviewhub-marks","v":2,"marks":null}"#,
            // A Tick time negative.
            r#"{"kind":"filesviewhub-marks","v":2,"marks":{},"ticks":{"a":{"k":-1}}}"#,
        ];
        for t in cases {
            refused(t);
        }
    }

    #[test]
    fn qa_newer_version_is_refused_and_older_reads() {
        // e9b055a: a backup from a newer app is refused with its own message, never read in part.
        let newer = "This backup was saved by a newer FilesViewHub. Use that version to add it.";
        for v in [3u64, 4, 4_294_967_295] {
            let t = format!(r#"{{"kind":"filesviewhub-marks","v":{v},"marks":{{"a":{{"m":"done","at":1}}}}}}"#);
            assert_eq!(read_backup(&t).err().as_deref(), Some(newer), "v {v}");
        }
        for v in [1, 2] {
            let t = format!(r#"{{"kind":"filesviewhub-marks","v":{v},"marks":{{}}}}"#);
            assert!(read_backup(&t).is_ok(), "v {v} reads");
        }
        // The current app's own backup is never refused as newer.
        assert!(read_backup(&serde_json::to_string(&backup(Marks::default(), Value::Null)).unwrap()).is_ok());
    }

    // ---------- observations where the spec is silent (print, assert only what is stable) ----------

    fn observe(label: &str, text: &str) -> Result<crate::Backup, String> {
        let r = read_backup(text);
        match &r {
            Ok(b) => eprintln!("QA observe [{label}]: READS, v={}, marks={}, ui={}", b.marks.v, b.marks.marks.len(), b.ui),
            Err(e) => eprintln!("QA observe [{label}]: REFUSED: {e}"),
        }
        r
    }

    #[test]
    fn qa_a_backup_saved_with_a_bom_reads() {
        let plain = r#"{"kind":"filesviewhub-marks","v":2,"marks":{"a":{"m":"done","at":1}}}"#;
        let with_bom = format!("\u{FEFF}{plain}");
        // A file saved by Notepad as "UTF-8 with BOM", read as read_to_string does.
        let p = super::scratch("backup").join("bom.json");
        std::fs::write(&p, with_bom.as_bytes()).unwrap();
        let text = std::fs::read_to_string(&p).unwrap();
        assert!(text.starts_with('\u{FEFF}'), "read_to_string keeps the BOM");
        let b = crate::read_backup(&text).expect("a backup re-saved by a text editor with a BOM reads");
        assert_eq!(b.marks.marks["a"].m, "done");
    }

    #[test]
    fn qa_observe_version_values() {
        for (label, t) in [
            ("v 1", r#"{"kind":"filesviewhub-marks","v":1,"marks":{}}"#),
            ("v 3", r#"{"kind":"filesviewhub-marks","v":3,"marks":{}}"#),
            ("v 0", r#"{"kind":"filesviewhub-marks","v":0,"marks":{}}"#),
            ("v 2.0", r#"{"kind":"filesviewhub-marks","v":2.0,"marks":{}}"#),
            ("v string", r#"{"kind":"filesviewhub-marks","v":"2","marks":{}}"#),
            ("v 99999999999", r#"{"kind":"filesviewhub-marks","v":99999999999,"marks":{}}"#),
            ("v twice, 2 then 1", r#"{"kind":"filesviewhub-marks","v":2,"v":1,"marks":{}}"#),
            ("v twice, string then 2", r#"{"kind":"filesviewhub-marks","v":"x","v":2,"marks":{}}"#),
            ("kind twice, hub last", r#"{"kind":"filesviewhub-marks","kind":"hub-marks","v":2,"marks":{}}"#),
            ("kind twice, fvh last", r#"{"kind":"hub-marks","kind":"filesviewhub-marks","v":2,"marks":{}}"#),
            ("mark m unknown", r#"{"kind":"filesviewhub-marks","v":2,"marks":{"a":{"m":"banana","at":1}}}"#),
            ("ui a string", r#"{"kind":"filesviewhub-marks","v":2,"marks":{},"ui":"big"}"#),
            ("ui an array", r#"{"kind":"filesviewhub-marks","v":2,"marks":{},"ui":[1]}"#),
            ("ticks null", r#"{"kind":"filesviewhub-marks","v":2,"marks":{},"ticks":null}"#),
            ("review null", r#"{"kind":"filesviewhub-marks","v":2,"marks":{},"review":null}"#),
        ] {
            let _ = observe(label, t);
        }
    }

    // ---------- merge, as import_marks does it (`decisions/2026/10/09/records-matched-to-the-built-app.md` "Stores") ----------

    /// What import_marks does with a backup's text: read it, then merge into the app's Marks.
    fn import(into: &mut Marks, text: &str) -> usize {
        let b = read_backup(text).unwrap();
        into.merge(b.marks)
    }

    #[test]
    fn qa_merge_keeps_the_later_mark_even_when_both_are_the_same_mark() {
        let mut mine = Marks::default();
        mine.marks.insert("same-older".into(), mark("done", 100));
        mine.marks.insert("same-newer".into(), mark("done", 300));
        mine.marks.insert("diff-older".into(), mark("next", 100));
        mine.marks.insert("diff-newer".into(), mark("next", 300));
        mine.marks.insert("tie".into(), mark("next", 200));
        let mut theirs = Marks::default();
        theirs.marks.insert("same-older".into(), mark("done", 200));
        theirs.marks.insert("same-newer".into(), mark("done", 200));
        theirs.marks.insert("diff-older".into(), mark("done", 200));
        theirs.marks.insert("diff-newer".into(), mark("done", 200));
        theirs.marks.insert("tie".into(), mark("done", 200));
        theirs.marks.insert("only-theirs".into(), mark("next", 1));
        let text = serde_json::to_string(&backup(theirs, Value::Null)).unwrap();
        import(&mut mine, &text);
        let got = |id: &str| (mine.marks[id].m.clone(), mine.marks[id].at);
        assert_eq!(got("same-older"), ("done".into(), 200), "same Mark, backup later: its time wins");
        assert_eq!(got("same-newer"), ("done".into(), 300));
        assert_eq!(got("diff-older"), ("done".into(), 200));
        assert_eq!(got("diff-newer"), ("next".into(), 300));
        assert_eq!(got("only-theirs"), ("next".into(), 1));
        eprintln!("QA observe [mark tie at same time, different Mark]: kept {:?}", got("tie"));
    }

    #[test]
    fn qa_merge_keeps_the_earlier_tick_time() {
        let mut mine = Marks::default();
        mine.ticks.entry("w".into()).or_default().insert("a".into(), 50);
        mine.ticks.entry("w".into()).or_default().insert("b".into(), 500);
        let mut theirs = Marks::default();
        theirs.ticks.entry("w".into()).or_default().insert("a".into(), 100);
        theirs.ticks.entry("w".into()).or_default().insert("b".into(), 200);
        theirs.ticks.entry("w".into()).or_default().insert("c".into(), 7);
        theirs.ticks.entry("w2".into()).or_default().insert("x".into(), 9);
        let text = serde_json::to_string(&backup(theirs, Value::Null)).unwrap();
        import(&mut mine, &text);
        assert_eq!(mine.ticks["w"]["a"], 50);
        assert_eq!(mine.ticks["w"]["b"], 200);
        assert_eq!(mine.ticks["w"]["c"], 7);
        assert_eq!(mine.ticks["w2"]["x"], 9);
    }

    #[test]
    fn qa_merge_keeps_the_later_review_choice() {
        let mut mine = Marks::default();
        mine.review.insert("older".into(), choice(true, 100));
        mine.review.insert("newer".into(), choice(true, 300));
        let mut theirs = Marks::default();
        theirs.review.insert("older".into(), choice(false, 200));
        theirs.review.insert("newer".into(), choice(false, 200));
        theirs.review.insert("new".into(), choice(false, 1));
        let text = serde_json::to_string(&backup(theirs, Value::Null)).unwrap();
        import(&mut mine, &text);
        assert!(!mine.review["older"].keep && mine.review["older"].at == 200);
        assert!(mine.review["newer"].keep && mine.review["newer"].at == 300);
        assert!(!mine.review["new"].keep);
    }

    #[test]
    fn qa_importing_your_own_backup_changes_nothing() {
        let mut mine = sample();
        let text = saved_text(sample(), json!({}), "self.json");
        let added = import(&mut mine, &text);
        assert_eq!(as_value(&mine), as_value(&sample()), "adding the same backup is idempotent");
        assert_eq!(added, 0, "nothing new was added");
    }

    #[test]
    fn qa_importing_twice_is_idempotent() {
        let mut theirs = sample();
        theirs.marks.insert("extra".into(), mark("next", 9));
        let text = serde_json::to_string(&backup(theirs, Value::Null)).unwrap();
        let mut mine = Marks::default();
        let first = import(&mut mine, &text);
        let after_first = as_value(&mine);
        let second = import(&mut mine, &text);
        assert_eq!(first, 3);
        assert_eq!(second, 0);
        assert_eq!(as_value(&mine), after_first);
    }
}

// ---------- a Marks backup added after a move was undone ----------

/// Settings, Marks, Add a backup runs `merge`, then forgets the choices of photos put back by an
/// Undo (`moves::put_back` + `forget_put_back`), as `import_marks` does. A backup saved before the
/// move must not bring back the choice that moved a photo; a choice made after the Undo, and the
/// choice of a photo still moved, stay.
#[test]
fn qa_backup_made_before_an_undone_move_brings_back_no_choice_of_the_photos_put_back() {
    use crate::moves;
    use crate::store::{Choice, Marks};
    let base = scratch("import-114");
    let _ = std::fs::remove_dir_all(&base);
    let (d, data) = (base.join("Photos"), base.join("data"));
    std::fs::create_dir_all(&d).unwrap();
    std::fs::create_dir_all(&data).unwrap();
    let jf = data.join("moves.json");
    for f in ["a.jpg", "b.jpg", "c.jpg", "Папка 🙂/d.jpg"] {
        std::fs::create_dir_all(d.join(f).parent().unwrap()).unwrap();
        std::fs::write(d.join(f), f).unwrap();
    }
    let seen = |f: &str| {
        let md = std::fs::metadata(d.join(f)).unwrap();
        let t = md.modified().unwrap().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        moves::Photo { rel: f.into(), size: md.len(), mtime: t, cloud: false, preview: false }
    };
    let sec = "0000ABCD:Photos";
    let key = |f: &str| format!("{sec}/{f}");
    let old = crate::store::now() - 100;
    // The backup: every photo marked Not needed before the move.
    let mut backup = Marks::default();
    for f in ["a.jpg", "b.jpg", "c.jpg", "Папка 🙂/d.jpg"] {
        backup.review.insert(key(f), Choice { keep: false, at: old, size: Some(5) });
    }
    let mut m = backup.clone();
    let out = moves::move_photos(&jf, sec, &d, &d.join(crate::shots::NOT_NEEDED), &[seen("a.jpg"), seen("b.jpg"), seen("c.jpg"), seen("Папка 🙂/d.jpg")], &|_, _| {}).unwrap();
    // c's old place is taken, so c stays moved and keeps its choice.
    std::fs::write(d.join("c.jpg"), "other").unwrap();
    moves::undo(&jf, out.id, &|_, _| {}).unwrap();
    let back = moves::put_back(&jf);
    assert_eq!(crate::forget_put_back(&mut m, &back), 3, "a, b and d are put back");
    // After the Undo, b is chosen again: Keep, later than the move.
    m.review.insert(key("b.jpg"), Choice { keep: true, at: crate::store::now() + 10, size: Some(5) });
    // Add the old backup, as import_marks does.
    m.merge(backup.clone());
    crate::forget_put_back(&mut m, &moves::put_back(&jf));
    assert!(!m.review.contains_key(&key("a.jpg")), "a's old choice is not brought back");
    assert!(!m.review.contains_key(&key("Папка 🙂/d.jpg")), "nor d's, in a non-ASCII folder");
    assert!(m.review[&key("b.jpg")].keep, "b's newer choice stays");
    assert!(!m.review[&key("c.jpg")].keep, "c, still moved, keeps its choice");
    let _ = std::fs::remove_dir_all(&base);
}
