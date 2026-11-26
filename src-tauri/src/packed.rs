//! The catalog in its compact form: the catalog file (version 3) and the parts of a
//! Section's Works and unlisted files sent to the page. A field the reader can make again, from
//! the Section's folder and the Work's or file's other fields, is left out only when the value
//! made again equals it byte for byte; any other value is written, so ids, Tick keys and paths
//! come back exact. A Work's folder, folder titles and media are written only when they differ
//! from the Work before it, an item's group when it differs from the item before it, and an
//! unlisted file's folder titles and reason when they differ from the file before it.
//! `src/lib/packed.ts` makes them again on the page by the same rules;
//! `scripts/packed.fixture.json` holds both forms of a sample, and both sides test against it.

use crate::scan::{Catalog, Item, Link, Reason, Role, Section, Unlisted, Work};
use crate::store::Kind;
use serde::de::{self, MapAccess, Visitor};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::borrow::Cow;

/// The version of the catalog file written: 1 held every field of every Work, 2 every field of
/// every unlisted file.
pub const VERSION: u32 = 3;
/// Works in one part sent to the page: about 6 MB with real-length names, so no reply nears
/// V8's longest string (about 536 MB) at any size of Section.
pub const PART: usize = 50_000;
/// Items in one part at most, past its first Work: a Photos Section's Events can hold many.
pub const PART_ITEMS: usize = 200_000;
/// Unlisted files in one part sent to the page: about 8 MB with long names and paths.
pub const UNLISTED_PART: usize = 50_000;
/// Bytes of names, paths and folder titles in one part at most, past its first file: long
/// folders written on every file can make a file take kilobytes.
pub const UNLISTED_PART_BYTES: usize = 32 << 20;

/// One Work, compact: see the module comment. `d` is the folder its items' paths start from, from
/// the Section's folder with `/`: a folder Work's own folder, the folder a file Work was found in,
/// or an Event's Photos Section or folder.
#[derive(Serialize, Deserialize, Default, Debug, PartialEq)]
pub struct CWork<'a> {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    d: Option<Cow<'a, str>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    p: Option<Cow<'a, str>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    id: Option<Cow<'a, str>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    t: Option<Cow<'a, str>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    f: Option<Cow<'a, [String]>>,
    #[serde(default, rename = "F", skip_serializing_if = "std::ops::Not::not")]
    folder_work: bool,
    i: Vec<CItem<'a>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    md: Option<Cow<'a, [String]>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    s: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    a: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    c: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    y: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    tk: Option<(i64, i64)>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    ev: bool,
    #[serde(default, rename = "do", skip_serializing_if = "std::ops::Not::not")]
    day_only: bool,
}

/// One item, compact: `r` is its path from the Work's `d`.
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct CItem<'a> {
    r: Cow<'a, str>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    n: Option<Cow<'a, str>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    p: Option<Cow<'a, str>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    e: Option<Cow<'a, str>>,
    s: u64,
    m: u64,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    c: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    k: Option<Cow<'a, str>>,
    /// The key is the path without its extension (a course's lesson).
    #[serde(default, rename = "K", skip_serializing_if = "std::ops::Not::not")]
    lesson: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    t: Option<Cow<'a, str>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    g: Option<Cow<'a, str>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    o: Option<Role>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    l: Option<Cow<'a, str>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    at: Option<i64>,
}

/// One unlisted file, compact: `r` is its path from the Section's folder.
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct CUnlisted<'a> {
    r: Cow<'a, str>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    n: Option<Cow<'a, str>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    p: Option<Cow<'a, str>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    f: Option<Cow<'a, [String]>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    o: Option<Reason>,
    s: u64,
    m: u64,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    c: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    w: Option<Cow<'a, str>>,
}

/// A Section with every field but its Works, which are `w`, and its unlisted files, which are
/// `u`: both packed in the file, and how many of each there are in what the page is sent first.
/// A file of version 2 holds the unlisted files whole, in `unlisted`, and no `u`. The other
/// fields and their attributes are Section's.
#[derive(Serialize, Deserialize)]
pub struct Head<'a, W, U> {
    id: Cow<'a, str>,
    name: Cow<'a, str>,
    path: Cow<'a, str>,
    drive: Cow<'a, str>,
    kind: Kind,
    guessed: bool,
    online: bool,
    #[serde(default)]
    gone: bool,
    files: usize,
    scanned_at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    unlisted: Option<Cow<'a, [Unlisted]>>,
    #[serde(default, skip_serializing_if = "<[String]>::is_empty")]
    empty: Cow<'a, [String]>,
    #[serde(default)]
    whole_tree: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    split_from: Option<Cow<'a, str>>,
    #[serde(default, skip_serializing_if = "<[Link]>::is_empty")]
    links: Cow<'a, [Link]>,
    w: W,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    u: Option<U>,
}

/// Section `s` with `w` for its Works and `u` for its unlisted files; with no `u` they are written
/// whole, as in a file of version 2.
fn head<W, U>(s: &Section, w: W, u: Option<U>) -> Head<'_, W, U> {
    Head {
        id: Cow::Borrowed(&s.id),
        name: Cow::Borrowed(&s.name),
        path: Cow::Borrowed(&s.path),
        drive: Cow::Borrowed(&s.drive),
        kind: s.kind,
        guessed: s.guessed,
        online: s.online,
        gone: s.gone,
        files: s.files,
        scanned_at: s.scanned_at,
        unlisted: u.is_none().then_some(Cow::Borrowed(&s.unlisted[..])),
        empty: Cow::Borrowed(&s.empty),
        whole_tree: s.whole_tree,
        split_from: s.split_from.as_deref().map(Cow::Borrowed),
        links: Cow::Borrowed(&s.links),
        w,
        u,
    }
}

/// What the page is sent first: every Section with how many Works and unlisted files it has, and
/// the number of the catalog they belong to, which each part asked for must name.
#[derive(Serialize)]
pub struct Heads<'a> {
    v: u32,
    stamp: u64,
    sections: Vec<Head<'a, usize, usize>>,
}

pub fn heads(cat: &Catalog, stamp: u64) -> Heads<'_> {
    Heads { v: VERSION, stamp, sections: cat.sections.iter().map(|s| head(s, s.works.len(), Some(s.unlisted.len()))).collect() }
}

/// The part of Section `id`'s unlisted files from `from`, packed; None when there is no such Section.
pub fn unlisted_part<'a>(cat: &'a Catalog, id: &str, from: usize) -> Option<Vec<CUnlisted<'a>>> {
    let s = cat.sections.iter().find(|s| s.id == id)?;
    Some(unlisted_part_of(s, from, UNLISTED_PART, UNLISTED_PART_BYTES))
}

/// At most `n` unlisted files of `s` from `from`, packed on their own, ending before the file that
/// would take the text written past `bytes`.
fn unlisted_part_of(s: &Section, from: usize, n: usize, bytes: usize) -> Vec<CUnlisted<'_>> {
    let all = &s.unlisted[from.min(s.unlisted.len())..from.saturating_add(n).min(s.unlisted.len())];
    let mut p = Loose::new(s);
    let mut held = 0;
    let mut out = vec![];
    for u in all {
        let c = p.pack(u);
        held += c.r.len() + c.n.as_ref().map_or(0, |x| x.len()) + c.p.as_ref().map_or(0, |x| x.len()) + c.f.as_ref().map_or(0, |f| f.iter().map(String::len).sum())
            + c.w.as_ref().map_or(0, |x| x.len());
        if !out.is_empty() && held > bytes {
            break;
        }
        out.push(c);
    }
    out
}

/// The part of Section `id`'s Works from `from`, packed; None when there is no such Section.
pub fn part<'a>(cat: &'a Catalog, id: &str, from: usize) -> Option<Vec<CWork<'a>>> {
    part_of(cat.sections.iter().find(|s| s.id == id)?, from, PART, PART_ITEMS)
}

/// At most `n` Works from `from`, ending before the Work that would take the items past `items`.
fn part_of(s: &Section, from: usize, n: usize, items: usize) -> Option<Vec<CWork<'_>>> {
    let works = s.works.get(from.min(s.works.len())..from.saturating_add(n).min(s.works.len()))?;
    let mut p = Packer::new(s);
    let mut held = 0;
    let mut out = vec![];
    for w in works {
        held += w.items.len();
        if !out.is_empty() && held > items {
            break;
        }
        out.push(p.work(w));
    }
    Some(out)
}

// The rules both sides make fields again by. Names and paths are searched for `.`, `/` and the
// separator only, so byte and UTF-16 positions give the same pieces.

/// The separator of paths in a Section's folder: `/` for one that starts with it (macOS, Linux), else `\`.
fn sep_of(sp: &str) -> char {
    if sp.starts_with('/') { '/' } else { '\\' }
}

/// `a` and path `b` (with `/`) joined as the system writes a path.
fn join_path(a: &str, b: &str, sep: char) -> String {
    let b = if sep == '/' { Cow::Borrowed(b) } else { Cow::Owned(b.replace('/', "\\")) };
    if a.ends_with(sep) { format!("{a}{b}") } else { format!("{a}{sep}{b}") }
}

/// The path of folder `d` (from the Section's folder `sp`).
fn base(sp: &str, d: &str, sep: char) -> String {
    if d.is_empty() { sp.to_string() } else { join_path(sp, d, sep) }
}

fn last(r: &str) -> &str {
    r.rsplit_once('/').map_or(r, |(_, n)| n)
}

/// A name without its extension; a name that starts with its only `.` keeps it.
fn stem(n: &str) -> &str {
    match n.rfind('.') {
        Some(i) if i > 0 => &n[..i],
        _ => n,
    }
}

/// The extension as the walk writes it, for a name whose extension is ASCII; None otherwise, so
/// the rule never rests on how two systems lower-case other letters.
fn ext_of(n: &str) -> Option<String> {
    let e = match n.rfind('.') {
        Some(i) => &n[i + 1..],
        None => "",
    };
    e.is_ascii().then(|| e.to_ascii_lowercase())
}

/// A lesson's key: its path without the extension (scan.rs `lesson_key`).
fn lesson_key(r: &str) -> &str {
    match r.rsplit_once('.') {
        Some((a, _)) if !a.ends_with('/') && !a.is_empty() && !r[a.len()..].contains('/') => a,
        _ => r,
    }
}

fn join_rel(d: &str, n: &str) -> String {
    if d.is_empty() { n.to_string() } else { format!("{d}/{n}") }
}

/// A Work's id made again: its folder's key, or its first file's without the extension.
fn work_id(sid: &str, d: &str, folder: bool, first: Option<&str>) -> Option<String> {
    if folder {
        Some(crate::store::key_in(sid, &format!("{d}/")))
    } else {
        first.map(|n| crate::store::key_in(sid, &join_rel(d, stem(n))))
    }
}

fn work_title<'a>(d: &'a str, folder: bool, first: Option<&'a str>) -> &'a str {
    if folder { last(d) } else { first.map_or("", stem) }
}

/// The folder `d` the item paths of `w` start from, as `d` is written; None when they do not
/// start from a folder in the Section.
fn folder_of(w: &Work, sp: &str, sep: char) -> Option<String> {
    let full = match w.items.first() {
        Some(i) => {
            let r = if sep == '/' { Cow::Borrowed(i.rel.as_str()) } else { Cow::Owned(i.rel.replace('/', "\\")) };
            let up = i.path.strip_suffix(r.as_ref())?;
            if up == sp { return Some(String::new()) }
            up.strip_suffix(sep)?.to_string()
        }
        None => w.path.clone(),
    };
    if full == sp {
        return Some(String::new());
    }
    let rest = full.strip_prefix(sp)?;
    let rest = if sp.ends_with(sep) { rest } else { rest.strip_prefix(sep)? };
    (!rest.is_empty()).then(|| if sep == '/' { rest.to_string() } else { rest.replace('\\', "/") })
}

/// Packs the Works of one Section in order, each against the Work before it.
struct Packer<'a> {
    sp: &'a str,
    sid: &'a str,
    sep: char,
    d: String,
    f: &'a [String],
    md: &'a [String],
}

impl<'a> Packer<'a> {
    fn new(s: &'a Section) -> Self {
        Packer { sp: &s.path, sid: &s.id, sep: sep_of(&s.path), d: String::new(), f: &[], md: &[] }
    }

    fn work(&mut self, w: &'a Work) -> CWork<'a> {
        let mut c = CWork { folder_work: w.is_folder, ev: w.event, day_only: w.day_only, y: w.year, tk: w.taken, ..Default::default() };
        if let Some(d) = folder_of(w, self.sp, self.sep) && d != self.d {
            c.d = Some(Cow::Owned(d.clone()));
            self.d = d;
        }
        let at = base(self.sp, &self.d, self.sep);
        let mut g = "";
        for i in &w.items {
            c.i.push(self.item(i, &at, g));
            g = &i.group;
        }
        let first = w.items.first().map(|i| i.name.as_str());
        let path = if w.is_folder { at } else { w.items.first().map(|i| i.path.clone()).unwrap_or_default() };
        if w.path != path {
            c.p = Some(Cow::Borrowed(&w.path));
        }
        if work_id(self.sid, &self.d, w.is_folder, first).as_deref() != Some(w.id.as_str()) {
            c.id = Some(Cow::Borrowed(&w.id));
        }
        if work_title(&self.d, w.is_folder, first) != w.title {
            c.t = Some(Cow::Borrowed(&w.title));
        }
        if w.folder != self.f {
            c.f = Some(Cow::Borrowed(&w.folder));
            self.f = &w.folder;
        }
        if w.media != self.md {
            c.md = Some(Cow::Borrowed(&w.media));
            self.md = &w.media;
        }
        if w.size != w.items.iter().map(|i| i.size).sum::<u64>() {
            c.s = Some(w.size);
        }
        if w.added != w.items.iter().map(|i| i.mtime).max().unwrap_or(0) {
            c.a = Some(w.added);
        }
        if w.cloud != w.items.iter().filter(|i| i.cloud).count() {
            c.c = Some(w.cloud);
        }
        c
    }

    fn item(&self, i: &'a Item, at: &str, group: &str) -> CItem<'a> {
        let some = |keep: bool, s: &'a str| keep.then_some(Cow::Borrowed(s));
        let lesson = i.key != i.rel && i.key == lesson_key(&i.rel);
        CItem {
            r: Cow::Borrowed(&i.rel),
            n: some(i.name != last(&i.rel), &i.name),
            p: some(i.path != join_path(at, &i.rel, self.sep), &i.path),
            e: some(ext_of(&i.name).as_deref() != Some(i.ext.as_str()), &i.ext),
            s: i.size,
            m: i.mtime,
            c: i.cloud,
            k: some(i.key != i.rel && !lesson, &i.key),
            lesson,
            t: some(i.title != stem(&i.name), &i.title),
            g: some(i.group != group, &i.group),
            o: (i.role != Role::Main).then_some(i.role),
            l: some(!i.lang.is_empty(), &i.lang),
            at: i.at,
        }
    }
}

/// Makes the Works of one Section again, in order, as `Packer` packed them.
pub struct Unpacker {
    sp: String,
    sid: String,
    sep: char,
    d: String,
    f: Vec<String>,
    md: Vec<String>,
}

impl Unpacker {
    pub fn new(sp: &str, sid: &str) -> Self {
        Unpacker { sp: sp.into(), sid: sid.into(), sep: sep_of(sp), d: String::new(), f: vec![], md: vec![] }
    }

    pub fn work(&mut self, c: CWork) -> Work {
        if let Some(d) = c.d {
            self.d = d.into_owned();
        }
        if let Some(f) = c.f {
            self.f = f.into_owned();
        }
        if let Some(md) = c.md {
            self.md = md.into_owned();
        }
        let at = base(&self.sp, &self.d, self.sep);
        let mut g = String::new();
        let items: Vec<Item> = c.i.into_iter().map(|i| self.item(i, &at, &mut g)).collect();
        let first = items.first().map(|i| i.name.as_str());
        let id = c.id.map(Cow::into_owned).or_else(|| work_id(&self.sid, &self.d, c.folder_work, first)).unwrap_or_default();
        let title = c.t.map_or_else(|| work_title(&self.d, c.folder_work, first).to_string(), Cow::into_owned);
        let path = c.p.map(Cow::into_owned).unwrap_or_else(|| if c.folder_work { at } else { items.first().map(|i| i.path.clone()).unwrap_or_default() });
        Work {
            id,
            title,
            path,
            folder: self.f.clone(),
            is_folder: c.folder_work,
            size: c.s.unwrap_or_else(|| items.iter().map(|i| i.size).sum()),
            added: c.a.unwrap_or_else(|| items.iter().map(|i| i.mtime).max().unwrap_or(0)),
            cloud: c.c.unwrap_or_else(|| items.iter().filter(|i| i.cloud).count()),
            items,
            media: self.md.clone(),
            year: c.y,
            taken: c.tk,
            event: c.ev,
            day_only: c.day_only,
        }
    }

    fn item(&self, c: CItem, at: &str, group: &mut String) -> Item {
        let rel = c.r.into_owned();
        let name = c.n.map_or_else(|| last(&rel).to_string(), Cow::into_owned);
        if let Some(g) = c.g {
            *group = g.into_owned();
        }
        Item {
            path: c.p.map_or_else(|| join_path(at, &rel, self.sep), Cow::into_owned),
            ext: c.e.map(Cow::into_owned).or_else(|| ext_of(&name)).unwrap_or_default(),
            key: c.k.map_or_else(|| if c.lesson { lesson_key(&rel).to_string() } else { rel.clone() }, Cow::into_owned),
            title: c.t.map_or_else(|| stem(&name).to_string(), Cow::into_owned),
            group: group.clone(),
            size: c.s,
            mtime: c.m,
            cloud: c.c,
            role: c.o.unwrap_or_default(),
            lang: c.l.map(Cow::into_owned).unwrap_or_default(),
            at: c.at,
            name,
            rel,
        }
    }
}

/// Packs and makes again the unlisted files of one Section in order, each against the one before it.
pub struct Loose<'a> {
    sp: &'a str,
    sep: char,
    f: Cow<'a, [String]>,
    o: Option<Reason>,
}

impl<'a> Loose<'a> {
    pub fn new(s: &'a Section) -> Self {
        Self::at(&s.path)
    }

    pub fn at(sp: &'a str) -> Self {
        Loose { sp, sep: sep_of(sp), f: Cow::Borrowed(&[]), o: None }
    }

    fn pack(&mut self, u: &'a Unlisted) -> CUnlisted<'a> {
        let some = |keep: bool, s: &'a str| keep.then_some(Cow::Borrowed(s));
        let f = (u.folder[..] != self.f[..]).then(|| {
            self.f = Cow::Borrowed(&u.folder);
            Cow::Borrowed(&u.folder[..])
        });
        let o = (self.o != Some(u.reason)).then(|| {
            self.o = Some(u.reason);
            u.reason
        });
        CUnlisted {
            r: Cow::Borrowed(&u.rel),
            n: some(u.name != last(&u.rel), &u.name),
            p: some(u.path != join_path(self.sp, &u.rel, self.sep), &u.path),
            f,
            o,
            s: u.size,
            m: u.mtime,
            c: u.cloud,
            w: u.work.as_deref().map(Cow::Borrowed),
        }
    }

    pub fn unpack(&mut self, c: CUnlisted) -> Unlisted {
        if let Some(f) = c.f {
            self.f = Cow::Owned(f.into_owned());
        }
        if c.o.is_some() {
            self.o = c.o;
        }
        let rel = c.r.into_owned();
        Unlisted {
            name: c.n.map_or_else(|| last(&rel).to_string(), Cow::into_owned),
            path: c.p.map_or_else(|| join_path(self.sp, &rel, self.sep), Cow::into_owned),
            folder: self.f.to_vec(),
            reason: self.o.unwrap_or(Reason::Unknown),
            size: c.s,
            mtime: c.m,
            cloud: c.c,
            work: c.w.map(Cow::into_owned),
            rel,
        }
    }
}

/// A Section's unlisted files packed in order, written as they are packed.
struct PackedLoose<'a>(&'a Section);

impl Serialize for PackedLoose<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeSeq;
        let mut p = Loose::new(self.0);
        let mut seq = s.serialize_seq(Some(self.0.unlisted.len()))?;
        for u in &self.0.unlisted {
            seq.serialize_element(&p.pack(u))?;
        }
        seq.end()
    }
}

/// A Section's Works packed in order, written as they are packed, so the file needs no copy of them.
struct Packed<'a>(&'a Section);

impl Serialize for Packed<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeSeq;
        let mut p = Packer::new(self.0);
        let mut seq = s.serialize_seq(Some(self.0.works.len()))?;
        for w in &self.0.works {
            seq.serialize_element(&p.work(w))?;
        }
        seq.end()
    }
}

/// The catalog file is written in the compact form (version 3), unless `migrate` kept it in an
/// older version: then every Section is written as that version wrote it.
impl Serialize for Catalog {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut st = s.serialize_struct("Catalog", 2)?;
        match self.v {
            1 => {
                st.serialize_field("v", &1)?;
                st.serialize_field("sections", &self.sections)?;
            }
            2 => {
                st.serialize_field("v", &2)?;
                st.serialize_field("sections", &self.sections.iter().map(|x| head(x, Packed(x), None::<PackedLoose>)).collect::<Vec<_>>())?;
            }
            _ => {
                st.serialize_field("v", &VERSION)?;
                st.serialize_field("sections", &self.sections.iter().map(|x| head(x, Packed(x), Some(PackedLoose(x)))).collect::<Vec<_>>())?;
            }
        }
        st.end()
    }
}

/// A catalog read from a file of an older version is written as the current one from now on
/// only when that form, read back, gives every Section exactly as it is; else it stays in its
/// version for good, the next reads included. Any other catalog is the current version. True
/// when it moves to the current version.
pub fn migrate(cat: &mut Catalog) -> bool {
    if !(1..VERSION).contains(&cat.v) {
        cat.v = VERSION;
        return false;
    }
    let mut packed = cat.clone();
    packed.v = VERSION;
    let same = serde_json::to_string(&packed).ok().and_then(|t| serde_json::from_str::<Catalog>(&t).ok()).is_some_and(|back| back.sections == cat.sections);
    if same {
        cat.v = VERSION;
    }
    same
}

/// Section `h` of a file of version `v`, which says where its unlisted files are: whole in
/// `unlisted` in version 2, packed in `u` from version 3. A head that holds them the other way is
/// an error, so the file is set aside rather than read without them.
fn unpack<E: de::Error>(h: Head<'static, Vec<CWork<'static>>, Vec<CUnlisted<'static>>>, v: u32) -> Result<Section, E> {
    if (v >= 3) != h.u.is_some() || (v >= 3 && h.unlisted.is_some()) {
        return Err(E::custom(format!("Section {} does not hold its unlisted files as version {v} does", h.id)));
    }
    let mut u = Unpacker::new(&h.path, &h.id);
    let works = h.w.into_iter().map(|c| u.work(c)).collect();
    let unlisted = match h.u {
        Some(cs) => {
            let mut l = Loose::at(&h.path);
            cs.into_iter().map(|c| l.unpack(c)).collect()
        }
        None => h.unlisted.map(Cow::into_owned).unwrap_or_default(),
    };
    Ok(Section {
        id: h.id.into_owned(),
        name: h.name.into_owned(),
        path: h.path.into_owned(),
        drive: h.drive.into_owned(),
        kind: h.kind,
        guessed: h.guessed,
        online: h.online,
        gone: h.gone,
        works,
        files: h.files,
        scanned_at: h.scanned_at,
        unlisted,
        empty: h.empty.into_owned(),
        whole_tree: h.whole_tree,
        split_from: h.split_from.map(Cow::into_owned),
        links: h.links.into_owned(),
    })
}

/// Reads a catalog file of either version: `v` comes first in both files, and says how its
/// Sections are written; Sections before any `v` (a catalog made as a JSON value, keys in order)
/// are read as version 1. `v` is kept, so the app can see a file of version 1 and write it again.
impl<'de> Deserialize<'de> for Catalog {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Catalog;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a catalog")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut m: A) -> Result<Catalog, A::Error> {
                let mut cat = Catalog::default();
                while let Some(k) = m.next_key::<Cow<str>>()? {
                    match k.as_ref() {
                        "v" => cat.v = m.next_value()?,
                        "sections" if cat.v >= 2 => {
                            let hs: Vec<Head<'static, Vec<CWork<'static>>, Vec<CUnlisted<'static>>>> = m.next_value()?;
                            cat.sections = hs.into_iter().map(|h| unpack(h, cat.v)).collect::<Result<_, _>>()?;
                        }
                        "sections" => cat.sections = m.next_value()?,
                        _ => {
                            m.next_value::<de::IgnoredAny>()?;
                        }
                    }
                }
                if cat.v > VERSION {
                    return Err(de::Error::custom(format!("catalog version {} is newer than this app reads", cat.v)));
                }
                Ok(cat)
            }
        }
        d.deserialize_map(V)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(rel: &str, path: &str) -> Item {
        Item { name: last(rel).into(), rel: rel.into(), path: path.into(), ext: ext_of(last(rel)).unwrap_or_default(), size: 3, mtime: 7, cloud: false,
            key: rel.into(), title: stem(last(rel)).into(), group: String::new(), role: Role::Main, lang: String::new(), at: None }
    }

    fn work(id: &str, title: &str, path: &str, folder: &[&str], is_folder: bool, items: Vec<Item>) -> Work {
        Work { id: id.into(), title: title.into(), path: path.into(), folder: folder.iter().map(|f| f.to_string()).collect(), is_folder,
            size: items.iter().map(|i| i.size).sum(), added: items.iter().map(|i| i.mtime).max().unwrap_or(0), cloud: 0, items, media: vec!["Reading".into()],
            year: None, taken: None, event: false, day_only: false }
    }

    fn section(id: &str, path: &str, works: Vec<Work>) -> Section {
        Section { id: id.into(), name: "S".into(), path: path.into(), drive: "D:\\".into(), kind: Kind::Books, guessed: false, online: true, gone: false, works,
            files: 3, scanned_at: 9, unlisted: vec![], empty: vec![], whole_tree: true, split_from: None, links: vec![] }
    }

    /// A Section as the page and the old file saw it: every field, as `Section` writes them.
    fn full(s: &Section) -> serde_json::Value {
        serde_json::to_value(s).unwrap()
    }

    fn round(cat: &Catalog) -> Catalog {
        serde_json::from_str(&serde_json::to_string(cat).unwrap()).unwrap()
    }

    /// Sections read from a sample tree by each kind, on Windows, macOS and Linux paths and as a
    /// whole drive, and Sections whose fields follow none of the rules.
    pub fn samples() -> Vec<Section> {
        let mut out = crate::scan::samples_for_packing();
        let odd = {
            let mut i1 = item("pdf/Book.pdf", "D:\\Lib\\A\\pdf\\Book.pdf");
            i1.title = "Book, cleaned".into();
            i1.key = "custom key".into();
            i1.group = "Part 1".into();
            i1.role = Role::Doc;
            i1.lang = "EN".into();
            i1.at = Some(-5);
            i1.cloud = true;
            i1.name = "Other name.pdf".into();
            i1.ext = "PDF".into();
            let mut i2 = item("pdf/Book.epub", "E:\\elsewhere\\Book.epub");
            i2.group = "Part 1".into();
            let mut i3 = item("Lesson.mp4", "D:\\Lib\\A\\Lesson.mp4");
            i3.key = "Lesson".into();
            let mut w1 = work("S/A/Book", "Book", "D:\\Lib\\A\\pdf\\Book.pdf", &["A"], false, vec![i1, i2, i3]);
            w1.size = 1;
            w1.added = 2;
            w1.cloud = 5;
            w1.year = Some(1999);
            w1.taken = Some((1, 2));
            w1.event = true;
            w1.day_only = true;
            w1.media = vec![];
            let w2 = work("Elsewhere/id", "Ж 😀", "Q:\\not\\in\\the\\Section", &[], true, vec![]);
            let w3 = work("S/.hidden", ".hidden", "D:\\Lib\\.hidden", &[], false, vec![item(".hidden", "D:\\Lib\\.hidden")]);
            let w4 = work("S/noext", "noext", "D:\\Lib\\noext", &[], false, vec![item("noext", "D:\\Lib\\noext"), item("Книга.FB2", "D:\\Lib\\Книга.FB2")]);
            let w5 = work("S/A/", "A", "D:\\Lib\\A", &[], true, vec![]);
            let w6 = work("S//", "", "D:\\Lib", &[], true, vec![]);
            let mut s = section("S", "D:\\Lib", vec![w1, w2, w3, w4, w5, w6]);
            let loose = |name: &str, rel: &str, path: &str, folder: &[&str], reason: Reason, work: Option<&str>| Unlisted { name: name.into(), rel: rel.into(),
                path: path.into(), folder: folder.iter().map(|f| f.to_string()).collect(), reason, size: 1, mtime: 2, cloud: false, work: work.map(String::from) };
            let mut cloud = loose("b.cfg", "A/b.cfg", "D:\\Lib\\A\\b.cfg", &["A"], Reason::Aside, None);
            cloud.cloud = true;
            s.unlisted = vec![
                loose("x.bin", "x.bin", "D:\\Lib\\x.bin", &[], Reason::Unknown, Some("S/A/")),
                loose("a.cfg", "A/a.cfg", "D:\\Lib\\A\\a.cfg", &["A"], Reason::Aside, None),
                cloud,
                loose("Other name.txt", "A/c.txt", "D:\\Lib\\A\\c.txt", &["A"], Reason::Unplaced, Some("S/A/Book")),
                loose("d.srt", "A/d.srt", "E:\\elsewhere\\d.srt", &["A title"], Reason::Captions, None),
                loose("Ж 😀.lrv", "Ж/Ж 😀.lrv", "D:\\Lib\\Ж\\Ж 😀.lrv", &["Ж"], Reason::Preview, None),
                loose("e.cfg", "Ж/e.cfg", "D:\\Lib\\Ж\\e.cfg", &[], Reason::Preview, None),
                loose("f", "f", "D:\\Lib\\f", &[], Reason::SetAside, None),
            ];
            s.empty = vec!["E".into()];
            s.split_from = Some("T".into());
            s.links = vec![Link { id: "S/L".into(), rel: "L".into(), folder: vec!["L".into()] }];
            s.gone = true;
            s
        };
        out.push(odd);
        let mut root = section("0A1B2C3D:", "/", vec![work("0A1B2C3D:x", "x", "/x.txt", &[], false, vec![item("x.txt", "/x.txt")])]);
        root.unlisted = ["a.cfg", "A/b.cfg", "A/B/c"].iter().map(|r| Unlisted { name: last(r).into(), rel: r.to_string(), path: format!("/{r}"), folder: r.split('/').rev().skip(1).map(String::from).collect::<Vec<_>>().into_iter().rev().collect(),
            reason: Reason::Aside, size: 1, mtime: 2, cloud: false, work: None }).collect();
        out.push(root);
        out.push(section("E", "D:\\Empty", vec![]));
        out
    }

    #[test]
    fn every_sample_comes_back_whole() {
        for s in samples() {
            let cat = Catalog { v: VERSION, sections: vec![s.clone()] };
            let back = round(&cat);
            assert_eq!(back.v, VERSION);
            assert_eq!(full(&back.sections[0]), full(&s), "{}", s.path);
        }
    }

    /// Unlisted files cut into parts at every place come back whole, each part on its own.
    #[test]
    fn unlisted_parts_put_together_are_the_unlisted_files() {
        for s in samples().into_iter().filter(|s| s.unlisted.len() > 1) {
            for n in 1..=s.unlisted.len() {
                let mut back = vec![];
                let mut from = 0;
                while from < s.unlisted.len() {
                    let part = unlisted_part_of(&s, from, n, usize::MAX);
                    from += part.len();
                    let mut l = Loose::new(&s);
                    back.extend(part.into_iter().map(|c| l.unpack(c)));
                }
                assert_eq!(back, s.unlisted, "{} in parts of {n}", s.path);
            }
        }
        // A part ends before the file that takes its text past the bytes; one file always goes.
        let s = samples().into_iter().find(|s| s.path == "D:\\Lib" && s.unlisted.len() > 5).unwrap();
        let (mut from, mut lens, mut back) = (0, vec![], vec![]);
        while from < s.unlisted.len() {
            let part = unlisted_part_of(&s, from, 100, 20);
            assert!(!part.is_empty());
            from += part.len();
            lens.push(part.len());
            let mut l = Loose::new(&s);
            back.extend(part.into_iter().map(|c| l.unpack(c)));
        }
        assert!(lens.len() > 1 && lens.iter().any(|n| *n > 1), "{lens:?}");
        assert_eq!(back, s.unlisted);
        let cat = Catalog { v: VERSION, sections: samples() };
        let s = &cat.sections[0];
        assert!(unlisted_part(&cat, &s.id, s.unlisted.len()).unwrap().is_empty());
        assert!(unlisted_part(&cat, "no such", 0).is_none());
    }

    /// A file of version 2 holds the unlisted files whole: it is read, and written again in
    /// version 3, with every Section as it was.
    #[test]
    fn a_version_2_file_is_read_and_written_again() {
        let secs = samples();
        let text = serde_json::to_string(&Catalog { v: 2, sections: secs.clone() }).unwrap();
        assert!(text.starts_with("{\"v\":2,") && text.contains("\"unlisted\":[") && !text.contains("\"u\":"));
        let mut cat: Catalog = serde_json::from_str(&text).unwrap();
        assert_eq!(cat.v, 2);
        assert_eq!(cat.sections, secs);
        assert!(migrate(&mut cat));
        let again = serde_json::to_string(&cat).unwrap();
        assert!(again.starts_with("{\"v\":3,") && !again.contains("\"unlisted\":"));
        let back: Catalog = serde_json::from_str(&again).unwrap();
        assert_eq!(back.sections, secs);
        assert!(again.len() < text.len());
    }

    /// A catalog `migrate` kept in version 2 is written as version 2 at every save.
    #[test]
    fn a_catalog_kept_in_version_2_stays_version_2() {
        let cat = Catalog { v: 2, sections: samples() };
        let text = serde_json::to_string(&cat).unwrap();
        assert!(text.starts_with("{\"v\":2,"));
        let back: Catalog = serde_json::from_str(&text).unwrap();
        assert_eq!((back.v, back.sections), (2, samples()));
    }

    #[test]
    fn the_head_sent_first_holds_every_field_of_a_section_but_its_works() {
        for s in samples() {
            let mut want = full(&s);
            want.as_object_mut().unwrap().remove("works");
            want.as_object_mut().unwrap().remove("unlisted");
            let mut got = serde_json::to_value(head(&s, s.works.len(), Some(s.unlisted.len()))).unwrap();
            assert_eq!(got.as_object_mut().unwrap().remove("w"), Some(s.works.len().into()));
            assert_eq!(got.as_object_mut().unwrap().remove("u"), Some(s.unlisted.len().into()));
            assert_eq!(got, want);
        }
    }

    #[test]
    fn parts_put_together_are_the_section() {
        let s = samples().into_iter().find(|s| s.works.len() > 3).unwrap();
        let cat = Catalog { v: VERSION, sections: vec![s.clone()] };
        let mut u = Unpacker::new(&s.path, &s.id);
        let works: Vec<Work> = part(&cat, &s.id, 0).unwrap().into_iter().map(|c| u.work(c)).collect();
        assert_eq!(serde_json::to_value(&works).unwrap(), full(&s)["works"]);
        assert!(part(&cat, &s.id, s.works.len()).unwrap().is_empty());
        assert!(part(&cat, "no such", 0).is_none());
    }

    /// A Section of many items is cut into several parts, each made whole on its own.
    #[test]
    fn a_part_ends_before_the_work_that_takes_it_past_its_items() {
        let s = samples().into_iter().find(|s| s.works.len() > 3 && s.works.iter().any(|w| w.items.len() > 1)).unwrap();
        let (mut from, mut lens) = (0, vec![]);
        let mut works = vec![];
        while from < s.works.len() {
            // Each part is packed on its own, so it is made whole on its own.
            let mut u = Unpacker::new(&s.path, &s.id);
            let part = part_of(&s, from, 100, 2).unwrap();
            assert!(!part.is_empty());
            assert!(part.len() == 1 || part.iter().map(|c| c.i.len()).sum::<usize>() <= 2);
            from += part.len();
            lens.push(part.len());
            works.extend(part.into_iter().map(|c| u.work(c)));
        }
        assert!(lens.len() > 1 && lens.iter().any(|n| *n > 1), "{lens:?}");
        assert_eq!(serde_json::to_value(&works).unwrap(), full(&s)["works"]);
    }

    /// A Work's id made again equals the one in a catalog of version 1 byte for byte, so Marks and
    /// Ticks stay with it; the file is written again in version 2.
    #[test]
    fn a_version_1_file_is_read_and_written_again() {
        #[derive(Serialize)]
        struct Old<'a> {
            v: u32,
            sections: &'a [Section],
        }
        let secs = samples();
        let text = serde_json::to_string(&Old { v: 1, sections: &secs }).unwrap();
        let mut cat: Catalog = serde_json::from_str(&text).unwrap();
        assert_eq!(cat.v, 1);
        assert!(migrate(&mut cat));
        let again = serde_json::to_string(&cat).unwrap();
        assert!(again.starts_with("{\"v\":3,"));
        let back: Catalog = serde_json::from_str(&again).unwrap();
        for (a, b) in secs.iter().zip(&back.sections) {
            assert_eq!(a.works.iter().map(|w| w.id.as_bytes()).collect::<Vec<_>>(), b.works.iter().map(|w| w.id.as_bytes()).collect::<Vec<_>>());
            assert_eq!(full(a), full(b));
        }
    }

    /// A catalog `migrate` kept in version 1 (its compact form would not give it back; no such
    /// catalog is known, so the version is set as `migrate` leaves it) is written whole as
    /// version 1 at every save, and read back as it was.
    #[test]
    fn a_catalog_kept_in_version_1_stays_version_1() {
        let mut cat = Catalog { v: 1, sections: samples() };
        let first = serde_json::to_string(&cat).unwrap();
        assert!(first.starts_with("{\"v\":1,"));
        cat.sections[0].name = "renamed".into();
        assert!(serde_json::to_string(&cat).unwrap().starts_with("{\"v\":1,"));
        let back: Catalog = serde_json::from_str(&first).unwrap();
        assert_eq!(back.v, 1);
        assert_eq!(back.sections, samples());
    }

    #[test]
    fn a_file_of_a_newer_version_or_with_compact_sections_before_its_version_is_not_read() {
        assert!(serde_json::from_str::<Catalog>(r#"{"v":4,"sections":[]}"#).is_err());
        assert!(serde_json::from_str::<Catalog>(r#"{"sections":[{"id":"S","w":[]}],"v":2}"#).is_err());
        assert_eq!(serde_json::from_str::<Catalog>(r#"{"v":2,"sections":[]}"#).unwrap().v, 2);
        assert_eq!(serde_json::from_str::<Catalog>(r#"{"v":3,"sections":[]}"#).unwrap().v, 3);
        // A head that holds its unlisted files as another version does is not read.
        let sec = |rest: &str| format!(r#"{{"id":"S","name":"S","path":"/L","drive":"/","kind":"Books","guessed":false,"online":true,"files":0,"scanned_at":0,"w":[]{rest}}}"#);
        for (v, rest, ok) in [(3, r#","u":[]"#, true), (3, "", false), (3, r#","unlisted":[]"#, false), (3, r#","unlisted":[],"u":[]"#, false), (2, r#","unlisted":[]"#, true), (2, "", true), (2, r#","u":[]"#, false)] {
            let text = format!(r#"{{"v":{v},"sections":[{}]}}"#, sec(rest));
            assert_eq!(serde_json::from_str::<Catalog>(&text).is_ok(), ok, "{text}");
        }
    }

    #[test]
    fn real_length_names_take_a_fifth_or_less() {
        let mut works = vec![];
        for a in 0..3 {
            for b in 0..100 {
                let author = format!("Author Surname {a:04}");
                let name = format!("{author} - A title of the book number {b:03} ({}).txt", 1990 + b % 30);
                let path = format!("Q:\\tmp\\Books\\{author}\\Series of books 1\\{name}");
                let mut w = work(&format!("00000000:tmp/Books/{author}/Series of books 1/{}", stem(&name)), stem(&name), &path, &[&author, "Series of books 1"], false, vec![item(&name, &path)]);
                w.year = Some(1990);
                works.push(w);
            }
        }
        let cat = Catalog { v: VERSION, sections: vec![section("00000000:tmp/Books", "Q:\\tmp\\Books", works)] };
        #[derive(Serialize)]
        struct Old<'a> {
            v: u32,
            sections: &'a [Section],
        }
        let old = serde_json::to_string(&Old { v: 1, sections: &cat.sections }).unwrap().len();
        let new = serde_json::to_string(&cat).unwrap().len();
        assert!(new * 5 <= old, "{new} of {old}");
        assert_eq!(full(&round(&cat).sections[0]), full(&cat.sections[0]));
    }

    /// The sample both sides test against: `scripts/packed.fixture.json` holds each sample
    /// packed and whole. `FVH_WRITE_FIXTURE=1` writes it again after a change of the format.
    #[test]
    fn the_fixture_for_the_page_is_current() {
        let secs = samples();
        let packed: Vec<serde_json::Value> = secs.iter().map(|s| serde_json::to_value(head(s, Packed(s), Some(PackedLoose(s)))).unwrap()).collect();
        let whole: Vec<serde_json::Value> = secs.iter().map(full).collect();
        // The first sample in parts of 3 Works, as the page is sent them.
        let parts: Vec<_> = (0..secs[0].works.len()).step_by(3).map(|from| part_of(&secs[0], from, 3, usize::MAX).unwrap()).collect();
        // The odd sample's unlisted files in parts of 3, each packed on its own.
        let odd = secs.iter().position(|s| s.unlisted.len() > 3).unwrap();
        let loose: Vec<_> = (0..secs[odd].unlisted.len()).step_by(3).map(|from| unlisted_part_of(&secs[odd], from, 3, usize::MAX)).collect();
        let text = serde_json::to_string(&serde_json::json!({ "packed": packed, "whole": whole, "parts": parts, "loose": { "of": odd, "parts": loose } })).unwrap() + "\n";
        let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../scripts/packed.fixture.json");
        if std::env::var_os("FVH_WRITE_FIXTURE").is_some() {
            std::fs::write(&file, &text).unwrap();
        }
        let have = std::fs::read_to_string(&file).unwrap_or_default().replace("\r\n", "\n");
        assert!(have == text, "packed.fixture.json is not current: run this test with FVH_WRITE_FIXTURE=1");
    }

    /// A catalog file of any version, read, packed and made again: `FVH_CATALOG` names it.
    /// Prints its size before and after, and stops at the first Work that differs.
    #[test]
    #[ignore]
    fn a_real_catalog_comes_back_whole() {
        let file = std::env::var("FVH_CATALOG").expect("set FVH_CATALOG to a catalog.json");
        let text = std::fs::read_to_string(&file).unwrap();
        let mut cat: Catalog = serde_json::from_str(&text).unwrap();
        let was = cat.v;
        assert!(migrate(&mut cat) || was == VERSION, "the current form would not give it back exactly");
        let again = serde_json::to_string(&cat).unwrap();
        println!("version {}: {} bytes; version {VERSION}: {} bytes ({:.1} times smaller)", was, text.len(), again.len(), text.len() as f64 / again.len() as f64);
        let back: Catalog = serde_json::from_str(&again).unwrap();
        let mut n = 0;
        for (a, b) in cat.sections.iter().zip(&back.sections) {
            for (x, y) in a.works.iter().zip(&b.works) {
                assert_eq!(x.id.as_bytes(), y.id.as_bytes());
                assert_eq!(serde_json::to_value(x).unwrap(), serde_json::to_value(y).unwrap(), "{}", x.id);
                n += 1;
            }
            assert_eq!(a.works.len(), b.works.len());
            let (mut ha, mut hb) = (full(a), full(b));
            ha.as_object_mut().unwrap().remove("works");
            hb.as_object_mut().unwrap().remove("works");
            assert_eq!(ha, hb);
        }
        println!("{n} Works, each the same");
        // With FVH_DUMP set to a folder, the parts as the page gets them, each Work's id and Tick
        // keys, and the unlisted files of each part whole go there, for `npm test` to make whole
        // and compare (packed.test.mjs).
        if let Ok(dir) = std::env::var("FVH_DUMP") {
            let dir = std::path::Path::new(&dir);
            let _ = std::fs::remove_dir_all(dir);
            std::fs::create_dir_all(dir).unwrap();
            let mut heads = vec![];
            let mut keys = String::new();
            for (k, s) in cat.sections.iter().enumerate() {
                heads.push(serde_json::to_value(head(s, s.works.len(), Some(s.unlisted.len()))).unwrap());
                let (mut from, mut n) = (0, 0);
                while from < s.works.len() {
                    let part = part(&cat, &s.id, from).unwrap();
                    from += part.len();
                    std::fs::write(dir.join(format!("part-{k}-{n}.json")), serde_json::to_string(&part).unwrap()).unwrap();
                    n += 1;
                }
                let (mut from, mut n) = (0, 0);
                while from < s.unlisted.len() {
                    // Each part, and the files it holds whole, for the page to make whole and compare.
                    let part = unlisted_part(&cat, &s.id, from).unwrap();
                    std::fs::write(dir.join(format!("loose-{k}-{n}.json")), serde_json::to_string(&part).unwrap()).unwrap();
                    std::fs::write(dir.join(format!("unlisted-{k}-{n}.json")), serde_json::to_string(&s.unlisted[from..from + part.len()]).unwrap()).unwrap();
                    from += part.len();
                    n += 1;
                }
                for w in &s.works {
                    keys += &w.id;
                    w.items.iter().for_each(|i| keys += &format!("\u{1}{}", i.key));
                    keys.push('\n');
                }
            }
            std::fs::write(dir.join("heads.json"), serde_json::to_string(&heads).unwrap()).unwrap();
            std::fs::write(dir.join("keys.txt"), keys).unwrap();
        }
    }
}
