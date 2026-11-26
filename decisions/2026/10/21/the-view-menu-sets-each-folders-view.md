# The View menu sets each folder's view

This changes the view picking of `decisions/2026/10/14/each-section-picks-its-view-at-each-level.md` and `decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`. Views were picked in two places: View's
Subfolders on the page, and the level tabs of Section settings and Folder settings. A pick in
View went where the open folder's view came from, the nearest folder with one of its own or the
Section, so a pick inside a folder changed its whole Section. And View offered its three views
on every page, also where there were no subfolders to show, so a pick in a Courses Section of
courses or in a folder of only books changed nothing.

View's Subfolders is now the one place a view is picked. At a Section's top it sets the
Section's view; inside a folder it sets that folder's own, which the folders below it take until
one has its own, as `decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md` had it. Same as <the nearest folder above with its own view, or the
Section>, first in the list with the view it gives, drops the folder's own. It is offered only where the page has
subfolders, and inside a folder only where the app places that folder on disk: a folder that holds
only Split-out Sections, or a Photos folder's year, has no view of its own to set, and never sets
the Section's in its place. Only the Inside a folder level has more than one view the app draws; the others
(the Section, One Work and the Photos levels) have one each, so nothing is picked for them yet.
A level that gets a second view gets its pick where that level shows. The picks are kept as
before: a Section's in settings.json `views`, a folder's in its `folder_kinds` entry.

Section settings and Folder settings keep the kind, the name, Show inside, Make it a Section,
Suggest kinds and a Photos Section's Folders with other things. Their samples stay on the right
as How <kind> looks: one made-up sample per level, in the view used there now, with nothing to
pick, so the kinds can be told apart before one is chosen. More views… in View is gone.

## Considered Options

- This folder only, its subfolders keeping the Section's view: each folder of a deep tree must
  be set by hand, and a Folder kind already holds for the folders below.
- No Same as in View, picking the view of the folder above to go back: the folder keeps its own
  pick, so a later change above no longer reaches it, and nothing says so.
- Kinds only in the dialogs, the samples gone: Courses, Books and Collections are hard to tell
  apart by their one line of text.
- Keep the level tabs as a second place to pick: two places set one thing, and the one in the
  dialog shows the change only once the folder is opened.
