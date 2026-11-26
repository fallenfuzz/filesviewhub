# FilesViewHub

Guidance for AI coding agents (Claude Code, Codex, Cursor and others) working in this repository.
`CLAUDE.md` imports this file, so every agent reads the same text.

FilesViewHub is a desktop app (Tauri 2, Rust core, Svelte and TypeScript UI) that turns folders a
person checks on any drive into a browsable catalog with thumbnails, search, folders and Next/Done
marks. Windows first, then macOS (`decisions/2026/10/05/filesviewhub-is-a-tauri-app-shipped-portable.md`);
Linux ships as an AppImage.

> Where things live: decisions and their reasons: `decisions/` (map of old numbers:
> `decisions/README.md`); words the code and docs use: `docs/glossary.md`; what is built:
> `docs/done.md`; how it looks: `DESIGN.md`; how skills use the issues and records: `docs/agents/`;
> remaining work: GitHub issues, labelled `bug`, `major`, `release`, `macos`,
> `linux`; product references: `docs/references.md`; how to send a change: `CONTRIBUTING.md`;
> instructions for people (also shown in Settings, Help): `README.md`. A package's release note is
> its annotated tag (`git tag -l 'filesviewhub-*'`, then `git show <tag>`). Update this file in the
> same change as anything it describes.

## Rules

- The app never writes into a checked folder
  (`decisions/2026/10/05/app-data-local-cache-movable.md`); its data is in `StoredData` next to the
  exe (`decisions/2026/10/07/app-data-in-storeddata-next-to-the-app.md`).
- Every store, tool and network call the app has appears on its "What this app does" screen
  (`decisions/2026/10/05/nothing-hidden-no-network-by-default.md`).
- Everything works on Windows, macOS and Linux: the app, the scripts handed to the person (one for
  each system) and the build tools. A gap on one system is an issue labelled `macos` or `linux`.
- Every visible word is in `src/lib/strings.ts` (`scripts/strings.test.mjs` checks it).
- Every context menu and ⋯ menu is `src/lib/Menu.svelte`, opened with `openMenu` from
  `src/lib/menus.svelte.ts`; every toast is `src/lib/ToastBox.svelte`, shown with `showToast`
  from `src/lib/toast.svelte.ts`, and every error is one, shown with `showError`. Never a native
  dialog box for a message.
- A built item goes into `docs/done.md` in the same change, and the commit closes its issue
  (`Closes #N`). A new remaining item is a new issue, never a file.
- A change that makes a decision record untrue adds a supersede note at its top and a new record,
  in the same commit (`decisions/README.md`).
- Text written into the repository is in English, plain words,
  with a hyphen, never an em dash.

## Output

- Windows: `release/FilesViewHub-<version>-build-<n>-windows.zip` (`FilesViewHub.exe`, `tools/`,
  `README.md` with `README-files/`, `LICENSE.txt` and `THIRD-PARTY-NOTICES.txt`); `npm run
  package` on Windows (`npm run fetch-tools` once first).
- Linux: `release/FilesViewHub-<version>-build-<n>-linux-x86_64.AppImage`, with FFmpeg and PDFium
  inside (ddjvu is the system's), from the same source plus `src-tauri/tauri.linux.conf.json`;
  `npm run package` on Linux. It carries the app's own source at the built commit, as a GPL 2
  library it loads asks (`decisions/2026/10/20/licences-and-notices-made-for-each-package.md`).
- Both are build artifacts; `release/` keeps the newest, older ones go to `release/archive/`.
- CI: `.github/workflows/check.yml` runs the gate on Windows, Linux and macOS (macOS may fail
  for now). A pushed `filesviewhub-*` tag makes a draft release of both packages, notes from the
  tag (`.github/workflows/release.yml`).
- From WSL, `bash scripts/release.sh` makes both (`windows` or `linux` for one): it stops on
  uncommitted changes, runs `scripts/check.mjs`, then packages on Windows through `cmd.exe` and on
  Linux through `scripts/linux.sh`.

## Code map

- `src-tauri/src/` (Rust core): `drives.rs` drives and the folder tree; `scan.rs` Sections and
  Works by kind (a Photos folder's Events too, a Photos Section's folders of other kinds, the links
  to Split-out Sections, and the kinds offered for its folders), the unlisted files and the kind
  preview; `photos.rs` Photo dates and Events; `shots.rs` Review's Copies, Similar shots and Best
  one, the Junk label (`decisions/2026/10/21/junk-alone-never-waits-for-review.md`) and the move
  scripts; `moves.rs` Review's move and its Undo
  (`decisions/2026/10/09/review-moves-set-aside-photos-itself-with-undo.md`); `store.rs` data
  files; `packed.rs` the catalog file's compact form (version 3) and the parts of its Works and
  unlisted files the page is sent; `thumbs.rs` thumbnails; `details.rs` lengths and
  page counts; `covers.rs` covers stored inside books and Office files; `pdf.rs` the PDFium child
  process; `tools.rs` running the bundled tools; `chosen.rs` the covers the person chose
  (`covers.json` and `Covers/` in StoredData); `web.rs` the cover search of Choose a cover…;
  `programs.rs` the program each type opens in; `logs.rs` the logs read for Settings, Log, the log
  plugin's setup and the logger that holds what is logged before the log starts; `lib.rs` commands.
- `src/` (Svelte UI): the view levels of each kind and the views built in `lib/views.ts`; the
  cards' order in `lib/sort.ts`; each folder's own kind and views in
  `lib/folders.svelte.ts`; where each folder shown is on disk in `lib/rels.ts`; the open Section's
  folders with their Works and files in `lib/tree.ts` (one pass per Section; checked by
  `scripts/tree.test.mjs`); the catalog's parts made whole in `lib/packed.ts` (checked against
  `scripts/packed.fixture.json`, which the Rust tests write); their made-up samples in
  `lib/Sample.svelte`; menu items built in `lib/Library.svelte` (Photos' years, months and Events
  too, handed to `lib/Photos.svelte`); Review is `lib/Review.svelte`, in its own window
  `lib/ReviewWindow.svelte` (`decisions/2026/10/27/review-has-its-own-window.md`); Choose a cover…
  is `lib/CoverPicker.svelte`; Settings, Help renders `README.md` through `vite.config.ts`;
  Settings, Licences is `lib/Licences.svelte`.
- `README.md` is for people and shows in Help; its pictures are in `README-files/`.
- `scripts/fetch-tools.mjs`: the bundled tools, pinned by SHA-256. `scripts/notices.mjs`: the
  licences and notices of every part, made into `tools/notices.json` and
  `tools/THIRD-PARTY-NOTICES.txt` by each package; texts no part ships itself are in
  `scripts/licences/`.
- `src-tauri/icons/icon.svg` is the app icon; every other file in `icons/` is made from it.

## UI direction

Folder columns made native (drives and Sections, folders, cover grid, a status bar showing what
the app watches, network state and cache), one chrome surface around a raised content pane, framed
as File Explorer in Windows 11: a rail when the columns are folded, an address bar with chevron
menus, a command bar. Where it departs from the earlier catalog page on purpose:
`decisions/2026/10/08/where-the-app-departs-from-the-hub.md`.

## Known issues

- WebView2 opens one connection to Microsoft as it starts, from its main process; the browser
  arguments in `lib.rs` (`WEBVIEW_ARGS`) stop every other one, and "What this app does" says so.
  To check: `scripts/net-check.ps1` (Windows) lists every connection of the app's process tree.
  The `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` variable adds to those arguments, so DevTools still
  works.
- The app goes online only when a person presses Search in Choose a cover… (`web.rs`,
  `decisions/2026/10/22/covers-are-chosen-never-fetched-by-themselves.md`): the search words go to
  Open Library and FantLab, the covers found show by their small pictures, and only the one used is
  fetched in full. Each request, and each redirect it is sent on, goes into `Logs/network.log`
  first. A chosen cover is kept in StoredData and wins over what the files give.
- With Windows' ransomware protection (Controlled Folder Access) on, Windows stops the app writing
  in Documents, Desktop, Pictures, Videos and Music; Save As then says "File not found". The save
  dialogs start in Downloads for this reason.
- On Windows a name ending in a dot or a space (`Books.`, on a share served by Samba, WSL or a Mac)
  is reached only through its `\\?\` form: every read of a person's file goes through
  `store::named`, and File Explorer is never asked to show one. PowerShell's file cmdlets refuse a
  `\\?\` path, so the move script uses .NET calls.

## Runbook

- The gate before each commit and push: `node scripts/check.mjs` (about 25 s). It runs `npm test`,
  `npm run check`, `cargo test --lib` and `cargo clippy --all-targets -- -D warnings` and prints
  one pass or fail line each, with a failed step's output; the whole output is in
  `.scratch/check.log`. From WSL it runs them on Windows through `cmd.exe`; `--linux` runs them in
  the Linux build folder instead. `scripts/pre-push.sh` is the pre-push hook and says how to
  install it. `npm test` includes `scripts/docs.test.mjs`, which fails on a file name or a
  decision record in this file, `README.md`, `CONTRIBUTING.md` or `docs/` that does not resolve, and once this file passes its size budget (`AGENTS_MD_BUDGET` there).
- By hand: `node scripts/dev.mjs run [data folder]` (Windows) starts the dev app with DevTools on
  9222; `scripts/smoke.mjs` fails on an error, `scripts/devtools-shot.mjs` takes a screenshot. A
  visible change is looked at in the running app.
- What has a newer version, and which locked crate or package has a security advisory:
  `node scripts/updates.mjs` (npm, Cargo.lock against OSV, the pinned tools, Rust and
  Node). It changes nothing and exits 1 while an advisory has a fix not yet taken; `--advisories`
  checks only those, and the pre-push hook runs it as a warning. An advisory whose fix waits on
  upstream goes in its `WAITING` list with the reason.
- Live development: `npm run tauri dev`. UI logic tests: `npm test` (`node --test`; each
  `scripts/*.test.mjs` loads its module through Vite's `runnerImport`, which does not compile
  runes, so a module tested this way is a plain `.ts` one). Rust tests: `cargo test --lib` in
  `src-tauri/`; lint: `cargo clippy --all-targets` there, kept at 0 warnings. The code is not
  `rustfmt`-formatted: never run `cargo fmt` on it.
- From WSL, Windows commands run in a `.cmd` file run by `cmd.exe /c`, with the Windows
  `.cargo\bin` on `PATH`; such a checkout's npm packages and `target/` are Windows-built.
- `cargo test --lib real_folder -- --ignored --nocapture` with `FVH_SCAN` set to a folder reads it
  read-only and prints its Works and unlisted files by reason; `FVH_KIND` forces a Section kind
  (`Collections`), `FVH_SHOW` lists the Works whose folder holds that text. `cargo test --lib
  demo_library -- --ignored --nocapture` with `FVH_DEMO_LIBRARY` set to a folder of made-up
  Cooking, Learning, Books and Kids Stuff folders prints what each becomes.
- The catalog file and the page share one compact form (`packed.rs`, `lib/packed.ts`). After a
  change to it, `FVH_WRITE_FIXTURE=1 cargo test --lib packed` writes `scripts/packed.fixture.json`
  again, which `npm test` reads. `cargo test --release --lib packed::tests::a_real -- --ignored
  --nocapture` with `FVH_CATALOG` set to a `catalog.json` of any version packs it, makes it whole
  again, checks every Work and unlisted file and prints both sizes; with `FVH_DUMP` set to a folder
  under `.scratch/` it also writes the parts there, and `node --test scripts/packed.test.mjs` with
  the same `FVH_DUMP` makes them whole and compares them byte for byte.
- `cargo test --lib web::tests::live -- --ignored --nocapture` with `FVH_WORDS` (search words; from
  WSL pass it with `WSLENV=FVH_WORDS/w`) searches both sources for real and prints what each found;
  set `FVH_DATA_DIR` to a folder under `.scratch/` that holds a `Logs` folder, so its network log
  lands there.
- The libraries BtbN's FFmpeg build links in have their licences in `scripts/ffmpeg-licences.json`,
  collected at the pinned FFmpeg-Builds commit by `node scripts/ffmpeg-licences.mjs <commit>` (git
  and curl; a run cut short resumes). Pinning a newer FFmpeg build means pinning its commits in
  `fetch-tools.mjs` and running it again; fetch-tools stops while the two differ. On Linux, `npm
  run package` makes the AppImage a second time when its libraries differ from the notices made
  from the last one.
- The tools live in `tools/` (gitignored), filled by `npm run fetch-tools`. A pinned URL that is
  gone (MSYS2 prunes old packages) stops the script: pin the newer file and its hash. FFmpeg is
  pinned to a month's last BtbN build, the only ones BtbN keeps. The script rewrites ddjvu's embedded manifest to UTF-8, since the
  MSYS2 build reads paths in the ANSI code page and turns Cyrillic into "?".
- Linux work from WSL runs through `scripts/linux.sh` (`package`, `test`, `clippy`, `run
  <command>`). `node_modules` and `tools/` differ per system, so the Linux ones live in a build
  folder on WSL's disk, `~/.cache/fvh-linux`; each command first
  copies the checkout there, runs `npm ci` or `fetch-tools` when their inputs changed, and sets
  `PATH`, `CARGO_TARGET_DIR=$HOME/.cache/fvh-linux-target` and the build number (`FVH_BUILD`, as
  the build folder has no `.git`). A Rust test that writes into the checkout (`FVH_WRITE_FIXTURE`)
  writes the build folder's copy: run it on Windows. Some tests use Windows paths and are skipped
  on Linux. A Windows drive under WSL (`/mnt/<letter>`) sets case aside as a FAT stick does, so a test
  there that renames only in case runs on such a drive (`mv X x` refuses there; Python's
  `os.rename` does it).
- Review checks: `cargo test --lib the_move_script_runs -- --ignored --nocapture` runs the move
  script for the system on a temp folder. `cargo test --lib moves::tests::across_drives --
  --ignored` with `FVH_OTHER_DRIVE` set to a folder on another drive than the temp folder's checks
  a move and its Undo across drives. `cargo test --release --lib resaved_in_a_real_folder --
  --ignored --nocapture` with `FVH_SHOTS` reads every JPEG under it (read-only) and prints the
  pictures found saved again (`decisions/2026/10/09/copies-include-pictures-saved-again.md`).
- Insta360 test files in `tests/insta360/` are cut from real (start, trailer; serial
  and location removed) by `tests/make_insta360_fixtures.py`.
- Thumbnail checks: `cargo test --lib fixtures -- --ignored --nocapture` (after `cargo build`)
  makes one from each file in `.scratch/fixtures`, which `tests/make_fixtures.py` makes.
- The catalog and the Marks in `App.svelte` are `$state.raw`: a change to either is a new object
  (`catalog = {...}`, `marks = await api...`), never an edit in place, which would not redraw. Code
  that runs once per Work must stay linear, each stall under 1 s: `scripts/big-folders.sh`
  makes the large folders and `scripts/stalls.mjs` measures the stalls.
- A command that takes the catalog's lock is `#[tauri::command(async)]`: the catalog is saved under
  that lock (`save_catalog` in `lib.rs`), which takes about 1 s on a large one, and a sync command
  runs on the main thread, so the window would wait. A command that changes the catalog takes the
  catalog's lock before the settings'. `commands_that_lock_the_catalog_are_async` in
  `lib.rs` fails on a sync command whose own body takes the catalog's lock.
- In development the data folder is `src-tauri/target/debug/StoredData`; `FVH_DATA_DIR` points it
  elsewhere. Scripts and tests write under `.scratch/` (gitignored).
- The icon files come from `icons/icon.svg`: `npx tauri icon icons/icon.svg` in `src-tauri/`
  (Windows), then delete the `android/`, `ios/` and `64x64.png` it also writes.
- The build number is the commit count plus `EARLIER_BUILDS` in `src-tauri/build.rs`, so the dev
  app shows it too; the version is in `src-tauri/tauri.conf.json` and `Cargo.toml`. Both show in
  Settings and, when the window has room, in the status bar. The version goes up (0.0.2, 0.0.3,
  ...) only in the package that ships a milestone the person sees, never per build. Each package
  is tagged `filesviewhub-<version>-build-<n>` (annotated, its message the release note since the
  last tag) and the tag is pushed.

## Decision records

`decisions/YYYY/MM/DD/<slug>.md`, dated the day it is written, cited by its path. New records have
the sections Context, Investigation, Decision, Risks and Alternatives considered. The words they
use are in `docs/glossary.md`; agent skills that keep a glossary and records point at these two.

## Environment

- Windows: Rust stable MSVC (rustup), Visual Studio Build Tools (C++ workload), Node 24.
- Linux (Ubuntu 24.04, also under WSL): rustup stable, Node 24, the apt packages
  `libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev
  libayatana-appindicator3-dev librsvg2-dev patchelf xdg-utils djvulibre-bin webkit2gtk-driver`,
  and `tauri-driver` (`cargo install tauri-driver`).
- Scripts in `tests/` use Python 3.
