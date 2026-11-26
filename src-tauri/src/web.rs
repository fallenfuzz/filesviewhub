//! Covers from the web (`decisions/2026/10/05/nothing-hidden-no-network-by-default.md`, `decisions/2026/10/22/covers-are-chosen-never-fetched-by-themselves.md`), the app's only network use. Nothing is sent until the
//! person presses Search in Choose a cover…: the search words go to Open Library and FantLab,
//! the small pictures of the covers found are fetched to choose from, and the one picked is
//! fetched in full and kept (`chosen.rs`). Every request, and each redirect it is sent on, is
//! written to the network log before it is sent.

use crate::tools::Fail;
use serde::Serialize;
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant, SystemTime};

pub const OPEN_LIBRARY: &str = "openlibrary";
pub const FANTLAB: &str = "fantlab";
/// The least time between two requests to a site's search, so neither service is pressed.
const PACE: Duration = Duration::from_millis(1500);
/// The least time between two pictures from one host.
const PACE_PICTURES: Duration = Duration::from_millis(250);
/// Covers shown from each source, at most.
const SHOWN: usize = 12;
/// Book formats a name may still end in after its extension is taken off ("x.fb2.mobi").
const FORMATS: &[&str] = &["pdf", "epub", "djvu", "djv", "fb2", "mobi", "azw3", "kfx", "cbz", "cbr", "chm", "doc", "docx", "rtf", "txt", "zip", "rar"];
/// The hosts a cover picture may come from, before any redirect.
const PICTURE_HOSTS: &[&str] = &["covers.openlibrary.org", "fantlab.ru"];

/// The picker's search: a small picture asked for in an earlier one, or before the picker closed,
/// is not fetched.
static ROUND: AtomicU64 = AtomicU64::new(0);

/// Starts a new round of small pictures (a new search, or the picker closed), and gives it.
pub fn next_round() -> u64 {
    ROUND.fetch_add(1, Ordering::SeqCst) + 1
}

/// Requests sent since the app started: none until a person asks.
pub static SENT: AtomicUsize = AtomicUsize::new(0);

/// A cover found on the web: the picture to keep, a small one to choose by, and what the source
/// says about the book.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Found {
    pub url: String,
    pub preview: String,
    pub from: String,
    pub title: String,
    pub by: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<i64>,
}

/// What one source gave for a search: its covers, or why it gave none.
#[derive(Serialize, Clone, Debug)]
pub struct Results {
    pub from: String,
    pub found: Vec<Found>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// The search words the picker starts with, from a Work's or folder's name: the title and the
/// author as the name gives them, with no years, edition notes, file ids, series in brackets,
/// format names or punctuation. The person can change them before anything is sent.
pub fn words_for(name: &str) -> String {
    let (bare, author) = split(name);
    let all = format!("{} {}", query(&bare), query(&author));
    all.split_whitespace().filter(|t| !FORMATS.contains(&t.to_lowercase().as_str())).collect::<Vec<_>>().join(" ")
}

/// A file-name title split into the book's title and its author: "Author - Title - Year",
/// "Author 3. Title" and "Title (Author)". A series in brackets after the title is not part of it.
fn split(title: &str) -> (String, String) {
    let parts: Vec<&str> = title.split(" - ").filter(|p| !(p.trim().len() == 4 && p.trim().chars().all(|c| c.is_ascii_digit()))).collect();
    let (bare, author) = if parts.len() > 1 {
        (parts[parts.len() - 1].to_string(), parts[..parts.len() - 1].join(" "))
    } else if let Some(k) = numbered_at(title) {
        (title[k..].to_string(), title[..k].to_string())
    } else if let Some((t, a)) = title.trim_end().strip_suffix(')').and_then(|t| t.rsplit_once(" (")) {
        (t.to_string(), a.to_string())
    } else {
        (title.to_string(), String::new())
    };
    let bare = match bare.trim_end().strip_suffix(')').and_then(|t| t.rsplit_once(" (")) {
        Some((t, _)) if !t.trim().is_empty() => t.to_string(),
        _ => bare,
    };
    (bare.trim().to_string(), author.trim().to_string())
}

/// Where the title starts after a number like "3. " in "Author 3. Title".
fn numbered_at(s: &str) -> Option<usize> {
    let b = s.as_bytes();
    (1..b.len().saturating_sub(1)).rev().find(|&i| b[i] == b'.' && b[i + 1] == b' ' && b[i - 1].is_ascii_digit()).map(|i| i + 2)
}

/// Search words from a title: no years, edition notes, file ids or punctuation.
fn query(s: &str) -> String {
    let s: String = s.chars().map(|c| if "[](){}_.,:;-".contains(c) { ' ' } else { c }).collect();
    let toks: Vec<&str> = s.split_whitespace().collect();
    let year = |t: &str| t.len() == 4 && (t.starts_with("19") || t.starts_with("20")) && t.chars().all(|c| c.is_ascii_digit());
    let id = |t: &str| t.len() >= 5 && t.chars().all(|c| c.is_ascii_digit());
    let ordinal = |t: &str| { let t = t.to_lowercase(); ["st", "nd", "rd", "th"].iter().any(|e| t.strip_suffix(e).is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))) };
    let mut out: Vec<&str> = vec![];
    for (i, t) in toks.iter().enumerate() {
        let edition = t.eq_ignore_ascii_case("edition") && i > 0 && ordinal(toks[i - 1]);
        if edition {
            out.pop();
        } else if !year(t) && !id(t) {
            out.push(t);
        }
    }
    out.join(" ")
}

/// Searches both sources for these words, each on its own: one that fails says why, and the
/// other's covers still show.
pub fn search(words: &str) -> Vec<Results> {
    let words = words.split_whitespace().collect::<Vec<_>>().join(" ");
    let why = format!("cover search for \"{words}\", asked in Choose a cover");
    let ask = |from: &str, url: String, read: fn(&serde_json::Value) -> Vec<Found>| {
        let r = get(&url, &why, None).map_err(err).and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).map_err(|e| e.to_string()));
        match r {
            Ok(v) => Results { from: from.into(), found: read(&v), error: None },
            Err(e) => Results { from: from.into(), found: vec![], error: Some(e) },
        }
    };
    if words.is_empty() {
        return vec![];
    }
    vec![
        ask(OPEN_LIBRARY, format!("https://openlibrary.org/search.json?q={}&fields=title,author_name,first_publish_year,cover_i&limit=30", encode(&words)), open_library),
        ask(FANTLAB, format!("https://api.fantlab.ru/search-works?q={}", encode(&words)), fantlab),
    ]
}

fn open_library(v: &serde_json::Value) -> Vec<Found> {
    let docs = v["docs"].as_array().into_iter().flatten();
    docs.filter_map(|d| {
        let id = d["cover_i"].as_i64().filter(|i| *i > 0)?;
        let by: Vec<&str> = d["author_name"].as_array().into_iter().flatten().filter_map(|a| a.as_str()).collect();
        Some(Found {
            url: format!("https://covers.openlibrary.org/b/id/{id}-L.jpg?default=false"),
            preview: format!("https://covers.openlibrary.org/b/id/{id}-M.jpg?default=false"),
            from: OPEN_LIBRARY.into(),
            title: d["title"].as_str().unwrap_or_default().into(),
            by: by.join(", "),
            year: d["first_publish_year"].as_i64(),
        })
    })
    .take(SHOWN)
    .collect()
}

fn fantlab(v: &serde_json::Value) -> Vec<Found> {
    let matches = v["matches"].as_array().into_iter().flatten();
    let mut seen = vec![];
    matches
        .filter_map(|m| {
            let pic = m["pic_edition_id"].as_i64().filter(|p| *p > 0).or_else(|| m["pic_edition_id_auto"].as_i64().filter(|p| *p > 0))?;
            if seen.contains(&pic) {
                return None;
            }
            seen.push(pic);
            let s = |k: &str| m[k].as_str().unwrap_or_default().trim().to_string();
            let or = |a: String, b: String| if a.is_empty() { b } else { a };
            Some(Found {
                url: format!("https://fantlab.ru/images/editions/big/{pic}"),
                preview: format!("https://fantlab.ru/images/editions/small/{pic}"),
                from: FANTLAB.into(),
                title: or(s("rusname"), s("name")),
                by: or(s("all_autor_rusname"), s("all_autor_name")),
                year: m["year"].as_i64().filter(|y| *y > 0),
            })
        })
        .take(SHOWN)
        .collect()
}

/// A cover picture's bytes: `why` says whether it is the small one to choose by or the one picked.
/// Only an address a search gave is fetched; with `round`, only while that round of small
/// pictures is the picker's (`next_round`).
pub fn picture(url: &str, why: &str, round: Option<u64>) -> Result<Vec<u8>, String> {
    if !url.starts_with("https://") || !PICTURE_HOSTS.contains(&host(url)) {
        return Err(format!("not a cover address: {url}"));
    }
    get(url, why, round).map_err(err)
}

fn err(f: Fail) -> String {
    match f {
        Fail::Retry(e) | Fail::Bad(e) => e,
    }
}

fn encode(s: &str) -> String {
    s.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) { (b as char).to_string() } else if b == b' ' { "+".into() } else { format!("%{b:02X}") }).collect()
}

static AGENT: LazyLock<ureq::Agent> = LazyLock::new(|| {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(20)))
        .user_agent(concat!("FilesViewHub/", env!("CARGO_PKG_VERSION")))
        // Each redirect is followed by `get`, so it goes into the network log first.
        .max_redirects(0)
        .build()
        .into()
});
/// When each host may be asked next.
static NEXT: LazyLock<Mutex<HashMap<String, Instant>>> = LazyLock::new(Default::default);

/// Waits for a host's turn: a site's search at most once in `PACE`, its pictures in
/// `PACE_PICTURES`. Requests at the same time each take their own turn.
fn wait_turn(url: &str) {
    let h = host(url).to_string();
    let pace = if h == "openlibrary.org" || h == "api.fantlab.ru" { PACE } else { PACE_PICTURES };
    let at = {
        let mut next = NEXT.lock().unwrap();
        let now = Instant::now();
        let at = next.get(&h).copied().filter(|t| *t > now).unwrap_or(now);
        next.insert(h, at + pace);
        at
    };
    std::thread::sleep(at.saturating_duration_since(Instant::now()));
}

/// Redirects followed for one request, at most.
const HOPS: usize = 5;

/// One GET request: paced, written to the network log first, and so is each redirect it is sent
/// on (an Open Library cover comes from archive.org). A missing page is no picture; any other
/// failure (offline, timed out, asked to slow down) is tried again later.
fn get(url: &str, why: &str, round: Option<u64>) -> Result<Vec<u8>, Fail> {
    wait_turn(url);
    // A small picture waits its turn; by then the picker that asked may be closed.
    if round.is_some_and(|r| r != ROUND.load(Ordering::SeqCst)) {
        return Err(Fail::Bad("no longer wanted".into()));
    }
    let (mut url, mut why) = (url.to_string(), why.to_string());
    let asked = why.clone();
    for _ in 0..=HOPS {
        note(&url, &why)?;
        let mut r = match AGENT.get(&url).call() {
            Ok(r) => r,
            Err(ureq::Error::StatusCode(c @ (404 | 410))) => return Err(Fail::Bad(format!("{c} from {url}"))),
            Err(e) => return Err(Fail::Retry(e.to_string())),
        };
        if !r.status().is_redirection() {
            return r.body_mut().with_config().limit(10 << 20).read_to_vec().map_err(|e| Fail::Retry(e.to_string()));
        }
        let to = r.headers().get("location").and_then(|l| l.to_str().ok()).ok_or_else(|| Fail::Bad(format!("a redirect with no address from {url}")))?;
        let to = redirect(&url, to).ok_or_else(|| Fail::Bad(format!("a redirect from {url} to {to}, not an https address on a listed host")))?;
        why = format!("{asked}, sent on by {}", host(&url));
        url = to;
    }
    Err(Fail::Bad(format!("more than {HOPS} redirects from {url}")))
}

/// The hosts a request may be sent on to, as "What this app does" lists them (`strings.ts`
/// `openLibraryWhat`, `fantLabWhat`); `*.us.archive.org` holds Open Library's pictures.
const HOSTS: &[&str] = &["openlibrary.org", "covers.openlibrary.org", "archive.org", "fantlab.ru", "api.fantlab.ru"];

fn listed(h: &str) -> bool {
    HOSTS.contains(&h) || h.strip_suffix(".us.archive.org").is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_alphanumeric()))
}

/// The address a redirect sends to: only https, only to a listed host; a path on the same host
/// is taken as it is.
fn redirect(from: &str, to: &str) -> Option<String> {
    let to = if to.starts_with("https://") {
        to.to_string()
    } else if to.starts_with('/') && !to.starts_with("//") {
        format!("https://{}{to}", host(from))
    } else {
        return None;
    };
    listed(host(&to)).then_some(to)
}

/// The host of an https address.
fn host(url: &str) -> &str {
    let rest = url.strip_prefix("https://").unwrap_or(url);
    rest.split(['/', '?', '#']).next().unwrap_or(rest)
}

pub fn log_file() -> PathBuf {
    crate::store::log_dir().join("network.log")
}

/// Adds a line to the network log: the time (UTC), the address and why it was asked. A request
/// whose line cannot be written is not sent.
fn note(url: &str, why: &str) -> Result<(), Fail> {
    let line = format!("{}\tGET {url}\t{why}\n", utc(SystemTime::now()));
    let f = log_file();
    let r = f.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|_| std::fs::OpenOptions::new().create(true).append(true).open(&f)).and_then(|mut f| f.write_all(line.as_bytes()));
    if let Err(e) = r {
        log::warn!("not sent, as the network log cannot be written ({e}): {url}");
        return Err(Fail::Bad(format!("the network log cannot be written, so nothing was sent: {e}")));
    }
    SENT.fetch_add(1, Ordering::SeqCst);
    log::info!("network: GET {url} ({why})");
    Ok(())
}

/// `2026-10-08 20:41:07Z`.
fn utc(t: SystemTime) -> String {
    let s = t.duration_since(SystemTime::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let (days, rest) = ((s / 86_400) as i64, s % 86_400);
    // Days since 1970 to a date (Howard Hinnant's civil_from_days).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02} {:02}:{:02}:{:02}Z", rest / 3600, rest % 3600 / 60, rest % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titles_split_into_title_and_author() {
        assert_eq!(split("Simmons, Dan - The Ninth of Av (Ilium) - 2001"), ("The Ninth of Av".into(), "Simmons, Dan".into()));
        assert_eq!(split("Allegiant (Veronica Roth)"), ("Allegiant".into(), "Veronica Roth".into()));
        assert_eq!(split("Asimov 3. Second Foundation"), ("Second Foundation".into(), "Asimov 3.".into()));
        assert_eq!(split("The Martian"), ("The Martian".into(), String::new()));
    }

    #[test]
    fn queries_drop_years_ids_and_editions() {
        assert_eq!(query("Networking, 3rd Edition (2019) 123456"), "Networking");
        assert_eq!(query("Пикник на обочине"), "Пикник на обочине");
    }

    #[test]
    fn search_words_hold_the_title_and_author_only() {
        assert_eq!(words_for("Simmons, Dan - The Ninth of Av (Ilium) - 2001"), "The Ninth of Av Simmons Dan");
        // The names the old lookup read wrongly: an id and a format left from "x.fb2.mobi",
        // and a title after its author.
        assert_eq!(words_for("Kori The-Expanse 2.0 Caliban-s-War.524185.fb2"), "Kori The Expanse 2 0 Caliban s War");
        assert_eq!(words_for("Python Crash Course A Hands-On, Project-B - Eric Matthes"), "Eric Matthes Python Crash Course A Hands On Project B");
        assert_eq!(words_for("Крайтон М. - Опасный пациент"), "Опасный пациент Крайтон М");
    }

    #[test]
    fn each_source_gives_its_covers_to_choose_from() {
        let ol = serde_json::json!({"docs": [
            {"title": "Caliban's war", "author_name": ["James S. A. Corey"], "first_publish_year": 2012, "cover_i": 7314238},
            {"title": "No picture", "author_name": ["X"]},
        ]});
        assert_eq!(open_library(&ol), [Found {
            url: "https://covers.openlibrary.org/b/id/7314238-L.jpg?default=false".into(),
            preview: "https://covers.openlibrary.org/b/id/7314238-M.jpg?default=false".into(),
            from: OPEN_LIBRARY.into(), title: "Caliban's war".into(), by: "James S. A. Corey".into(), year: Some(2012),
        }]);
        let fl = serde_json::json!({"matches": [
            {"name": "Caliban's War", "rusname": "Война Калибана", "year": 2012, "pic_edition_id": 0, "pic_edition_id_auto": 509213, "all_autor_name": "Daniel Abraham, Ty Franck", "all_autor_rusname": ""},
            {"name": "", "rusname": "Рецензия", "year": 2013, "pic_edition_id": 0, "pic_edition_id_auto": 0},
            {"name": "Again", "rusname": "", "pic_edition_id": 509213},
        ]});
        let f = fantlab(&fl);
        assert_eq!(f.len(), 1, "no picture, or one shown already: not offered");
        assert_eq!((f[0].title.as_str(), f[0].by.as_str(), f[0].url.as_str()), ("Война Калибана", "Daniel Abraham, Ty Franck", "https://fantlab.ru/images/editions/big/509213"));
    }

    #[test]
    fn only_a_cover_address_is_fetched() {
        let before = SENT.load(Ordering::SeqCst);
        assert!(picture("https://example.com/x.jpg", "t", None).is_err());
        assert!(picture("http://covers.openlibrary.org/b/id/1-L.jpg", "t", None).is_err());
        assert!(picture("https://fantlab.ru.example.com/x", "t", None).is_err());
        // A small picture of a search the picker has left is not fetched.
        let r = next_round();
        next_round();
        assert_eq!(picture("https://covers.openlibrary.org/b/id/1-M.jpg?default=false", "t", Some(r)).unwrap_err(), "no longer wanted");
        assert!(search("  ").is_empty());
        assert_eq!(SENT.load(Ordering::SeqCst), before, "nothing was sent");
    }

    /// Searches both sources for the words in FVH_WORDS and prints what each found; it makes real
    /// requests, written to the network log in FVH_DATA_DIR.
    #[test]
    #[ignore]
    fn live() {
        for r in search(&std::env::var("FVH_WORDS").unwrap()) {
            println!("{}: {:?}", r.from, r.error);
            for f in r.found {
                println!("  {} | {} | {:?} | {}", f.title, f.by, f.year, f.url);
            }
        }
    }

    #[test]
    fn a_redirect_goes_only_to_https() {
        let from = "https://covers.openlibrary.org/b/id/1-L.jpg?default=false";
        assert_eq!(redirect(from, "https://archive.org/download/x.jpg").as_deref(), Some("https://archive.org/download/x.jpg"));
        assert_eq!(redirect(from, "/b/id/2-L.jpg").as_deref(), Some("https://covers.openlibrary.org/b/id/2-L.jpg"));
        assert_eq!(redirect(from, "http://archive.org/x.jpg"), None);
        assert_eq!(redirect(from, "//archive.org/x.jpg"), None);
        assert_eq!(redirect(from, "https://ia800404.us.archive.org/view_archive.php?x").as_deref(), Some("https://ia800404.us.archive.org/view_archive.php?x"));
        assert_eq!(redirect(from, "https://evil.example/x.jpg"), None, "a host not listed");
        assert_eq!(redirect(from, "https://archive.org.evil.example/x"), None);
        assert_eq!(redirect(from, "https://a.b.us.archive.org/x"), None);
        assert_eq!(host(from), "covers.openlibrary.org");
    }

    #[test]
    fn times_are_written_in_utc() {
        assert_eq!(utc(SystemTime::UNIX_EPOCH + Duration::from_secs(1_791_491_607)), "2026-10-08 20:33:27Z");
    }
}
