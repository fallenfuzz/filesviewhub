# FilesViewHub is a Tauri app with a Rust core, shipped as a portable zip

The Hub becomes a desktop app, FilesViewHub: Tauri 2 with a Rust core (scanning, watching,
thumbnails, the catalog) and a TypeScript UI in the system web view (WebView2 on Windows,
WKWebView on macOS). Windows comes first, macOS next. It ships as a portable zip, the `.exe` plus a
`tools\` folder, with no installer and no updater; the version shows in About as `0.0.1 (build N)`.
`build.py` and the Hub page are frozen with no fixes; the app starts from scratch and imports
nothing (no notes, no Marks).

Most of `build.py` is specific to one Dropbox folder (fixed Sections, hand-written notes), so a
generic app rewrites the scanner in any language; Rust gives a small, fast app that can watch a
folder of 100k files. Thumbnails come from bundled tools whatever the language.

## Considered Options

- Tauri shell with the Python scanner as a sidecar: reuses more code, but two runtimes, a 40 MB+
  sidecar, and a batch build that fits a folder watcher badly.
- Electron: about 120 MB and more memory, nothing this app needs that Tauri lacks.
- pywebview with PyInstaller: stays in Python, but no real installer or updater story.
- A per-user installer with the Tauri updater: the person tests and fixes alone first and shares
  no updates, so a zip is simpler; an installer can be added later from the same build.

## Consequences

- Thumbnails need the app to decode files itself, so the zip bundles a stripped LGPL ffmpeg
  (decoders and image output only), pdfium and ddjvu; LibreOffice is used for slides only if
  installed. Windows Explorer's own thumbnails were rejected: none for PDF or DjVu by default, no
  HEVC without a paid extension, no reliable video length, and no way to skip black frames.
  Opening a file always uses the person's own program, mapped per type in Settings; the app plays
  nothing itself.
- Unsigned, so Windows SmartScreen warns on first run; the README explains "More info, Run anyway".
