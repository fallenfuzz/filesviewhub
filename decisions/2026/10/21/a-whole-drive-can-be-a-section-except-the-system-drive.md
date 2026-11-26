# A whole drive can be a Section, except the system drive

A USB stick or a second disk often holds its files at the top, with no folder around them. The
folder tree gives every drive row a tick, as a folder has, so the whole drive can be a Section.
Its name is the drive's label, else its letter (`E:`) or its mount point on Linux.

The system drive (the drive Windows runs from, and any other letter with its serial, such as a
`subst` of a folder on it) and Computer (`/` on Linux and macOS) take no tick: only folders on
them can be ticked. All of either holds the
system and its programs, hundreds of thousands of files that are slow to read and that nobody
browses, and `decisions/2026/10/05/checked-folders-on-any-drive.md` has the app never offer "everything". Home on Linux is a person's own
folder, so it takes a tick. A drive mounted inside a ticked folder (a network share in Home) stays
a drive of its own: the tick never takes it in, and it is ticked or not by itself.

A whole drive's id is `{serial}:`, with nothing after the colon. Every key inside it is that id and
the path on the drive (`{serial}:Books/a.pdf`), with no `/` after the colon. This is the key the
same file has when only its folder is ticked, so a Work keeps its id and its Marks whether the
drive or a folder on it is ticked, and a Split-out Section of a folder on it has that folder's id
(`decisions/2026/10/05/checked-folders-on-any-drive.md`, `decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`).

## Considered Options

- Every drive, the system drive too: one tick reads Windows or `/proc`, for no use.
- Every drive, with a warning on the system drive: a warning a person clicks past, for the same
  result.
- A whole drive's keys as `{serial}:/Books/a.pdf`: no change to how keys are joined, but a Work
  would get another id when its folder is ticked in place of its drive, and its Marks would be
  orphaned.
