// Prints the release note of the next package: every change-log entry added since the last
// filesviewhub-* tag, grouped by type (change-logs/README.md). With no tag yet, every entry.
//   node scripts/release-note.mjs            the note, for `git tag -a -F -`
//   node scripts/release-note.mjs <tag>      the entries added since that tag
import { execFileSync } from 'node:child_process'
import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const repo = join(dirname(fileURLToPath(import.meta.url)), '..')
const git = (...args) => execFileSync('git', ['-C', repo, ...args], { encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] }).trim()
let since = process.argv[2]
if (!since) {
  try { since = git('describe', '--tags', '--abbrev=0', '--match', 'filesviewhub-*') } catch { since = '' }
}
const added = (since
  ? git('diff', '--name-only', '--diff-filter=A', `${since}..HEAD`, '--', 'change-logs')
  : git('ls-files', 'change-logs')).split('\n').filter((f) => /^change-logs\/\d{4}\/\d{2}\/\d{2}\/[a-z]+-.+\.md$/.test(f))
const TITLES = { feature: 'New', fix: 'Fixed', refactor: 'Changed inside', docs: 'Docs', chore: 'Other' }
const groups = new Map(Object.keys(TITLES).map((t) => [t, []]))
for (const f of added.sort()) {
  const type = f.split('/').pop().split('-')[0]
  const text = readFileSync(join(repo, f), 'utf8').trim().split('\n')
  const credits = text.filter((l) => /^(Suggested|Contributed) by @/.test(l))
  const body = text.filter((l) => !credits.includes(l)).join(' ').replace(/\s+/g, ' ').trim()
  ;(groups.get(type) ?? groups.get('chore')).push(`- ${body}${credits.length ? ` (${credits.join('; ')})` : ''}`)
}
const out = [...groups].filter(([, items]) => items.length).map(([t, items]) => `${TITLES[t]}\n${items.join('\n')}`)
console.log(out.length ? out.join('\n\n') : `No change-log entries since ${since || 'the start'}.`)
