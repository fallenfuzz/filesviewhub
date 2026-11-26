# Marks and Ticks carry the time they were set

Every Mark and every Tick holds the time it was set. The store becomes
`{v: 3, marks: {id: {m, at}}, ticks: {id: {key: at}}}`, where `m` is `next` or `done` and `at`
is an ISO time, or null when it is not known. It lives under its own key, `hub-marks-v3`, and
the export carries the same object. Setting the Mark a Work already has keeps its date, and so
does ticking an item that is already ticked. When the v3 store is missing, the page reads the v2
store, `{v: 2, marks: {id: m}, ticks: {id: [key]}}`, and carries it over with every date null.
The v2 key is left in place for an older page still open in another tab. Import accepts both
versions. When a backup has the Mark a Work already has, the known date is kept.

The app will import what people already have in the page. A "Done lately" history, an In
progress list ordered by last activity, and streaks all need dates, and a date can only be
recorded when the Mark is set, never filled in afterwards. Changing the format later would mean
migrating Marks that people keep in the app.

## Considered Options

- Dates in a separate map next to v2's `marks` and `ticks`: smaller change, but the date and the
  Mark could drift apart, and the app would import two things that must agree.
- One date per Work for its Ticks (the last activity) instead of one per Tick: smaller, but loses
  when each lesson was done, which a history or a streak needs.
- Date only, without the time: shorter, but cannot order two things done on the same day.

## Consequences

- Marks and Ticks set before v3 have no date, and nothing in the page can give them one.
- Until the first change is saved, the page reads the v2 store on each load, so a Mark set in an
  older page still shows up. After that, the two stores are separate.
