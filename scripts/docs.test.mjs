// The docs an agent or a person reads first must point at files that are there. A path that no
// longer resolves sends the reader to the wrong place; this test fails on it instead.
// Checked: every `backticked` file name in the docs below.
// A short name (`lib/tree.ts`, `scan.rs`) resolves under any of the folders in BASES.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const repo = join(dirname(fileURLToPath(import.meta.url)), '..');
const DOCS = ['AGENTS.md', 'README.md', '.github/README.md', 'SECURITY.md', 'CONTRIBUTING.md', 'DESIGN.md', 'change-logs/README.md', 'docs/agents/issue-tracker.md',
  'docs/agents/triage-labels.md', 'docs/agents/domain.md', 'docs/glossary.md', 'docs/done.md', 'docs/references.md'];
const BASES = ['', 'src/', 'src-tauri/', 'src-tauri/src/', 'scripts/', 'tests/', 'docs/'];
const FILE = /^[A-Za-z0-9_.@/-]*[A-Za-z0-9_-]\.(rs|ts|mjs|js|svelte|md|json|py|sh|ps1|cmd|html|toml|svg|txt)$/;

// Names the docs use for files outside the repo, or made at build or run time. Each says why.
const ALLOWED = new Map([
  ['catalog.json', 'StoredData, made by the app'],
  ['settings.json', 'StoredData, made by the app'],
  ['covers.json', 'StoredData, made by the app'],
  ['marks.json', 'StoredData, made by the app'],
  ['moves.json', 'StoredData, made by the app'],
  ['photo-dates.json', 'StoredData, made by the app'],
  ['photo-shots.json', 'StoredData, made by the app'],
  ['tools/notices.json', 'made by each package'],
  ['tools/THIRD-PARTY-NOTICES.txt', 'made by each package'],
  ['LICENSE.txt', 'in the package, made from LICENSE'],
  ['THIRD-PARTY-NOTICES.txt', 'in the package, made by notices.mjs'],
]);

const resolves = (name) => BASES.some((b) => existsSync(join(repo, b, name)));

test('every file the docs name is there', () => {
  const missing = [];
  for (const doc of DOCS) {
    const text = readFileSync(join(repo, doc), 'utf8').replace(/^```[\s\S]*?^```/gm, '');
    for (const [, name] of text.matchAll(/`([^`\n]+)`/g)) {
      if (FILE.test(name) && !ALLOWED.has(name) && !resolves(name)) missing.push(`${doc}: ${name}`);
    }
  }
  assert.deepEqual(missing, [], 'fix the path in the doc; a file made at run time goes in ALLOWED with a reason');
});

// Decision records are `decisions/YYYY/MM/DD/slug.md`; `decisions/README.md` maps the old
// numbered names and is frozen (decisions/2026/11/25/decision-records-dated-not-numbered.md).
const app = repo;
const SKIP = new Set(['node_modules', 'dist', 'target', 'tools', 'release', 'gen', '.git', '.scratch']);
const walk = (dir) => readdirSync(dir, { withFileTypes: true }).flatMap((e) => SKIP.has(e.name) ? []
  : e.isDirectory() ? walk(join(dir, e.name)) : [join(dir, e.name)]);
const TEXT = /\.(rs|ts|mjs|js|svelte|md|py|sh|ps1|html|toml|yml)$/;

test('decision records are dated, their slugs unique, and every cited one is there', () => {
  const records = walk(join(app, 'decisions')).map((f) => f.slice(app.length + 1).replaceAll('\\', '/'))
    .filter((f) => f !== 'decisions/README.md');
  assert.deepEqual(records.filter((f) => !/^decisions\/\d{4}\/\d{2}\/\d{2}\/[a-z0-9-]+\.md$/.test(f)), [],
    'a record is decisions/YYYY/MM/DD/slug.md, the slug in lower case with hyphens');
  const slugs = records.map((f) => f.split('/').pop());
  assert.deepEqual(slugs.filter((s, i) => slugs.indexOf(s) !== i), [], 'two records share a slug: pick another for the newer one');
  const missing = [], old = [];
  const docs = walk(app).filter((f) => TEXT.test(f));
  for (const file of docs) {
    const text = readFileSync(file, 'utf8');
    for (const [, path] of text.matchAll(/(decisions\/\d{4}\/\d{2}\/\d{2}\/[a-z0-9-]+\.md)/g)) {
      if (!existsSync(join(app, path))) missing.push(`${file}: ${path}`);
    }
    if (!file.endsWith(join('decisions', 'README.md')) && !file.endsWith('docs.test.mjs')) {
      for (const [m] of text.matchAll(/\bADRs? \d{4}/g)) old.push(`${file}: ${m}`);
    }
  }
  assert.deepEqual(missing, [], 'the path cites a record that is not in decisions/');
  assert.deepEqual(old, [], 'cite a record by its path, decisions/YYYY/MM/DD/slug.md; decisions/README.md maps the old numbers');
  const map = readFileSync(join(app, 'decisions', 'README.md'), 'utf8');
  const rows = [...map.matchAll(/^\| `[^`]+` \| \[`[^`]+`\]\(([^)]+)\) \|$/gm)].map((m) => m[1]);
  assert.ok(rows.length > 0, 'decisions/README.md has lost its map');
  assert.deepEqual(rows.filter((r) => !existsSync(join(app, 'decisions', r))), [], 'decisions/README.md is frozen: a row points at a record that moved');
});

// AGENTS.md is loaded on every turn and by every subagent, so its size is paid on each request.
// A ratchet: lowering it is welcome; raising it is a choice, made here with its reason.
// 17 500: the macOS package and its build environment, a third system to ship.
const AGENTS_MD_BUDGET = 17_500;

test('AGENTS.md stays within its size budget', () => {
  const size = Buffer.byteLength(readFileSync(join(repo, 'AGENTS.md'), 'utf8'));
  assert.ok(size <= AGENTS_MD_BUDGET, `AGENTS.md is ${size} bytes, over its ${AGENTS_MD_BUDGET}: move reference detail into a skill or docs/ behind a one-line pointer, or raise the budget here with a reason`);
});
