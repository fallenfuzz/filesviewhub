# A folder is a Work or a place, never both

In every Section a folder is either a Work (one card) or a place (a folder column entry that
holds Works), never both. Learning (chapter folders inside a course) and Books (format folders
joining the Work above) already follow this; Kids breaks it. In Kids a folder that holds files
of its own and subfolders falls into one of two shapes, and each is resolved to one identity:

- **A Work with parts.** A book or activity pack whose subfolders hold its parts (`CD1`-`CD3`,
  `Audio`, `UK`/`US`, `resource`, `... Звук`, `Picture Dictionary`, `123`). The folder is one
  Work; its subfolders, at any depth, are groups in its dialog, not Works and not column
  entries. A reader with three CDs was four Works (the book and three CD cards titled "CD1" to "CD3",
  each with its own Mark and borrowing the book's cover); it becomes one card, one Mark, a Tick
  per track.
- **A shelf.** A folder of separate Works that also has loose files (`Readers/LEVEL 2`:
  loose books beside book folders). The folder is only a place. Each loose file, or the
  loose files that share one title in several formats (as Books groups them), is a Work of its
  own. Before this, the loose books were one Work named "LEVEL 2" that could take one Mark.

The build tells the shapes apart by what the subfolders hold (audio only, or a part name such
as audio, CD, UK, US, Звук, resource, glossary, dictionary, a bare number), lists every folder
it treats as a shelf in the build report, and takes a notes override when the guess is wrong:
the same pattern as Learning's `COURSES`. Neither folder names nor counts settle it alone:
`Reader A` is one book with four CD folders, `Readers Stage 2` a shelf with one stray file and
many book folders.

A Work that holds both a book file and its audio carries both Media, Reading and Listening, so
either filter finds it; the graded readers are meant to be read along with their audio.

## Considered Options

- Keep part subfolders as separate Works: one book showed as two to five cards with different
  counts and Marks, and search found only one of them.
- One "loose files" Work inside a shelf: honest about the folder, but still a fake Work that
  cannot say which of 27 books was read.
- A hand-kept list of shelves only: every new folder would need an entry; the guess plus report
  plus override covers new folders and still lets a wrong call be fixed.
- Majority file kind as the single Medium: a book with 30 tracks would vanish from Reading.

## Consequences

- The same book as a loose file and as a folder in one shelf (`A_Story.pdf` beside
  `A Story_есть`) stays two Works; the build reports likely duplicates rather than
  merging them, since a wrong merge hides a book and the build never changes the person's folders.
- Marks set on former part Works (a `CD2` card) become Orphaned Marks; no migration, the
  gone-Works report lists them.
- Needs a build change (Kids Works, the shelf report, the notes override, Medium as a list)
  and a page change (filters matching any of a Work's Media); that is its own task.
