# In Photos, an Event is the Work, and its id is not a path

The Photos Section reads one flat folder of about 5,400 Photos, so no folder marks a Work. The build
groups Photos into Events (runs with no long gap between them), and an Event is the Work: it gets the
card, it takes a Mark (Done means gone through), and its Photos are items in its dialog that take no
Tick. An Event's id is its Section's folder, `#` and its first Photo's name without the extension
(`Pictures/Camera#2018-02-12 19.32.38`), since no path of its own exists to
follow `decisions/2026/09/30/work-id-is-its-path.md`. In place of folder columns, Photos is narrowed by the year and month of each Event's
first Photo; an Event that crosses midnight or New Year sits in one month only. Photos with no date
in the name or EXIF form one Undated Event, never dated by mtime, which on a synced folder is often
the copy date.

## Considered Options

- A Day as the Work (`#2018-02-12`): a stable id with no gap to tune, but a trip becomes many cards
  and a party past midnight splits in two.
- A Photo as the Work, with Events as a view only: 5,400 cards and ids, the heaviest option for the
  page.
- Marks off in Photos: Next and Done read oddly on photos, but the folder is a backlog to sort, and
  Done on an Event shows how much of it is gone through at no cost.

## Consequences

- Changing the gap moves Event boundaries and so changes ids: Done marks become Orphaned and names in
  `notes/photos.py` become unused notes. Tune the gap on the real data once, before anyone marks or
  names an Event.
- A Photo added before an Event's first Photo changes that Event's id the same way. The folder is
  frozen (2017 to 2022), so this is accepted.
