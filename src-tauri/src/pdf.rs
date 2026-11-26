//! PDF covers through PDFium. PDFium runs in a second copy of the app (`--pdf-cover`), so a file
//! that hangs or crashes it is killed after the time limit and never takes the app down.

use crate::tools::{self, Fail};
use image::DynamicImage;
use pdfium_render::prelude::*;
use std::io::Write;
use std::path::Path;
use std::time::Duration;

pub const FLAG: &str = "--pdf-cover";
/// Asks the second copy for the page count instead of the cover.
const PAGES: &str = "--pages";

fn exe() -> std::io::Result<std::path::PathBuf> {
    let exe = std::env::current_exe()?;
    // A test runs from a folder under target/debug (deps/, or build/<crate>/<hash>/ in newer
    // cargo); the app is built in target/debug by `cargo build`: the nearest folder above holding it.
    #[cfg(test)]
    let exe = {
        let app = format!("filesviewhub{}", std::env::consts::EXE_SUFFIX);
        exe.ancestors().skip(1).map(|d| d.join(&app)).find(|p| p.is_file()).unwrap_or(exe)
    };
    Ok(exe)
}

/// How many pages a PDF has, from a second copy of the app.
pub fn pages(dll: &Path, file: &Path) -> Result<u32, Fail> {
    let file = crate::store::named(file);
    let out = tools::run(&exe()?, &[FLAG.as_ref(), dll.as_os_str(), file.as_os_str(), PAGES.as_ref()], Duration::from_secs(60))?;
    String::from_utf8_lossy(&out).trim().parse().map_err(|_| Fail::Bad("no page count".into()))
}

/// The first of pages 1 to 3 that is not blank, from a second copy of the app.
pub fn cover(dll: &Path, file: &Path) -> Result<DynamicImage, Fail> {
    let file = crate::store::named(file);
    let out = tools::run(&exe()?, &[FLAG.as_ref(), dll.as_os_str(), file.as_os_str()], Duration::from_secs(60))?;
    image::load_from_memory(&out).map_err(|e| Fail::Bad(e.to_string()))
}

/// The second copy: writes the cover as PNG and exits. A message with "Cannot open" means the
/// file or PDFium could not be read now, and is tried again later.
pub fn helper(args: &[String]) -> i32 {
    let (Some(dll), Some(file)) = (args.first(), args.get(1)) else { return 64 };
    let fail = |code: i32, why: String| {
        eprintln!("{why}");
        code
    };
    // Only the PDFium in the app's own tools folder is loaded, never another library named here.
    let same = |a: &Path, b: &Path| a.canonicalize().ok().is_some_and(|a| b.canonicalize().ok() == Some(a));
    if !tools::Tools::find().path("pdfium", 0).is_some_and(|own| same(Path::new(dll), &own)) {
        return fail(64, format!("Not the app's own PDFium: {dll}"));
    }
    let pdfium = match Pdfium::bind_to_library(dll) {
        Ok(b) => Pdfium::new(b),
        Err(e) => return fail(5, format!("Cannot open PDFium: {e}")),
    };
    let f = match std::fs::File::open(file) {
        Ok(f) => f,
        Err(e) => return fail(4, format!("Cannot open the file: {e}")),
    };
    let doc = match pdfium.load_pdf_from_reader(f, None) {
        Ok(d) => d,
        Err(e) => return fail(2, format!("PDFium cannot read it: {e:?}")),
    };
    let box_ = crate::thumbs::BOX as i32;
    let cfg = PdfRenderConfig::new().set_target_width(box_).set_maximum_height(box_);
    let pages = doc.pages();
    if args.get(2).is_some_and(|a| a == PAGES) {
        println!("{}", pages.len());
        return 0;
    }
    for i in 0..pages.len().min(3) {
        let Ok(page) = pages.get(i) else { continue };
        let Ok(img) = page.render_with_config(&cfg).and_then(|b| b.as_image()) else { continue };
        if crate::thumbs::blank(&img) {
            continue;
        }
        let mut png = std::io::Cursor::new(vec![]);
        if img.write_to(&mut png, image::ImageFormat::Png).is_err() {
            return fail(2, "the page could not be encoded".into());
        }
        let mut out = std::io::stdout().lock();
        return if out.write_all(png.get_ref()).and_then(|_| out.flush()).is_ok() { 0 } else { 1 };
    }
    fail(3, "blank".into())
}
