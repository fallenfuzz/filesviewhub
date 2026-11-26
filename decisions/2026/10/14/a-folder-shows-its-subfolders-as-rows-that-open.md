# A folder shows its subfolders as rows that open in place

This replaces the tiles of `decisions/2026/10/08/a-folder-shows-its-subfolders-as-tiles.md`. In the app, a folder with subfolders shows one expander row
per subfolder, as Windows 11 Settings does, before the Works it keeps itself ("In this folder").
A row names the folder and counts what it holds; a click, Enter or Space opens it in place, to
the folder's own subfolders as tiles and its first 12 Works. Rows start closed, and each one's
state is kept in settings.json with the other UI settings. A tile showed a few covers of a
subfolder but not its Works' titles or Marks; a row shows its Works where they are, without
leaving the page, and a long folder stays short while every row is closed.

View, and each row's menu, have Expand all and Collapse all. With a Filter on, only rows with a
match show, with their counts; a search lists every match without rows. A Photos Section keeps
its calendar (`decisions/2026/10/08/photos-is-a-section-kind-in-the-app.md`). View still offers every Work below, with no folders.

The folder columns and the lone-folder rule of `decisions/2026/10/08/a-folder-shows-its-subfolders-as-tiles.md` are unchanged.

## Considered Options

The four designs of the prototype on branch `prototype/folder-browsing` (its design
comment): A and D expander rows, B groups with a heading, C shelves. D was chosen.

- Rows start open: a folder with many subfolders becomes a long page of cards at once, and the
  person scrolls past what they did not ask for.
- A chevron at the row's end: the whole row is already the target, and the folder icon shows
  whether it is open.
- Panels inside panels (a subfolder's rows inside an open row): its subfolders show as tiles,
  which open the folder.
