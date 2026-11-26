# App data lives in LocalAppData; only the cache can move, and Marks are never in it

FilesViewHub keeps its catalog, Marks and settings in `%LOCALAPPDATA%\FilesViewHub\`, and
thumbnails and probe results in `%LOCALAPPDATA%\FilesViewHub\Cache\` by default. Settings can move
the cache (for example to a large second drive), moving the existing files or starting empty, as
the person chooses. Deleting or moving the cache never touches Marks. Nothing goes in Roaming,
which syncs across domain profiles and would carry gigabytes of thumbnails, and nothing is written
into a checked folder.

## Considered Options

- Data next to the `.exe` (true portable): only useful for running from a USB stick, which nobody
  needs, and it breaks when the zip is unpacked into a read-only place.
- A cache per drive, inside it, for external disks: writes into the person's folders.
