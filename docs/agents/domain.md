# Domain docs

How skills that read or write the domain model use this repository. It has one context.

## Before exploring, read these

- `AGENTS.md`: the code map, rules and Runbook. `CLAUDE.md` imports it.
- `docs/glossary.md`: the words the code and docs use. It is the glossary a skill reads and writes,
  whatever file name the skill defaults to.
- `decisions/`: decision records, `decisions/YYYY/MM/DD/<slug>.md`, not `docs/adr/`. Read the ones
  that touch the area you work in. `decisions/README.md` maps the old numbered names.
- `DESIGN.md` for anything a person sees.

## Writing

- A new term goes into `docs/glossary.md` when it is settled, in its format: the term in bold, one
  short definition, the words to avoid.
- A new decision is `decisions/YYYY/MM/DD/<slug>.md`, dated the day it is written, with the
  sections Context, Investigation, Decision, Risks and Alternatives considered. Never a number.
- A change that makes a record untrue adds a supersede note at its top and writes the new record in
  the same commit.

## Use the project's words

Name things as `docs/glossary.md`, `AGENTS.md` and the code name them (Section, Work, Mark, Tick,
Event, Checked folder, Drive). If what you need has no word yet, either you are inventing one the
project does not use, or there is a gap to settle with the person first.

## Flag a conflict with a record

If what you propose contradicts a record, say so, citing its path:

> Contradicts `decisions/2026/10/05/app-data-local-cache-movable.md`, but worth reopening because...
