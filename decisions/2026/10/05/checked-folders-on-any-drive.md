# The person checks folders on any drive; a Work id is its Drive and its path on it

On first run, and later in Settings, FilesViewHub shows every drive as a tree with checkboxes, as
Dropbox selective sync does. Nothing is checked by default and the app never offers "everything".
Each top-most checked folder becomes a Section, named after the folder, renamable, and mergeable
with another (a Books folder on D: and one on an external drive as one Books Section). For the
app this replaces `decisions/2026/09/30/work-id-is-its-path.md`'s "path from the Root": a Work id is its Drive's volume serial number
and its path on that Drive, so an external drive that comes back as `F:` instead of `E:` keeps its
Marks. An unplugged Drive's Works stay in the catalog, greyed, with their thumbnails.

The catalog stays current by a quick rescan on launch (file names, sizes and dates only; changed
files are thumbnailed again) and a folder watcher while the app is open; Refresh and Rebuild
thumbnails exist per folder and for everything. A Dropbox or OneDrive online-only file is never
read: reading it makes the cloud client download it, so it shows a cloud tile until it is on disk.

## Considered Options

- One Root: simpler, but a person's library sits on several drives.
- The drive letter in the id: Windows can give an external drive a different letter each time.
- Watcher only: misses everything changed while the app was closed. Rescan only: stale while open.
