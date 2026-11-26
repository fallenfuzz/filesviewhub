//! Pictures kept inside a file: the cover of an epub, fb2, mobi or azw3 book, a comic archive's
//! first page, and the preview Office and OpenDocument files save of their first page. Read in
//! the app itself; nothing here runs a tool.

use crate::tools::Fail;
use std::fs::File;
use std::io::Read;
use std::path::Path;

fn bad(why: &str) -> Fail {
    Fail::Bad(why.into())
}

fn zip_of(path: &Path) -> Result<zip::ZipArchive<File>, Fail> {
    zip::ZipArchive::new(File::open(crate::store::named(path))?).map_err(|e| match e {
        zip::result::ZipError::Io(e) => Fail::Retry(e.to_string()),
        e => Fail::Bad(e.to_string()),
    })
}

fn read_entry(z: &mut zip::ZipArchive<File>, name: &str) -> Option<Vec<u8>> {
    read_capped(z, name, ENTRY_CAP)
}

/// The most a cover's entry may unpack to: a crafted file (a zip bomb) stops here instead of
/// taking all memory.
const ENTRY_CAP: u64 = 64 << 20;

/// An entry's bytes, or None when it is missing, unreadable or unpacks to more than `cap`.
fn read_capped(z: &mut zip::ZipArchive<File>, name: &str, cap: u64) -> Option<Vec<u8>> {
    let f = z.by_name(name).ok()?;
    let mut v = Vec::with_capacity(f.size().min(cap) as usize);
    f.take(cap + 1).read_to_end(&mut v).ok()?;
    (v.len() as u64 <= cap).then_some(v)
}

/// Sorts "page 2" before "page 10".
fn natural(s: &str) -> Vec<(u64, String)> {
    let mut out = vec![];
    let mut num = String::new();
    let mut txt = String::new();
    for c in s.chars() {
        if c.is_ascii_digit() {
            if !txt.is_empty() {
                out.push((u64::MAX, std::mem::take(&mut txt)));
            }
            num.push(c);
        } else {
            if !num.is_empty() {
                out.push((num.parse().unwrap_or(u64::MAX - 1), String::new()));
                num.clear();
            }
            txt.extend(c.to_lowercase());
        }
    }
    if !num.is_empty() {
        out.push((num.parse().unwrap_or(u64::MAX - 1), String::new()));
    }
    if !txt.is_empty() {
        out.push((u64::MAX, txt));
    }
    out
}

const PICTURE: &[&str] = &["jpg", "jpeg", "png", "webp", "gif", "bmp"];

fn is_picture(name: &str) -> bool {
    name.rsplit_once('.').is_some_and(|(_, e)| PICTURE.contains(&e.to_lowercase().as_str()))
}

/// The epub's cover: the item its metadata names, then one marked as the cover image, then one
/// with "cover" in its name, then the first picture.
pub fn epub(path: &Path) -> Result<Vec<u8>, Fail> {
    let mut z = zip_of(path)?;
    let container = read_entry(&mut z, "META-INF/container.xml").ok_or_else(|| bad("not an epub"))?;
    let container = String::from_utf8_lossy(&container);
    let doc = roxmltree::Document::parse(&container).map_err(|e| Fail::Bad(e.to_string()))?;
    let opf = doc.descendants().find(|n| n.has_tag_name("rootfile")).and_then(|n| n.attribute("full-path")).ok_or_else(|| bad("epub has no package file"))?.to_string();
    let text = read_entry(&mut z, &opf).ok_or_else(|| bad("epub package file is missing"))?;
    let text = String::from_utf8_lossy(&text);
    let opts = roxmltree::ParsingOptions { allow_dtd: true, ..Default::default() };
    let doc = roxmltree::Document::parse_with_options(&text, opts).map_err(|e| Fail::Bad(e.to_string()))?;
    let base = opf.rsplit_once('/').map(|(b, _)| format!("{b}/")).unwrap_or_default();
    let images: Vec<_> = doc
        .descendants()
        .filter(|n| n.has_tag_name("item") && n.attribute("media-type").is_some_and(|m| m.starts_with("image/")))
        .collect();
    let named = doc.descendants().find(|n| n.has_tag_name("meta") && n.attribute("name") == Some("cover")).and_then(|n| n.attribute("content"));
    let tests: [&dyn Fn(&roxmltree::Node) -> bool; 4] = [
        &|i| named.is_some() && i.attribute("id") == named,
        &|i| i.attribute("properties").is_some_and(|p| p.contains("cover-image")),
        &|i| format!("{}{}", i.attribute("id").unwrap_or(""), i.attribute("href").unwrap_or("")).to_lowercase().contains("cover"),
        &|_| true,
    ];
    let pick = tests.iter().find_map(|t| images.iter().find(|i| t(i))).ok_or_else(|| bad("epub has no pictures"))?;
    let href = pick.attribute("href").unwrap_or("");
    let href = percent_decode(href);
    for name in [format!("{base}{href}"), href.clone()] {
        if let Some(v) = read_entry(&mut z, &name) {
            return Ok(v);
        }
    }
    Err(bad("epub cover is missing"))
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = vec![];
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && let Some(v) = s.get(i + 1..i + 3).and_then(|h| u8::from_str_radix(h, 16).ok()) {
            out.push(v);
            i += 3;
            continue;
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn find(hay: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    if from >= hay.len() {
        return None;
    }
    hay[from..].windows(needle.len()).position(|w| w == needle).map(|p| p + from)
}

/// The fb2's cover: the picture its `coverpage` points to. Other pictures are illustrations.
/// Read as bytes, since fb2 files are often not UTF-8.
pub fn fb2(path: &Path) -> Result<Vec<u8>, Fail> {
    let d = std::fs::read(crate::store::named(path))?;
    let start = find(&d, b"<coverpage", 0).ok_or_else(|| bad("fb2 has no cover"))?;
    let end = find(&d, b"</coverpage>", start).unwrap_or(d.len().min(start + 2000));
    let cover = &d[start..end];
    let at = find(cover, b"href=\"#", 0).map(|p| p + 7).or_else(|| find(cover, b"href='#", 0).map(|p| p + 7)).ok_or_else(|| bad("fb2 has no cover"))?;
    let id_end = cover[at..].iter().position(|c| *c == b'"' || *c == b'\'').ok_or_else(|| bad("fb2 cover link is broken"))?;
    let id = &cover[at..at + id_end];
    let mut pos = 0;
    while let Some(b) = find(&d, b"<binary", pos) {
        let tag_end = find(&d, b">", b).ok_or_else(|| bad("fb2 is cut short"))?;
        let tag = &d[b..tag_end];
        let close = find(&d, b"</binary>", tag_end).ok_or_else(|| bad("fb2 is cut short"))?;
        let quoted = |q: u8| [b"id=".as_slice(), &[q], id, &[q]].concat();
        if find(tag, &quoted(b'"'), 0).is_some() || find(tag, &quoted(b'\''), 0).is_some() {
            let text: Vec<u8> = d[tag_end + 1..close].iter().copied().filter(|c| !c.is_ascii_whitespace()).collect();
            use base64::Engine;
            return base64::engine::general_purpose::STANDARD.decode(&text).map_err(|e| Fail::Bad(e.to_string()));
        }
        pos = close;
    }
    Err(bad("fb2 cover picture is missing"))
}

fn be32(d: &[u8], at: usize) -> Option<u32> {
    d.get(at..at + 4).map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
}

/// The cover of a mobi or azw3 book: the record its EXTH header marks (type 201), otherwise the
/// first picture record.
pub fn mobi(path: &Path) -> Result<Vec<u8>, Fail> {
    let d = std::fs::read(crate::store::named(path))?;
    let not = || bad("not a mobi file");
    let n = d.get(76..78).map(|b| u16::from_be_bytes([b[0], b[1]]) as usize).ok_or_else(not)?;
    let mut offs: Vec<usize> = (0..n).map(|i| be32(&d, 78 + 8 * i).map(|v| v as usize)).collect::<Option<_>>().ok_or_else(not)?;
    offs.push(d.len());
    let rec = |i: usize| -> &[u8] { d.get(offs[i]..offs[i + 1].max(offs[i])).unwrap_or(&[]) };
    let r0 = rec(0);
    if r0.get(16..20) != Some(b"MOBI") {
        return Err(not());
    }
    let hlen = be32(r0, 20).ok_or_else(not)? as usize;
    let first = be32(r0, 108).ok_or_else(not)? as usize;
    let mut cover = None;
    if be32(r0, 128).unwrap_or(0) & 0x40 != 0 && r0.get(16 + hlen..20 + hlen) == Some(b"EXTH") {
        let count = be32(r0, 16 + hlen + 8).unwrap_or(0);
        let mut pos = 16 + hlen + 12;
        for _ in 0..count {
            let (Some(t), Some(len)) = (be32(r0, pos), be32(r0, pos + 4)) else { break };
            if t == 201 {
                cover = be32(r0, pos + 8).map(|v| v as usize);
            }
            if len < 8 {
                break;
            }
            pos += len as usize;
        }
    }
    let magic: [&[u8]; 3] = [b"\xff\xd8", b"\x89PNG", b"GIF8"];
    let start = if first < n { first } else { 1 };
    let mut order: Vec<usize> = cover.filter(|c| first < n && first + c < n).map(|c| vec![first + c]).unwrap_or_default();
    order.extend(start..n);
    for i in order {
        let r = rec(i);
        if magic.iter().any(|m| r.starts_with(m)) {
            if r.len() > 4000 {
                return Ok(r.to_vec());
            }
            if cover.is_none() {
                break; // without a marked cover only the first picture can be one
            }
        }
    }
    Err(bad("no cover picture in the book"))
}

/// A comic archive's first page, by page order.
pub fn cbz(path: &Path) -> Result<Vec<u8>, Fail> {
    let mut z = zip_of(path)?;
    let mut pages: Vec<String> = z
        .file_names()
        // A name zip cannot decode is no page it could open by name either.
        .filter_map(Result::ok)
        .filter(|n| is_picture(n) && !n.rsplit('/').next().unwrap_or("").starts_with('.'))
        .map(String::from)
        .collect();
    pages.sort_by_key(|n| natural(n));
    let first = pages.first().ok_or_else(|| bad("no pictures in the archive"))?.clone();
    read_entry(&mut z, &first).ok_or_else(|| bad("the archive's first picture cannot be read"))
}

/// The preview of the first page that Word, PowerPoint and LibreOffice save inside the file,
/// when they did.
pub fn office(path: &Path) -> Result<Vec<u8>, Fail> {
    let mut z = zip_of(path)?;
    for name in ["docProps/thumbnail.jpeg", "docProps/thumbnail.jpg", "docProps/thumbnail.png", "Thumbnails/thumbnail.png"] {
        if let Some(v) = read_entry(&mut z, name) {
            return Ok(v);
        }
    }
    Err(bad("no preview saved in the file"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pages_sort_by_number() {
        let mut v = vec!["p10.jpg", "p2.jpg", "p1.jpg"];
        v.sort_by_key(|n| natural(n));
        assert_eq!(v, ["p1.jpg", "p2.jpg", "p10.jpg"]);
    }

    #[test]
    fn an_entry_that_unpacks_past_the_cap_is_not_read() {
        use std::io::Write;
        let path = std::env::temp_dir().join(format!("fvh-cap-{}.zip", std::process::id()));
        let mut w = zip::ZipWriter::new(File::create(&path).unwrap());
        w.start_file("cover.jpg", zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated)).unwrap();
        w.write_all(&[0u8; 100]).unwrap();
        w.finish().unwrap();
        let mut z = zip::ZipArchive::new(File::open(&path).unwrap()).unwrap();
        assert_eq!(read_capped(&mut z, "cover.jpg", 100).map(|v| v.len()), Some(100));
        assert_eq!(read_capped(&mut z, "cover.jpg", 99), None, "one byte over the cap");
        assert_eq!(read_capped(&mut z, "missing.jpg", 100), None);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn hrefs_decode() {
        assert_eq!(percent_decode("Images/cover%20art.jpg"), "Images/cover art.jpg");
        assert_eq!(percent_decode("a%2"), "a%2");
    }
}
