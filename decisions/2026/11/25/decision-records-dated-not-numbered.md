# Decision records are dated, not numbered

## Context
Records were `docs/adr/NNNN-slug.md`, numbered in order and never changed once accepted: a
reversal was a new record. The app moves into its own public repository, where people work on
it at the same time on their own branches. Two branches that each take the next free number
clash on merge, and an unchanged record that the code no longer matches misleads the next
reader, who has no way to know a later record replaced it.

## Decision
A record is `decisions/YYYY/MM/DD/slug.md`, dated the day it is written. The slug is its
identity and is unique across `decisions/`. Code and docs cite the full path, in backticks.
New records have the sections Context, Investigation, Decision, Risks and Alternatives
considered; the records moved here keep their shape.

When a change makes a record untrue, the same commit adds a supersede note at its top -
"Superseded on YYYY-MM-DD by `decisions/YYYY/MM/DD/<new-slug>.md`: <one line>" - and writes the
new record. The record's text stays as it was.

The numbered records were moved once, each dated by its first commit. `decisions/README.md`
maps every old name to its new path and is frozen. `scripts/docs.test.mjs` fails on a cited
record path that does not resolve, a slug used twice, and a map row whose path is gone.

## Risks
Citations are long, so a code comment that cites two or three records gets a long line. Old
commit messages and issues still say "ADR NNNN"; they resolve only through the map.

## Alternatives considered
Keep the numbers and renumber on merge: every open branch rewrites its citations. Cite by slug
alone: shorter, but a reader cannot open it without a search. Leave old records numbered beside
new dated ones: two citation forms to check, and the numbers never go away.
