//! The bundled tools (`decisions/2026/10/05/filesviewhub-is-a-tauri-app-shipped-portable.md`): a `tools` folder next to the app, described by its `tools.json`
//! (written by `scripts/fetch-tools.mjs`). Each runs with a time limit, no window, low priority
//! and no input, and may read only local files.

use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Tool {
    pub id: String,
    pub name: String,
    pub version: String,
    pub licence: String,
    /// Relative to the tools folder; the first one is what runs.
    pub files: Vec<String>,
    pub source: String,
    /// Every file is there.
    #[serde(default)]
    pub found: bool,
}

#[derive(Deserialize, Default)]
struct Manifest {
    tools: Vec<Tool>,
}

/// A path with its `.` and `..` steps worked out by name, not through the file system.
fn tidy(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir if matches!(out.components().next_back(), Some(std::path::Component::Normal(_))) => {
                out.pop();
            }
            c => out.push(c),
        }
    }
    out
}

pub struct Tools {
    pub dir: PathBuf,
    pub list: Vec<Tool>,
}

impl Tools {
    /// `tools` next to the app (inside the AppImage on Linux); in a development build, `tools/` as the fetch script leaves it.
    pub fn find() -> Tools {
        let mut places = vec![];
        if let Some(d) = std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf)) {
            places.push(d.join("tools"));
            // In the Linux AppImage the app is `usr/bin/filesviewhub` and its resources are in `usr/lib`.
            #[cfg(target_os = "linux")]
            places.push(d.join("../lib/FilesViewHub/tools"));
        }
        if cfg!(debug_assertions) {
            places.push(Path::new(env!("CARGO_MANIFEST_DIR")).join("../tools"));
        }
        // Without its `..` steps, which Settings' Open folder refuses (`reveal`).
        let dir = tidy(places.iter().find(|p| p.join("tools.json").is_file()).unwrap_or(&places[0]));
        let m: Manifest = std::fs::read_to_string(dir.join("tools.json")).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
        let list = m
            .tools
            .into_iter()
            .map(|mut t| {
                t.found = t.files.iter().all(|f| dir.join(f).is_file());
                t
            })
            .collect::<Vec<_>>();
        for t in list.iter().filter(|t| !t.found) {
            log::warn!("{} is missing from {}", t.name, dir.display());
        }
        if list.is_empty() {
            log::warn!("no tools found in {}; run `npm run fetch-tools`", dir.display());
        }
        Tools { dir, list }
    }

    /// File `n` of tool `id`, when the whole tool is there.
    pub fn path(&self, id: &str, n: usize) -> Option<PathBuf> {
        let t = self.list.iter().find(|t| t.id == id && t.found)?;
        t.files.get(n).map(|f| self.dir.join(f))
    }
}

/// Why no picture came from a file.
#[derive(Debug)]
pub enum Fail {
    /// The file could not be read now (gone, locked, offline), a tool is missing or took too long:
    /// tried again next time.
    Retry(String),
    /// The file was read and holds no usable picture: remembered, not tried again until it changes.
    Bad(String),
}

impl From<std::io::Error> for Fail {
    fn from(e: std::io::Error) -> Self {
        Fail::Retry(e.to_string())
    }
}

/// Errors that say the file could not be read, not that it holds no picture.
const UNREADABLE: &[&str] = &["No such file", "Input/output error", "Permission denied", "used by another process", "Cannot open", "Unable to open"];

/// Runs a tool and returns what it wrote, killing it after `timeout`.
pub fn run(exe: &Path, args: &[&std::ffi::OsStr], timeout: Duration) -> Result<Vec<u8>, Fail> {
    let mut cmd = Command::new(exe);
    cmd.args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        const BELOW_NORMAL_PRIORITY_CLASS: u32 = 0x0000_4000;
        cmd.creation_flags(CREATE_NO_WINDOW | BELOW_NORMAL_PRIORITY_CLASS);
    }
    let mut child = cmd.spawn().map_err(|e| Fail::Retry(format!("{} did not start: {e}", exe.display())))?;
    let mut out = child.stdout.take().unwrap();
    let mut err = child.stderr.take().unwrap();
    let out_t = std::thread::spawn(move || {
        let mut v = vec![];
        let _ = out.read_to_end(&mut v);
        v
    });
    let err_t = std::thread::spawn(move || {
        let mut v = vec![];
        let _ = err.read_to_end(&mut v);
        v
    });
    let start = Instant::now();
    let status = loop {
        if let Some(s) = child.try_wait()? {
            break s;
        }
        if start.elapsed() > timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Err(Fail::Retry(format!("took longer than {} s", timeout.as_secs())));
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let stdout = out_t.join().unwrap_or_default();
    let stderr = String::from_utf8_lossy(&err_t.join().unwrap_or_default()).trim().to_string();
    if status.success() {
        return Ok(stdout);
    }
    let last = stderr.lines().last().unwrap_or("").chars().take(200).collect::<String>();
    let why = if last.is_empty() { format!("exit code {}", status.code().unwrap_or(-1)) } else { last };
    if UNREADABLE.iter().any(|u| stderr.contains(u)) { Err(Fail::Retry(why)) } else { Err(Fail::Bad(why)) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tools_path_loses_its_dot_dot_steps() {
        assert_eq!(tidy(Path::new("/tmp/.mount_x/usr/bin/../lib/FilesViewHub/tools")), PathBuf::from("/tmp/.mount_x/usr/lib/FilesViewHub/tools"));
        assert_eq!(tidy(Path::new("app/src-tauri/./../tools")), PathBuf::from("app/tools"));
        assert_eq!(tidy(Path::new("../tools")), PathBuf::from("../tools"), "a leading .. stays");
    }
}
