# macOS ships as a disk image of the app folder, one for each Mac type, its tools built from source

## Context

The app is portable (`decisions/2026/10/05/filesviewhub-is-a-tauri-app-shipped-portable.md`) and
keeps its data in `StoredData` next to itself
(`decisions/2026/10/07/app-data-in-storeddata-next-to-the-app.md`). Windows ships a zip and
Linux an AppImage. A Mac package needs a shape, a build for each processor, and the three tools,
and BtbN, where the other FFmpeg builds come from, makes none for macOS.

## Investigation

- A disk image is what people expect on a Mac. Dragging `FilesViewHub.app` alone into
  Applications would put `StoredData` loose in `/Applications`.
- A Mac starts an app from a disk image, or one not moved out of Downloads, from a read-only
  place, so the app cannot write `StoredData` there.
- FFmpeg built from the pinned commit with only its own decoders and macOS's zlib, bzip2 and
  iconv is LGPL 2.1 or later with nothing else inside. It decodes every format the other builds
  do except AV1, which needs dav1d. It takes about 10 minutes on a CI runner.
- `ddjvu -format=pnm` needs neither JPEG nor TIFF, so DjVuLibre builds with no other library.
  The only loss is the JPEG pictures some DjVu pages hold.
- bblanchon's PDFium has a build for each Mac type.

## Decision

- The package is `FilesViewHub-<version>-build-<n>-macos-<arm64|x64>.dmg`, one for each Mac type,
  each built on a runner of that type. It holds one `FilesViewHub` folder: `FilesViewHub.app`,
  its tools and notices inside the app (`Contents/Resources/tools`), and `README.md`,
  `LICENSE.txt` and `THIRD-PARTY-NOTICES.txt` next to it. The person drags the folder out, and
  `StoredData` goes into it, next to the app.
- `scripts/fetch-tools.mjs` builds FFmpeg (at `FFMPEG_COMMIT`) and ddjvu (DjVuLibre 3.5.30) from
  source on the Mac and fetches PDFium. DjVuLibre's exact source ships in the app.
- The app needs macOS 13 or later (CSS subgrid in WKWebView).
- Until the app is signed with a Developer ID (issue #3), the app and each tool get an ad-hoc
  signature only, with no hardened runtime. The README says how to open it the first time.

## Risks

- Gatekeeper may still start the app from a read-only copy after the folder is dragged out. The
  app then says it cannot write and names the folder, and the README gives the `xattr` command.
- No AV1 thumbnails on a Mac.
- Signing with hardened runtime later needs PDFium signed with the same Developer ID, or the
  library-validation entitlement.

## Alternatives considered

- A universal app for both Mac types: one file, but each tool built or fetched twice and merged.
  Two disk images match how other apps ship for a Mac.
- A zip like Windows: also portable, but a disk image is the usual Mac download.
- Prebuilt static FFmpeg builds for macOS: faster, but a new source, GPL, and many libraries
  whose licences would need collecting.
- `StoredData` in `~/Library/Application Support`: no read-only start problem, but it differs from
  the other systems. It is left until the app is checked on a real Mac (issue #4).
