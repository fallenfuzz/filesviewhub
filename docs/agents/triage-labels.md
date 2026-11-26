# Triage labels

Skills speak of five triage roles. Each is a GitHub label of the same name; an issue carries at most
one of them at a time.

| Role | Label | Meaning |
|---|---|---|
| needs triage | `needs-triage` | The maintainer has not looked at it yet (new issues get it from the issue forms) |
| needs info | `needs-info` | Waiting on the reporter |
| ready for agent | `ready-for-agent` | Fully specified; an agent can build it unattended |
| ready for human | `ready-for-human` | Needs a person to build or decide |
| won't fix | `wontfix` | Will not be done; the issue says why |

`.github/labels.json` lists every label the repository uses, triage and kinds of work, with
colours; `node scripts/labels.mjs` makes or updates them on GitHub with `gh`.
