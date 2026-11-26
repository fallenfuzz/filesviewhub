# A folder inside a Section takes its own kind, or becomes a Section of its own

A person often checks one folder that holds different things (`Data` with `Photos`, `Books` and
`Videos` inside it), and one kind for all of it is wrong for most of it. The app offers both
shapes, and the person picks one per folder:

- **A Folder kind.** The folder stays in its Section. Its kind and views hold for it and every
  folder below it, until a deeper folder sets its own. Its Works keep the Section's name and
  colour. A folder with the kind Photos shows its own years, months and Events from that folder
  down, with Review for that folder alone. Its Event ids use that folder in place of the Section's.
- **A Split-out Section.** Any folder, at any depth, nested or not, becomes a Section with its own
  name, colour, kind, views and rail entry. It can go into Show inside like any Section.
  The Section it came from keeps the folder as a row or tile in the new Section's colour, and
  that row or tile opens the new Section, so its Works are listed once. Put back returns it as a
  folder with a Folder kind, keeping its kind and views.

Folder settings… on every folder's menu is the Section settings dialog for a folder: no
name field, the kinds with Same as <Section> first, the levels Inside a folder and One Work (or the
Photos levels), each with Same as <Section> first, and Make it a Section. A Photos Section shows no
folders (`decisions/2026/10/08/photos-is-a-section-kind-in-the-app.md`), so its Section settings lists its top-level subfolders under Folders with other
things, each with the same two choices.

A kind change on a folder works as on a Section: a dry run counts the cards and the
Marks set aside, the folder is read again, and Undo puts it back. Work ids do not change with
either shape, since a Work id is its Drive and path (`decisions/2026/10/05/checked-folders-on-any-drive.md`), so Marks stay on the Works both
shapes still make.

The app guesses a kind for each top-level subfolder. When a guess differs from the Section's kind
and the subfolder holds 10% of the Section's files or 50 files, it offers once per Section: Set a
kind per folder, or Make each a Section. It never applies either itself. Section settings keeps
Suggest kinds for its folders to ask again.

Folder kinds and Split-out Sections are kept by Drive and path. A folder renamed or moved on disk
loses its Folder kind (the Section's applies, and a toast says so once). A Split-out Section whose
folder is gone shows as missing, as a Checked folder does. Neither is matched again by itself
(`decisions/2026/09/30/work-id-is-its-path.md`).

## Considered Options

- Folder kinds only: a friend's folder of mixed things stays one rail entry, and a Photos
  folder deep in it cannot be found from the rail.
- Split-out Sections only: the person must split each folder that needs a different kind, even
  when they want one Section in the rail.
- Show inside as the answer: it joins the same folder on two drives. It cannot nest one
  Section inside another.
- A split folder hidden from the Section it came from, as an unticked folder is: browsing that
  Section no longer finds it.
- Apply the guessed kinds automatically: a wrong guess changes cards and sets Marks aside with
  no request from the person.
