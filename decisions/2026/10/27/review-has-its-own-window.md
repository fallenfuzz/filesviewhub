# Review has its own window

Review covered the main window: it could not be moved to another screen, made full screen or put
aside to look something up in the app, and closing it was the only way back. Going through
hundreds of photos is long work, done in several sittings and best on the largest screen.

Review now opens in a second window of the app, `review`:

- The core makes it (`open_review` in `lib.rs`), at the size the main window starts at, the same web view
  data folder and the same browser arguments, so it adds no store, process or network call
  (`decisions/2026/10/05/nothing-hidden-no-network-by-default.md`, `decisions/2026/10/07/app-data-in-storeddata-next-to-the-app.md`). Its page is the app's own page with `?review`, which mounts
  `ReviewWindow.svelte` in place of the app.
- Its top bar is its title bar: it moves the window, and holds Back to the app (the window goes to
  the taskbar and the main window comes up), Full screen (F11; Esc leaves it first), minimise,
  maximise and close.
- One Review window at a time. A Review asked for while it is open shows in it, on top.
- It reads the catalog, choices, notes and thumbnails itself, as the main window does, and tells
  the main window what changed (`review-news`): a choice, so the counts follow; a move, so the
  catalog is read again; and that Review closed with photos marked Not needed, which the main
  window says in a toast with Move. Choices made in the main window show in Review when its
  window comes back into focus.
- Move started in Review opens in its window, and Review goes on at the sets still waiting.
- The app ends with its main window: the Review window closes with it.

## Considered Options

- Keep Review over the main window, with a full-screen button and a Hide button that brings back
  a "Resume review" chip: one window, but no second screen and no way to look at the app beside
  Review.
- Open Review in a browser tab: no access to the app's commands without a server.
