//! QA: an independent suite for "a clip among Photos is one frame" (thumbs.rs `clip`,
//! `source_in`), written against the spec, not against the module's own test.
//! A child module of `thumbs`, so it reaches the private planning and making code. Real-ffmpeg
//! tests are `#[ignore]`d: they need `tools/` (`npm run fetch-tools`) and the clips that
//! `.scratch/qa-clips/lib/Trip` holds; run them with `cargo test --lib qa_clips -- --ignored`.

use super::*;
use crate::scan::{Catalog, Item, Section, Work};
use crate::store::Kind;

fn item(name: &str, cloud: bool) -> Item {
    let ext = name.rsplit_once('.').map(|(_, e)| e.to_lowercase()).unwrap_or_default();
    Item { name: name.into(), rel: name.into(), key: name.into(), path: format!("D:\\qa\\{name}"), ext, size: 7, mtime: 9, cloud, title: name.into(), group: String::new(), role: Role::Main, lang: String::new(), at: None }
}

fn event(items: Vec<Item>) -> Work {
    Work { id: "e".into(), title: "e".into(), path: "D:\\qa".into(), folder: vec![], is_folder: true, items, media: vec![], size: 0, added: 0, cloud: 0, year: None, taken: Some((0, 1)), event: true, day_only: false }
}

fn section(works: Vec<Work>, kind: Kind) -> Section {
    Section { id: "s".into(), name: "s".into(), path: "D:\\qa".into(), drive: "D".into(), kind, guessed: false, online: true, gone: false, works, files: 0, scanned_at: 0, unlisted: vec![], empty: vec![], whole_tree: false, split_from: None, links: vec![] }
}

fn ext_of(s: &Source) -> &str {
    s.ext.as_str()
}

/// The event's own picture is a photo whenever it has one, even when a clip comes first.
#[test]
fn qa_mixed_event_picture_is_a_photo_even_after_a_clip() {
    let w = event(vec![item("20240301_090000.mp4", false), item("20240301_091000.jpg", false), item("20240301_092000.mov", false)]);
    let s = sources(&w);
    assert_eq!(ext_of(&s[0]), "jpg", "first source: {:?}", s.iter().map(|s| &s.path).collect::<Vec<_>>());
    assert!(!s[0].frame);
    // The clips that follow, as fallbacks, are one frame each and never a course strip.
    let clips: Vec<&Source> = s.iter().filter(|s| class_of(&s.ext) == Some(Class::Video)).collect();
    assert_eq!(clips.len(), 2);
    assert!(clips.iter().all(|c| c.frame && c.shots.is_empty()));
}

/// Each grid item: photos keep their photo key, clips get the one-frame key, different from the
/// old strip key (`t2`), so an old strip is never shown for a clip.
#[test]
fn qa_mixed_event_grid_items() {
    let w = event(vec![item("a.jpg", false), item("b.mp4", false), item("c.heic", false), item("d.MOV", false)]);
    assert!(grid(&w), "a mixed event is a grid");
    for i in &w.items {
        let s = source_in(&w, i);
        if class_of(&i.ext) == Some(Class::Video) {
            assert!(s.frame, "{} should be one frame", i.name);
            assert_ne!(s.key, key_of(i), "{} still uses the strip key", i.name);
            assert_eq!(s.key, fnv(&format!("c1|{}|{}|{}", i.path, i.size, i.mtime)));
        } else {
            assert!(!s.frame);
            assert_eq!(s.key, key_of(i), "{} photo key changed", i.name);
        }
    }
}

/// An event of clips only: its picture is its first clip's frame, the same file as that clip's
/// grid thumbnail (one file, made once).
#[test]
fn qa_clips_only_event_shares_its_first_clip_frame() {
    let w = event(vec![item("a.mp4", false), item("b.mp4", false), item("c.mp4", false)]);
    let s = sources(&w);
    assert!(s.iter().all(|s| s.frame && s.shots.is_empty()));
    assert_eq!(s.len(), 2, "two clips to try, as before");
    assert_eq!(s[0].key, source_in(&w, &w.items[0]).key);
}

/// A cloud-only clip is never a source, in the event's picture or the grid.
#[test]
fn qa_cloud_clip_is_never_read() {
    let w = event(vec![item("a.mp4", true), item("b.jpg", false)]);
    assert!(sources(&w).iter().all(|s| !s.path.ends_with("a.mp4")));
}

/// Outside events nothing changed: a folder of several videos is a course strip, a lone video a
/// strip of its own (frame false), with their old keys.
#[test]
fn qa_non_event_videos_unchanged() {
    let mut w = event(vec![item("01.mp4", false), item("02.mp4", false), item("03.mp4", false)]);
    w.event = false;
    w.taken = None;
    w.media = vec!["Watching".into()];
    let s = sources(&w);
    assert!(!s[0].shots.is_empty(), "course strip first");
    assert!(s[0].key != source_in(&w, &w.items[0]).key);
    assert!(!s[1].frame && s[1].key == key_of(&w.items[0]));
    let mut one = w.clone();
    one.is_folder = false;
    one.items.truncate(1);
    let s = sources(&one);
    assert_eq!(s.len(), 1);
    assert!(!s[0].frame && s[0].shots.is_empty() && s[0].key == key_of(&one.items[0]));
    // A non-event folder of videos is not a grid, so `source_in` is only reached for events
    // and books/pictures; it still returns the plain source here.
    assert!(!source_in(&w, &w.items[1]).frame);
}

fn scratch(sub: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_MANIFEST_DIR")).join("../.scratch/qa-clips").join(sub);
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// A cache from before the change: the old strip entries of an event's clips (Work picture and
/// grid item) are dropped from the index and their files deleted; a non-event video's strip and
/// the photos' thumbnails stay; the clips' new one-frame keys are planned.
#[test]
fn qa_plan_drops_old_strip_entries_of_event_clips() {
    let dir = scratch("plan-old-index");
    let ev = event(vec![item("a.jpg", false), item("b.mp4", false), item("c.mp4", false)]);
    let mut lesson = event(vec![item("lesson.mp4", false)]);
    lesson.id = "lesson".into();
    lesson.event = false;
    lesson.is_folder = false;
    lesson.taken = None;
    lesson.items[0].path = "D:\\qa\\courses\\lesson.mp4".into();
    let old_clip = key_of(&ev.items[1]);
    let old_clip2 = key_of(&ev.items[2]);
    let photo = key_of(&ev.items[0]);
    let keep = key_of(&lesson.items[0]);
    let mut files = HashMap::new();
    for k in [&old_clip, &old_clip2, &photo, &keep] {
        files.insert(k.clone(), Entry { ok: Some((100, 50)), no: None, n: if k == &photo { 1 } else { 5 } });
        let f = file_of(&dir, k);
        std::fs::create_dir_all(f.parent().unwrap()).unwrap();
        std::fs::write(&f, b"x").unwrap();
    }
    std::fs::write(index_file(&dir), serde_json::to_string(&Index { v: 1, files }).unwrap()).unwrap();
    let t = Thumbs::new(dir.clone());
    let cat = Catalog { v: 1, sections: vec![section(vec![ev.clone()], Kind::Photos), section(vec![lesson], Kind::Courses)] };
    t.plan(&cat);
    let g = t.inner.lock().unwrap();
    assert!(!g.index.files.contains_key(&old_clip), "old strip entry of a clip kept");
    assert!(!g.index.files.contains_key(&old_clip2));
    assert!(!file_of(&dir, &old_clip).exists(), "old strip file not deleted");
    assert!(!file_of(&dir, &old_clip2).exists());
    assert!(g.index.files.contains_key(&photo) && file_of(&dir, &photo).exists(), "photo thumbnail lost");
    assert!(g.index.files.contains_key(&keep) && file_of(&dir, &keep).exists(), "non-event video strip lost");
    // The event's picture is Ready from the photo; the clips' new frames wait to be made.
    let done = g.done.get("e").expect("event picture ready from its photo");
    assert_eq!(done.key, photo);
    let lazy_b = g.lazy.get(&format!("e#{}", ev.items[1].key)).expect("clip b planned");
    assert!(lazy_b[0].frame && lazy_b[0].key == source_in(&ev, &ev.items[1]).key);
    assert!(g.done.values().all(|t| t.key != old_clip && t.key != old_clip2), "an old strip is shown");
    drop(g);
    // Planning again changes nothing (idempotent): the index is the same size.
    let n = t.inner.lock().unwrap().index.files.len();
    t.plan(&cat);
    assert_eq!(t.inner.lock().unwrap().index.files.len(), n);
}

fn lib(name: &str) -> Item {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../.scratch/qa-clips/lib/Trip").join(name);
    let p = std::fs::canonicalize(&p).unwrap_or(p);
    let mut i = item(name, false);
    i.path = p.to_string_lossy().trim_start_matches("\\\\?\\").to_string();
    i
}

/// With the real ffmpeg: a clip of an event is one frame (n == 1), fits the box and is not
/// blank; the same file outside an event is still a strip (n > 1).
#[test]
#[ignore]
fn qa_real_clip_is_one_frame() {
    let dir = scratch("real-ffmpeg");
    let t = Thumbs::new(dir.clone());
    let w = event(vec![lib("20240301_100000.jpg"), lib("20240301_101500.mp4"), lib("20240301_102000.mov")]);
    for i in &w.items[1..] {
        let s = source_in(&w, i);
        let (img, n) = t.make(&s).unwrap_or_else(|e| panic!("{}: {e:?}", i.name));
        let (tw, th, tn) = t.write(&s.key, &img, n).unwrap();
        println!("{} -> {tw}x{th} n={tn} at {}", i.name, file_of(&dir, &s.key).display());
        assert_eq!(n, 1);
        assert_eq!(tn, 1);
        assert!(tw <= BOX && th <= BOX && tw > 0);
        assert!(!blank(&img));
        let strip = t.make(&source(i)).unwrap();
        println!("{} as a lesson -> n={}", i.name, strip.1);
        assert!(strip.1 > 1, "control: outside an event it is a strip");
    }
}

/// With the real ffmpeg: a broken .mp4 in an event gives a Bad failure (remembered, not tried
/// again on every start), and the event's picture still comes from its photo.
#[test]
#[ignore]
fn qa_real_broken_clip_fails_once() {
    let dir = scratch("real-broken");
    let t = Thumbs::new(dir.clone());
    let w = event(vec![lib("20240301_102500.mp4"), lib("20240301_103000.jpg")]);
    let s = source_in(&w, &w.items[0]);
    let r = t.make(&s);
    println!("broken: {:?}", r.as_ref().map(|(_, n)| *n).map_err(|e| format!("{e:?}")));
    assert!(matches!(r, Err(Fail::Bad(_))), "a broken clip should be a Bad failure");
    let got = t.make_for("e#broken", std::slice::from_ref(&s));
    assert!(got.is_none());
    let e = t.inner.lock().unwrap().index.files.get(&s.key).cloned().expect("failure remembered");
    assert!(e.ok.is_none() && e.no.is_some());
    let pic = t.make_for("e", &sources(&w)).expect("event picture from the photo");
    assert_eq!(pic.n, 1);
    assert_eq!(pic.key, key_of(&w.items[1]));
}
