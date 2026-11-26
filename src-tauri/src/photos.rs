//! Photos (`decisions/2026/10/04/an-event-is-the-work-in-photos.md`, `decisions/2026/10/04/photos-take-no-marks-review-is-the-progress.md`, `decisions/2026/10/08/photos-is-a-section-kind-in-the-app.md`): when each Photo was taken, and Events, the runs of Photos with
//! no long gap between them. The date comes from the file name, then the file (an Insta360
//! camera's trailer, EXIF), never the modified time. Times are kept as seconds since 1970 of the time the camera showed, read as if UTC, since
//! neither a name nor EXIF says the time zone.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// The gap that starts a new Event unless Settings says otherwise, in hours.
pub const GAP_HOURS: u32 = 4;
#[cfg(test)]
const GAP: i64 = GAP_HOURS as i64 * 3600;
const MONTHS: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
const DAYS: [&str; 7] = ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"];
/// Insta360 photos, videos and their preview copies, which end in the camera's trailer.
const INSTA360: &[&str] = &["insp", "insv", "lrv"];
/// The last 32 bytes of an Insta360 file.
const INSTA360_MAGIC: &[u8] = b"8db42d694ccc418790edff439fe026bf";
/// Names cameras and phones give their files, for the kind guess.
const CAMERA: &[&str] = &["img", "dsc", "dscn", "dscf", "pxl", "vid", "mvimg", "mov", "gopr", "dji", "p_", "photo", "pic", "screenshot", "whatsapp"];

/// Seconds since 1970 of a date and time, or None when it is no real date.
pub fn secs(y: i64, mo: i64, d: i64, h: i64, mi: i64, s: i64) -> Option<i64> {
    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let days_in = [31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    if !(1990..=2100).contains(&y) || !(1..=12).contains(&mo) || d < 1 || d > days_in[(mo - 1) as usize] || h > 23 || mi > 59 || s > 59 {
        return None;
    }
    // Days from 1970-01-01 (Howard Hinnant's days_from_civil).
    let y2 = if mo <= 2 { y - 1 } else { y };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let doy = (153 * (mo + if mo > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some((era * 146_097 + doe - 719_468) * 86_400 + h * 3600 + mi * 60 + s)
}

/// Year, month, day, hour, minute and weekday (0 is Thursday, 1 January 1970) of a time.
pub fn parts(t: i64) -> (i64, i64, i64, i64, i64, usize) {
    let (days, rest) = (t.div_euclid(86_400), t.rem_euclid(86_400));
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(m <= 2), m, d, rest / 3600, rest % 3600 / 60, days.rem_euclid(7) as usize)
}

fn num(b: &[u8], at: usize, n: usize) -> Option<i64> {
    let s = b.get(at..at + n)?;
    s.iter().all(u8::is_ascii_digit).then(|| s.iter().fold(0, |a, c| a * 10 + i64::from(c - b'0')))
}

/// The date and time taken, from the names phones and apps give files. A name with a date but no
/// time gives none: an invented time would put the Photo in a false Event, so EXIF decides.
pub fn name_date(name: &str) -> Option<i64> {
    let b = name.as_bytes();
    for i in 0..b.len() {
        if i > 0 && b[i - 1].is_ascii_digit() {
            continue;
        }
        // 2018-02-12 19.32.38, 2018-02-12_19-32-38, 2018-02-12T19:32:38
        if let (Some(y), Some(mo), Some(d)) = (num(b, i, 4), num(b, i + 5, 2), num(b, i + 8, 2)) && b[i + 4] == b'-' && b[i + 7] == b'-' {
            let sep = |k: usize, set: &[u8]| b.get(k).is_some_and(|c| set.contains(c));
            if sep(i + 10, b" _T") && sep(i + 13, b"-:.") && sep(i + 16, b"-:.")
                && let (Some(h), Some(mi), Some(s)) = (num(b, i + 11, 2), num(b, i + 14, 2), num(b, i + 17, 2))
                && let Some(t) = secs(y, mo, d, h, mi, s)
            {
                return Some(t);
            }
            // WhatsApp and macOS: 2018-02-12 at 19.32.38, or at 7.32.38 PM
            if b.get(i + 10..i + 14) == Some(b" at ") {
                let k = i + 14;
                let hl = if num(b, k, 2).is_some() { 2 } else { 1 };
                if let (Some(h), Some(mi), Some(s)) = (num(b, k, hl), num(b, k + hl + 1, 2), num(b, k + hl + 4, 2)) {
                    let rest = String::from_utf8_lossy(&b[(k + hl + 6).min(b.len())..]).trim_start().to_uppercase();
                    let h = if rest.starts_with("PM") { h % 12 + 12 } else if rest.starts_with("AM") { h % 12 } else { h };
                    if let Some(t) = secs(y, mo, d, h, mi, s) {
                        return Some(t);
                    }
                }
            }
        }
        // IMG_20180212_193238, PXL_20210101_101112345, Screenshot_20190305-081500
        if let (Some(y), Some(mo), Some(d)) = (num(b, i, 4), num(b, i + 4, 2), num(b, i + 6, 2))
            && b.get(i + 8).is_some_and(|c| *c == b'_' || *c == b'-')
            && let (Some(h), Some(mi), Some(s)) = (num(b, i + 9, 2), num(b, i + 11, 2), num(b, i + 13, 2))
            && let Some(t) = secs(y, mo, d, h, mi, s)
        {
            return Some(t);
        }
    }
    None
}

/// The day taken, from a name that gives a date but no time (WhatsApp's IMG-20180212-WA0001,
/// 2018-02-12.jpg): the day's start by the camera's clock. Used only when neither the name's time
/// nor EXIF is there. The date stands alone in the name, so a long number is no date.
pub fn name_day(name: &str) -> Option<i64> {
    let b = name.as_bytes();
    let digit = |k: usize| b.get(k).is_some_and(u8::is_ascii_digit);
    for i in 0..b.len() {
        if i > 0 && digit(i - 1) {
            continue;
        }
        let ymd = if num(b, i, 8).is_some() && !digit(i + 8) {
            Some((num(b, i, 4)?, num(b, i + 4, 2)?, num(b, i + 6, 2)?))
        } else if let (Some(y), Some(m), Some(d)) = (num(b, i, 4), num(b, i + 5, 2), num(b, i + 8, 2)) {
            (b[i + 4] == b'-' && b[i + 7] == b'-' && !digit(i + 10)).then_some((y, m, d))
        } else {
            None
        };
        if let Some((y, m, d)) = ymd && (1990..=2100).contains(&y) && let Some(t) = secs(y, m, d, 0, 0, 0) {
            return Some(t);
        }
    }
    None
}

/// A name a camera or phone gives: one with a date, or a camera's prefix.
pub fn camera_name(name: &str) -> bool {
    let n = name.to_lowercase();
    name_date(name).is_some() || CAMERA.iter().any(|p| n.starts_with(p))
}

/// The time EXIF says a picture was taken, reading only the start of the file.
fn exif_date(path: &Path) -> Option<i64> {
    let f = std::fs::File::open(crate::store::named(path)).ok()?;
    let ex = exif::Reader::new().read_from_container(&mut std::io::BufReader::new(f)).ok()?;
    let field = ex.get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY).or_else(|| ex.get_field(exif::Tag::DateTime, exif::In::PRIMARY))?;
    let exif::Value::Ascii(v) = &field.value else { return None };
    let b = v.first()?;
    // 2018:02:12 19:32:38
    secs(num(b, 0, 4)?, num(b, 5, 2)?, num(b, 8, 2)?, num(b, 11, 2)?, num(b, 14, 2)?, num(b, 17, 2)?)
}

/// The time an Insta360 camera's clock showed when it took a photo or started a video. The camera
/// ends each file with a trailer: records, each followed by its id (u16) and length (u32), little
/// endian, then 32 bytes, the trailer's length, its version and the magic. The info record (0x101)
/// is protobuf; its field 7 is the time as the number 20210304060708. Reads only the file's end.
fn insta360_date(path: &Path) -> Option<i64> {
    use std::io::{Read, Seek, SeekFrom};
    let mut f = std::fs::File::open(crate::store::named(path)).ok()?;
    let len = f.metadata().ok()?.len();
    let mut tail = [0u8; 78];
    f.seek(SeekFrom::Start(len.checked_sub(78)?)).ok()?;
    f.read_exact(&mut tail).ok()?;
    if &tail[46..] != INSTA360_MAGIC {
        return None;
    }
    // Where the trailer starts: no record lies before it.
    let start = len.checked_sub(u64::from(u32::from_le_bytes([tail[38], tail[39], tail[40], tail[41]])))?;
    // Where the header of the record nearest the end starts; the records run back from there.
    let mut at = len - 78;
    for _ in 0..32 {
        let mut h = [0u8; 6];
        f.seek(SeekFrom::Start(at)).ok()?;
        f.read_exact(&mut h).ok()?;
        let n = u64::from(u32::from_le_bytes([h[2], h[3], h[4], h[5]]));
        if n == 0 || at < start + n {
            return None;
        }
        if u16::from_le_bytes([h[0], h[1]]) == 0x101 {
            let mut b = vec![0; usize::try_from(n).ok().filter(|n| *n <= 1 << 20)?];
            f.seek(SeekFrom::Start(at - n)).ok()?;
            f.read_exact(&mut b).ok()?;
            return insta360_info_date(&b);
        }
        at = at.checked_sub(n + 6)?;
    }
    None
}

/// Field 7 of an Insta360 info record, the time as 20210304060708.
fn insta360_info_date(b: &[u8]) -> Option<i64> {
    let varint = |i: &mut usize| -> Option<u64> {
        let mut v = 0u64;
        for shift in (0..64).step_by(7) {
            let c = *b.get(*i)?;
            *i += 1;
            v |= u64::from(c & 0x7f) << shift;
            if c < 0x80 {
                return Some(v);
            }
        }
        None
    };
    let mut i = 0;
    while i < b.len() {
        let key = varint(&mut i)?;
        match key & 7 {
            0 => {
                let v = varint(&mut i)?;
                if key >> 3 == 7 {
                    let v = i64::try_from(v).ok()?;
                    return secs(v / 10_000_000_000, v / 100_000_000 % 100, v / 1_000_000 % 100, v / 10_000 % 100, v / 100 % 100, v % 100);
                }
            }
            1 => i += 8,
            2 => {
                let n = usize::try_from(varint(&mut i)?).ok()?;
                i = i.checked_add(n)?;
            }
            5 => i += 4,
            _ => return None,
        }
    }
    None
}

/// EXIF names the camera or phone that took a picture held in memory.
pub fn exif_camera(bytes: &[u8]) -> bool {
    let Ok(ex) = exif::Reader::new().read_from_container(&mut std::io::Cursor::new(bytes)) else { return false };
    [exif::Tag::Make, exif::Tag::Model].iter().any(|t| {
        ex.get_field(*t, exif::In::PRIMARY).is_some_and(|f| matches!(&f.value, exif::Value::Ascii(v) if v.iter().any(|s| s.iter().any(|c| !c.is_ascii_whitespace() && *c != 0))))
    })
}

/// EXIF dates read before, by path, size and modified time, kept in the cache as `photo-dates.json`
/// so a folder of thousands is read once.
#[derive(Serialize, Deserialize, Default)]
pub struct Dates {
    v: u32,
    by: HashMap<String, Option<i64>>,
    #[serde(skip)]
    file: Option<PathBuf>,
    #[serde(skip)]
    changed: bool,
}

impl Dates {
    pub fn load(cache: Option<&Path>) -> Dates {
        let file = cache.map(|c| c.join("photo-dates.json"));
        let d: Dates = file.as_ref().map(|f| crate::store::load(f)).unwrap_or_default();
        let by = if d.v == 1 { d.by } else { HashMap::new() };
        Dates { v: 1, by, file, changed: false }
    }

    /// When a Photo was taken: its name, then, for a file on this PC, an Insta360 file's trailer
    /// or a picture's EXIF. A file only in the cloud is never opened.
    pub fn taken(&mut self, name: &str, path: &Path, size: u64, mtime: u64, cloud: bool, picture: bool) -> Option<i64> {
        if let Some(t) = name_date(name) {
            return Some(t);
        }
        let insta = path.extension().is_some_and(|e| INSTA360.iter().any(|x| e.eq_ignore_ascii_case(x)));
        if !(picture || insta) || cloud {
            return None;
        }
        let key = format!("{}|{size}|{mtime}", path.display());
        if let Some(t) = self.by.get(&key) {
            return *t;
        }
        let t = if insta { insta360_date(path) } else { None }.or_else(|| if picture { exif_date(path) } else { None });
        self.by.insert(key, t);
        self.changed = true;
        t
    }

    pub fn save(&self) {
        if let (true, Some(f)) = (self.changed, &self.file) && let Err(e) = crate::store::save(f, self) {
            log::warn!("cannot save photo-dates.json: {e}");
        }
    }
}

/// Runs of times with no gap longer than `gap`: each run as the indexes of its times, which must
/// be in order.
pub fn runs(times: &[i64], gap: i64) -> Vec<std::ops::Range<usize>> {
    let mut out = vec![];
    let mut start = 0;
    for i in 1..=times.len() {
        if i == times.len() || times[i] - times[i - 1] > gap {
            if i > start {
                out.push(start..i);
            }
            start = i;
        }
    }
    out
}

/// An Event's title: its day, or its first and last day.
pub fn title(a: i64, b: i64) -> String {
    let day = |t: i64| {
        let (y, m, d, _, _, wd) = parts(t);
        format!("{} {d} {} {y}", DAYS[wd], &MONTHS[(m - 1) as usize][..3])
    };
    if a.div_euclid(86_400) == b.div_euclid(86_400) { day(a) } else { format!("{} to {}", day(a), day(b)) }
}

/// The year and month folders an Event sits in: "2018", "02 February".
pub fn folder(t: i64) -> Vec<String> {
    let (y, m, ..) = parts(t);
    vec![y.to_string(), format!("{m:02} {}", MONTHS[(m - 1) as usize])]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn show(t: Option<i64>) -> Option<String> {
        t.map(|t| {
            let (y, m, d, h, mi, _) = parts(t);
            format!("{y}-{m:02}-{d:02} {h:02}:{mi:02}:{:02}", t.rem_euclid(60))
        })
    }

    #[test]
    fn dates_come_from_the_names_phones_give() {
        let cases = [
            ("2018-02-12 19.32.38.jpg", Some("2018-02-12 19:32:38")),
            ("2018-02-12 19.32.38-1.jpg", Some("2018-02-12 19:32:38")),
            ("IMG_20180212_193238.jpg", Some("2018-02-12 19:32:38")),
            ("VID_20180212_193238.mp4", Some("2018-02-12 19:32:38")),
            ("PXL_20210101_101112345.jpg", Some("2021-01-01 10:11:12")),
            ("Screenshot_20190305-081500.png", Some("2019-03-05 08:15:00")),
            ("20180212_193238.jpg", Some("2018-02-12 19:32:38")),
            ("WhatsApp Image 2018-02-12 at 19.32.38.jpeg", Some("2018-02-12 19:32:38")),
            ("WhatsApp Image 2018-02-12 at 7.32.38 PM.jpeg", Some("2018-02-12 19:32:38")),
            ("Screenshot 2023-01-01 at 10.11.12.png", Some("2023-01-01 10:11:12")),
            ("2018-02-12_19-32-38.jpg", Some("2018-02-12 19:32:38")),
            ("IMG-20180212-WA0001.jpg", None),
            ("DSC_0042.JPG", None),
            ("IMG_99991399_250000.jpg", None),
            ("Фото 2019-07-01 12.00.00.jpg", Some("2019-07-01 12:00:00")),
        ];
        for (name, want) in cases {
            assert_eq!(show(name_date(name)).as_deref(), want, "{name}");
        }
    }

    #[test]
    fn insta360_files_give_the_time_in_their_trailer() {
        // Cut from an Insta360 X5's files by tests/make_insta360_fixtures.py, with made-up times.
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/insta360");
        let cases = [("photo.insp", "2021-03-04 05:06:07"), ("video.insv", "2021-03-04 06:07:08"), ("preview.lrv", "2021-03-04 07:08:09")];
        for (name, want) in cases {
            assert_eq!(show(insta360_date(&dir.join(name))).as_deref(), Some(want), "{name}");
            let mut d = Dates::load(None);
            let t = d.taken(name, &dir.join(name), 1, 1, false, name.ends_with(".insp"));
            assert_eq!(show(t).as_deref(), Some(want), "{name} by taken");
            assert_eq!(d.taken(name, &dir.join(name), 1, 1, true, false), None, "{name} only in the cloud is not opened");
        }
        // A name with the time still comes first.
        let mut d = Dates::load(None);
        assert_eq!(show(d.taken("VID_20200101_101010_00_001.insv", &dir.join("video.insv"), 1, 1, false, false)).as_deref(), Some("2020-01-01 10:10:10"));
    }

    #[test]
    fn a_broken_insta360_trailer_gives_no_date() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../.scratch/insta360-test");
        let _ = std::fs::create_dir_all(&dir);
        let good = std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/insta360/video.insv")).unwrap();
        let try_bytes = |name: &str, b: &[u8]| {
            let p = dir.join(name);
            std::fs::write(&p, b).unwrap();
            insta360_date(&p)
        };
        assert!(try_bytes("good.insv", &good).is_some());
        // No magic, too short, a record longer than the file, a cut info record.
        let mut no_magic = good.clone();
        *no_magic.last_mut().unwrap() = b'x';
        assert_eq!(try_bytes("no-magic.insv", &no_magic), None);
        assert_eq!(try_bytes("short.insv", &good[good.len() - 40..]), None);
        let mut long = good.clone();
        let at = long.len() - 78 + 2;
        long[at..at + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(try_bytes("long.insv", &long), None);
        // A trailer length that leaves the info record outside the trailer.
        let mut short_trailer = good.clone();
        let at = short_trailer.len() - 40;
        short_trailer[at..at + 4].copy_from_slice(&400u32.to_le_bytes());
        assert_eq!(try_bytes("short-trailer.insv", &short_trailer), None);
        assert_eq!(insta360_info_date(&[0x38, 0x80]), None);
        assert_eq!(insta360_info_date(&[0x12, 0x7f, 0x01]), None);
        // A time that is no real date.
        assert_eq!(insta360_info_date(&[0x38, 0x00]), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_name_with_only_the_day_gives_that_day() {
        let day = |n: &str| show(name_day(n)).map(|s| s[..10].to_string());
        assert_eq!(day("IMG-20200705-WA0012.jpg").as_deref(), Some("2020-07-05"));
        assert_eq!(day("VID-20191231-WA0003.mp4").as_deref(), Some("2019-12-31"));
        assert_eq!(day("2018-02-12.jpg").as_deref(), Some("2018-02-12"));
        assert_eq!(day("DSC_0042.JPG"), None);
        assert_eq!(day("IMG_99991399.jpg"), None);
        assert_eq!(day("123456789.jpg"), None);
        assert_eq!(day("00001234.jpg"), None);
    }

    #[test]
    #[expect(clippy::single_range_in_vec_init, reason = "one Event holding Photo 0")]
    fn events_split_at_a_long_gap() {
        let h = 3600;
        assert_eq!(runs(&[0, h, 6 * h, 6 * h + 10], GAP), [0..2, 2..4]);
        assert_eq!(runs(&[0], GAP), [0..1]);
        assert!(runs(&[], GAP).is_empty());
    }

    #[test]
    fn titles_and_folders_read_as_dates() {
        let a = secs(2019, 1, 2, 17, 44, 0).unwrap();
        assert_eq!(title(a, a + 7000), "Wed 2 Jan 2019");
        assert_eq!(title(a, a + 86_400), "Wed 2 Jan 2019 to Thu 3 Jan 2019");
        assert_eq!(folder(a), ["2019", "01 January"]);
    }
}
