# The app imports nothing from the Hub

FilesViewHub reads only its own backups (`kind: "filesviewhub-marks"`). A Hub Marks backup
(`kind: "hub-marks"`), its settings and its notes are refused with "This file is not a marks backup
from FilesViewHub." This keeps `decisions/2026/10/05/filesviewhub-is-a-tauri-app-shipped-portable.md` and replaces every place that said otherwise: "the app will
import what people already have in the page" in `decisions/2026/09/30/work-id-is-its-path.md`, `decisions/2026/10/01/settings-in-one-versioned-store.md` and `decisions/2026/10/01/marks-and-ticks-carry-when-they-were-set.md`, and "A Hub Marks backup
still imports" in `decisions/2026/10/08/where-the-app-departs-from-the-hub.md`. A Hub id is a path from the Dropbox root, and an app id is a Drive's
volume serial and a path on that Drive (`decisions/2026/10/05/checked-folders-on-any-drive.md`), so an import would have to guess which checked
folder each Hub path lives in.

## Considered Options

- Import Hub backups by matching each Hub path to a checked folder: keeps the Hub's Marks, but a
  path that fits two checked folders, or none, puts a Mark on the wrong Work or drops it.
