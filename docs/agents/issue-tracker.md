# Issue tracker: GitHub Issues

Issues for this repository are GitHub Issues, read and written with the `gh` CLI. A pull request
from someone outside the project is triaged as a request, with the same labels.

- Create: `gh issue create --title "..." --body-file <file>`.
- Read: `gh issue view <number> --comments`.
- List: `gh issue list --label <label>`; the kinds of work are `bug`, `major`, `release`, `macos`
  and `linux`, the triage state is one label from `docs/agents/triage-labels.md`.
- Comment: `gh issue comment <number> --body-file <file>`.
- Label: `gh issue edit <number> --add-label <label>` / `--remove-label <label>`.
- Close: the commit or pull request that builds it says `Closes #<number>`, and the item goes into
  `docs/done.md` in the same change.

## When a skill says "publish to the issue tracker"

Create a GitHub issue.

## When a skill says "fetch the relevant ticket"

Run `gh issue view <number> --comments`.
