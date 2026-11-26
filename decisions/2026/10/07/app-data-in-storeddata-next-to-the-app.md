# App data lives in StoredData next to the app; replacing the app keeps it

This replaces `decisions/2026/10/05/app-data-local-cache-movable.md`. FilesViewHub keeps its settings, catalog, Marks, log and, by default, its
cache in a `StoredData` folder next to `FilesViewHub.exe` (on macOS, next to the `.app`, never
inside the bundle). Nothing goes in `%LOCALAPPDATA%`, Roaming or any other system folder, and
nothing is written into a checked folder. The zip carries no `StoredData`, so an update is
replacing the `.exe` and `tools\`, or unpacking the new zip over the old folder. A backup is a
copy of `StoredData`; removing the app and all it saved is deleting its folder. Settings can still
move the cache to another drive; Marks never move with it.

## Considered Options

- `%LOCALAPPDATA%\FilesViewHub` (`decisions/2026/10/05/app-data-local-cache-movable.md`): survives a deleted app folder, but ties the app to a
  system folder the person does not see, and splits backup and removal into two places.
- A `portable` marker file switching between the two: two behaviours to explain and test.

## Consequences

- The app folder must be writable. If the app cannot write `StoredData` at start (for example
  under `Program Files`, or a macOS app run from Downloads, which macOS starts from a read-only
  copy), it says so, names the folder and closes.
- Deleting the app folder to update deletes the Marks with it; the README says to replace files
  instead.
