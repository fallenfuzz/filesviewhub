# Change logs

One file per change, so parallel branches never conflict on a shared changelog.

## Format

**Path:** `change-logs/YYYY/MM/DD/<type>-<short-slug>.md`. `YYYY/MM/DD` is the day you write the
entry; never move it afterwards. Which package a change ships in comes from git:
`node scripts/release-note.mjs` lists the entries added since the last `filesviewhub-*` tag, and
that list is the tag's release note.

**Types:** `feature-`, `fix-`, `refactor-`, `docs-`, `chore-`.

**Content:** plain text, 1-3 sentences on what changed for the person using the app (or, for
`refactor-`, `docs-` and `chore-`, for the person working on it). No frontmatter, no headings.

## Credits

A credit line stands on its own line at the end of the file:

- `Suggested by @<login> (#<issue>)`: the author of the issue behind the change, when that is not
  the maintainer.
- `Contributed by @<login> (#<pull request>)`: the author of the pull request, when that is not the
  maintainer.

Login only, never a name or an email.

## Example

```
# change-logs/2026/11/25/fix-cover-search-keeps-the-words.md
Choose a cover… keeps the search words you edited when you search again.

Contributed by @someone
```
