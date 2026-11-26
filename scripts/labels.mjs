// Makes or updates on GitHub every label in .github/labels.json (docs/agents/triage-labels.md),
// with the gh CLI, in the repository gh sees from here. Labels not in the file are left alone.
//   node scripts/labels.mjs
import { execFileSync } from 'node:child_process'
import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const repo = join(dirname(fileURLToPath(import.meta.url)), '..')
const labels = JSON.parse(readFileSync(join(repo, '.github', 'labels.json'), 'utf8'))
const gh = (...args) => execFileSync('gh', args, { cwd: repo, encoding: 'utf8' })
const have = new Set(JSON.parse(gh('label', 'list', '--limit', '200', '--json', 'name')).map((l) => l.name))
for (const { name, color, description } of labels) {
  gh('label', have.has(name) ? 'edit' : 'create', name, '--color', color, '--description', description)
  console.log(`${have.has(name) ? 'updated' : 'made'}  ${name}`)
}
