# FilesViewHub glossary

A browsable catalog of the media kept in folders a person picks, on any drive. It exists so the
collection can be searched and tracked from one place instead of by walking folders.

## Language

**FilesViewHub**:
The desktop app. "Hub" alone, in older records, is the catalog it shows; its first form was a
static catalog page built for one folder, which some records call the Hub page.
_Avoid_: viewer, the build

**Checked folder**:
A folder the person ticked in FilesViewHub's folder tree; everything under it is in the catalog except
the subfolders unticked inside it. A whole Drive can be one, except the system drive and Computer
(`decisions/2026/10/21/a-whole-drive-can-be-a-section-except-the-system-drive.md`).
_Avoid_: root, source, library, watched folder

**Drive**:
A disk or partition, internal or external, that holds Checked folders. The Works on an unplugged
Drive stay in the catalog. On Linux, Home and Computer (the whole file system) are Drives too.
_Avoid_: volume, disk, device

**Section**:
One area of the catalog (Cooking, Learning, Books, Kids, Photos): a top-most Checked folder, a
Split-out Section, or several merged into one.
_Avoid_: page, tab, category

**Split-out Section**:
A Section made from a folder inside another Section, at any depth. It is a Section in every way (its
own name, colour, kind and views) and its Works belong to it alone; the Section it came from shows
the folder as a link to it. It can be put back, keeping its kind and views.
_Avoid_: subsection, child Section, nested Section

**Section kind**:
How a Section makes Works from its files: Courses, Books and documents, Collections, Videos and
files, or Photos. Every Section has exactly one.
_Avoid_: type, mode, template, preset

**Folder kind**:
A Section kind set on one folder inside a Section, for that folder and everything below it until a
deeper folder sets its own. The folder and its Works stay in the Section. A folder with no Folder
kind is "Same as" its Section.
_Avoid_: sub-kind, override, folder type

**Medium**:
How a Work is consumed: Watching, Reading or Listening. A Work has one or more (a book with its
audio is both Reading and Listening). A filter within a Section, never a Section of its own;
audiobooks are Books with the Listening medium. Photos have no Medium.
_Avoid_: type, format, kind

**Work**:
One book, course, video or audiobook as a person thinks of it, regardless of how many files or
formats hold it. A Work is what gets a card. In Kids, a Work is a folder that directly holds
files (an activity pack, an album), not a single file; its subfolders holding parts of it (CDs,
audio, UK/US versions, resources) are inside the Work, not Works of their own. A Kids folder of
separate Works (a Shelf) is only their Path, and each title loose in it (in one or more formats) is a Work. An audiobook is
likewise the folder that directly holds its tracks; folders above it (an author, a series) are only its Path. In Learning, a course
is one Work with its chapter folders inside it; a folder that holds several separate courses (a
vendor or topic folder) is only their Path. In Books, a folder named after a
format (DJVU, EPUB, FB2...) is not part of the Path: its files join the same Work in the folder above.
_Avoid_: item, file, entry

**Photo**:
One picture or short video clip from a camera or phone. A Photo is an item inside its Event, never
a Work of its own, and takes no Tick.
_Avoid_: image, media, clip, asset

**Event**:
A run of Photos taken with no long gap between them; the Work in Photos, and what gets the card. It
exists only in the catalog: no folder holds it, and it may gather Photos from several subfolders. An Event sits in the year and month of its first Photo.
Photos whose name gives only the day form an Event of that day with no times (`decisions/2026/10/09/a-name-with-only-the-day-makes-an-event-of-that-day.md`). Photos with
no known date taken form one Undated Event.
_Avoid_: album, session, trip, day

**Similar shots**:
Photos of the same moment: taken seconds apart, or looking almost the same within minutes. Each set
has a Best one; the others are tagged Extra in the Event and offered in Review. Two files of one
picture are Copies, not Similar shots.
_Avoid_: burst, near-duplicate, series, stack

**Best one**:
The one Photo of a set of Similar shots the app suggests keeping: the sharpest where the detail is,
with the least shake and the fewest burnt-out or black areas. A suggestion only; nothing is deleted.
When the shots look about the same, the set is Your call.
_Avoid_: best shot, keeper, pick, favourite

**Your call**:
A set of Similar shots with no clear Best one: the person picks the one to keep.
_Avoid_: tie, close call, unsure

**Copies**:
Two or more files of one picture under different names: the same content, or the picture saved
again smaller or at another quality (`decisions/2026/10/09/copies-include-pictures-saved-again.md`). The largest is kept.
_Avoid_: duplicates, dupes

**Junk**:
A Photo the app guesses is probably not needed (a screenshot, a forwarded picture, a blurred or very
dark shot), with its reason. Shown like any other Photo, tagged with its reason, and listed under
Probably not needed. Junk alone never waits for Review (`decisions/2026/10/21/junk-alone-never-waits-for-review.md`).
_Avoid_: trash, spam, bad photo

**Review**:
Choosing, for each Photo the app suggests may not be needed (an extra Similar shot, a Copy),
whether to keep it or set it aside (on screen: "Not needed"). A Photo is "to review" until that
choice is made. The count of Photos to review is the only progress in Photos; it is not a Mark.
Photos set aside move only when
the person moves them, into a `Not needed` folder inside the photo folder or a folder they pick, and
a move can be undone.
_Avoid_: sort, sorted, done, clean up

**Person**:
A face that recurs across Photos, named by hand. Only present when the optional photo index has been
run.
_Avoid_: face, contact, tag

**Shelf**:
A folder that holds separate Works and is only a place: an entry in the folder columns, never a
card. A folder is a Work or a Shelf, never both.
_Avoid_: collection, series, group

**Path**:
The folders under its Section a Work sits in, shown on its card; each folder in it narrows the catalog to
that folder. A leading "!" in a folder name only pins it to the top in a file list and is not shown.
_Avoid_: location, breadcrumb

**Work id**:
The Work's Drive and its path on that Drive: without the extension for a Work grouped from files of the
same name, ending in "/" for a folder. It does not change when a format is added, only when the
Work is renamed or moved. An Event has no folder of its own: its id is the folder whose kind is
Photos (its Section's, or a folder's with the Folder kind Photos), "#" and the name of its first
Photo without the extension.
_Avoid_: key, link, href

**Mark**:
A note a person sets by hand on a Work: Next (want to do it) or Done (watched, read, listened,
cooked). Never set automatically. Photos take no Mark. A Work holds at most one Mark, and has none by default. A Mark
remembers when it was set; setting the same Mark again keeps that date.
_Avoid_: status, flag, bookmark, progress

**Tick**:
Done on one item inside a Work (a lesson, a track, a title in a collection), set by hand in the
Work's dialog and belonging to that Work, never read as a Mark. Ticks show progress; ticking every
item offers the Work's Mark but never sets it. A Tick remembers when it was set.
_Avoid_: checkbox, sub-mark

**In progress**:
A Work with at least one Tick on an item it still has and no Done Mark, whatever its Next Mark. A
Work with every item ticked stays In progress until it is marked Done. Never stored: it follows from
the Ticks and the Mark. The In progress list shows the latest-ticked first.
_Avoid_: started, continue, partly done

**Unlisted file**:
A file in a Section folder that no Work links to. The app lists every one under Unlisted with its
reason: not placed (a known type no Work took), unknown type, captions without a video, in a linked
folder or code and config outside any Work, or in a program folder. Code, config and exercise files inside
a Work are not unlisted: the Work lists them under Code & exercise files, and captions show as CC on
the lesson they belong to. Unlisted files are not Works: they take no Mark or Tick. The app's
screens call them Files not on a card.
_Avoid_: hidden file, orphan, missing file

**Orphaned Mark**:
A Mark or Tick whose Work or item no longer exists in its Section, usually after a rename or move.
Never matched automatically: the app suggests Works of the same name in the same
Section, the app also finds Works in that Section by part of a new name, and the person picks one
to move the Mark and its Ticks to, or re-marks by hand.
