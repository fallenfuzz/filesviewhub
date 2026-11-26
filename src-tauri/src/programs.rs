//! Settings, Programs: every type of file in the catalog, the program the system opens it in,
//! the programs it offers for it, and the person's own choice (`decisions/2026/10/05/filesviewhub-is-a-tauri-app-shipped-portable.md`). The list is made each
//! time it is asked for, so a new type or a new system default shows at once; only the person's
//! choices are stored (`open_with` in settings.json).
//!
//! A choice is a program's id: on Windows the shell handler's name (an `.exe` path), or for a
//! Store app `pkg:<package family>/<name>`, which survives app updates and a change of display
//! language; on macOS the `.app` path. A file opens in it through the shell handler, which
//! starts Store apps too; an `.exe` the person picked that the system does not offer is started
//! directly. Programs that run scripts are never offered for a type the app does not start.

use crate::scan::{class_of, Catalog, Class, CAPTIONS};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};

/// Types Windows or macOS would run rather than show.
pub const RUNS: [&str; 70] = ["exe", "bat", "cmd", "com", "ps1", "psm1", "vbs", "vbe", "js", "jse", "wsf", "wsh", "msi", "msp", "lnk", "scr", "hta", "pif", "reg", "cpl", "jar", "app", "command", "sh", "py", "pyw", "chm", "msc", "url", "appref-ms", "application", "settingcontent-ms", "library-ms", "search-ms", "iqy", "searchconnector-ms", "rdp", "pyz", "pyzw", "theme", "themepack", "deskthemepack", "appinstaller", "msix", "msixbundle", "appx", "appxbundle", "msu", "wsb", "jnlp", "fileloc", "webloc", "inetloc", "mpkg", "scf", "wsc", "sct", "inf", "psc1", "ps1xml", "xll", "gadget", "diagcab", "terminal", "workflow", "tool", "pkg", "desktop", "appimage", "run"];

/// Programs that run what they open: never offered for a type in `RUNS`, so a click on a
/// script cannot start it by way of the Change menu.
const INTERPRETERS: [&str; 25] = ["powershell.exe", "pwsh.exe", "powershell_ise.exe", "wscript.exe", "cscript.exe", "cmd.exe", "mshta.exe", "msiexec.exe", "rundll32.exe", "regedit.exe", "java.exe", "javaw.exe", "bash.exe", "explorer.exe", "python.exe", "pythonw.exe", "py.exe", "pyw.exe", "node.exe", "hh.exe", "mmc.exe", "regsvr32.exe", "wsl.exe", "conhost.exe", "wt.exe"];

/// Whether `program` may not open files of type `ext`: a program or script type (`RUNS`) is never
/// opened by a program that runs what it opens. The page offers none; this holds whatever it sends.
pub fn refused(ext: &str, program: &str) -> bool {
    RUNS.contains(&ext) && interpreter(program)
}

/// Types the catalog does not list as Works that still open from a click: camera raw photos,
/// camcorder video, and documents that hold no program.
pub const OPENS: [&str; 19] = ["cr2", "cr3", "nef", "arw", "dng", "raf", "orf", "rw2", "heif", "avif", "mts", "m2ts", "vob", "csv", "ods", "vsd", "vsdx", "vss", "pub"];

/// Whether a click shows a file of type `ext` in its folder instead of opening it, when the
/// person picked no program for its type. Only the types the catalog knows (video, audio,
/// pictures, books, captions) and `OPENS` open directly, and of those not a program or script
/// type (`RUNS`, `.chm`). Every other type is shown, so a type missing from `RUNS` is shown too,
/// and so is a file with no type, which macOS and Linux run when it is executable.
pub fn reveals(ext: &str) -> bool {
    RUNS.contains(&ext) || (class_of(ext).is_none() && !CAPTIONS.contains(&ext) && !OPENS.contains(&ext))
}

/// The groups, in the order the list shows them.
pub const GROUPS: [&str; 5] = ["Video", "Audio", "Pictures", "Books & documents", "Other types"];

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Program {
    /// What `open_with` stores; empty for a default the shell names but offers no handler for.
    pub id: String,
    pub name: String,
}

#[derive(Serialize, Debug)]
pub struct Row {
    pub ext: String,
    pub group: &'static str,
    /// Files of this type in the catalog (Works and unlisted files).
    pub files: usize,
    /// Shown in its folder when clicked, never opened, unless the person picked a program (`reveals`).
    pub reveals: bool,
    /// The person's choice, if any.
    pub yours: Option<Program>,
    /// The choice is a program no longer on this PC: neither offered by the system nor a file.
    pub gone: bool,
    /// The system's program for it; None when it has none.
    pub default: Option<Program>,
    /// Programs the system offers for it, the default among them.
    pub choices: Vec<Program>,
}

#[derive(Serialize, Debug, Default)]
pub struct Programs {
    pub rows: Vec<Row>,
    /// Icons by program id or, for a default with no id, by name: `data:image/png;base64,...`.
    pub icons: HashMap<String, String>,
    /// The system was asked which programs open each type; not yet on Linux.
    pub listed: bool,
}

fn group(ext: &str) -> &'static str {
    match class_of(ext) {
        Some(Class::Video) => GROUPS[0],
        Some(Class::Audio) => GROUPS[1],
        Some(Class::Image) => GROUPS[2],
        Some(Class::Book) => GROUPS[3],
        None => GROUPS[4],
    }
}

fn ext_of(name: &str) -> Option<String> {
    let (stem, ext) = name.rsplit_once('.')?;
    (!stem.is_empty() && !ext.is_empty() && ext.len() <= 12 && !ext.contains([' ', '/', '\\'])).then(|| ext.to_lowercase())
}

/// The types in the catalog with their file counts and one file of each (for macOS, which
/// asks by file), plus every type the person chose a program for, even with no files now.
/// Sorted by group, then by file count.
pub fn types(cat: &Catalog, open_with: &BTreeMap<String, String>) -> Vec<(String, usize, Option<String>)> {
    let mut by: HashMap<String, (usize, Option<String>)> = HashMap::new();
    for s in &cat.sections {
        // The sample is a file on disk now: macOS asks by file, and a cloud-only one may be fetched.
        let files = s.works.iter().flat_map(|w| w.items.iter().map(|i| (i.ext.to_lowercase(), &i.path, i.cloud)));
        let loose = s.unlisted.iter().filter_map(|u| ext_of(&u.name).map(|e| (e, &u.path, u.cloud)));
        for (ext, path, cloud) in files.chain(loose) {
            if ext.is_empty() {
                continue;
            }
            let e = by.entry(ext).or_default();
            e.0 += 1;
            if e.1.is_none() && s.online && !cloud {
                e.1 = Some(path.clone());
            }
        }
    }
    for ext in open_with.keys() {
        by.entry(ext.to_lowercase()).or_default();
    }
    let mut v: Vec<_> = by.into_iter().map(|(e, (n, p))| (e, n, p)).collect();
    v.sort_by(|a, b| {
        let g = |e: &str| GROUPS.iter().position(|g| *g == group(e)).unwrap_or(GROUPS.len());
        g(&a.0).cmp(&g(&b.0)).then(b.1.cmp(&a.1)).then(a.0.cmp(&b.0))
    });
    v
}

/// The whole list for `types`, asking the system about each one.
pub fn list(types: Vec<(String, usize, Option<String>)>, open_with: &BTreeMap<String, String>) -> Programs {
    let mut out = Programs { listed: cfg!(any(windows, target_os = "macos")), ..Default::default() };
    sys::with(|sys| {
        for (ext, files, sample) in types {
            let (default, mut choices) = sys.offered(&ext, sample.as_deref());
            if RUNS.contains(&ext.as_str()) {
                choices.retain(|p| !interpreter(&p.id));
            }
            let offered = |id: &str| choices.iter().chain(default.iter()).find(|p| sys::same_program(&p.id, id)).cloned();
            let yours = open_with.get(&ext).map(|id| offered(id).unwrap_or_else(|| Program { id: id.clone(), name: name_of(id) }));
            let gone = yours.as_ref().is_some_and(|p| offered(&p.id).is_none() && !startable(&p.id));
            for p in default.iter().chain(&choices).chain(yours.iter()) {
                let key = if p.id.is_empty() { &p.name } else { &p.id };
                if !out.icons.contains_key(key)
                    && let Some(icon) = sys.icon(&p.id, &p.name)
                {
                    out.icons.insert(key.clone(), icon);
                }
            }
            out.rows.push(Row { reveals: reveals(&ext), group: group(&ext), ext, files, yours, gone, default, choices });
        }
    });
    out
}

/// Opens `path` in the program `id`: through the system's handler for its type, or as an
/// `.exe` the person picked. An id that is neither (a program since removed) is an error.
pub fn open(id: &str, ext: &str, path: &str) -> Result<(), String> {
    match sys::invoke(id, ext, path) {
        Some(r) => r,
        None if startable(id) => start(id, path),
        None if id.trim().is_empty() => Err(format!("The program picked for .{ext} is not installed any more. Choose another in Settings, Programs.")),
        None => Err(format!("{} is not installed any more. Choose another program for .{ext} in Settings, Programs.", name_of(id))),
    }
}

/// Opens `path`, which keeps its `\\?\` start since a name in it ends in a dot or a space
///, in the program `id` or, with none, in the system's program for its type. Explorer and
/// the shell handlers would drop that end and open another file, so the program is started
/// directly: an `.exe` the person picked, or the open command Windows keeps for the type. A Store
/// app, a program that runs what it opens, or a command that takes the file in another form, is
/// refused.
pub fn open_as_named(id: Option<&str>, ext: &str, path: &str) -> Result<(), String> {
    if let Some(id) = id.filter(|id| startable(id)) {
        return start(id, path);
    }
    #[cfg(windows)]
    let command = if id.is_none() { sys::command(ext) } else { None };
    #[cfg(not(windows))]
    let command: Option<String> = None;
    let Some((exe, args)) = command.and_then(|c| command_line(&c, path)).filter(|(exe, _)| startable(exe) && !interpreter(exe)) else {
        return Err(format!("A name here ends in a dot or a space, which only a desktop program (an .exe), not a Store app, can open. Choose one for .{ext} in Settings, Programs."));
    };
    let mut c = std::process::Command::new(exe);
    #[cfg(windows)]
    std::os::windows::process::CommandExt::raw_arg(&mut c, args);
    #[cfg(not(windows))]
    c.arg(args);
    c.spawn().map(drop).map_err(|e| e.to_string())
}

/// A type's open command (`"C:\x\app.exe" "%1"`) as its program and the rest of the line with
/// `path` put in for the file, in quotes unless the command quotes it or takes the rest of the line
/// as one (`--single-argument`). `None` when the command wants the file in another form (`%I`, a
/// shell item; `%u`, a URL) or not on its line at all (DDE).
fn command_line(command: &str, path: &str) -> Option<(String, String)> {
    let command = command.trim_start();
    let (exe, rest) = match command.strip_prefix('"') {
        Some(r) => r.split_once('"')?,
        None => command.split_once(' ').unwrap_or((command, "")),
    };
    let (mut args, mut put) = (String::new(), false);
    let mut cs = rest.chars();
    while let Some(c) = cs.next() {
        if c != '%' {
            args.push(c);
            continue;
        }
        match cs.next() {
            Some('1' | 'L' | 'l' | 'V' | 'v') => {
                if args.ends_with('"') || args.ends_with("--single-argument ") {
                    args.push_str(path);
                } else {
                    args.push_str(&format!("\"{path}\""));
                }
                put = true;
            }
            Some('*') => {}
            Some('%') => args.push('%'),
            _ => return None,
        }
    }
    put.then(|| (exe.to_string(), args))
}

#[cfg(target_os = "macos")]
fn start(id: &str, path: &str) -> Result<(), String> {
    std::process::Command::new("open").args(["-a", id, path]).spawn().map(drop).map_err(|e| e.to_string())
}

#[cfg(not(target_os = "macos"))]
fn start(id: &str, path: &str) -> Result<(), String> {
    std::process::Command::new(id).arg(path).spawn().map(drop).map_err(|e| e.to_string())
}

#[cfg_attr(not(any(windows, target_os = "macos")), allow(dead_code))]
fn same(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

/// A program the app can start itself: a full path to an `.exe` on Windows, an `.app` (or a
/// program file) on macOS. A relative name is never one: Windows would look for it in PATH.
fn startable(id: &str) -> bool {
    let p = std::path::Path::new(id);
    if !p.is_absolute() {
        return false;
    }
    if cfg!(windows) {
        p.is_file() && p.extension().is_some_and(|e| e.eq_ignore_ascii_case("exe"))
    } else if cfg!(target_os = "macos") {
        // An app is a folder named `.app`; any other folder is not a program.
        (p.is_dir() && p.extension().is_some_and(|e| e.eq_ignore_ascii_case("app"))) || p.is_file()
    } else {
        p.is_file()
    }
}

/// A program that runs what it opens (`INTERPRETERS`).
fn interpreter(id: &str) -> bool {
    let file = id.rsplit(['\\', '/']).next().unwrap_or(id).to_lowercase();
    INTERPRETERS.contains(&file.as_str())
}

/// A program's name from its id: the file name without `.exe` or `.app`, or the id itself.
fn name_of(id: &str) -> String {
    let file = id.rsplit(['\\', '/']).next().unwrap_or(id);
    let lower = file.to_lowercase();
    let stem = if lower.ends_with(".exe") || lower.ends_with(".app") { &file[..file.len() - 4] } else { file };
    if stem.is_empty() { id.to_string() } else { stem.to_string() }
}

fn png_url(png: &[u8]) -> String {
    use base64::Engine;
    format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(png))
}

/// An image file made into a 32 px PNG data URL.
#[cfg_attr(not(windows), allow(dead_code))]
fn icon_file(path: &std::path::Path) -> Option<String> {
    small_png(image::open(path).ok()?)
}

/// A picture made into a 32 px PNG data URL.
#[cfg_attr(not(any(windows, target_os = "macos")), allow(dead_code))]
fn small_png(img: image::DynamicImage) -> Option<String> {
    let mut png = std::io::Cursor::new(Vec::new());
    img.thumbnail(32, 32).write_to(&mut png, image::ImageFormat::Png).ok()?;
    Some(png_url(&png.into_inner()))
}

#[cfg(windows)]
mod sys {
    //! The Windows shell: `AssocQueryStringW` for the default, `SHAssocEnumHandlers` for the
    //! programs it offers, `IAssocHandler::Invoke` to open a file in one.
    use super::{icon_file, png_url, same, Program};
    use windows::core::{HSTRING, PWSTR};
    use windows::Win32::Graphics::Gdi::*;
    use windows::Win32::System::Com::*;
    use windows::Win32::UI::Shell::*;
    use windows::Win32::UI::WindowsAndMessaging::*;

    pub struct Sys {
        com: bool,
        /// Where each handler keeps its icon, by id, noted as the handlers are listed.
        places: std::cell::RefCell<std::collections::HashMap<String, (String, i32)>>,
    }

    impl Drop for Sys {
        fn drop(&mut self) {
            if self.com {
                unsafe { CoUninitialize() };
            }
        }
    }

    /// Runs `f` on a thread of its own with COM set up for the shell, so the caller's thread
    /// (the UI's, or a pool thread) is never left in a COM state it did not choose.
    pub fn with<T: Send>(f: impl FnOnce(&Sys) -> T + Send) -> T {
        std::thread::scope(|s| {
            s.spawn(|| {
                let com = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) }.is_ok();
                f(&Sys { com, places: Default::default() })
            })
            .join()
            .expect("the shell thread stopped")
        })
    }

    fn text(p: windows::core::Result<PWSTR>) -> String {
        match p {
            Ok(p) if !p.is_null() => unsafe {
                let s = p.to_string().unwrap_or_default();
                CoTaskMemFree(Some(p.0 as _));
                s
            },
            _ => String::new(),
        }
    }

    fn assoc(ext: &str, what: ASSOCSTR) -> Option<String> {
        let mut buf = vec![0u16; 1024];
        let mut n = buf.len() as u32;
        let hr = unsafe { AssocQueryStringW(ASSOCF_INIT_IGNOREUNKNOWN | ASSOCF_NOTRUNCATE, what, &HSTRING::from(format!(".{ext}")), None, Some(PWSTR(buf.as_mut_ptr())), &mut n) };
        if hr.is_err() || n == 0 {
            return None;
        }
        let s = String::from_utf16_lossy(&buf[..(n as usize - 1).min(buf.len())]);
        (!s.trim().is_empty()).then_some(s)
    }

    /// The command Windows runs to open a file of this type; `None` for a Store app.
    pub fn command(ext: &str) -> Option<String> {
        assoc(ext, ASSOCSTR_COMMAND)
    }

    fn handlers(ext: &str, filter: ASSOC_FILTER) -> Vec<IAssocHandler> {
        let Ok(en) = (unsafe { SHAssocEnumHandlers(&HSTRING::from(format!(".{ext}")), filter) }) else { return vec![] };
        let mut v = vec![];
        loop {
            let mut h = [None];
            let mut got = 0u32;
            if unsafe { en.Next(&mut h, Some(&mut got)) }.is_err() || got == 0 {
                break;
            }
            v.extend(h[0].take());
            if v.len() > 64 {
                break;
            }
        }
        v
    }

    /// Where a handler keeps its icon: a file and index, or `@{<package>?ms-resource:...}`.
    fn icon_location(h: &IAssocHandler) -> Option<(String, i32)> {
        let mut path = PWSTR::null();
        let mut index = 0;
        if unsafe { h.GetIconLocation(&mut path, &mut index) }.is_err() || path.is_null() {
            return None;
        }
        let place = unsafe { path.to_string().unwrap_or_default() };
        unsafe { CoTaskMemFree(Some(path.0 as _)) };
        Some((place, index))
    }

    /// The package family of a Store app from its icon location: `Microsoft.Windows.Photos_8wekyb3d8bbwe`
    /// from `@{Microsoft.Windows.Photos_2026.11090.22001.0_x64__8wekyb3d8bbwe?...}`. A package
    /// name holds no `_`, so the family is the first and last parts of the full name.
    pub fn family(place: &str) -> Option<String> {
        let full = place.strip_prefix("@{")?.split(['?', '}']).next()?;
        let (name, publisher) = (full.split('_').next()?, full.rsplit('_').next()?);
        (!name.is_empty() && !publisher.is_empty() && full.contains('_')).then(|| format!("{name}_{publisher}"))
    }

    /// A handler as a Program: its id is its name, or for a Store app `pkg:<family>/<name's
    /// last part>`, which neither an update (the path holds the version) nor the display
    /// language (a Store app's name is its display name) changes.
    fn program(h: &IAssocHandler, place: Option<&(String, i32)>) -> Program {
        let raw = text(unsafe { h.GetName() });
        let name = text(unsafe { h.GetUIName() });
        let name = if name.is_empty() { super::name_of(&raw) } else { name };
        let id = match place.and_then(|(p, _)| family(p)) {
            Some(f) if !raw.is_empty() => format!("pkg:{f}/{}", raw.rsplit(['\\', '/']).next().unwrap_or(&raw)),
            _ => raw,
        };
        Program { name, id }
    }

    /// Whether a stored id is this handler's: the same id, or the same Store package.
    pub fn same_program(stored: &str, id: &str) -> bool {
        let fam = |s: &str| s.strip_prefix("pkg:").map(|r| r.split('/').next().unwrap_or(r).to_lowercase());
        same(stored, id) || fam(stored).is_some_and(|f| fam(id) == Some(f))
    }

    impl Sys {
        pub fn offered(&self, ext: &str, _sample: Option<&str>) -> (Option<Program>, Vec<Program>) {
            let mut hs = handlers(ext, ASSOC_FILTER_RECOMMENDED);
            if hs.is_empty() {
                hs = handlers(ext, ASSOC_FILTER_NONE);
            }
            let mut choices: Vec<Program> = vec![];
            for h in &hs {
                let place = icon_location(h);
                let p = program(h, place.as_ref());
                if p.id.is_empty() || choices.iter().any(|c| same(&c.id, &p.id)) {
                    continue;
                }
                if let Some(place) = place {
                    self.places.borrow_mut().insert(p.id.clone(), place);
                }
                choices.push(p);
            }
            let friendly = assoc(ext, ASSOCSTR_FRIENDLYAPPNAME);
            let exe = assoc(ext, ASSOCSTR_EXECUTABLE);
            let default = friendly.as_ref().map(|f| {
                choices
                    .iter()
                    .find(|c| exe.as_ref().is_some_and(|e| same(&c.id, e)))
                    .or_else(|| choices.iter().find(|c| same(&c.name, f)))
                    .cloned()
                    .unwrap_or_else(|| {
                        // A default the shell lists no handler for (Notepad++ for .txt): its
                        // `.exe` is the id when there is one, and it joins the choices.
                        let id = exe.clone().filter(|e| std::path::Path::new(e).is_file()).unwrap_or_default();
                        let p = Program { id, name: f.clone() };
                        if !p.id.is_empty() {
                            choices.push(p.clone());
                        }
                        p
                    })
            });
            choices.sort_by_key(|c| (default.as_ref() != Some(c), c.name.to_lowercase()));
            (default, choices)
        }

        /// A program's icon: a Store app's from its package, any other from its `.exe`. A
        /// default with no handler (empty id) has none.
        pub fn icon(&self, id: &str, _name: &str) -> Option<String> {
            if id.is_empty() {
                return None;
            }
            let (path, index) = self.places.borrow().get(id).cloned().unwrap_or((id.to_string(), 0));
            if path.starts_with("@{") {
                let mut buf = vec![0u16; 1024];
                unsafe { SHLoadIndirectString(&HSTRING::from(&path), &mut buf, None) }.ok()?;
                let file = String::from_utf16_lossy(&buf[..buf.iter().position(|&c| c == 0).unwrap_or(0)]);
                return icon_file(std::path::Path::new(&file));
            }
            exe_icon(&path, index)
        }
    }

    /// Opens `path` in the handler whose id is `id`; None when the system offers no such
    /// handler. It runs on a thread of its own that stays a few seconds after `Invoke`, pumping
    /// messages: a handler can finish starting the program after `Invoke` returns (DDE,
    /// `DelegateExecute`), and calls back into this thread to read the file it was given.
    pub fn invoke(id: &str, ext: &str, path: &str) -> Option<Result<(), String>> {
        let (id, ext, path) = (id.to_string(), ext.to_string(), path.to_string());
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let com = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) }.is_ok();
            let found = handlers(&ext, ASSOC_FILTER_NONE).into_iter().find(|h| same_program(&id, &program(h, icon_location(h).as_ref()).id));
            let ran = found.is_some();
            let r = found.map(|h| unsafe {
                SHCreateItemFromParsingName::<_, _, IShellItem>(&HSTRING::from(&path), None)
                    .and_then(|item| item.BindToHandler::<_, IDataObject>(None, &BHID_DataObject))
                    .and_then(|data| h.Invoke(&data))
                    .map_err(|e| e.message().to_string())
            });
            let _ = tx.send(r);
            if ran {
                let until = std::time::Instant::now() + std::time::Duration::from_secs(5);
                let mut msg = MSG::default();
                while std::time::Instant::now() < until {
                    while unsafe { PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE) }.as_bool() {
                        unsafe {
                            let _ = TranslateMessage(&msg);
                            DispatchMessageW(&msg);
                        }
                    }
                    std::thread::sleep(std::time::Duration::from_millis(30));
                }
            }
            if com {
                unsafe { CoUninitialize() };
            }
        });
        rx.recv_timeout(std::time::Duration::from_secs(30)).unwrap_or_else(|_| Some(Err("The program did not start in time.".into())))
    }

    /// A 32 px icon from an `.exe` or `.dll`, as a PNG data URL. A negative index is a
    /// resource id, which `PrivateExtractIconsW` reads as such.
    fn exe_icon(path: &str, index: i32) -> Option<String> {
        unsafe {
            let mut name = [0u16; 260];
            let wide: Vec<u16> = path.encode_utf16().collect();
            if wide.len() >= name.len() {
                return None;
            }
            name[..wide.len()].copy_from_slice(&wide);
            let mut icons = [HICON::default()];
            let got = PrivateExtractIconsW(&name, index, 32, 32, Some(&mut icons), None, 0);
            let large = icons[0];
            if got == 0 || got == u32::MAX || large.is_invalid() {
                return None;
            }
            let out = icon_png(large);
            let _ = DestroyIcon(large);
            out
        }
    }

    unsafe fn icon_png(icon: HICON) -> Option<String> {
        unsafe {
            let mut info = ICONINFO::default();
            GetIconInfo(icon, &mut info).ok()?;
            let free = |info: &ICONINFO| {
                if !info.hbmColor.is_invalid() {
                    let _ = DeleteObject(info.hbmColor.into());
                }
                if !info.hbmMask.is_invalid() {
                    let _ = DeleteObject(info.hbmMask.into());
                }
            };
            if info.hbmColor.is_invalid() {
                free(&info);
                return None;
            }
            let mut bm = BITMAP::default();
            GetObjectW(info.hbmColor.into(), std::mem::size_of::<BITMAP>() as i32, Some(&mut bm as *mut _ as _));
            let (w, h) = (bm.bmWidth, bm.bmHeight);
            if !(1..=256).contains(&w) || !(1..=256).contains(&h) {
                free(&info);
                return None;
            }
            let header = BITMAPINFOHEADER { biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32, biWidth: w, biHeight: -h, biPlanes: 1, biBitCount: 32, biCompression: BI_RGB.0, ..Default::default() };
            let mut bi = BITMAPINFO { bmiHeader: header, ..Default::default() };
            let mut px = vec![0u8; (w * h * 4) as usize];
            let dc = GetDC(None);
            let lines = GetDIBits(dc, info.hbmColor, 0, h as u32, Some(px.as_mut_ptr() as _), &mut bi, DIB_RGB_COLORS);
            ReleaseDC(None, dc);
            free(&info);
            if lines == 0 {
                return None;
            }
            let opaque = px.chunks(4).all(|p| p[3] == 0);
            for p in px.chunks_mut(4) {
                p.swap(0, 2);
                if opaque {
                    p[3] = 255;
                }
            }
            let img = image::RgbaImage::from_raw(w as u32, h as u32, px)?;
            let mut png = std::io::Cursor::new(Vec::new());
            img.write_to(&mut png, image::ImageFormat::Png).ok()?;
            Some(png_url(&png.into_inner()))
        }
    }
}

#[cfg(target_os = "macos")]
mod sys {
    //! Launch Services through `NSWorkspace`: the app that opens a file of the type, and the apps
    //! that can. It asks by file, so it uses one file of each type from the catalog.
    use super::{same, small_png, Program};
    use objc2_app_kit::{NSBitmapImageFileType, NSBitmapImageRep, NSWorkspace};
    use objc2_foundation::{NSDictionary, NSString, NSURL};

    pub struct Sys;

    pub fn with<T>(f: impl FnOnce(&Sys) -> T) -> T {
        f(&Sys)
    }

    fn program(url: &NSURL) -> Option<Program> {
        let path = url.path()?.to_string();
        Some(Program { name: super::name_of(&path), id: path })
    }

    pub fn same_program(stored: &str, id: &str) -> bool {
        same(stored, id)
    }

    /// macOS opens a file in an app with `open -a`, done by the caller.
    pub fn invoke(_id: &str, _ext: &str, _path: &str) -> Option<Result<(), String>> {
        None
    }

    impl Sys {
        pub fn offered(&self, _ext: &str, sample: Option<&str>) -> (Option<Program>, Vec<Program>) {
            let Some(sample) = sample else { return (None, vec![]) };
            // Each type's objects are let go as soon as it is answered, not when the list ends.
            objc2::rc::autoreleasepool(|_| self.ask(sample))
        }

        fn ask(&self, sample: &str) -> (Option<Program>, Vec<Program>) {
            let ws = NSWorkspace::sharedWorkspace();
            let url = NSURL::fileURLWithPath(&NSString::from_str(sample));
            let default = ws.URLForApplicationToOpenURL(&url).and_then(|u| program(&u));
            let mut choices: Vec<Program> = vec![];
            for u in ws.URLsForApplicationsToOpenURL(&url).iter() {
                if let Some(p) = program(&u)
                    && !choices.iter().any(|c| same(&c.id, &p.id))
                {
                    choices.push(p);
                }
            }
            choices.sort_by_key(|c| (default.as_ref() != Some(c), c.name.to_lowercase()));
            (default, choices)
        }

        /// An app's icon as Finder shows it, as a 32 px PNG data URL. Launch Services gives every
        /// size; the smallest one of at least 32 px is written as a PNG by AppKit itself.
        pub fn icon(&self, id: &str, _name: &str) -> Option<String> {
            if !std::path::Path::new(id).exists() {
                return None;
            }
            objc2::rc::autoreleasepool(|_| {
                let tiff = NSWorkspace::sharedWorkspace().iconForFile(&NSString::from_str(id)).TIFFRepresentation()?;
                let reps: Vec<_> = NSBitmapImageRep::imageRepsWithData(&tiff).iter().filter_map(|r| r.downcast::<NSBitmapImageRep>().ok()).collect();
                let rep = reps.iter().filter(|r| r.pixelsWide() >= 32).min_by_key(|r| r.pixelsWide()).or_else(|| reps.iter().max_by_key(|r| r.pixelsWide()))?;
                // SAFETY: an empty dictionary is a valid property list for any file type.
                let png = unsafe { rep.representationUsingType_properties(NSBitmapImageFileType::PNG, &NSDictionary::new()) }?.to_vec();
                small_png(image::load_from_memory_with_format(&png, image::ImageFormat::Png).ok()?)
            })
        }
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
mod sys {
    //! Linux: no list yet; a picked program is started directly.
    use super::Program;

    pub struct Sys;

    pub fn with<T>(f: impl FnOnce(&Sys) -> T) -> T {
        f(&Sys)
    }

    pub fn same_program(stored: &str, id: &str) -> bool {
        stored == id
    }

    pub fn invoke(_id: &str, _ext: &str, _path: &str) -> Option<Result<(), String>> {
        None
    }

    impl Sys {
        pub fn offered(&self, _ext: &str, _sample: Option<&str>) -> (Option<Program>, Vec<Program>) {
            (None, vec![])
        }
        pub fn icon(&self, _id: &str, _name: &str) -> Option<String> {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "macos")]
    #[test]
    fn a_mac_app_has_its_icon() {
        let url = sys::with(|s| s.icon("/System/Applications/Calculator.app", "Calculator")).expect("no icon for Calculator");
        assert!(url.starts_with("data:image/png;base64,") && url.len() > 200, "{url:.60}");
        assert!(sys::with(|s| s.icon("/Nowhere/Gone.app", "Gone")).is_none(), "an app that is gone has no icon");
    }

    /// A catalog of one Section: Works holding `items` (paths), and `loose` unlisted files.
    fn catalog(items: &[&str], loose: &[&str]) -> Catalog {
        let item = |p: &&str| {
            let name = p.rsplit('\\').next().unwrap();
            serde_json::json!({ "name": name, "rel": name, "path": p, "ext": name.rsplit_once('.').map(|(_, e)| e).unwrap_or(""), "size": 1, "mtime": 0, "cloud": false })
        };
        let un = |p: &&str| serde_json::json!({ "name": p.rsplit('\\').next().unwrap(), "rel": "", "path": p, "folder": [], "reason": "unknown", "size": 1, "cloud": false });
        serde_json::from_value(serde_json::json!({ "v": 1, "sections": [{
            "id": "S", "name": "S", "path": "D:\\S", "drive": "D:", "kind": "Files", "guessed": false, "online": true,
            "works": [{ "id": "S:w", "title": "w", "path": "D:\\S\\w", "folder": [], "is_folder": true, "items": items.iter().map(item).collect::<Vec<_>>(),
                "media": [], "size": 0, "added": 0, "cloud": 0 }],
            "files": 0, "scanned_at": 0, "unlisted": loose.iter().map(un).collect::<Vec<_>>() }] }))
        .unwrap()
    }

    #[test]
    fn types_come_from_the_catalog_grouped_and_counted() {
        let cat = catalog(&["D:\\S\\a.mp4", "D:\\S\\b.MP4", "D:\\S\\c.Mp4", "D:\\S\\d.pdf", "D:\\S\\e.jpg", "D:\\S\\f.mp3", "D:\\S\\g.zip"], &["D:\\S\\x.nfo", "D:\\S\\y.ini", "D:\\S\\noext", "D:\\S\\.hidden", "D:\\S\\h.mkv"]);
        let t = types(&cat, &BTreeMap::new());
        let got: Vec<(&str, usize, &'static str)> = t.iter().map(|(e, n, _)| (e.as_str(), *n, group(e))).collect();
        assert_eq!(got, [
            ("mp4", 3, "Video"), ("mkv", 1, "Video"), ("mp3", 1, "Audio"), ("jpg", 1, "Pictures"), ("pdf", 1, "Books & documents"),
            ("ini", 1, "Other types"), ("nfo", 1, "Other types"), ("zip", 1, "Other types"),
        ]);
        // One file of each type, for macOS.
        assert_eq!(t[0].2.as_deref(), Some("D:\\S\\a.mp4"));
    }

    #[test]
    fn a_choice_stays_listed_with_no_files_and_a_gone_type_drops_out() {
        let cat = catalog(&["D:\\S\\a.mp4"], &[]);
        let mut ow = BTreeMap::new();
        ow.insert("cbz".to_string(), "C:\\Program Files\\Calibre2\\ebook-viewer.exe".to_string());
        let t = types(&cat, &ow);
        assert_eq!(t.iter().map(|(e, n, _)| (e.as_str(), *n)).collect::<Vec<_>>(), [("mp4", 1), ("cbz", 0)]);
        // A new scan with no mp4 left: it is gone from the list.
        let t = types(&catalog(&["D:\\S\\a.pdf"], &[]), &BTreeMap::new());
        assert_eq!(t.iter().map(|(e, ..)| e.as_str()).collect::<Vec<_>>(), ["pdf"]);
    }

    #[test]
    fn hundreds_of_types_list_quickly() {
        let names: Vec<String> = (0..400).map(|i| format!("D:\\S\\f{i}.x{i}")).collect();
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let t = types(&catalog(&refs, &[]), &BTreeMap::new());
        assert_eq!(t.len(), 400);
        assert!(t.iter().all(|(e, ..)| group(e) == "Other types"));
    }

    #[test]
    fn names_from_ids() {
        assert_eq!(name_of("C:\\Program Files\\VideoLAN\\VLC\\vlc.exe"), "vlc");
        assert_eq!(name_of("C:\\PROGRAM FILES\\X\\FOXIT.EXE"), "FOXIT");
        assert_eq!(name_of("/Applications/VLC.app"), "VLC");
        assert_eq!(name_of("Photos"), "Photos");
    }

    #[test]
    fn shown_types_are_marked() {
        let r = list(vec![("ps1".into(), 2, None), ("mp4".into(), 1, None)], &BTreeMap::new());
        assert!(r.rows[0].reveals && !r.rows[1].reveals);
    }

    #[test]
    fn a_removed_program_is_marked_gone() {
        let mut ow = BTreeMap::new();
        ow.insert("qqq".to_string(), "C:\\Nowhere\\gone.exe".to_string());
        let r = list(vec![("qqq".into(), 1, None)], &ow);
        assert!(r.rows[0].gone);
        assert_eq!(r.rows[0].yours.as_ref().map(|p| p.name.as_str()), Some("gone"));
    }

    #[test]
    fn a_removed_program_is_a_clear_error() {
        let e = open("C:\\Nowhere\\gone.exe", "qqq", "C:\\x.qqq").unwrap_err();
        assert!(e.contains("gone is not installed any more") && e.contains(".qqq"), "{e}");
    }

    #[cfg(windows)]
    #[test]
    fn store_apps_are_known_by_their_package() {
        let place = "@{Microsoft.Windows.Photos_2026.11090.22001.0_x64__8wekyb3d8bbwe?ms-resource://Microsoft.Windows.Photos/Files/Assets/PhotosAppList.png}";
        assert_eq!(sys::family(place).as_deref(), Some("Microsoft.Windows.Photos_8wekyb3d8bbwe"));
        assert_eq!(sys::family("C:\\Program Files\\x.exe"), None);
        // A new version or another display language keeps the choice.
        assert!(sys::same_program("pkg:Microsoft.Windows.Photos_8wekyb3d8bbwe/Photos", "pkg:Microsoft.Windows.Photos_8wekyb3d8bbwe/Фотографии"));
        assert!(!sys::same_program("pkg:Microsoft.Windows.Photos_8wekyb3d8bbwe/Photos", "pkg:Microsoft.Paint_8wekyb3d8bbwe/mspaint.exe"));
        assert!(sys::same_program("C:\\A\\vlc.exe", "c:\\a\\VLC.EXE"));
    }

    /// A type's open command takes the path in place of `%1`, keeping its own quotes; one that
    /// wants the file in another form is not used.
    #[test]
    fn an_open_command_takes_the_path() {
        let p = r"\\?\UNC\srv\share\Books.\a.pdf";
        let line = |c: &str| command_line(c, p).map(|(e, a)| format!("{e}|{a}"));
        assert_eq!(line(r#""C:\Program Files\Foxit\Foxit.exe" "%1""#).as_deref(), Some(r#"C:\Program Files\Foxit\Foxit.exe| "\\?\UNC\srv\share\Books.\a.pdf""#));
        assert_eq!(line(r"C:\Chrome\chrome.exe --single-argument %1").as_deref(), Some(r"C:\Chrome\chrome.exe|--single-argument \\?\UNC\srv\share\Books.\a.pdf"));
        assert_eq!(line(r#""C:\x.exe" /open "%1" %* 100%%"#).as_deref(), Some(r#"C:\x.exe| /open "\\?\UNC\srv\share\Books.\a.pdf"  100%"#));
        assert_eq!(command_line(r"C:\Tools\viewer.exe %1", r"\\?\UNC\srv\share\Notes \a.pdf").map(|(_, a)| a).as_deref(), Some(r#""\\?\UNC\srv\share\Notes \a.pdf""#), "a path not in quotes gets them");
        assert_eq!(line(r#""C:\Office\EXCEL.EXE" /dde"#), None, "the file is not on the line");
        assert_eq!(line(r"C:\WINDOWS\Explorer.exe /idlist,%I,%L"), None);
        assert_eq!(line(r#""C:\Office\WINWORD.EXE" /n "%1" /o "%u""#), None);
        assert_eq!(line(r#""C:\unclosed.exe %1"#), None);
        let e = open_as_named(Some("pkg:Microsoft.Windows.Photos_8wekyb3d8bbwe/Photos"), "jpg", p).unwrap_err();
        assert!(e.contains("desktop program") && e.contains(".jpg"), "{e}");
    }

    #[test]
    fn interpreters_are_known() {
        assert!(interpreter("C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe"));
        assert!(interpreter("C:\\Windows\\System32\\WScript.exe"));
        assert!(!interpreter("C:\\Program Files\\Notepad++\\notepad++.exe"));
    }

    #[test]
    fn a_script_type_is_never_opened_by_a_program_that_runs_it() {
        assert!(refused("ps1", "C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe"));
        assert!(refused("js", "C:\\Windows\\System32\\WScript.exe"));
        assert!(!refused("js", "C:\\Program Files\\Notepad++\\notepad++.exe"), "an editor for .js is fine");
        assert!(!refused("txt", "C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe"), "not a script type");
        assert!(refused("py", "C:\\Python314\\python.exe"));
        assert!(refused("exe", "C:\\Windows\\explorer.exe"));
        assert!(RUNS.contains(&"chm") && RUNS.contains(&"url") && RUNS.contains(&"desktop") && RUNS.contains(&"appimage"));
        assert!(RUNS.contains(&"library-ms") && RUNS.contains(&"search-ms") && RUNS.contains(&"iqy"));
        assert!(["searchconnector-ms", "rdp", "pyz", "msix", "appinstaller", "wsb", "webloc"].iter().all(|e| RUNS.contains(e)));
    }

    #[test]
    fn only_known_types_open_from_a_click() {
        for e in ["mp4", "flac", "jpg", "pdf", "srt", "cr3", "mts", "csv"] {
            assert!(!reveals(e), ".{e} opens");
        }
        for e in ["", "zip", "html", "xml", "unknownthing", "exe", "chm", "desktop"] {
            assert!(reveals(e), ".{e} is shown in its folder");
        }
        assert!(OPENS.iter().all(|e| class_of(e).is_none() && !RUNS.contains(e)), "OPENS holds no Work type and no program type");
    }

    /// What this PC offers for common types: `cargo test --lib programs::tests::live -- --ignored --nocapture`.
    #[cfg(windows)]
    #[test]
    #[ignore]
    fn live() {
        let exts = ["jpg", "mp4", "mp3", "pdf", "epub", "srt", "nfo", "txt", "xyzq"];
        let r = list(exts.iter().map(|e| (e.to_string(), 1, None)).collect(), &BTreeMap::new());
        for row in &r.rows {
            println!(".{} default {:?}", row.ext, row.default.as_ref().map(|p| (&p.name, &p.id)));
            for c in &row.choices {
                println!("    {} [{}] icon {}", c.name, c.id, r.icons.get(&c.id).map(|i| i.len()).unwrap_or(0));
            }
        }
        let jpg = &r.rows[0];
        assert!(jpg.default.is_some() && !jpg.choices.is_empty());
        assert!(jpg.choices.iter().all(|c| r.icons.contains_key(&c.id)), "every offered program has an icon");
        assert!(r.rows.iter().find(|x| x.ext == "xyzq").unwrap().default.is_none());
        for row in &r.rows {
            if let Some(d) = &row.default {
                assert!(d.id.is_empty() || row.choices.first() == Some(d), ".{}: the default leads the choices", row.ext);
            }
        }
    }
}
