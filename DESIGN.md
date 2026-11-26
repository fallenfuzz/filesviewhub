# Design

How FilesViewHub looks and behaves. Follow it for any UI change. Where a part already exists in the
app, match that part exactly; do not invent a new one.

## The frame

File Explorer in Windows 11, made into a catalog: one chrome surface (`--side`, `--bar`) around a
raised content pane (`--pane`). From left to right, the library column (Next up, In progress,
Done, Recently added, Recently opened, Sections by drive), the folder columns, then the cover grid
or rows of the open folder. With the columns folded, a rail holds them. Above the pane: an address
bar with chevron menus at each step, and a command bar. Below: a status bar with what the app
watches, the network state and the cache. Where the app departs from a plain file browser, a
decision record says why (`decisions/2026/10/08/where-the-app-departs-from-the-hub.md`).

## Look

- Colours come from the variables in `src/app.css` (`--bg`, `--pane`, `--ink`, `--muted`,
  `--focus`, `--sel`, `--next`, `--done`, `--danger` and the rest), never a literal colour in a
  component. Light and dark follow the system; both are defined there.
- Text is `--text` (Golos Text, then Segoe UI Variable), 14 px.
- Buttons are `.btn` (6 px corners), the main action `.btn.primary`. A selected thing is a tint of
  the accent (`--sel`, `--sel-line`), never a black block.
- Focus is always visible: a 2 px `--focus` outline. Every control works by keyboard.
- Motion is short and stops under `prefers-reduced-motion`.
- It looks current, Windows 11 era: no bare system boxes, no dated controls.

## Words

- Every visible word is in `src/lib/strings.ts`; `scripts/strings.test.mjs` fails on one written in
  a component.
- Plain words a person uses, not code terms. Numbers are true and carry their unit (12 photos,
  3.4 GB). A search box says only "Search".
- The words for the things the app shows (Section, Work, Mark, Tick, Event) are the ones in
  `docs/glossary.md`.

## Parts to reuse

- Menus: every context menu and ⋯ menu is `src/lib/Menu.svelte`, opened with `openMenu` from
  `src/lib/menus.svelte.ts`.
- Messages: every toast is `src/lib/ToastBox.svelte`, shown with `showToast`; every error is a
  toast, shown with `showError` (`src/lib/toast.svelte.ts`). A change that can be taken back offers
  Undo in its toast.
- Questions: a yes or no question is `ask` from `src/lib/ask.svelte.ts`, drawn by
  `src/lib/AskBox.svelte`, never the system's confirm box.
- Dialogs: Section settings and Folder settings share one dialog shape
  (`src/lib/SectionSettings.svelte`); a new settings dialog takes the same shape.
- Folders shown as tiles or rows: `src/lib/FolderTile.svelte` and the views in `src/lib/views.ts`.
