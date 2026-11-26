<!-- 1-3 sentences: what changes and why. Link the issue: Closes #N. -->

## Checked

<!-- What you ran or looked at, and on which system (Windows, macOS, Linux). Say what you did not check. -->

## Checklist

The rules are in `AGENTS.md` and `CONTRIBUTING.md`. Delete the lines that do not apply.

- [ ] `node scripts/check.mjs` passes
- [ ] A change-log entry under `change-logs/YYYY/MM/DD/`, dated the day it was written
- [ ] Every new visible word is in `src/lib/strings.ts`
- [ ] No `cargo fmt`; clippy at 0 warnings
- [ ] A visible change: screenshots before and after
- [ ] A change that makes a `decisions/` record untrue: supersede note added, new record written
- [ ] A built item: added to `docs/done.md`
- [ ] Works on Windows, macOS and Linux, or the gap is an issue
