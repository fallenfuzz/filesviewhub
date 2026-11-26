# A click opens only the types the app knows; every other file is shown in its folder

## Context

A click on a file opened it through the system, except for the types in `programs::RUNS`, which
were shown in their folder. A list of types that run is never complete: each new Windows or
macOS handler is a gap until someone adds it. macOS and Linux also run a file with no extension
when it has the execute bit, and `type_of` gives such a file an empty type, which `RUNS` cannot
list (security re-audit, issues #13 and #14).

## Investigation

- In a real library of about 72,000 files, the types that would stop opening if only the
  catalog's Work types (`scan::class_of`) opened are camera raw photos (`.cr3`, `.cr2`),
  camcorder video (`.mts`), Visio and Publisher files and `.csv`. Archives, web pages, `.xml`,
  `.ini` and database files are rare and gain nothing from opening in place.
- `.chm` is a Book type and also runs scripts, so it stays shown in its folder.

## Decision

- `programs::reveals` decides. A type opens from a click only when `class_of` knows it, it is a
  caption type, or it is in `programs::OPENS` (camera raw, camcorder video, a few documents that
  hold no program), and it is not in `RUNS`. Every other type is shown in its folder, a file
  with no type among them.
- A program the person picked for a type opens it, as before. `RUNS` still keeps a program that
  runs what it opens (`programs::refused`) from being picked for a program or script type.
- Settings, Programs marks each type a click shows as "Shown in its folder" until a program is
  picked for it.

## Risks

- A safe type missing from `OPENS` is shown in its folder: one more click, never a program run.
  Add it to `OPENS` when someone asks.

## Alternatives considered

- Keep `RUNS` and only show a file with no extension and the execute bit: each new handler
  stays a gap until it is listed.
