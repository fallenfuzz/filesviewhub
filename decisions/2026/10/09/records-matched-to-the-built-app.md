# Earlier records matched to the app as built

These points replace what `decisions/2026/09/29/folder-shows-own-works-and-one-box-shape.md`, `decisions/2026/09/30/hub-lists-every-file.md`, `decisions/2026/09/30/a-folder-is-a-work-or-a-place-never-both.md`, `decisions/2026/09/30/work-id-is-its-path.md`, `decisions/2026/10/01/settings-in-one-versioned-store.md`, `decisions/2026/10/01/marks-and-ticks-carry-when-they-were-set.md`, `decisions/2026/10/04/photo-ai-is-an-optional-separate-index.md`, `decisions/2026/10/04/sorting-ends-in-a-move-script-never-a-delete.md`, `decisions/2026/10/05/filesviewhub-is-a-tauri-app-shipped-portable.md`, `decisions/2026/10/05/checked-folders-on-any-drive.md` and `decisions/2026/10/05/nothing-hidden-no-network-by-default.md`
say, where the built app differs.

- **Tools (`decisions/2026/10/05/filesviewhub-is-a-tauri-app-shipped-portable.md`).** The zip carries the stock LGPL ffmpeg build (BtbN `lgpl-shared`, about 150
  MB unpacked), not a stripped one. The app does not use LibreOffice: an Office file's thumbnail is
  the preview Word, PowerPoint or LibreOffice saved inside it, else a generated tile. A stripped
  ffmpeg and slides through LibreOffice are remaining work.
- **About (`decisions/2026/10/05/filesviewhub-is-a-tauri-app-shipped-portable.md`, `decisions/2026/10/05/nothing-hidden-no-network-by-default.md`).** The app has no About screen. Settings shows "Version <version>
  (build N)" below its list of tabs, on every tab. The Copy log button is on Settings, Log.
- **Network (`decisions/2026/10/05/nothing-hidden-no-network-by-default.md`).** With every network feature off, the app itself makes no network call.
  WebView2 still opens one connection to Microsoft as it starts; the app starts it with every
  other connection off and cannot stop that one, and "What this app does" says so.
- **A Drive not connected (`decisions/2026/10/05/checked-folders-on-any-drive.md`).** Its Sections stay in the catalog with their cards and
  thumbnails. The Section's row in the library column is greyed, its drive's heading there says
  "not connected", and its pane says it shows what was read last time; the cards are not greyed.
- **Every file listed (`decisions/2026/09/30/hub-lists-every-file.md`).** The app lists every file that no Work holds under Unlisted,
  with its reason, as the Hub did. It does not count the files it walked against the files it
  listed, and nothing stops when the two differ.
- **Card shapes (`decisions/2026/09/29/folder-shows-own-works-and-one-box-shape.md`).** Normal and Large cards in a view share one box shape, as the Hub's
  did. Compact rows show each cover in its own shape.
- **Shelves (`decisions/2026/09/30/a-folder-is-a-work-or-a-place-never-both.md`).** The app tells a Work with parts from a Shelf by what the subfolders hold,
  as the Hub did. It has no notes, so no override for a wrong guess and no report of its Shelves.
- **Checks on ids (`decisions/2026/09/30/work-id-is-its-path.md`).** The app does not stop or warn when two Works share an id, an item
  sits outside its Work's folder, or two items in a Work share a key.
- **Stores (`decisions/2026/09/30/work-id-is-its-path.md`, `decisions/2026/10/01/settings-in-one-versioned-store.md`, `decisions/2026/10/01/marks-and-ticks-carry-when-they-were-set.md`).** `hub-marks-v3`, `hub-settings`, ISO dates and null dates are
  the Hub's. The app keeps Marks, Ticks and Review choices in `marks.json` in StoredData as
  `{v: 2, marks: {id: {m, at}}, ticks: {id: {key: at}}, review}`, with `at` in seconds since 1970
  and never null, and its window settings as `ui` in `settings.json`. Its backup is
  `marks.json` with `kind: "filesviewhub-marks"` and the window settings as `ui` added. Adding from a backup keeps the
  later of two Marks for a Work, even when both are the same Mark, keeps the earlier time of a
  Tick, and replaces the window settings.
- **Photo AI (`decisions/2026/10/04/photo-ai-is-an-optional-separate-index.md`).** `photos/index.py`, `cache/photos-index.json` and `build.py` are the
  Hub's. The app has no AI index; it is remaining work.
- **The set-aside list (`decisions/2026/10/04/sorting-ends-in-a-move-script-never-a-delete.md`).** The app saves the move script only, not the plain list beside
  it. Saving the list is remaining work.

`decisions/2026/10/08/the-app-saves-the-move-script-for-its-own-system.md` and `decisions/2026/10/09/review-moves-set-aside-photos-itself-with-undo.md` cite `decisions/2026/10/05/app-data-local-cache-movable.md` for the rule that the app never writes into a checked folder. `decisions/2026/10/07/app-data-in-storeddata-next-to-the-app.md` replaced `decisions/2026/10/05/app-data-local-cache-movable.md` and keeps that rule, so the citations hold.
