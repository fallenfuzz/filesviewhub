# Photo AI is an optional, separate index the build only reads

The Photos Section is complete without AI: dates, Events, Similar shots, Copies, Junk heuristics, On
this day and the lightbox all come from Pillow, `imagehash` and numpy. Content tags, People and text
in photos come from an optional indexer, `photos/index.py`, with its own venv (`photos/.venv`). It uses
the GPU if present, else the CPU, and writes `cache/photos-index.json` keyed by file. `build.py` reads
that file if it exists and never imports a model, so the Hub builds on any machine, and someone
without a GPU loses only those three features. The file's format is fixed before the indexer is
written, so no AI field leaks into the core.

## Considered Options

- Models inside `build.py`: one command, but every build needs torch and hours of CPU on a machine
  without a GPU, against the rule that tools run without AI.
- No AI at all: People and search by content are the features a flat photo folder lacks most, and
  the hardware to run them locally is common.
