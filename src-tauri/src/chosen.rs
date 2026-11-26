//! Covers the person chose (`decisions/2026/10/22/covers-are-chosen-never-fetched-by-themselves.md`): a picture of their own (a file, or one pasted) or one
//! picked from the web, for a Work or a folder tile. Each is a JPEG in StoredData's `Covers/`,
//! listed in `covers.json` by id, so it survives a restart and the cache being deleted. A picture
//! no cover uses any more is deleted when the list is next read, so Undo can still bring it back.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

/// The `from` of a picture of one's own.
pub const OWN: &str = "own";
/// A chosen picture is kept at most this many pixels on its long side.
const KEEP: u32 = 1200;
/// A picture smaller than this on both sides is no cover.
const LEAST: u32 = 50;

/// A chosen cover: its picture in `Covers/`, and where it came from.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Chosen {
    pub file: String,
    /// `own`, or the site it was picked from (`web::OPEN_LIBRARY`, `web::FANTLAB`).
    pub from: String,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Covers {
    v: u32,
    /// By Work id, or `dir:` and a folder's place for a folder tile.
    by: BTreeMap<String, Chosen>,
    /// The data folder.
    #[serde(skip)]
    data: PathBuf,
    /// Why the list may not be saved: it could not be read, or a newer FilesViewHub wrote it.
    /// Saving would lose what it holds.
    #[serde(skip)]
    frozen: Option<String>,
}

/// A picture the app kept itself: `<16 hex digits>.jpg`, never a path.
fn own_file(f: &str) -> bool {
    f.len() == 20 && f.ends_with(".jpg") && f[..16].bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

/// Where a chosen cover may come from.
fn known_from(f: &str) -> bool {
    [OWN, crate::web::OPEN_LIBRARY, crate::web::FANTLAB].contains(&f)
}

pub fn list_file(data: &Path) -> PathBuf {
    data.join("covers.json")
}

pub fn pictures_dir(data: &Path) -> PathBuf {
    data.join("Covers")
}

/// A picture as a cover is kept: turned as the camera held it, at most `KEEP` pixels on its long
/// side, as JPEG. Fails on anything that is not a picture.
pub fn as_kept(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let img = crate::thumbs::picture_bytes(bytes).map_err(|_| "This is not a picture the app can read.".to_string())?;
    if img.width() < LEAST && img.height() < LEAST {
        return Err("This picture is too small for a cover.".into());
    }
    let img = if img.width() > KEEP || img.height() > KEEP { img.resize(KEEP, KEEP, image::imageops::FilterType::CatmullRom) } else { img };
    let mut buf = vec![];
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 90).encode_image(&crate::thumbs::flatten(&img)).map_err(|e| e.to_string())?;
    Ok(buf)
}

/// `as_kept`, where a decoder that panics on bad bytes fails only this call.
pub fn checked(bytes: &[u8]) -> Result<Vec<u8>, String> {
    std::panic::catch_unwind(|| as_kept(bytes)).map_err(|_| "This is not a picture the app can read.".to_string())?
}

/// Keeps a picture in a data folder's `Covers/` as it is kept (`as_kept`), and gives the cover
/// for it. Called with no lock held: a large picture takes seconds, and a decoder that panics
/// on bad bytes fails only this call.
pub fn store(data: &Path, bytes: &[u8], from: &str) -> Result<Chosen, String> {
    let buf = checked(bytes)?;
    let c = Chosen { file: format!("{}.jpg", crate::thumbs::fnv_bytes(&buf)), from: from.into() };
    let dir = pictures_dir(data);
    let f = dir.join(&c.file);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let tmp = f.with_extension("tmp");
    std::fs::write(&tmp, &buf).and_then(|_| std::fs::rename(&tmp, &f)).map_err(|e| e.to_string())?;
    Ok(c)
}

impl Covers {
    /// Reads the list in a data folder, and deletes the pictures it no longer uses. Pictures are
    /// deleted only after the list was read as it is: one that could not be read or that a newer
    /// FilesViewHub wrote keeps them all, and so does any start while a damaged list set aside
    /// by `store::load` (`covers.json.damaged`) is there, as it may name them.
    pub fn load(data: &Path) -> Covers {
        let file = list_file(data);
        let mut c: Covers = crate::store::load(&file);
        let read = c.v == 1 && !file.with_extension("json.damaged").exists();
        if c.v > 1 {
            c.frozen = Some("These covers were saved by a newer FilesViewHub.".into());
        } else if c.v == 0 && file.exists() {
            c.frozen = Some(format!("{} could not be read.", file.display()));
        }
        if c.v != 1 {
            c.by.clear();
            c.v = 1;
        }
        c.data = data.to_path_buf();
        c.by.retain(|_, x| own_file(&x.file) && known_from(&x.from));
        if read {
            let used: HashSet<&str> = c.by.values().map(|x| x.file.as_str()).collect();
            for e in std::fs::read_dir(pictures_dir(data)).into_iter().flatten().filter_map(Result::ok) {
                let n = e.file_name().to_string_lossy().to_string();
                // Only the app's own `<key>.jpg` files, never anything else put there.
                if own_file(&n) && !used.contains(n.as_str()) {
                    let _ = std::fs::remove_file(e.path());
                }
            }
        }
        c
    }

    pub fn get(&self, id: &str) -> Option<&Chosen> {
        self.by.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &Chosen)> {
        self.by.iter()
    }

    /// The picture of a chosen cover.
    pub fn path(&self, c: &Chosen) -> PathBuf {
        pictures_dir(&self.data).join(&c.file)
    }

    /// Keeps a picture as the cover of these ids (`store`, then `set`), the lock held: only for
    /// pictures already small, as at start.
    pub fn keep(&mut self, ids: &[String], bytes: &[u8], from: &str) -> Result<Chosen, String> {
        let c = store(&self.data, bytes, from)?;
        self.set(ids, c.clone())?;
        Ok(c)
    }

    /// Sets a cover kept in `Covers/` for these ids. Fails on anything but a picture the app kept
    /// itself, from a known source.
    pub fn set(&mut self, ids: &[String], c: Chosen) -> Result<(), String> {
        if !own_file(&c.file) || !known_from(&c.from) {
            return Err("Not a cover the app kept.".into());
        }
        if !self.path(&c).is_file() {
            return Err("That picture is no longer kept.".into());
        }
        self.change(|by| {
            for id in ids {
                by.insert(id.clone(), c.clone());
            }
        })
    }

    /// Removes the cover of these ids: each shows the cover its files give again. The pictures
    /// stay until the next start, for Undo. Gives each id's cover removed.
    pub fn remove(&mut self, ids: &[String]) -> Result<BTreeMap<String, Chosen>, String> {
        let gone: BTreeMap<String, Chosen> = ids.iter().filter_map(|id| Some((id.clone(), self.by.get(id)?.clone()))).collect();
        if !gone.is_empty() {
            self.change(|by| by.retain(|id, _| !gone.contains_key(id)))?;
        }
        Ok(gone)
    }

    /// Undo of Remove: puts back each id's cover, except on an id given another cover since.
    /// All or nothing: every cover is checked before the list is saved once.
    pub fn restore(&mut self, gone: BTreeMap<String, Chosen>) -> Result<(), String> {
        let free: Vec<(String, Chosen)> = gone.into_iter().filter(|(id, _)| !self.by.contains_key(id)).collect();
        for (_, c) in &free {
            if !own_file(&c.file) || !known_from(&c.from) {
                return Err("Not a cover the app kept.".into());
            }
            if !self.path(c).is_file() {
                return Err("That picture is no longer kept.".into());
            }
        }
        self.change(|by| by.extend(free))
    }

    /// Why no cover can be set now, if one cannot: checked before a picture is stored, so a
    /// refused one leaves no file behind.
    pub fn writable(&self) -> Result<(), String> {
        self.frozen.clone().map_or(Ok(()), Err)
    }

    /// True when any of these ids has a chosen cover.
    pub fn any(&self, ids: &[String]) -> bool {
        ids.iter().any(|id| self.by.contains_key(id))
    }

    /// Changes the list and saves it; the list in memory changes only once it is saved.
    fn change(&mut self, f: impl FnOnce(&mut BTreeMap<String, Chosen>)) -> Result<(), String> {
        if let Some(why) = &self.frozen {
            return Err(why.clone());
        }
        let mut next = self.clone();
        f(&mut next.by);
        crate::store::save(&list_file(&self.data), &next)?;
        self.by = next.by;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(w: u32, h: u32) -> Vec<u8> {
        let img = image::RgbImage::from_fn(w, h, |x, y| image::Rgb([(x % 255) as u8, (y % 255) as u8, 90]));
        let mut out = std::io::Cursor::new(vec![]);
        image::DynamicImage::ImageRgb8(img).write_to(&mut out, image::ImageFormat::Png).unwrap();
        out.into_inner()
    }

    fn dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("fvh-chosen-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn a_picture_of_ones_own_is_kept_and_removed() {
        let d = dir("own");
        let ids = vec!["w".to_string(), "w copy".to_string()];
        let mut c = Covers::load(&d);
        let kept = c.keep(&ids, &png(2000, 1000), OWN).unwrap();
        assert_eq!(kept.from, OWN);
        let saved = image::open(c.path(&kept)).unwrap();
        assert_eq!((saved.width(), saved.height()), (KEEP, KEEP / 2), "kept at most 1200 px");
        // It survives a restart, for the Work and its copy.
        let c2 = Covers::load(&d);
        assert_eq!(c2.get("w"), Some(&kept));
        assert_eq!(c2.get("w copy"), Some(&kept));
        // Removed: the Work goes back; the picture stays for Undo until the next start.
        let mut c = c2;
        let gone = c.remove(&ids).unwrap();
        assert_eq!(gone.get("w"), Some(&kept));
        assert!(c.get("w").is_none());
        assert!(c.path(&kept).is_file());
        c.restore(gone).unwrap();
        assert_eq!(Covers::load(&d).get("w"), Some(&kept), "Undo brings it back");
        Covers::load(&d).remove(&ids).unwrap();
        let c = Covers::load(&d);
        assert!(c.get("w").is_none(), "removed for good after a restart");
        assert!(!c.path(&kept).exists(), "a picture no cover uses is deleted at the next start");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn what_is_not_a_picture_is_refused() {
        let d = dir("bad");
        let mut c = Covers::load(&d);
        assert!(c.keep(&["w".into()], b"not a picture", OWN).is_err());
        assert!(c.keep(&["w".into()], &png(20, 20), OWN).is_err(), "too small");
        assert!(c.get("w").is_none());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_web_cover_is_kept_with_its_site() {
        let d = dir("web");
        let mut c = Covers::load(&d);
        let kept = c.keep(&["dir:S/Books".into()], &png(300, 450), crate::web::OPEN_LIBRARY).unwrap();
        assert_eq!(Covers::load(&d).get("dir:S/Books").map(|x| x.from.as_str()), Some(crate::web::OPEN_LIBRARY));
        assert!(Covers::load(&d).path(&kept).is_file());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_list_that_cannot_be_read_keeps_every_picture() {
        for (name, text) in [("damaged", "{not json"), ("newer", r#"{"v":2,"by":{}}"#)] {
            let d = dir(name);
            let mut c = Covers::load(&d);
            let kept = c.keep(&["w".into()], &png(300, 450), OWN).unwrap();
            std::fs::write(list_file(&d), text).unwrap();
            let mut c = Covers::load(&d);
            assert!(c.path(&kept).is_file(), "{name}: a picture was deleted");
            if name == "newer" {
                assert!(c.writable().is_err() && c.keep(&["v".into()], &png(300, 450), OWN).is_err(), "a newer list was overwritten");
                assert_eq!(std::fs::read_to_string(list_file(&d)).unwrap(), text);
            } else {
                // A new cover may be chosen; the pictures the set-aside list names stay through
                // later starts while it is there.
                c.keep(&["v".into()], &png(300, 460), OWN).unwrap();
                Covers::load(&d);
                Covers::load(&d);
                assert!(c.path(&kept).is_file(), "a picture the damaged list names was deleted");
            }
            let _ = std::fs::remove_dir_all(&d);
        }
    }

    #[test]
    fn only_a_picture_the_app_kept_is_set() {
        let d = dir("paths");
        let mut c = Covers::load(&d);
        std::fs::create_dir_all(pictures_dir(&d)).unwrap();
        for file in ["../covers.json", "C:\\Windows\\x.jpg", "/etc/passwd", "0123456789ABCDEF.jpg"] {
            assert!(c.set(&["w".into()], Chosen { file: file.into(), from: OWN.into() }).is_err(), "{file}");
        }
        let kept = c.keep(&["w".into()], &png(300, 450), OWN).unwrap();
        assert!(c.set(&["v".into()], Chosen { from: "evil".into(), ..kept.clone() }).is_err());
        // An entry written into covers.json by hand is dropped as it is read.
        std::fs::write(list_file(&d), r#"{"v":1,"by":{"x":{"file":"../covers.json","from":"own"},"w":{"file":"FILE","from":"own"}}}"#.replace("FILE", &kept.file)).unwrap();
        let c = Covers::load(&d);
        assert!(c.get("x").is_none());
        assert_eq!(c.get("w"), Some(&kept));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn undo_puts_back_only_what_is_still_free() {
        let d = dir("undo");
        let mut c = Covers::load(&d);
        let a = c.keep(&["w".into()], &png(300, 450), OWN).unwrap();
        let b = c.keep(&["copy".into()], &png(310, 450), OWN).unwrap();
        let gone = c.remove(&["w".into(), "copy".into()]).unwrap();
        let newer = c.keep(&["copy".into()], &png(320, 450), OWN).unwrap();
        c.restore(gone).unwrap();
        assert_eq!(c.get("w"), Some(&a), "each id gets its own cover back");
        assert_eq!(c.get("copy"), Some(&newer), "a cover chosen since stays");
        assert_ne!(a, b);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn only_the_apps_own_pictures_are_deleted() {
        let d = dir("others");
        std::fs::create_dir_all(pictures_dir(&d)).unwrap();
        std::fs::write(pictures_dir(&d).join("mine.jpg"), b"x").unwrap();
        std::fs::write(pictures_dir(&d).join("0123456789abcdef.jpg"), b"x").unwrap();
        Covers::load(&d);
        assert!(pictures_dir(&d).join("0123456789abcdef.jpg").exists(), "no list read: nothing deleted");
        std::fs::write(list_file(&d), r#"{"v":1,"by":{}}"#).unwrap();
        Covers::load(&d);
        assert!(pictures_dir(&d).join("mine.jpg").exists());
        assert!(!pictures_dir(&d).join("0123456789abcdef.jpg").exists());
        let _ = std::fs::remove_dir_all(&d);
    }
}
