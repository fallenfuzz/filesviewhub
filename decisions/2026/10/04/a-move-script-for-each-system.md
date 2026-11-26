# The move script ships once for each system

`decisions/2026/10/04/sorting-ends-in-a-move-script-never-a-delete.md` gave Review's result as one PowerShell script. Everything the Hub hands a person must now
work on Windows and macOS, and later Linux, so Review offers two scripts that do the same thing: a
`.ps1` for Windows and a `.sh` for macOS and Linux. Both only move the files set aside into a
`Not needed` folder inside the photo folder, never delete, and skip a file that is already there.

## Considered Options

- One Python script: the same on every system, but most people have no Python installed.
- Moving files from the page itself: a browser page cannot touch files, and the app is not built yet.
