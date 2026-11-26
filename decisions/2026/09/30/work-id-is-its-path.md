# A Work's id is its own path, and Ticks are kept apart from Marks

A Work's id is its path from the folder the catalog page was built for, in its exact case: a Work made of files grouped by
name drops the extension (`Books/Jane Doe/Сказки`), a folder Work ends in `/`
(`Learning/.../Course One/`), so a file and a folder of the same name never share an id. The
id does not depend on which format the build opens first, so a PDF added next to an EPUB, or a
course grouped differently inside its folder, keeps its Mark. A Work that is one file by design (a
Cooking file, a loose audiobook file) keeps its extension, since `Soup.mp4` and `Soup.pdf` there
are two Works. The app keeps the same rule, with
paths from the root it is pointed at, because it will keep people's Marks under these ids.

Ticks are stored per Work, as the paths of the ticked items inside the Work's folder, by the same
rule (a lesson or title drops the extension, a Kids file or an audiobook track keeps it), apart from
the Marks: a lesson that later becomes a Work of its own cannot read an old Tick as a Done Mark,
and a course moved as a whole keeps its Ticks. Marks and Ticks are one versioned object,
`{v: 2, marks, ticks}`, both in the browser (under a key of its own, so an older page open in
another tab never reads it) and in the export file. The build stops when two Works share an id, an
item sits outside its Work's folder, or two items in a Work share a key.

A rename or move still orphans a Mark, and it is still never matched automatically: the Orphaned
list suggests Works in the same Section with the same name and no Mark of their own, and the Mark
moves with its Ticks only when the person picks one.

## Considered Options

- A hash of the path: as stable, but an export and a debugging session can no longer be read.
- A random id stored per Work: survives renames, but a read-only Dropbox and a page with no server
  have nowhere to keep it. Open again for the app, which has its own store.
- Ticks in the Marks map under a `tick:` prefix and their full path: moving a course orphans every
  Tick.
- Moving a Mark automatically when exactly one Work has the orphan's name: breaks the rule that a
  Mark is only ever set by hand, and a wrong match is silent.

## Consequences

- Notes in `notes/*.py` keep their own keys; the build lists every note that matches no Work, since
  a move drops a note as silently as it orphans a Mark.
- Files grouped by a loose name match (`[2015]` and `[2015}`, a trailing `_`) take the id of the
  file the build opens first; the build lists such Works so the names can be fixed in Dropbox.
- Stored marks in the older shape (Work or lesson link to `next`/`done`, under `hub-marks`) are
  left where they are; there were none to migrate.
