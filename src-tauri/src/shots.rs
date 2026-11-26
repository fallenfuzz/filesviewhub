//! Sort in a Photos Section (`decisions/2026/10/04/sorting-ends-in-a-move-script-never-a-delete.md`, `decisions/2026/10/04/photos-take-no-marks-review-is-the-progress.md`, `decisions/2026/10/04/a-move-script-for-each-system.md`, `decisions/2026/10/08/photos-is-a-section-kind-in-the-app.md`): what Review suggests a person may not need.
//! Each picture is read once in the background and measured (a fingerprint of its look, how
//! bright and how sharp it is, its content hash), kept in the cache as `photo-shots.json` by file
//! (path, size, date). From those numbers come Copies (the same bytes, or the same picture saved
//! again smaller or at another quality), Similar shots (one moment
//! shot several times) with the Best one of each, and Junk. A clip is never measured or
//! suggested. A cloud-only file is never opened (`decisions/2026/10/05/checked-folders-on-any-drive.md`). No AI: plain arithmetic on pixels.

use crate::scan::{class_of, Catalog, Class, Item, Section, Work};
use crate::thumbs::Thumbs;
use crate::tools::Fail;
use image::{DynamicImage, GenericImageView};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

/// Tiles per side for "sharp where the detail is".
const GRID: usize = 6;
/// A lead smaller than this is a close call: the person picks (Your call).
const CLOSE: f32 = 0.08;
/// Seconds between shots of one moment: a set takes a shot this soon after its last one.
const BURST: i64 = 15;
/// Seconds a set of shots of one moment may span, from its first shot.
const MOMENT: i64 = 60;
/// Fingerprint bits a shot of one moment may differ by from the closest shot in its set.
const SCENE: u32 = 14;
/// Fingerprint bits a Similar shot may differ by from its set's first shot.
const NEAR: u32 = 5;
/// The long side pictures are measured at, so shots of one set compare fairly.
const MEASURE: u32 = 1024;
/// The long side the look is judged at: brightness, blur and the fingerprint.
const LOOK: u32 = 360;
/// Fingerprint bits a picture saved again may differ by from the one it was saved from.
const RESAVED_BITS: u32 = 3;
/// The most the 9 by 8 grey picture of a picture saved again differs, on average per pixel, from
/// the one it was saved from. On a real library, pictures saved again differ by under 0.6 and
/// different pictures by over 1.8.
const RESAVED_DIFF: u32 = 72;
/// The folder the move scripts put set-aside files in, inside the Section's folder (`decisions/2026/10/04/sorting-ends-in-a-move-script-never-a-delete.md`).
pub const NOT_NEEDED: &str = "Not needed";

/// One picture's numbers. `w` is 0 when the file could not be drawn (HEIC, a broken file): it
/// then has only its hash, for Copies.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Shot {
    /// SHA-1 of the file, for Copies.
    pub sha: String,
    /// Size in pixels, turned as the camera was held.
    pub w: u32,
    pub h: u32,
    /// EXIF names a camera or phone.
    pub cam: bool,
    /// Mean brightness, 0 to 255.
    pub lum: f32,
    /// Edge strength over the whole picture; low means blurred.
    pub edge: f32,
    /// The look as 64 bits (a difference hash): near pictures differ in few bits.
    pub dh: u64,
    /// Fine detail (Laplacian variance) per tile, `GRID` by `GRID`.
    pub tiles: Vec<f32>,
    /// Blur in one direction, 0 to 1.
    pub shake: f32,
    /// The share of burnt-out and black pixels.
    pub clip: f32,
    /// The look as 9 by 8 grey pixels in hex, the picture the fingerprint is made from; finds a
    /// picture saved again. Empty when not measured yet.
    #[serde(default)]
    pub tiny: String,
}

/// Luma as Pillow makes it, so the limits tuned on the Photos prototype hold.
fn luma(img: &DynamicImage) -> (Vec<u8>, usize, usize) {
    let rgb = img.to_rgb8();
    let (w, h) = (rgb.width() as usize, rgb.height() as usize);
    let g = rgb.pixels().map(|p| ((p[0] as u32 * 19595 + p[1] as u32 * 38470 + p[2] as u32 * 7471 + 0x8000) >> 16) as u8).collect();
    (g, w, h)
}

fn shrink(img: &DynamicImage, side: u32) -> DynamicImage {
    if img.width().max(img.height()) > side { img.thumbnail(side, side) } else { img.clone() }
}

fn var(v: impl Iterator<Item = f32> + Clone) -> f32 {
    let n = v.clone().count().max(1) as f64;
    let mean = v.clone().map(f64::from).sum::<f64>() / n;
    (v.map(|x| (f64::from(x) - mean).powi(2)).sum::<f64>() / n) as f32
}

/// Measures a picture, turned the way the camera held it.
pub fn measure(img: &DynamicImage) -> Shot {
    let (w, h) = img.dimensions();
    // Detail, shake and clipping at the measuring size.
    let big = shrink(img, MEASURE);
    let (g, gw, gh) = luma(&big);
    let px = |x: usize, y: usize| f32::from(g[y * gw + x]);
    let (lw, lh) = (gw.saturating_sub(2), gh.saturating_sub(2));
    let mut lap = vec![0f32; lw * lh];
    for y in 0..lh {
        for x in 0..lw {
            lap[y * lw + x] = px(x, y + 1) + px(x + 2, y + 1) + px(x + 1, y) + px(x + 1, y + 2) - 4.0 * px(x + 1, y + 1);
        }
    }
    let mut tiles = Vec::with_capacity(GRID * GRID);
    for r in 0..GRID {
        for c in 0..GRID {
            let (y0, y1, x0, x1) = (r * lh / GRID, (r + 1) * lh / GRID, c * lw / GRID, (c + 1) * lw / GRID);
            tiles.push(var((y0..y1).flat_map(|y| lap[y * lw + x0..y * lw + x1].iter().copied())));
        }
    }
    let mean_diff = |dx: usize, dy: usize| {
        let mut s = 0f64;
        let mut n = 0usize;
        for y in 0..gh - dy {
            for x in 0..gw - dx {
                s += f64::from((px(x + dx, y + dy) - px(x, y)).abs());
                n += 1;
            }
        }
        s / n.max(1) as f64
    };
    let (gx, gy) = if gw > 1 && gh > 1 { (mean_diff(1, 0), mean_diff(0, 1)) } else { (0.0, 0.0) };
    let shake = ((gx - gy).abs() / (gx + gy + 1e-6)) as f32;
    let clip = g.iter().filter(|&&v| !(12..=240).contains(&v)).count() as f32 / g.len().max(1) as f32;

    // Brightness, edges and the fingerprint at the small size.
    let small = shrink(&big, LOOK);
    let (s, sw, sh) = luma(&small);
    let lum = s.iter().map(|&v| f64::from(v)).sum::<f64>() / s.len().max(1) as f64;
    // Pillow's FIND_EDGES: the 3x3 edge kernel clamped to 0..255, its border copied as is.
    let edges = (0..sh).flat_map(|y| (0..sw).map(move |x| (x, y))).map(|(x, y)| {
        if x == 0 || y == 0 || x + 1 == sw || y + 1 == sh {
            return f32::from(s[y * sw + x]);
        }
        let mut v = 9 * i32::from(s[y * sw + x]);
        for yy in y - 1..=y + 1 {
            for xx in x - 1..=x + 1 {
                v -= i32::from(s[yy * sw + xx]);
            }
        }
        v.clamp(0, 255) as f32
    });
    let edge = var(edges.collect::<Vec<_>>().into_iter());
    let tiny = image::imageops::resize(&image::GrayImage::from_raw(sw as u32, sh as u32, s).unwrap_or_default(), 9, 8, image::imageops::FilterType::CatmullRom);
    let mut dh = 0u64;
    for i in 0..64 {
        let (r, c) = (i / 8, i % 8);
        if tiny.get_pixel(c, r)[0] > tiny.get_pixel(c + 1, r)[0] {
            dh |= 1 << i;
        }
    }
    let tiny = tiny.as_raw().iter().map(|b| format!("{b:02x}")).collect();
    Shot { sha: String::new(), w, h, cam: false, lum: lum as f32, edge, dh, tiles, shake, clip, tiny }
}

/// Reads and measures one picture file.
pub fn read(path: &Path) -> Result<Shot, Fail> {
    let bytes = std::fs::read(crate::store::named(path))?;
    let sha = sha1_smol::Sha1::from(&bytes).digest().to_string();
    let mut shot = match crate::thumbs::picture_bytes(&bytes) {
        Ok(img) => measure(&img),
        Err(Fail::Bad(_)) => Shot::default(),
        Err(e) => return Err(e),
    };
    shot.sha = sha;
    shot.cam = crate::photos::exif_camera(&bytes);
    Ok(shot)
}

/// Why a shot is the Best one of its set, for the words in `strings.ts`.
#[derive(Serialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Why {
    /// The others have more burnt-out or black areas.
    Clip,
    /// The others look shaken.
    Shake,
    /// Sharper where the detail is, by `more` percent.
    Sharp,
}

/// How sharp each of a group of pictures is where the detail is, against the others: every spot
/// counts, the detailed ones most, wherever they are; 1.0 is sharpest at every spot.
fn detail(shots: &[&Shot]) -> Vec<f32> {
    let tiles = GRID * GRID;
    let ok = |s: &Shot| s.tiles.len() == tiles;
    let top: Vec<f32> = (0..tiles).map(|t| shots.iter().filter(|s| ok(s)).map(|s| s.tiles[t]).fold(0.0, f32::max)).collect();
    let sum: f32 = top.iter().sum::<f32>() + 1e-6;
    shots.iter().map(|s| if !ok(s) { 0.0 } else { (0..tiles).map(|t| s.tiles[t] / (top[t] + 1e-6) * (top[t] / sum)).sum() }).collect()
}

/// The Best one of a set of Similar shots: its index, why, by how much (percent more fine
/// detail than the next one), and whether it is a close call. In a close call the index is the
/// sharpest, the first when equal: the one Review starts with kept.
pub fn pick(shots: &[&Shot]) -> (usize, Why, u32, bool) {
    let n = shots.len();
    let sharp = detail(shots);
    let low = |v: Vec<f32>| { let m = v.iter().copied().fold(f32::INFINITY, f32::min); v.into_iter().map(|x| x - m).collect::<Vec<_>>() };
    let shake = low(shots.iter().map(|s| s.shake).collect());
    let clip = low(shots.iter().map(|s| s.clip).collect());
    let score: Vec<f32> = (0..n).map(|i| sharp[i] - 1.5 * shake[i] - 5.0 * clip[i]).collect();
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|a, b| score[*b].total_cmp(&score[*a]).then(a.cmp(b)));
    let (b, x) = (order[0], order[1]);
    if score[b] - score[x] < CLOSE {
        // Rounded as Review ranks them, so photos it calls equally sharp go to the first.
        let top = (0..n).fold(0, |m, i| if rounded(sharp[i]) > rounded(sharp[m]) { i } else { m });
        return (top, Why::Sharp, 0, true);
    }
    let more = (100.0 * (sharp[b] - sharp[x]) / sharp[x].max(1e-6)).round().max(0.0) as u32;
    // The reason is what gave the Best one most of its lead over the next one, so it never reads
    // "about 0% more fine detail" when less shake or fewer burnt-out areas decided it.
    let parts = [(Why::Sharp, sharp[b] - sharp[x]), (Why::Shake, 1.5 * (shake[x] - shake[b])), (Why::Clip, 5.0 * (clip[x] - clip[b]))];
    let why = parts.iter().fold(parts[0], |m, p| if p.1 > m.1 { *p } else { m }).0;
    (b, why, more, false)
}

/// What looks like a picture nobody needs, by its name and numbers; clips never are.
#[derive(Serialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Junk {
    Whatsapp,
    Screenshot,
    Dark,
    Blurred,
}

pub fn junk(name: &str, s: &Shot) -> Option<Junk> {
    // WhatsApp's own names: IMG-20180212-WA0001.
    let up = name.to_uppercase();
    if up.match_indices("-WA").any(|(k, _)| up.as_bytes().get(k + 3..k + 7).is_some_and(|d| d.iter().all(u8::is_ascii_digit))) {
        return Some(Junk::Whatsapp);
    }
    if name.to_lowercase().contains("screenshot") {
        return Some(Junk::Screenshot);
    }
    if s.w == 0 {
        return None;
    }
    if s.lum < 28.0 {
        return Some(Junk::Dark);
    }
    if s.edge < 300.0 {
        return Some(Junk::Blurred);
    }
    // A tall phone screen with no camera, not a messenger-sized photo: WhatsApp sends a photo at
    // 1600 on its long side with the camera's details taken out, so a 20:9 one is 720 by 1600.
    let (a, b) = (s.w.max(s.h) as f32, s.w.min(s.h));
    if !s.cam && a / b as f32 >= 1.95 && [720, 1080, 1440].contains(&b) && a != 1600.0 {
        return Some(Junk::Screenshot);
    }
    None
}

/// What Review says about one photo; only photos in a set of Similar shots or Copies, or with a
/// Junk label, get one.
#[derive(Serialize, Clone, Debug, Default, PartialEq)]
pub struct Note {
    /// Its set of Similar shots, named by the set's first photo's path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub set: Option<String>,
    /// The Best one of its set.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub best: bool,
    /// Its set is a close call: the person picks.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub call: bool,
    /// On the Best one: why, and by how much.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub why: Option<(Why, u32)>,
    /// A copy: the path of the first file with the same content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub copy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub junk: Option<Junk>,
    /// In a set of Similar shots: another photo of its set is the same picture, edited (taken at
    /// the same second) or sent on (without the camera's details), not another shot.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub version: bool,
    /// In a set or a group of Copies: it names the camera or phone that took it, which a copy
    /// sent over a messenger has lost.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub cam: bool,
    /// In a set or a group of Copies: its size in pixels, as the camera was held.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub px: Option<(u32, u32)>,
    /// In a set of Similar shots: how sharp it is where the detail is, against the others there
    /// (1.0 sharpest at every spot), for Review's sharpness rank.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<f32>,
    /// The same against its group of Copies, the file kept included, for a set of Copies.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub copy_detail: Option<f32>,
}

/// A detail value rounded, so pictures the same to the eye share a rank.
fn rounded(x: f32) -> f32 {
    (x * 1000.0).round() / 1000.0
}

/// Review's notes for every Photos Section, by file path, and how many pictures are still to be
/// read.
#[derive(Serialize, Clone, Debug, Default)]
pub struct Sorting {
    pub notes: HashMap<String, Note>,
    pub left: usize,
}

fn is_picture(i: &Item) -> bool {
    class_of(&i.ext) == Some(Class::Image)
}

/// The Events of a Section by the folder Review covers (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`): a Photos Section's, or each
/// folder's with the Folder kind Photos, by its id (`scan::place_of`), in the Section's order.
pub fn places(sec: &Section) -> Vec<(&str, Vec<&Work>)> {
    let mut out: Vec<(&str, Vec<&Work>)> = vec![];
    for w in sec.works.iter().filter(|w| w.event) {
        let at = crate::scan::place_of(w);
        match out.iter_mut().find(|(p, _)| *p == at) {
            Some((_, ws)) => ws.push(w),
            None => out.push((at, vec![w])),
        }
    }
    out
}

/// Review's notes for a Section's Photos, from what is measured so far: each Photos folder on its
/// own, so a picture is never a Copy or a Similar shot of one in another Photos folder.
pub fn sort_section(sec: &Section, shot: &dyn Fn(&Item) -> Option<Shot>, out: &mut HashMap<String, Note>) {
    for (_, works) in places(sec) {
        sort_place(&works, shot, out);
    }
}

/// Review's notes for the Events of one Photos Section or Photos folder.
fn sort_place(works: &[&Work], shot: &dyn Fn(&Item) -> Option<Shot>, out: &mut HashMap<String, Note>) {
    let mut notes: HashMap<&str, Note> = HashMap::new();
    let pics: Vec<(&Item, Shot)> = works.iter().flat_map(|w| &w.items).filter(|i| is_picture(i)).filter_map(|i| shot(i).map(|s| (i, s))).collect();
    for (i, s) in &pics {
        if let Some(j) = junk(&i.name, s) {
            notes.entry(&i.path).or_default().junk = Some(j);
        }
    }
    let by_path: HashMap<&str, &Shot> = pics.iter().map(|(i, s)| (i.path.as_str(), s)).collect();
    // Similar shots, per Event: a photo joins a set if it is seconds after the set's last photo,
    // within a minute of its first and looks like one of its photos, so a moment shot from a
    // moving hand stays one set; or if it looks almost the same as the set's first photo within
    // minutes.
    for w in works {
        let shots: Vec<(&Item, &Shot, i64)> = w.items.iter().filter_map(|i| Some((i, *by_path.get(i.path.as_str())?, i.at?))).filter(|(_, s, _)| s.w > 0).collect();
        let mut sets: Vec<Vec<usize>> = vec![];
        let mut open: Vec<usize> = vec![];
        for (k, (_, x, t)) in shots.iter().enumerate() {
            let joined = open.iter().rev().take(6).copied().find(|&g| {
                let (a, last) = (shots[sets[g][0]], shots[*sets[g].last().unwrap()]);
                let d = (a.1.dh ^ x.dh).count_ones();
                let far: Vec<u32> = sets[g].iter().map(|&m| (shots[m].1.dh ^ x.dh).count_ones()).collect();
                // Near one of its shots, and never far from any, so a set cannot drift to another scene.
                let burst = t - last.2 <= BURST && t - a.2 <= MOMENT && far.iter().min().is_some_and(|&d| d <= SCENE) && far.iter().all(|&d| d <= 2 * SCENE);
                let near = d <= NEAR && t - a.2 <= 600;
                burst || near || (a.1.sha == x.sha)
            });
            match joined {
                Some(g) => sets[g].push(k),
                None => {
                    open.push(sets.len());
                    sets.push(vec![k]);
                }
            }
        }
        for set in sets.iter().filter(|s| s.len() > 1) {
            let members: Vec<&Shot> = set.iter().map(|&k| shots[k].1).collect();
            let (b, why, more, call) = pick(&members);
            let sharp = detail(&members);
            // One picture twice: the same look and shape, and one of them without the camera's
            // details, or both taken at the same second with files far apart in size (an edited
            // copy; frames of a burst are about the same size).
            let one = |a: &(&Item, &Shot, i64), b: &(&Item, &Shot, i64)| {
                let ratio = |s: &Shot| s.w as f32 / s.h.max(1) as f32;
                let (big, small) = (a.0.size.max(b.0.size) as f64, a.0.size.min(b.0.size).max(1) as f64);
                (a.1.dh ^ b.1.dh).count_ones() <= RESAVED_BITS && (ratio(a.1) - ratio(b.1)).abs() < 0.02 && (a.1.cam != b.1.cam || (a.2 == b.2 && big / small >= 1.3))
            };
            let version: Vec<bool> = set.iter().map(|&k| set.iter().any(|&m| m != k && one(&shots[k], &shots[m]))).collect();
            let name = shots[set[0]].0.path.clone();
            for (n, &k) in set.iter().enumerate() {
                let note = notes.entry(&shots[k].0.path).or_default();
                note.set = Some(name.clone());
                note.px = Some((shots[k].1.w, shots[k].1.h));
                note.cam = shots[k].1.cam;
                note.version = version[n];
                note.detail = Some(rounded(sharp[n]));
                note.best = n == b;
                note.call = call;
                note.why = (n == b && !call).then_some((why, more));
            }
        }
    }
    // Copies: the same bytes. The one kept is the first in the Section's order that nothing else
    // suggests setting aside, so a set of Copies never has every file suggested.
    let suggested = |n: Option<&Note>| n.is_some_and(|n| (n.set.is_some() && !n.best) || n.junk.is_some());
    let mut same: HashMap<&str, Vec<&str>> = HashMap::new();
    for (i, s) in &pics {
        same.entry(&s.sha).or_default().push(&i.path);
    }
    for paths in same.values().filter(|p| p.len() > 1) {
        let keep = *paths.iter().find(|p| !suggested(notes.get(**p))).unwrap_or(&paths[0]);
        for p in paths.iter().filter(|p| **p != keep) {
            notes.entry(p).or_default().copy = Some(keep.to_string());
        }
    }
    // Copies saved again: the same picture made smaller or saved at another quality, at any date.
    // The one kept is the largest that nothing else suggests setting aside.
    let order: HashMap<&str, usize> = pics.iter().enumerate().map(|(k, (i, _))| (i.path.as_str(), k)).collect();
    let set_of: HashMap<&str, String> = notes.iter().filter_map(|(p, n)| Some((*p, n.set.clone()?))).collect();
    for group in resaved(&pics, &set_of) {
        let fresh: Vec<&str> = group.iter().copied().filter(|p| notes.get(p).is_none_or(|n| n.copy.is_none())).collect();
        if fresh.len() < 2 {
            continue;
        }
        let px = |p: &str| by_path[p].w as u64 * by_path[p].h as u64;
        // Largest first, then the first in the Section's order.
        let mut ranked = fresh.clone();
        ranked.sort_by(|a, b| px(b).cmp(&px(a)).then(order[a].cmp(&order[b])));
        let keep = *ranked.iter().find(|p| !suggested(notes.get(**p))).unwrap_or(&ranked[0]);
        for p in fresh.into_iter().filter(|p| *p != keep) {
            notes.entry(p).or_default().copy = Some(keep.to_string());
        }
    }
    // Each group of Copies outside a set, with the file kept (in a set or not): their sizes and
    // sharpness, as Review's step of Copies shows them.
    let mut groups: HashMap<&str, Vec<&str>> = HashMap::new();
    for (p, n) in &notes {
        // Keyed by the path as `by_path` holds it, so the notes stay free to change below.
        if let (Some((&keep, _)), None) = (n.copy.as_deref().and_then(|c| by_path.get_key_value(c)), &n.set) {
            groups.entry(keep).or_default().push(*p);
        }
    }
    for (keep, copies) in groups {
        let paths: Vec<&str> = std::iter::once(keep).chain(copies).collect();
        let drawn: Vec<&str> = paths.into_iter().filter(|p| by_path[p].w > 0).collect();
        let sharp = detail(&drawn.iter().map(|p| by_path[p]).collect::<Vec<_>>());
        for (p, d) in drawn.iter().zip(sharp) {
            let note = notes.entry(p).or_default();
            note.px = Some((by_path[p].w, by_path[p].h));
            note.cam = by_path[p].cam;
            note.copy_detail = Some(rounded(d));
        }
    }
    out.extend(notes.into_iter().map(|(p, n)| (p.to_string(), n)));
}

/// Groups of pictures that are one picture saved again, by path. Two shots of one Similar set
/// are left to that set. Found through four 16-bit parts of the fingerprint: two fingerprints
/// within `RESAVED_BITS` of each other share at least one part.
fn resaved<'a>(pics: &[(&'a Item, Shot)], set_of: &HashMap<&str, String>) -> Vec<Vec<&'a str>> {
    let tiny: Vec<Vec<u8>> = pics.iter().map(|(_, s)| (0..s.tiny.len() / 2).filter_map(|k| u8::from_str_radix(&s.tiny[2 * k..2 * k + 2], 16).ok()).collect()).collect();
    let mut up: Vec<usize> = (0..pics.len()).collect();
    fn root(up: &mut [usize], mut k: usize) -> usize {
        while up[k] != k {
            up[k] = up[up[k]];
            k = up[k];
        }
        k
    }
    let mut parts: HashMap<(u32, u16), Vec<usize>> = HashMap::new();
    for (k, (_, s)) in pics.iter().enumerate().filter(|(k, (_, s))| s.w > 0 && tiny[*k].len() == 72) {
        for part in 0..4 {
            parts.entry((part, (s.dh >> (16 * part)) as u16)).or_default().push(k);
        }
    }
    for ks in parts.values() {
        for (n, &a) in ks.iter().enumerate() {
            for &b in &ks[n + 1..] {
                let (x, y) = (&pics[a].1, &pics[b].1);
                let shape = (x.w as f32 / x.h as f32 - y.w as f32 / y.h as f32).abs() < 0.02;
                let diff: u32 = tiny[a].iter().zip(&tiny[b]).map(|(p, q)| u32::from(p.abs_diff(*q))).sum();
                let one_set = set_of.get(pics[a].0.path.as_str()).is_some_and(|s| set_of.get(pics[b].0.path.as_str()) == Some(s));
                if shape && !one_set && (x.dh ^ y.dh).count_ones() <= RESAVED_BITS && diff <= RESAVED_DIFF && x.sha != y.sha {
                    let (ra, rb) = (root(&mut up, a), root(&mut up, b));
                    up[ra] = rb;
                }
            }
        }
    }
    let mut groups: HashMap<usize, Vec<&'a str>> = HashMap::new();
    for (k, pic) in pics.iter().enumerate() {
        let r = root(&mut up, k);
        groups.entry(r).or_default().push(pic.0.path.as_str());
    }
    groups.into_values().filter(|g| g.len() > 1).collect()
}

#[derive(Serialize, Deserialize, Default)]
struct Index {
    v: u32,
    files: HashMap<String, Shot>,
}

struct Job {
    key: String,
    path: String,
}

struct Inner {
    index: Index,
    keys: HashMap<String, String>,
    todo: VecDeque<Job>,
    /// Pictures read since the last `shots` event.
    news: bool,
    dirty: bool,
    saved: Instant,
}

/// Reads and measures the pictures of every Photos Section in the background.
pub struct Shots {
    thumbs: Arc<Thumbs>,
    inner: Mutex<Inner>,
    wake: Condvar,
}

fn key_of(i: &Item) -> String {
    format!("{}|{}|{}", i.path, i.size, i.mtime)
}

fn index_file(dir: &Path) -> std::path::PathBuf {
    dir.join("photo-shots.json")
}

impl Shots {
    pub fn new(thumbs: Arc<Thumbs>) -> Arc<Shots> {
        let index = Self::load(&thumbs.dir());
        Arc::new(Shots {
            thumbs,
            inner: Mutex::new(Inner { index, keys: HashMap::new(), todo: VecDeque::new(), news: false, dirty: false, saved: Instant::now() }),
            wake: Condvar::new(),
        })
    }

    fn load(dir: &Path) -> Index {
        let i: Index = crate::store::load(&index_file(dir));
        if i.v == 1 { i } else { Index { v: 1, files: HashMap::new() } }
    }

    /// Reads the index again after the cache moved.
    pub fn reload(&self) {
        self.inner.lock().unwrap().index = Self::load(&self.thumbs.dir());
    }

    pub fn start(self: &Arc<Self>, app: AppHandle) {
        let threads = std::thread::available_parallelism().map_or(2, |n| (n.get() / 3).clamp(1, 4));
        for _ in 0..threads {
            let t = self.clone();
            std::thread::spawn(move || t.work());
        }
        let t = self.clone();
        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_millis(2000));
            let (news, left, save) = {
                let mut g = t.inner.lock().unwrap();
                let save = g.dirty && (g.todo.is_empty() || g.saved.elapsed() > Duration::from_secs(20));
                (std::mem::take(&mut g.news), g.todo.len(), save)
            };
            if news {
                let _ = app.emit("shots", left);
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

    /// Plans what to read for the catalog: every picture of an Event (a Photos Section's, or a
    /// Photos folder's) that is on this PC.
    pub fn plan(&self, cat: &Catalog) {
        {
            let mut g = self.inner.lock().unwrap();
            let g = &mut *g;
            g.todo.clear();
            g.keys.clear();
            for s in &cat.sections {
                for i in s.works.iter().filter(|w| w.event).flat_map(|w| &w.items).filter(|i| is_picture(i) && !i.cloud) {
                    let key = key_of(i);
                    // A picture measured before `tiny` existed is read again once.
                    if s.online && !g.index.files.get(&key).is_some_and(|s| s.w == 0 || !s.tiny.is_empty()) {
                        g.todo.push_back(Job { key: key.clone(), path: i.path.clone() });
                    }
                    g.keys.insert(i.path.clone(), key);
                }
            }
            // A picture still read into some card keeps its measure, so a folder's kind changed
            // away from Photos and back (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`) is not read again.
            let before = g.index.files.len();
            let used: std::collections::HashSet<String> = cat.sections.iter().flat_map(|s| &s.works).flat_map(|w| &w.items).filter(|i| is_picture(i)).map(key_of).collect();
            g.index.files.retain(|k, _| used.contains(k));
            g.dirty |= g.index.files.len() != before;
            log::info!("photos for Review: {} measured, {} to read", g.index.files.len(), g.todo.len());
        }
        self.wake.notify_all();
    }

    /// Review's notes for every Photos Section and Photos folder of the catalog.
    pub fn sorting(&self, cat: &Catalog) -> Sorting {
        let g = self.inner.lock().unwrap();
        let mut out = Sorting { notes: HashMap::new(), left: g.todo.len() };
        let shot = |i: &Item| g.index.files.get(&key_of(i)).cloned();
        for s in &cat.sections {
            sort_section(s, &shot, &mut out.notes);
        }
        out
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
            let got = if crate::scan::now_cloud(&job.path) { Err(Fail::Retry("only in the cloud now".into())) } else { read(Path::new(&job.path)) };
            let mut g = self.inner.lock().unwrap();
            match got {
                Ok(s) => {
                    g.index.files.insert(job.key, s);
                    g.news = true;
                    g.dirty = true;
                }
                Err(Fail::Bad(e)) => log::warn!("cannot read {} for Review: {e}", job.path),
                Err(Fail::Retry(_)) => {}
            }
        }
    }
}

/// The photos set aside in Review in a Photos Section, or in one of its Photos folders, by the
/// id its Events start with (`place`, `decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`). Their paths are inside that folder. A choice
/// made for another file of another size at the same path does not count.
pub fn set_aside<'a>(sec: &'a Section, place: &str, review: &HashMap<String, crate::store::Choice>) -> Vec<&'a Item> {
    sec.works
        .iter()
        .filter(|w| w.event && crate::scan::place_of(w) == place)
        .flat_map(|w| &w.items)
        .filter(|i| review.get(&crate::store::key_in(place, &i.rel)).is_some_and(|c| !c.keep && c.size.is_none_or(|s| s == i.size)))
        .collect()
}

/// What Review's move takes from a Photos Section or Photos folder: the photos set aside (only
/// those whose path is in `only`, when given: the ones ticked in Move), then the Insta360 previews
/// that go with them, and how many previews. Paths are inside that folder.
pub fn to_move(sec: &Section, place: &str, review: &HashMap<String, crate::store::Choice>, only: Option<&std::collections::HashSet<String>>) -> (Vec<crate::moves::Photo>, usize) {
    use crate::moves::Photo;
    let all = set_aside(sec, place, review);
    let items: Vec<&Item> = all.iter().copied().filter(|i| only.is_none_or(|o| o.contains(&i.rel))).collect();
    // A dual-lens camera saves one shot as two videos with one preview: it moves only when both do,
    // so it stays while a video of its shot stays (kept, not reviewed or unticked).
    let moving: std::collections::HashSet<&str> = items.iter().map(|i| i.rel.as_str()).collect();
    let left: Vec<&Item> = sec.works.iter().filter(|w| w.event && crate::scan::place_of(w) == place).flat_map(|w| &w.items).filter(|i| i.ext == "insv" && !moving.contains(i.rel.as_str())).collect();
    let mut photos: Vec<Photo> = items.iter().map(|i| Photo { rel: i.rel.clone(), size: i.size, mtime: i.mtime, cloud: i.cloud, preview: false }).collect();
    // A preview was never reviewed, so it moves as it is now.
    let now = |p: &str| std::fs::metadata(crate::store::named(Path::new(p))).ok().and_then(|m| m.modified().ok()).and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs());
    // Unlisted files' paths are from the Section's folder.
    let inner = crate::store::under(place, &sec.id).map(|r| format!("{r}/")).unwrap_or_default();
    let stays = previews_of(sec, &inner, &left);
    let previews: Vec<_> = previews_of(sec, &inner, &items).into_iter().filter(|u| !stays.iter().any(|v| std::ptr::eq(*v, *u))).collect();
    let n = previews.len();
    photos.extend(previews.into_iter().map(|u| Photo { rel: u.rel[inner.len()..].to_string(), size: u.size, mtime: now(&u.path), cloud: u.cloud, preview: true }));
    (photos, n)
}

/// The Insta360 previews (.lrv) of set-aside videos (.insv). Unlisted beside their video, they
/// move with it, so none is left behind to show as a video of its own. `inner` is the Photos
/// folder's path in the Section and `/`, or none for a Photos Section.
fn previews_of<'a>(sec: &'a Section, inner: &str, items: &[&Item]) -> Vec<&'a crate::scan::Unlisted> {
    let dir = |rel: &str| rel.rsplit_once('/').map_or("", |(d, _)| d).to_string();
    let key = crate::scan::insta360_key;
    let videos: std::collections::HashSet<(String, String)> = items.iter().filter(|i| i.ext == "insv").filter_map(|i| Some((dir(&i.rel), key(&i.name)?))).collect();
    sec.unlisted
        .iter()
        .filter(|u| u.reason == crate::scan::Reason::Preview && u.rel.starts_with(inner))
        .filter(|u| key(&u.name).is_some_and(|k| videos.contains(&(dir(&u.rel[inner.len()..]), k))))
        .collect()
}

/// The photo folder as the script names it: with the `\\?\` start on Windows when its name, or a
/// folder's in `files`, ends in a dot or a space, which .NET would drop and reach the sibling
/// without that end.
pub fn script_root(root: &Path, files: &[String]) -> String {
    let odd = crate::store::ends_oddly(root) || files.iter().any(|f| crate::store::ends_oddly(Path::new(f)));
    match crate::store::verbatim(root).filter(|_| cfg!(windows) && odd) {
        Some(v) => v.to_string_lossy().into_owned(),
        None => root.to_string_lossy().into_owned(),
    }
}

/// The script that moves set-aside photos into `Not needed` inside the Section's folder, for
/// Windows (PowerShell) or macOS and Linux (sh). It never deletes, and skips a file that is gone
/// or already there (`decisions/2026/10/04/sorting-ends-in-a-move-script-never-a-delete.md`, `decisions/2026/10/04/a-move-script-for-each-system.md`).
pub fn move_script(root: &str, files: &[String], windows: bool) -> String {
    let n = files.len();
    if windows {
        // PowerShell ends a single-quoted string at any of these, so each is doubled.
        let q = |s: &str| {
            let mut out = String::from("'");
            for c in s.chars() {
                if matches!(c, '\'' | '\u{2018}' | '\u{2019}' | '\u{201A}' | '\u{201B}') {
                    out.push(c);
                }
                out.push(c);
            }
            out + "'"
        };
        let list: Vec<String> = files.iter().map(|f| format!("  {}", q(&f.replace('/', "\\")))).collect();
        format!(
            "\u{feff}# FilesViewHub: moves the {n} photos you marked Not needed in Review into \"{NOT_NEEDED}\" inside\r\n\
             # the photo folder. Nothing is deleted. Look through \"{NOT_NEEDED}\" and empty it yourself.\r\n\
             # To run it: right-click this file and pick Run with PowerShell.\r\n\
             $root = {root}\r\n\
             $files = @(\r\n{list}\r\n)\r\n\
             $moved = 0; $skipped = 0\r\n\
             foreach ($rel in $files) {{\r\n\
             \x20 $from = [System.IO.Path]::Combine($root, $rel)\r\n\
             \x20 $to = [System.IO.Path]::Combine($root, '{NOT_NEEDED}', $rel)\r\n\
             \x20 if ([System.IO.File]::Exists($to) -or [System.IO.Directory]::Exists($to)) {{ Write-Host \"Already in {NOT_NEEDED}: $rel\"; $skipped++; continue }}\r\n\
             \x20 if (-not [System.IO.File]::Exists($from)) {{ Write-Host \"Not there any more: $rel\"; $skipped++; continue }}\r\n\
             \x20 try {{\r\n\
             \x20   [void][System.IO.Directory]::CreateDirectory([System.IO.Path]::GetDirectoryName($to))\r\n\
             \x20   [System.IO.File]::Move($from, $to)\r\n\
             \x20   $moved++\r\n\
             \x20 }} catch {{ Write-Host \"Could not move ${{rel}}: $_\"; $skipped++ }}\r\n\
             }}\r\n\
             Write-Host \"$moved moved into {NOT_NEEDED}, $skipped skipped.\"\r\n\
             Read-Host 'Press Enter to close'\r\n",
            root = q(root),
            list = list.join("\r\n"),
        )
    } else {
        let q = |s: &str| format!("'{}'", s.replace('\'', "'\\''"));
        let list: Vec<String> = files.iter().map(|f| format!("  {}", q(f))).collect();
        format!(
            "#!/bin/sh\n\
             # FilesViewHub: moves the {n} photos you marked Not needed in Review into \"{NOT_NEEDED}\" inside\n\
             # the photo folder. Nothing is deleted. Look through \"{NOT_NEEDED}\" and empty it yourself.\n\
             # To run it: sh \"<this file>\" in Terminal.\n\
             root={root}\n\
             moved=0; skipped=0\n\
             for rel in \\\n{list}\n\
             do\n\
             \x20 from=\"$root/$rel\"; to=\"$root/{NOT_NEEDED}/$rel\"\n\
             \x20 if [ -e \"$to\" ]; then echo \"Already in {NOT_NEEDED}: $rel\"; skipped=$((skipped+1)); continue; fi\n\
             \x20 if [ ! -e \"$from\" ]; then echo \"Not there any more: $rel\"; skipped=$((skipped+1)); continue; fi\n\
             \x20 if mkdir -p \"$(dirname \"$to\")\" && mv -n \"$from\" \"$to\"; then moved=$((moved+1)); else echo \"Could not move $rel\"; skipped=$((skipped+1)); fi\n\
             done\n\
             echo \"$moved moved into {NOT_NEEDED}, $skipped skipped.\"\n",
            root = q(root),
            list = list.join(" \\\n"),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Kind;
    use image::{Rgb, RgbImage};

    /// A busy picture: fine stripes and blocks, so blur and shake show.
    fn scene(w: u32, h: u32, seed: u32) -> DynamicImage {
        DynamicImage::ImageRgb8(RgbImage::from_fn(w, h, |x, y| {
            let v = ((x * 7 + y * 13 + seed) % 23) * 9 + if (x / 16 + y / 16) % 2 == 0 { 30 } else { 0 };
            Rgb([v as u8, (v / 2 + 40) as u8, (255 - v) as u8])
        }))
    }

    fn blur(img: &DynamicImage, s: f32) -> DynamicImage {
        img.blur(s)
    }

    /// Smears along x only, as a shaken hand does.
    fn shake(img: &DynamicImage, px: u32) -> DynamicImage {
        let src = img.to_rgb8();
        let (w, h) = src.dimensions();
        DynamicImage::ImageRgb8(RgbImage::from_fn(w, h, |x, y| {
            let mut acc = [0u32; 3];
            for k in 0..px {
                let xx = (x + w + k - px / 2) % w;
                let p = src.get_pixel(xx, y);
                for c in 0..3 {
                    acc[c] += u32::from(p[c]);
                }
            }
            Rgb([(acc[0] / px) as u8, (acc[1] / px) as u8, (acc[2] / px) as u8])
        }))
    }

    #[test]
    fn the_best_one_is_the_unspoiled_one_in_either_order() {
        let good = scene(800, 600, 1);
        let spoils: [(&str, DynamicImage); 3] =
            [("blurred", blur(&good, 1.5)), ("shaken", shake(&good, 9)), ("burnt out", good.brighten(120))];
        for (what, bad) in spoils {
            let (a, b) = (measure(&good), measure(&bad));
            let (best, _, _, call) = pick(&[&a, &b]);
            assert!(best == 0 && !call, "{what}: picked {best}, call {call}");
            let (best, _, _, call) = pick(&[&b, &a]);
            assert!(best == 1 && !call, "{what}, other order: picked {best}, call {call}");
        }
    }

    #[test]
    fn the_reason_is_what_decided_it_never_zero_percent_sharper() {
        // As sharp as each other: the one less shaken wins, and says so.
        let a = Shot { tiles: vec![10.0; 36], shake: 0.0, ..Default::default() };
        let b = Shot { shake: 0.2, ..a.clone() };
        let (best, why, _, call) = pick(&[&b, &a]);
        assert_eq!((best, why, call), (1, Why::Shake, false));
        // The same with burnt-out areas.
        let c = Shot { clip: 0.1, ..a.clone() };
        assert_eq!(pick(&[&c, &a]).1, Why::Clip);
        // Clearly sharper: the percent is more than 0.
        let d = Shot { tiles: vec![5.0; 36], ..a.clone() };
        let (_, why, more, _) = pick(&[&d, &a]);
        assert!(why == Why::Sharp && more > 0);
    }

    #[test]
    fn the_same_picture_twice_is_your_call() {
        let a = measure(&scene(800, 600, 1));
        assert!(pick(&[&a, &a.clone()]).3);
    }

    #[test]
    fn a_close_call_starts_with_the_sharpest_or_the_first() {
        let a = Shot { tiles: vec![10.0; 36], ..Default::default() };
        // A hair sharper, too little to be a Best one: still the one kept.
        let b = Shot { tiles: vec![10.3; 36], ..a.clone() };
        assert_eq!(pick(&[&a, &b]), (1, Why::Sharp, 0, true));
        assert_eq!(pick(&[&a, &a.clone(), &a.clone()]).0, 0);
    }

    #[test]
    fn near_pictures_have_near_fingerprints() {
        // Light from the left, the same a little brighter, and light from the right.
        let ramp = |up: bool| DynamicImage::ImageRgb8(RgbImage::from_fn(800, 600, move |x, y| {
            let v = (if up { x } else { 799 - x }) * 200 / 800 + (y / 75) * 6;
            Rgb([v as u8; 3])
        }));
        let a = measure(&ramp(false));
        let b = measure(&ramp(false).brighten(8));
        let c = measure(&ramp(true));
        assert!((a.dh ^ b.dh).count_ones() <= NEAR);
        assert!((a.dh ^ c.dh).count_ones() > NEAR);
    }

    #[test]
    fn junk_by_name_and_by_look() {
        let sharp = Shot { w: 4000, h: 3000, cam: true, lum: 120.0, edge: 2000.0, ..Default::default() };
        assert_eq!(junk("IMG-20180212-WA0001.jpg", &sharp), Some(Junk::Whatsapp));
        assert_eq!(junk("Hawaii-Waikiki.jpg", &sharp), None);
        assert_eq!(junk("Screenshot_20190305-081500.png", &sharp), Some(Junk::Screenshot));
        assert_eq!(junk("a.jpg", &Shot { lum: 20.0, ..sharp.clone() }), Some(Junk::Dark));
        assert_eq!(junk("a.jpg", &Shot { edge: 100.0, ..sharp.clone() }), Some(Junk::Blurred));
        assert_eq!(junk("a.jpg", &Shot { w: 1080, h: 2340, cam: false, ..sharp.clone() }), Some(Junk::Screenshot));
        assert_eq!(junk("a.jpg", &Shot { w: 1080, h: 2340, cam: true, ..sharp.clone() }), None);
        // A 20:9 camera photo sent over WhatsApp; a screenshot of that size keeps its name.
        assert_eq!(junk("2021-01-04 10.41.03.jpg", &Shot { w: 720, h: 1600, cam: false, ..sharp.clone() }), None);
        assert_eq!(junk("Screenshot_20210104-104103.png", &Shot { w: 720, h: 1600, cam: false, ..sharp.clone() }), Some(Junk::Screenshot));
        assert_eq!(junk("a.jpg", &sharp), None);
        assert_eq!(junk("a.heic", &Shot::default()), None);
    }

    fn item(rel: &str, at: i64) -> Item {
        Item { name: rel.into(), rel: rel.into(), path: format!("P/{rel}"), ext: "jpg".into(), size: 1, mtime: 1, cloud: false, key: rel.into(),
            title: rel.into(), group: String::new(), role: crate::scan::Role::Main, lang: String::new(), at: Some(at) }
    }

    #[test]
    fn sets_copies_and_junk_in_one_section() {
        let look = Shot { sha: "x".into(), w: 4000, h: 3000, cam: true, lum: 120.0, edge: 2000.0, dh: 0, tiles: vec![10.0; 36], ..Default::default() };
        let items = vec![item("a.jpg", 0), item("b.jpg", 1), item("c.jpg", 5000), item("d.jpg", 9000), item("e.jpg", 20000)];
        let shots: HashMap<String, Shot> = HashMap::from([
            ("P/a.jpg".into(), Shot { sha: "1".into(), ..look.clone() }),
            ("P/b.jpg".into(), Shot { sha: "2".into(), tiles: vec![5.0; 36], ..look.clone() }),
            ("P/c.jpg".into(), Shot { sha: "3".into(), dh: u64::MAX, lum: 10.0, ..look.clone() }),
            ("P/d.jpg".into(), Shot { sha: "1".into(), dh: 0xFF00, ..look.clone() }),
            ("P/e.jpg".into(), Shot { sha: "2".into(), dh: 0xFF0000, ..look.clone() }),
        ]);
        let work = crate::scan::Work { id: "S#a".into(), title: String::new(), path: String::new(), folder: vec![], is_folder: true, items, media: vec![],
            size: 0, added: 0, cloud: 0, year: None, taken: Some((0, 9000)), event: true, day_only: false };
        let sec = Section { id: "S".into(), name: String::new(), path: String::new(), drive: String::new(), kind: Kind::Photos, guessed: false, online: true,
            gone: false, works: vec![work], files: 4, scanned_at: 0, unlisted: vec![], empty: vec![], whole_tree: false, split_from: None, links: vec![] };
        let mut out = HashMap::new();
        sort_section(&sec, &|i| shots.get(&i.path).cloned(), &mut out);
        assert!(out["P/a.jpg"].best && out["P/a.jpg"].set.as_deref() == Some("P/a.jpg"));
        assert!(!out["P/b.jpg"].best && out["P/b.jpg"].set.as_deref() == Some("P/a.jpg"));
        assert_eq!(out["P/c.jpg"].junk, Some(Junk::Dark));
        assert_eq!(out["P/d.jpg"].copy.as_deref(), Some("P/a.jpg"));
        // b is an extra shot, so its copy e is kept and b is the copy.
        assert_eq!(out["P/b.jpg"].copy.as_deref(), Some("P/e.jpg"));
        // b is in its set's step, not beside e, so e says nothing.
        assert!(!out.contains_key("P/e.jpg"));
        // d, the same bytes as a, joined a's set: ranked there, with its size.
        assert!(out["P/d.jpg"].detail.is_some() && out["P/a.jpg"].detail.is_some());
        assert_eq!(out["P/a.jpg"].px, Some((4000, 3000)));
        assert!(out["P/a.jpg"].cam);
    }

    #[test]
    fn a_moment_with_pauses_and_a_moving_hand_is_one_set() {
        // Taken as on 2018-07-07: 7 s between two pairs, each shot near the one before, the last
        // far from the first. A photo 20 s after the moment's last one starts a set of its own.
        let look = Shot { w: 4000, h: 3000, cam: true, lum: 120.0, edge: 2000.0, tiles: vec![10.0; 36], ..Default::default() };
        let at = [(0, 0u64), (1, 0x3FF), (8, 0x3_FFFF), (9, 0x3_FFFF), (13, 0x3FF_FFFF), (33, 0x3FF_FFFF), (34, 0x3FF_FFFF)];
        let items: Vec<Item> = at.iter().enumerate().map(|(k, (t, _))| item(&format!("{k}.jpg"), *t)).collect();
        let shots: HashMap<String, Shot> = at.iter().enumerate().map(|(k, (_, dh))| (format!("P/{k}.jpg"), Shot { sha: k.to_string(), dh: *dh, ..look.clone() })).collect();
        let work = crate::scan::Work { id: "S#a".into(), title: String::new(), path: String::new(), folder: vec![], is_folder: true, items, media: vec![],
            size: 0, added: 0, cloud: 0, year: None, taken: Some((0, 34)), event: true, day_only: false };
        let sec = Section { id: "S".into(), name: String::new(), path: String::new(), drive: String::new(), kind: Kind::Photos, guessed: false, online: true,
            gone: false, works: vec![work], files: 7, scanned_at: 0, unlisted: vec![], empty: vec![], whole_tree: false, split_from: None, links: vec![] };
        let mut out = HashMap::new();
        sort_section(&sec, &|i| shots.get(&i.path).cloned(), &mut out);
        let set = |k: usize| out.get(&format!("P/{k}.jpg")).and_then(|n| n.set.clone());
        for k in 1..5 {
            assert_eq!(set(k).as_deref(), Some("P/0.jpg"), "{k}: {out:?}");
        }
        assert_eq!(set(5).as_deref(), Some("P/5.jpg"));
        assert_eq!(set(6).as_deref(), Some("P/5.jpg"));
    }

    #[test]
    fn an_edited_or_sent_copy_in_a_set_is_a_version() {
        // A camera original, its edited copy (the same second, twice the bytes), a copy sent on (no
        // camera details, later), a burst frame of the same second and size, and another shot of
        // the moment that looks alike but is not the same picture.
        let look = Shot { w: 3024, h: 4032, cam: true, lum: 120.0, edge: 2000.0, tiles: vec![10.0; 36], ..Default::default() };
        let at = [("orig", 0, 0u64, true, 3024), ("edit", 0, 0b11, true, 3024), ("sent", 5, 0b1, false, 1200), ("other", 9, 0xFF, true, 3024)];
        let bytes = |n: &str| if n == "edit" { 2_000_000 } else { 1_000_000 };
        let items: Vec<Item> = at.iter().map(|(n, t, ..)| Item { size: bytes(n), ..item(&format!("{n}.jpg"), *t) }).collect();
        let shots: HashMap<String, Shot> = at.iter().map(|(n, _, dh, cam, w)| (format!("P/{n}.jpg"), Shot { sha: n.to_string(), dh: *dh, cam: *cam, w: *w, h: w * 4 / 3, ..look.clone() })).collect();
        let work = crate::scan::Work { id: "S#a".into(), title: String::new(), path: String::new(), folder: vec![], is_folder: true, items, media: vec![],
            size: 0, added: 0, cloud: 0, year: None, taken: Some((0, 9)), event: true, day_only: false };
        let sec = Section { id: "S".into(), name: String::new(), path: String::new(), drive: String::new(), kind: Kind::Photos, guessed: false, online: true,
            gone: false, works: vec![work], files: 4, scanned_at: 0, unlisted: vec![], empty: vec![], whole_tree: false, split_from: None, links: vec![] };
        let mut out = HashMap::new();
        sort_section(&sec, &|i| shots.get(&i.path).cloned(), &mut out);
        let v = |n: &str| out[&format!("P/{n}.jpg")].version;
        assert!(out["P/other.jpg"].set.is_some(), "{out:?}");
        assert!(v("orig") && v("edit") && v("sent") && !v("other"), "{out:?}");
        // Two burst frames of one second, the same size: shots, not one photo.
        let frames = vec![Item { size: 1_000_000, ..item("f1.jpg", 0) }, Item { size: 1_050_000, ..item("f2.jpg", 0) }];
        let shots: HashMap<String, Shot> = [("P/f1.jpg", 0u64), ("P/f2.jpg", 0b11)].into_iter().map(|(p, dh)| (p.to_string(), Shot { sha: p.into(), dh, ..look.clone() })).collect();
        let sec = Section { works: vec![crate::scan::Work { items: frames, ..sec.works[0].clone() }], ..sec };
        let mut out = HashMap::new();
        sort_section(&sec, &|i| shots.get(&i.path).cloned(), &mut out);
        assert!(out["P/f1.jpg"].set.is_some() && !out["P/f1.jpg"].version && !out["P/f2.jpg"].version, "{out:?}");
    }

    #[test]
    fn a_picture_saved_again_smaller_is_a_copy_at_any_date() {
        // A soft scene, as photos are: a ramp with a few blocks.
        let photo = |w: u32, h: u32, flip: bool| DynamicImage::ImageRgb8(RgbImage::from_fn(w, h, move |x, y| {
            let (x, y) = (x * 1600 / w, y * 1200 / h);
            let x = if flip { 1599 - x } else { x };
            let v = x / 10 + y / 12 + if (x / 400 + y / 300) % 2 == 0 { 40 } else { 0 };
            Rgb([v as u8, (v / 2 + 30) as u8, (220 - v.min(220)) as u8])
        }));
        let big = measure(&photo(1600, 1200, false));
        let small = measure(&photo(1600, 1200, false).resize_exact(800, 600, image::imageops::FilterType::Triangle));
        let other = measure(&photo(1600, 1200, true));
        let shots: HashMap<String, Shot> = HashMap::from([
            ("P/small.jpg".into(), Shot { sha: "1".into(), cam: true, ..small }),
            ("P/big.jpg".into(), Shot { sha: "2".into(), cam: true, ..big }),
            ("P/other.jpg".into(), Shot { sha: "3".into(), cam: true, ..other }),
        ]);
        let work = |id: &str, items: Vec<Item>| crate::scan::Work { id: id.into(), title: String::new(), path: String::new(), folder: vec![], is_folder: true, items,
            media: vec![], size: 0, added: 0, cloud: 0, year: None, taken: None, event: true, day_only: false };
        let sec = Section { id: "S".into(), name: String::new(), path: String::new(), drive: String::new(), kind: Kind::Photos, guessed: false, online: true,
            gone: false, works: vec![work("S#small", vec![item("small.jpg", 0)]), work("S#big", vec![item("big.jpg", 90_000_000), item("other.jpg", 90_100_000)])],
            files: 3, scanned_at: 0, unlisted: vec![], empty: vec![], whole_tree: false, split_from: None, links: vec![] };
        let mut out = HashMap::new();
        sort_section(&sec, &|i| shots.get(&i.path).cloned(), &mut out);
        assert_eq!(out.get("P/small.jpg").and_then(|n| n.copy.as_deref()), Some("P/big.jpg"), "{out:?}");
        assert!(out.get("P/big.jpg").is_none_or(|n| n.copy.is_none()));
        assert!(out.get("P/other.jpg").is_none_or(|n| n.copy.is_none()));
        // The two are ranked against each other: the large one has more fine detail.
        let d = |p: &str| out[p].copy_detail.unwrap();
        assert!(d("P/big.jpg") > d("P/small.jpg"), "{out:?}");
        assert_eq!(out["P/small.jpg"].px, Some((800, 600)));
    }

    /// Review covers each Photos folder alone (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`): the same picture in two Photos folders of
    /// one Section is no Copy, and the move of one takes only its own photos, with paths in it.
    #[test]
    fn review_covers_each_photos_folder_alone() {
        let look = Shot { sha: "same".into(), w: 4000, h: 3000, cam: true, lum: 120.0, edge: 2000.0, dh: 0, tiles: vec![10.0; 36], ..Default::default() };
        let at = |rel: &str, path: &str, t| Item { path: path.into(), ..item(rel, t) };
        let event = |id: &str, items| crate::scan::Work { id: id.into(), title: String::new(), path: String::new(), folder: vec![], is_folder: true, items,
            media: vec![], size: 0, added: 0, cloud: 0, year: None, taken: Some((0, 0)), event: true, day_only: false };
        let mut card = event("S/Books/x", vec![at("x.jpg", "D/Books/x.jpg", 0)]);
        card.event = false;
        let works = vec![
            event("S/Phone#a", vec![at("a.jpg", "D/Phone/a.jpg", 0), at("Sub/far.jpg", "D/Phone/Sub/far.jpg", 90_000)]),
            event("S/Old/Camera#a", vec![at("a.jpg", "D/Old/Camera/a.jpg", 0)]),
            card,
        ];
        let sec = Section { id: "S".into(), name: String::new(), path: "D".into(), drive: String::new(), kind: Kind::Files, guessed: false, online: true,
            gone: false, works, files: 4, scanned_at: 0, unlisted: vec![], empty: vec![], whole_tree: false, split_from: None, links: vec![] };
        assert_eq!(places(&sec).iter().map(|(p, ws)| (*p, ws.len())).collect::<Vec<_>>(), [("S/Phone", 1), ("S/Old/Camera", 1)], "a Work that is no Event is in none");
        let mut out = HashMap::new();
        sort_section(&sec, &|_| Some(look.clone()), &mut out);
        assert_eq!(out.get("D/Phone/Sub/far.jpg").and_then(|n| n.copy.as_deref()), Some("D/Phone/a.jpg"), "a Copy inside one Photos folder: {out:?}");
        assert!(out.get("D/Old/Camera/a.jpg").is_none_or(|n| n.copy.is_none() && n.set.is_none()), "nothing across Photos folders: {out:?}");
        assert!(!out.contains_key("D/Books/x.jpg"), "a Work that is no Event is not reviewed");
        let c = crate::store::Choice { keep: false, at: 1, size: None };
        let review = HashMap::from([("S/Phone/Sub/far.jpg".to_string(), c), ("S/Old/Camera/a.jpg".to_string(), c)]);
        assert_eq!(set_aside(&sec, "S/Phone", &review).iter().map(|i| i.path.as_str()).collect::<Vec<_>>(), ["D/Phone/Sub/far.jpg"]);
        assert_eq!(to_move(&sec, "S/Old/Camera", &review, None).0.iter().map(|p| p.rel.as_str()).collect::<Vec<_>>(), ["a.jpg"], "paths inside its own folder");
    }

    #[test]
    fn a_choice_for_another_file_at_the_same_path_does_not_count() {
        let mut sec = Section { id: "S".into(), name: String::new(), path: String::new(), drive: String::new(), kind: Kind::Photos, guessed: false, online: true,
            gone: false, works: vec![], files: 0, scanned_at: 0, unlisted: vec![], empty: vec![], whole_tree: false, split_from: None, links: vec![] };
        sec.works.push(crate::scan::Work { id: "S#a".into(), title: String::new(), path: String::new(), folder: vec![], is_folder: true,
            items: vec![item("a.jpg", 0), item("b.jpg", 1)], media: vec![], size: 0, added: 0, cloud: 0, year: None, taken: None, event: true, day_only: false });
        let c = |keep, size| crate::store::Choice { keep, at: 1, size };
        let review = HashMap::from([("S/a.jpg".to_string(), c(false, Some(1))), ("S/b.jpg".to_string(), c(false, Some(7)))]);
        assert_eq!(set_aside(&sec, "S", &review).iter().map(|i| i.rel.as_str()).collect::<Vec<_>>(), ["a.jpg"]);
    }

    /// Move and Save as a script take only the photos ticked in Move.
    #[test]
    fn only_the_ticked_photos_move_and_go_in_the_script() {
        let mut sec = Section { id: "S".into(), name: String::new(), path: String::new(), drive: String::new(), kind: Kind::Photos, guessed: false, online: true,
            gone: false, works: vec![], files: 0, scanned_at: 0, unlisted: vec![], empty: vec![], whole_tree: false, split_from: None, links: vec![] };
        sec.works.push(crate::scan::Work { id: "S#2019/a".into(), title: String::new(), path: String::new(), folder: vec![], is_folder: true,
            items: vec![item("2019/a.jpg", 0), item("2019/b.jpg", 1), item("2021/c.jpg", 2), item("2021/kept.jpg", 3)], media: vec![], size: 0, added: 0, cloud: 0, year: None, taken: None, event: true, day_only: false });
        let c = crate::store::Choice { keep: false, at: 1, size: None };
        let review = HashMap::from(["S/2019/a.jpg", "S/2019/b.jpg", "S/2021/c.jpg"].map(|k| (k.to_string(), c)));
        let rels = |only: Option<&[&str]>| {
            let only: Option<std::collections::HashSet<String>> = only.map(|o| o.iter().map(|x| x.to_string()).collect());
            to_move(&sec, "S", &review, only.as_ref()).0.into_iter().map(|p| p.rel).collect::<Vec<_>>()
        };
        assert_eq!(rels(None), ["2019/a.jpg", "2019/b.jpg", "2021/c.jpg"], "no list: every photo set aside");
        assert_eq!(rels(Some(&["2019/a.jpg", "2021/c.jpg"])), ["2019/a.jpg", "2021/c.jpg"], "b unticked stays");
        assert_eq!(rels(Some(&["2019/a.jpg", "2021/kept.jpg", "gone.jpg"])), ["2019/a.jpg"], "a ticked path not set aside now never moves");
        assert!(rels(Some(&[])).is_empty(), "nothing ticked: nothing moves");
        let files = rels(Some(&["2019/a.jpg", "2021/c.jpg"]));
        for windows in [true, false] {
            let script = move_script("/p", &files, windows);
            let has = |f: &str| script.contains(&if windows { f.replace('/', "\\") } else { f.to_string() });
            assert!(has("2019/a.jpg") && has("2021/c.jpg"), "{script}");
            assert!(!script.contains("b.jpg") && !script.contains("kept.jpg"), "{script}");
        }
    }

    #[test]
    fn move_scripts_quote_every_name() {
        let files = vec!["2018/it's.jpg".to_string(), "Фото 1.jpg".to_string(), "Mom’s party.jpg".to_string()];
        let ps = move_script(r"D:\Photos & Videos\!x", &files, true);
        assert!(ps.starts_with('\u{feff}'));
        assert!(ps.contains(r"$root = 'D:\Photos & Videos\!x'"));
        assert!(ps.contains(r"'2018\it''s.jpg'"));
        assert!(ps.contains("'Mom’’s party.jpg'"));
        assert!(!ps.contains("Remove-Item"));
        let sh = move_script("/Users/a/Photos", &files, false);
        assert!(sh.contains(r"'2018/it'\''s.jpg'"));
        assert!(sh.contains("mv -n") && !sh.contains("rm "));
    }

    /// Runs the move script for this system on a folder of its own: set-aside files move into
    /// `Not needed` with their subfolders, nothing else moves or goes, and a second run skips them.
    /// On Windows it runs in `Photos.` beside a `Photos` holding the same names too, which it must
    /// never touch, and in both a photo in `2019.` moves, never the one in `2019` beside it.
    #[test]
    #[ignore]
    fn the_move_script_runs() {
        let n = |p: &Path| crate::store::named(p).into_owned();
        let base = std::env::temp_dir().join(format!("fvh-script-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(n(&base.join("dot").join("Photos.")));
        let _ = std::fs::remove_dir_all(&base);
        let names = ["a.jpg", "it's [1].jpg", "Mom’s $x `y.jpg", "Фото.jpg", "2018/c.jpg", "keep.jpg"];
        let mut places = vec![(base.join("plain").join("Photos"), None)];
        if cfg!(windows) {
            places.push((base.join("dot").join("Photos."), Some(base.join("dot").join("Photos"))));
        }
        for (root, sibling) in places {
            for dir in [Some(&root), sibling.as_ref()].into_iter().flatten() {
                for sub in ["2018", "2019.", "2019"] {
                    std::fs::create_dir_all(n(&dir.join(sub))).unwrap();
                }
                std::fs::write(n(&dir.join("2019.").join("d.jpg")), if Some(dir) == sibling.as_ref() { "sibling" } else { "2019./d.jpg" }).unwrap();
                std::fs::write(n(&dir.join("2019").join("d.jpg")), "inner sibling").unwrap();
                for f in names {
                    std::fs::write(n(&dir.join(f)), if Some(dir) == sibling.as_ref() { "sibling" } else { f }).unwrap();
                }
            }
            let aside: Vec<String> = ["a.jpg", "it's [1].jpg", "Mom’s $x `y.jpg", "Фото.jpg", "2018/c.jpg", "2019./d.jpg", "gone.jpg"].map(String::from).to_vec();
            let windows = cfg!(windows);
            let script = base.join(format!("{}.{}", root.parent().unwrap().file_name().unwrap().to_string_lossy(), if windows { "ps1" } else { "sh" }));
            // As `save_move_script` writes it.
            std::fs::write(&script, move_script(&script_root(&root, &aside), &aside, windows)).unwrap();
            let run = || {
                let mut c = if windows {
                    let mut c = std::process::Command::new("powershell");
                    c.args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File"]).arg(&script);
                    c
                } else {
                    let mut c = std::process::Command::new("sh");
                    c.arg(&script);
                    c
                };
                let out = c.stdin(std::process::Stdio::null()).output().unwrap();
                String::from_utf8_lossy(&out.stdout).to_string()
            };
            let first = run();
            println!("{}: {first}", root.display());
            for f in ["a.jpg", "it's [1].jpg", "Mom’s $x `y.jpg", "Фото.jpg", "2018/c.jpg", "2019./d.jpg"] {
                assert!(!n(&root.join(f)).exists(), "{f} still in place");
                assert_eq!(std::fs::read_to_string(n(&root.join(NOT_NEEDED).join(f))).unwrap(), f);
            }
            assert!(n(&root.join("keep.jpg")).exists());
            assert_eq!(std::fs::read_to_string(n(&root.join("2019").join("d.jpg"))).unwrap(), "inner sibling");
            assert!(!n(&root.join(NOT_NEEDED).join("2019")).exists(), "nothing of 2019 moved");
            assert!(first.contains("6 moved") && first.contains("1 skipped"), "{first}");
            let second = run();
            assert!(second.contains("0 moved") && second.contains("7 skipped"), "{second}");
            if let Some(sib) = &sibling {
                for f in names {
                    assert_eq!(std::fs::read_to_string(sib.join(f)).unwrap(), "sibling", "{f} of the sibling");
                }
                assert!(!sib.join(NOT_NEEDED).exists(), "nothing moved into the sibling");
            }
        }
        let _ = std::fs::remove_dir_all(n(&base.join("dot").join("Photos.")));
        std::fs::remove_dir_all(&base).unwrap();
    }

    /// The port against the Photos prototype on real photos, read only. `FVH_SHOTS` is the photo
    /// folder; `FVH_PROTO` a JSON of the prototype's sets (`names`, `best`, `close`) and its numbers
    /// for some photos (`lum`, `sharp`, `dh`, `junk`), made by `tests/proto_sets.py`. Prints how
    /// often both agree.
    #[test]
    #[ignore]
    fn against_the_prototype() {
        let dir = std::path::PathBuf::from(std::env::var("FVH_SHOTS").expect("FVH_SHOTS"));
        let proto: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(std::env::var("FVH_PROTO").expect("FVH_PROTO")).unwrap()).unwrap();
        let (mut n, mut lum, mut sharp, mut dh, mut jk) = (0, 0f64, 0f64, 0u32, 0);
        let mut far = vec![];
        for (name, p) in proto["stats"].as_object().unwrap() {
            let Ok(s) = read(&dir.join(name)) else { continue };
            n += 1;
            lum += (f64::from(s.lum) - p["lum"].as_f64().unwrap()).abs();
            let e = p["sharp"].as_f64().unwrap();
            sharp += (f64::from(s.edge) - e).abs() / e.max(1.0);
            let d = (s.dh ^ u64::from_str_radix(p["dh"].as_str().unwrap(), 16).unwrap()).count_ones();
            dh += d;
            if d > 8 { far.push((name.clone(), d)); }
            let mine = junk(name, &s).map(|j| format!("{j:?}")).unwrap_or_default();
            jk += usize::from(mine.is_empty() == p["junk"].as_str().unwrap().is_empty());
        }
        println!("{n} photos: brightness off by {:.1} on average, edges by {:.0}%, fingerprints by {:.1} bits, junk agrees on {jk}", lum / n as f64, 100.0 * sharp / n as f64, f64::from(dh) / n as f64);
        println!("fingerprints far apart: {far:?}");
        let (mut sets, mut same, mut calls) = (0, 0, 0);
        for set in proto["sets"].as_array().unwrap() {
            let names: Vec<&str> = set["names"].as_array().unwrap().iter().map(|x| x.as_str().unwrap()).collect();
            let Ok(shots) = names.iter().map(|x| read(&dir.join(x))).collect::<Result<Vec<_>, _>>() else { continue };
            let (k, _, _, call) = pick(&shots.iter().collect::<Vec<_>>());
            let close = set["close"].as_bool().unwrap();
            sets += 1;
            calls += usize::from(call == close);
            same += usize::from(call && close || !call && !close && names[k] == set["best"].as_str().unwrap());
        }
        println!("{sets} sets: same close call {calls}, same outcome (both close, or the same Best one) {same}");
    }

    /// Reads every JPEG under `FVH_SHOTS` (read-only) and prints the pictures found saved again.
    #[test]
    #[ignore]
    fn resaved_in_a_real_folder() {
        let dir = std::path::PathBuf::from(std::env::var("FVH_SHOTS").expect("FVH_SHOTS"));
        let mut files = vec![];
        let mut todo = vec![dir];
        while let Some(d) = todo.pop() {
            for e in std::fs::read_dir(&d).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() { todo.push(p) } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("jpg") || x.eq_ignore_ascii_case("jpeg")) { files.push(p) }
            }
        }
        let n = std::thread::available_parallelism().map_or(4, |n| n.get());
        let shots: Vec<(Item, Shot)> = std::thread::scope(|sc| {
            let hs: Vec<_> = files.chunks(files.len().div_ceil(n).max(1)).map(|ch| sc.spawn(move || {
                ch.iter().filter_map(|p| Some((item(&p.to_string_lossy(), 0), read(p).ok()?))).collect::<Vec<_>>()
            })).collect();
            hs.into_iter().flat_map(|h| h.join().unwrap()).collect()
        });
        let pics: Vec<(&Item, Shot)> = shots.iter().map(|(i, s)| (i, s.clone())).collect();
        let groups = resaved(&pics, &HashMap::new());
        let pairs: usize = groups.iter().map(|g| g.len() * (g.len() - 1) / 2).sum();
        println!("{} pictures read, {} groups saved again, {pairs} pairs", pics.len(), groups.len());
        for g in groups.iter().take(25) { println!("{g:?}"); }
    }
}
