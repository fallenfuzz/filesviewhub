# Each Section picks its view at each level

Section settings shows, for the kind picked, a tab for each view level (for a folder Section: the
Section, inside a folder, one Work; for Photos: all years, a year, a month, a day or event). Each
tab shows that level's views as samples drawn from made-up content, never the person's files, and
the person picks one. The pick is kept per Section and level in settings.json as
`views[section][level] = view`; a missing entry is the level's first view, which is what the app
showed before.

A change of view only redraws: nothing is read again and no Mark changes, so it is applied at once
with Undo. A change of kind still reads the Section again. A name change alone is made in the
catalog in place, with no read.

What a folder shows (rows that open, or every Work below) was one choice for every Section, in
Settings, Sections and in View. It is now the Section's own "Inside a folder" view. View keeps these
choices, with Folder tiles added, for the Section open, plus More views…, which opens its settings on that tab. The
choice in Settings, Sections is gone. Settings kept with every Work below picked give each Section
that view once.

Only views the app draws are offered. Each view still to build is its own issue, and its sample
appears when it is built.

## Considered Options

The four designs of the prototype on branch `prototype/view-samples` (its design
comment): A level tabs over a gallery of samples, B every level on one page, C a walk through the
levels in one big sample, D samples in each page's View menu. A was chosen.

- The Section's own files as the preview: a large Section makes the preview slow, and the
  person cannot compare views of content they know is the same.
- Keep the choice in Settings, Sections as the default for Sections with no pick: two places set
  one thing, and the Default tile would change meaning with a setting elsewhere.
- Drop the Subfolders choice from View: switching back and forth on the page is the common case.
