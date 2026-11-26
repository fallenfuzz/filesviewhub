# The app saves the move script for the system it runs on

`decisions/2026/10/04/a-move-script-for-each-system.md` offered Review's move script twice, a `.ps1` and a `.sh`, because the Hub page could not tell
which system it was opened on. The app can: on Windows it saves the PowerShell script, on macOS and
Linux the shell script. Both still exist, do the same thing and only move into `Not needed`. The script
holds the photo folder's full path as the app sees it, so it is run where the app runs, and the app
saves it where the person picks, never inside a checked folder (`decisions/2026/10/05/app-data-local-cache-movable.md`).

## Considered Options

- Both scripts, as `decisions/2026/10/04/a-move-script-for-each-system.md` had them: the second one names a path the other system cannot reach, so it
  is never the right one to run.
