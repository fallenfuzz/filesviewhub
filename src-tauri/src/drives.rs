//! Drives and the folders on them, for the folder tree, and the volume serial number a Work id
//! starts with (`decisions/2026/10/05/checked-folders-on-any-drive.md`).

use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Serialize, Clone, Debug)]
pub struct Drive {
    /// The drive's root as the system shows it now, like `D:\`.
    pub path: String,
    pub label: String,
    /// Stays the same when Windows gives the drive another letter.
    pub serial: u32,
    /// "fixed", "removable", "network" or "other".
    pub kind: String,
    pub free: u64,
    pub total: u64,
    /// The system drive (Windows' own) or Computer (`/`): only folders on it can be ticked, never
    /// all of it, which holds the system and its programs. Any other drive can be.
    pub system: bool,
}

/// The drives connected now, as the check every 10 s needs them: the serials of local
/// drives, and the roots of network drives, whose serial is not asked, since asking waits on
/// their server.
#[derive(Debug, Default)]
pub struct Present {
    pub serials: Vec<u32>,
    pub network: Vec<String>,
}

#[derive(Serialize, Debug)]
pub struct Folder {
    pub name: String,
    pub path: String,
}

#[cfg(windows)]
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(windows)]
pub fn list() -> Vec<Drive> {
    read(false).0
}

/// The drives connected now, with no free space and no network drive's serial asked.
#[cfg(windows)]
pub fn present() -> Present {
    let (ds, network) = read(true);
    Present { serials: ds.iter().map(|d| d.serial).collect(), network }
}

/// The drives, and with `quick`, the network drives apart as their roots only, and no free space.
#[cfg(windows)]
fn read(quick: bool) -> (Vec<Drive>, Vec<String>) {
    use windows_sys::Win32::Storage::FileSystem::{
        GetDiskFreeSpaceExW, GetDriveTypeW, GetLogicalDrives, GetVolumeInformationW,
    };
    let mask = unsafe { GetLogicalDrives() };
    let system = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());
    let (mut out, mut network) = (Vec::new(), Vec::new());
    for i in 0..26u32 {
        if mask & (1 << i) == 0 {
            continue;
        }
        let root = format!("{}:\\", (b'A' + i as u8) as char);
        let w = wide(&root);
        let kind = match unsafe { GetDriveTypeW(w.as_ptr()) } {
            2 => "removable",
            3 => "fixed",
            4 => "network",
            5 => continue, // CD and DVD drives
            0 | 1 => continue,
            _ => "other",
        };
        if quick && kind == "network" {
            network.push(root);
            continue;
        }
        let mut label = [0u16; 261];
        let mut serial = 0u32;
        let ok = unsafe {
            GetVolumeInformationW(
                w.as_ptr(),
                label.as_mut_ptr(),
                label.len() as u32,
                &mut serial,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                0,
            )
        };
        if ok == 0 {
            continue; // an empty card reader
        }
        let (mut free, mut total) = (0u64, 0u64);
        if !quick {
            unsafe { GetDiskFreeSpaceExW(w.as_ptr(), &mut free, &mut total, std::ptr::null_mut()) };
        }
        let n = label.iter().position(|&c| c == 0).unwrap_or(0);
        out.push(Drive {
            system: system.eq_ignore_ascii_case(&root[..2]),
            path: root,
            label: String::from_utf16_lossy(&label[..n]),
            serial,
            kind: kind.into(),
            free,
            total,
        });
    }
    mark_system(&mut out);
    (out, network)
}

/// A drive with the system drive's serial is the system drive under another letter (`subst`, a
/// letter mapped to a folder on it): it is the system drive too, so all of it is never ticked.
#[cfg(any(windows, test))]
pub fn mark_system(ds: &mut [Drive]) {
    let sys: Vec<u32> = ds.iter().filter(|d| d.system).map(|d| d.serial).collect();
    for d in ds.iter_mut() {
        d.system |= sys.contains(&d.serial);
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
pub fn list() -> Vec<Drive> {
    // macOS comes next (`decisions/2026/10/05/filesviewhub-is-a-tauri-app-shipped-portable.md`); until then the whole file system is one Drive.
    vec![Drive { path: "/".into(), label: "Computer".into(), serial: 0, kind: "fixed".into(), free: 0, total: 0, system: true }]
}

#[cfg(not(any(windows, target_os = "linux")))]
pub fn present() -> Present {
    Present { serials: list().iter().map(|d| d.serial).collect(), network: vec![] }
}

/// On Linux: Home, then each mounted drive a person keeps files on, then the whole file system
/// as Computer. A drive's serial comes from its file system UUID, so it stays the same
/// across reboots and mount points; Home's and Computer's are fixed.
#[cfg(target_os = "linux")]
pub fn list() -> Vec<Drive> {
    read(true)
}

/// The drives connected now, with no free space asked, which waits on a network drive's server
///; a serial is worked out from the mount table, so a network drive's is known too.
#[cfg(target_os = "linux")]
pub fn present() -> Present {
    Present { serials: read(false).iter().map(|d| d.serial).collect(), network: vec![] }
}

/// The drives, with their free space when `space`.
#[cfg(target_os = "linux")]
fn read(space: bool) -> Vec<Drive> {
    let with_space = |d: Drive| if space { linux::with_space(d) } else { d };
    let home = std::env::var("HOME").unwrap_or_default();
    let user = std::env::var("USER").ok().filter(|u| !u.is_empty()).unwrap_or_else(|| home.rsplit('/').next().unwrap_or_default().to_string());
    let mounts = std::fs::read_to_string("/proc/self/mountinfo").unwrap_or_default();
    let uuids = linux::disk_names("/dev/disk/by-uuid");
    let labels = linux::disk_names("/dev/disk/by-label");
    let mut out = Vec::new();
    if home.len() > 1 && Path::new(&home).is_dir() {
        out.push(with_space(Drive { path: home, label: "Home".into(), serial: linux::HOME, kind: "fixed".into(), free: 0, total: 0, system: false }));
    }
    for m in mounts.lines().filter_map(linux::parse) {
        if !linux::keep(&m, &user) {
            continue;
        }
        let dev = std::fs::canonicalize(&m.source).ok();
        let find = |names: &[(PathBuf, String)]| dev.as_ref().and_then(|d| names.iter().find(|(p, _)| p == d)).map(|(_, n)| n.clone());
        let serial = linux::serial(find(&uuids).as_deref(), &m);
        if out.iter().any(|d: &Drive| d.serial == serial) {
            continue; // the same file system mounted twice
        }
        out.push(with_space(Drive { label: find(&labels).unwrap_or_default(), serial, kind: linux::kind(&m).into(), path: m.point, free: 0, total: 0, system: false }));
    }
    out.push(with_space(Drive { path: "/".into(), label: "Computer".into(), serial: 0, kind: "fixed".into(), free: 0, total: 0, system: true }));
    out
}

#[cfg(target_os = "linux")]
mod linux {
    use super::Drive;
    use std::path::PathBuf;

    /// Home's serial; Computer's is 0.
    pub const HOME: u32 = 1;

    /// One line of `/proc/self/mountinfo`.
    #[derive(Debug, PartialEq)]
    pub struct Mount {
        /// The folder of the file system mounted: `/` unless a bind mount or a btrfs subvolume.
        pub root: String,
        pub point: String,
        pub fstype: String,
        pub source: String,
    }

    /// The kernel writes a space, tab, newline and backslash in a path as `\040`, `\011`, `\012`
    /// and `\134`.
    fn unescape(s: &str) -> String {
        let b = s.as_bytes();
        let mut out = Vec::with_capacity(b.len());
        let mut i = 0;
        while i < b.len() {
            if b[i] == b'\\' && i + 3 < b.len() && b[i + 1..i + 4].iter().all(|c| (b'0'..=b'7').contains(c)) {
                out.push((b[i + 1] - b'0') * 64 + (b[i + 2] - b'0') * 8 + (b[i + 3] - b'0'));
                i += 4;
            } else {
                out.push(b[i]);
                i += 1;
            }
        }
        String::from_utf8_lossy(&out).into()
    }

    pub fn parse(line: &str) -> Option<Mount> {
        let (left, right) = line.split_once(" - ")?;
        let l: Vec<&str> = left.split(' ').collect();
        let mut r = right.split(' ');
        Some(Mount { root: unescape(l.get(3)?), point: unescape(l.get(4)?), fstype: r.next()?.into(), source: unescape(r.next()?) })
    }

    /// File systems that hold no files a person keeps.
    const SYSTEM_TYPES: &[&str] = &[
        "proc", "sysfs", "tmpfs", "devtmpfs", "devpts", "ramfs", "rootfs", "overlay", "squashfs", "cgroup", "cgroup2",
        "securityfs", "pstore", "bpf", "debugfs", "tracefs", "configfs", "fusectl", "mqueue", "hugetlbfs", "autofs",
        "binfmt_misc", "nsfs", "efivarfs", "rpc_pipefs", "selinuxfs", "nfsd", "fuse.portal", "fuse.gvfsd-fuse",
        "fuse.lxcfs", "fuse.snapfuse",
    ];
    /// Folders the system mounts its own things in.
    const SYSTEM_PLACES: &[&str] = &[
        "/boot", "/efi", "/proc", "/sys", "/dev", "/run", "/snap", "/var", "/tmp", "/usr", "/etc", "/opt", "/nix",
        "/gnu", "/lib", "/lib64", "/bin", "/sbin", "/root", "/mnt/wsl", "/mnt/wslg",
    ];

    fn within(path: &str, top: &str) -> bool {
        path == top || path.strip_prefix(top).is_some_and(|r| r.starts_with('/'))
    }

    /// A mount shown as a drive: no system file system, nothing in a system folder except a drive
    /// mounted for `user` in `/run/media/<user>`, and not `/` or `/home`, which Computer and Home
    /// show.
    pub fn keep(m: &Mount, user: &str) -> bool {
        if SYSTEM_TYPES.contains(&m.fstype.as_str()) || m.point == "/" || m.point == "/home" {
            return false;
        }
        if !user.is_empty() && m.point.strip_prefix(&format!("/run/media/{user}/")).is_some_and(|r| !r.is_empty()) {
            return true;
        }
        !SYSTEM_PLACES.iter().any(|t| within(&m.point, t))
    }

    pub fn kind(m: &Mount) -> &'static str {
        const NETWORK: &[&str] = &["nfs", "nfs4", "cifs", "smb3", "smbfs", "fuse.sshfs", "sshfs", "fuse.rclone", "davfs", "fuse.davfs2"];
        if NETWORK.contains(&m.fstype.as_str()) {
            "network"
        } else if m.point.starts_with("/run/media/") || m.point.starts_with("/media/") {
            "removable"
        } else {
            "fixed"
        }
    }

    /// FNV-1a, so a serial is the same in every build and on every computer.
    fn hash(s: &str) -> u32 {
        s.bytes().fold(0x811c_9dc5u32, |h, b| (h ^ b as u32).wrapping_mul(0x0100_0193))
    }

    /// A drive's serial, from its file system UUID when it has one. A FAT or exFAT UUID
    /// (`1A2B-3C4D`) is the volume serial Windows shows, and an NTFS one ends with it, so those are
    /// used as they are; any other UUID is hashed. A bind mount or a btrfs subvolume adds the
    /// folder it mounts. With no UUID (a network share, a WSL Windows drive) the mount source is
    /// used, `//server/share` or `C:\`, which also stays the same across reboots.
    pub fn serial(uuid: Option<&str>, m: &Mount) -> u32 {
        let sub = if m.root == "/" { String::new() } else { format!("\0{}", m.root) };
        let n = match uuid {
            Some(u) if sub.is_empty() && (u.len() == 9 && u.as_bytes()[4] == b'-' || u.len() == 16) => {
                let hex = u.replace('-', "");
                u32::from_str_radix(&hex[hex.len() - 8..], 16).unwrap_or_else(|_| hash(u))
            }
            Some(u) => hash(&format!("{u}{sub}")),
            None => hash(&format!("{}\0{}{sub}", m.fstype, m.source)),
        };
        if n == 0 || n == HOME { n ^ 0x8000_0000 } else { n }
    }

    /// The names in a `/dev/disk/by-*` folder, with the device each one points to.
    pub fn disk_names(dir: &str) -> Vec<(PathBuf, String)> {
        let Ok(rd) = std::fs::read_dir(dir) else { return vec![] };
        rd.filter_map(Result::ok).filter_map(|e| Some((std::fs::canonicalize(e.path()).ok()?, udev_name(&e.file_name().to_string_lossy())))).collect()
    }

    /// udev writes a space and other bytes in a label as `\x20`.
    pub fn udev_name(s: &str) -> String {
        let b = s.as_bytes();
        let mut out = Vec::with_capacity(b.len());
        let mut i = 0;
        while i < b.len() {
            let esc = (b[i] == b'\\' && b.get(i + 1) == Some(&b'x')).then(|| s.get(i + 2..i + 4).and_then(|h| u8::from_str_radix(h, 16).ok())).flatten();
            match esc {
                Some(c) => {
                    out.push(c);
                    i += 4;
                }
                None => {
                    out.push(b[i]);
                    i += 1;
                }
            }
        }
        String::from_utf8_lossy(&out).into()
    }

    pub fn with_space(mut d: Drive) -> Drive {
        let Ok(c) = std::ffi::CString::new(d.path.as_bytes()) else { return d };
        let mut st: libc::statvfs = unsafe { std::mem::zeroed() };
        if unsafe { libc::statvfs(c.as_ptr(), &mut st) } == 0 {
            d.free = st.f_bavail as u64 * st.f_frsize as u64;
            d.total = st.f_blocks as u64 * st.f_frsize as u64;
        }
        d
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        const WSL: &str = r"88 71 8:48 / / rw,relatime - ext4 /dev/sdd rw,discard
84 88 0:34 / /mnt/wsl rw,relatime shared:1 - tmpfs none rw
85 88 0:36 / /usr/lib/wsl/drivers ro,nosuid,nodev,noatime - 9p drivers ro,aname=drivers
90 89 8:48 / /mnt/wslg/distro ro,relatime shared:3 - ext4 /dev/sdd rw
119 88 0:24 / /sys rw,nosuid,nodev,noexec,noatime shared:15 - sysfs sysfs rw
122 88 0:63 / /run rw,nosuid,nodev shared:18 - tmpfs none rw,mode=755
140 88 0:72 / /mnt/c rw,noatime - 9p C:\134 rw,aname=drvfs;path=C:\;uid=1000
142 88 0:74 / /mnt/e rw,noatime - 9p E:\134 rw,aname=drvfs;path=E:\;uid=1000
252 88 8:48 /var/lib/docker /var/lib/docker rw,relatime shared:206 - ext4 /dev/sdd rw
256 253 0:87 / /run/user/1000/doc rw,nosuid,nodev,relatime shared:203 - fuse.portal portal rw";

        const DESKTOP: &str = r"26 1 259:2 /@ / rw,relatime shared:1 - btrfs /dev/nvme0n1p2 rw,subvol=/@
27 26 259:2 /@home /home rw,relatime shared:2 - btrfs /dev/nvme0n1p2 rw,subvol=/@home
28 26 259:1 / /boot rw,relatime shared:3 - vfat /dev/nvme0n1p1 rw
29 26 7:0 / /snap/core22/1380 ro,nodev,relatime shared:4 - squashfs /dev/loop0 ro
30 26 8:1 / /mnt/data rw,relatime shared:5 - ext4 /dev/sda1 rw
31 26 8:33 / /run/media/anna/My\040USB\040stick rw,nosuid,nodev,relatime shared:6 - exfat /dev/sdc1 rw
32 26 8:34 / /run/media/bob/Other rw,nosuid,nodev,relatime shared:7 - vfat /dev/sdc2 rw
33 26 0:50 / /home/anna/NAS rw,relatime shared:8 - cifs //nas/photos rw
34 26 8:49 / /media/anna/Книги\040📚 rw,relatime shared:9 - ntfs3 /dev/sdd1 rw
35 26 0:60 / /tmp/.mount_FilesVabc ro,nosuid,nodev,relatime shared:10 - fuse.FilesViewHub FilesViewHub ro
36 26 0:61 / /run/media/anna rw,relatime shared:11 - tmpfs tmpfs rw";

        fn kept(all: &str, user: &str) -> Vec<String> {
            all.lines().filter_map(parse).filter(|m| keep(m, user)).map(|m| m.point).collect()
        }

        #[test]
        fn keeps_the_drives_a_person_uses() {
            assert_eq!(kept(WSL, "user"), ["/mnt/c", "/mnt/e"]);
            assert_eq!(kept(DESKTOP, "anna"), ["/mnt/data", "/run/media/anna/My USB stick", "/home/anna/NAS", "/media/anna/Книги 📚"]);
        }

        #[test]
        fn reads_escaped_paths() {
            let m = parse(DESKTOP.lines().nth(5).unwrap()).unwrap();
            assert_eq!(m, Mount { root: "/".into(), point: "/run/media/anna/My USB stick".into(), fstype: "exfat".into(), source: "/dev/sdc1".into() });
            assert_eq!(parse(WSL.lines().nth(6).unwrap()).unwrap().source, r"C:\");
            assert_eq!(udev_name(r"My\x20USB\x20stick"), "My USB stick");
            assert_eq!(udev_name(r"\xd0\x9a\xd0\xbd\xd0\xb8\xd0\xb3\xd0\xb8"), "Книги");
            assert_eq!(udev_name("Книги_📚"), "Книги_📚");
        }

        #[test]
        fn kinds() {
            let m: Vec<Mount> = DESKTOP.lines().filter_map(parse).collect();
            assert_eq!([kind(&m[4]), kind(&m[5]), kind(&m[7]), kind(&m[8])], ["fixed", "removable", "network", "removable"]);
        }

        #[test]
        fn a_drive_keeps_its_serial_wherever_it_is_mounted() {
            let at = |point: &str| Mount { root: "/".into(), point: point.into(), fstype: "exfat".into(), source: "/dev/sdc1".into() };
            // A FAT or exFAT stick has the serial Windows shows for it, an NTFS drive the low half of its own.
            assert_eq!(serial(Some("1A2B-3C4D"), &at("/run/media/anna/STICK")), 0x1A2B_3C4D);
            assert_eq!(serial(Some("1A2B-3C4D"), &Mount { source: "/dev/sdb1".into(), ..at("/media/x") }), 0x1A2B_3C4D);
            assert_eq!(serial(Some("01D9E2F35A6B7C8D"), &at("/mnt/win")), 0x5A6B_7C8D);
            let ext4 = "3f6c8a2e-91b4-4d1a-b2c7-5e0f9a7d1c22";
            let a = serial(Some(ext4), &at("/mnt/data"));
            assert_eq!(a, serial(Some(ext4), &Mount { source: "/dev/sdb1".into(), ..at("/data") }));
            assert_ne!(a, serial(Some("3f6c8a2e-91b4-4d1a-b2c7-5e0f9a7d1c23"), &at("/mnt/data")));
            // Two subvolumes of one btrfs are two drives.
            assert_ne!(serial(Some(ext4), &Mount { root: "/@data".into(), ..at("/data") }), a);
            // No UUID: the mount source.
            let c = Mount { root: "/".into(), point: "/mnt/c".into(), fstype: "9p".into(), source: r"C:\".into() };
            let e = Mount { source: r"E:\".into(), ..at("/mnt/c") };
            assert_eq!(serial(None, &c), serial(None, &Mount { point: "/win/c".into(), root: "/".into(), fstype: "9p".into(), source: r"C:\".into() }));
            assert_ne!(serial(None, &c), serial(None, &Mount { fstype: "9p".into(), ..e }));
            // Never Computer's or Home's.
            assert_ne!(serial(Some("0000-0001"), &at("/x")), HOME);
            assert_ne!(serial(Some("0000-0000"), &at("/x")), 0);
        }

        #[test]
        fn a_path_is_on_the_innermost_drive_holding_it() {
            let d = |path: &str, serial| Drive { path: path.into(), label: String::new(), serial, kind: "fixed".into(), free: 0, total: 0, system: false };
            let ds = [d("/home/anna", HOME), d("/home/anna/NAS", 7), d("/", 0)];
            let on = |p: &str| super::super::drive_of(&ds, std::path::Path::new(p)).map(|d| d.serial);
            assert_eq!([on("/home/anna/Photos"), on("/home/anna/NAS/2024"), on("/home/anna2/x"), on("/home/anna")], [Some(HOME), Some(7), Some(0), Some(HOME)]);
            assert_eq!(super::super::rel_on_drive(&ds[0], std::path::Path::new("/home/anna/Photos/2024")), "Photos/2024");
        }
    }
}

/// A path without the `\\?\` prefix Windows adds to some paths.
pub fn plain(path: &Path) -> String {
    let p = path.to_string_lossy();
    p.strip_prefix(r"\\?\").unwrap_or(&p).to_string()
}

/// The Drive a path sits on, from the drives connected now.
pub fn drive_of<'a>(drives: &'a [Drive], path: &Path) -> Option<&'a Drive> {
    innermost(drives.iter(), path, crate::store::CASE_ASIDE)
}

/// Home on Linux: a folder of Computer shown as a Drive, so no mount of its own.
pub fn is_home(d: &Drive) -> bool {
    #[cfg(target_os = "linux")]
    return d.serial == linux::HOME;
    #[cfg(not(target_os = "linux"))]
    {
        let _ = d;
        false
    }
}

/// The mounted Drive a path sits on: as `drive_of`, with Home passed over. A tick of a folder
/// never takes in a drive mounted inside it, such as a network share in Home.
pub fn mount_of<'a>(drives: &'a [Drive], path: &Path) -> Option<&'a Drive> {
    innermost(drives.iter().filter(|d| !is_home(d)), path, crate::store::CASE_ASIDE)
}

/// The Drive with the longest path holding `path`, with case aside when `fold` is set: on Linux
/// `/media/usb` and `/media/USB` are two mount points.
fn innermost<'a>(drives: impl Iterator<Item = &'a Drive>, path: &Path, fold: bool) -> Option<&'a Drive> {
    let case = |s: &str| if fold { s.to_lowercase() } else { s.to_string() };
    let p = case(&plain(path));
    drives
        .filter(|d| {
            let d = case(&d.path);
            p.strip_prefix(&d).is_some_and(|r| r.is_empty() || d.ends_with(['\\', '/']) || r.starts_with(['\\', '/']))
        })
        .max_by_key(|d| d.path.len())
}

/// A path's place on its Drive, with forward slashes, like `Documents/Learning`.
pub fn rel_on_drive(drive: &Drive, path: &Path) -> String {
    let p = plain(path);
    p[drive.path.len().min(p.len())..].replace('\\', "/").trim_matches('/').to_string()
}

/// The path of a place on a Drive, given the Drive's current letter.
pub fn join_on_drive(drive: &Drive, rel: &str) -> PathBuf {
    let mut p = PathBuf::from(&drive.path);
    for part in rel.split('/').filter(|s| !s.is_empty()) {
        p.push(part);
    }
    p
}

/// Hidden or system folders (`$RECYCLE.BIN`, `System Volume Information`) never appear.
pub fn is_hidden(path: &Path, md: &std::fs::Metadata) -> bool {
    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    if name.starts_with('$') || name.starts_with('.') {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const HIDDEN: u32 = 0x2;
        const SYSTEM: u32 = 0x4;
        if md.file_attributes() & (HIDDEN | SYSTEM) != 0 {
            return true;
        }
    }
    let _ = md;
    false
}

/// The folders directly inside a folder, sorted by name, for one level of the folder tree.
pub fn folders(path: &Path) -> std::io::Result<Vec<Folder>> {
    let mut out: Vec<Folder> = std::fs::read_dir(crate::store::named(path))?
        .filter_map(Result::ok)
        .filter_map(|e| {
            let md = e.metadata().ok()?;
            let p = path.join(e.file_name());
            (md.is_dir() && !is_hidden(&p, &md)).then(|| Folder {
                name: e.file_name().to_string_lossy().into(),
                path: p.to_string_lossy().into(),
            })
        })
        .collect();
    out.sort_by_key(|f| f.name.to_lowercase());
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(path: &str, serial: u32, system: bool) -> Drive {
        Drive { path: path.into(), label: String::new(), serial, kind: "fixed".into(), free: 0, total: 0, system }
    }

    /// A `subst` letter of the system drive has its serial: it is the system drive too.
    #[test]
    fn a_letter_of_the_system_drive_is_the_system_drive() {
        let mut ds = [d(r"C:\", 7, true), d(r"E:\", 9, false), d(r"S:\", 7, false)];
        mark_system(&mut ds);
        assert_eq!(ds.map(|x| x.system), [true, false, true]);
    }

    /// A drive mounted inside another is its own mount; Home is no mount of its own.
    #[test]
    fn a_drive_mounted_inside_another_is_its_own() {
        let ds = [d("/home/anna", 1, false), d("/home/anna/NAS", 7, false), d("/", 0, true)];
        let on = |p: &str| mount_of(&ds, Path::new(p)).map(|x| x.serial);
        assert_eq!(on("/home/anna/NAS/2024"), Some(7));
        assert_eq!(on("/home/anna/Photos"), if cfg!(target_os = "linux") { Some(0) } else { Some(1) });
    }

    /// Mount points apart only by case are two Drives on Linux; Windows and macOS set case aside.
    #[test]
    fn mount_points_apart_by_case_are_two_drives_only_on_linux() {
        let ds = [d("/media/usb", 11, false), d("/media/USB", 12, false), d("/", 0, true)];
        let on = |p: &str, fold| innermost(ds.iter(), Path::new(p), fold).map(|x| x.serial);
        assert_eq!([on("/media/usb/A", false), on("/media/USB/B", false), on("/media/Usb/C", false)], [Some(11), Some(12), Some(0)]);
        assert_eq!(on("/media/usb/A", true), on("/media/USB/B", true), "case aside: one Drive");
        let ws = [d(r"E:\", 5, false), d(r"C:\", 1, true)];
        assert_eq!(innermost(ws.iter(), Path::new(r"e:\Books"), true).map(|x| x.serial), Some(5));
        let at = |p: &str| [drive_of(&ds, Path::new(p)), mount_of(&ds, Path::new(p))].map(|x| x.map(|x| x.serial));
        if crate::store::CASE_ASIDE {
            assert!(at("/media/usb/A").iter().all(|x| matches!(x, Some(11 | 12))));
            assert_eq!(at("/media/usb/A"), at("/media/USB/B"));
            assert_eq!(drive_of(&ws, Path::new(r"e:\Books")).map(|x| x.serial), Some(5));
        } else {
            assert_eq!([at("/media/usb/A"), at("/media/USB/B")], [[Some(11); 2], [Some(12); 2]]);
        }
    }
}
