// Size budgets, so no file grows past what a person or an agent can read in one go. Each number is
// a ratchet: lowering it is welcome; raising it is a choice, made here with its reason on the line.
// The shapes follow how each kind of file grows with the app:
// - a doc of fixed purpose has its own budget, and any other doc gets DOC_DEFAULT;
// - a file that gains an entry per change (docs/done.md, decisions/, change-logs/) has a budget per
//   entry and per area, so it grows by new areas or files, never by one entry or area swelling;
// - a source file gets SOURCE_LINES; the few already past it are pinned near their size today.
// AGENTS.md has its own budget in docs.test.mjs. Text files are LF on every system (.gitattributes),
// so the bytes are the same on Windows, macOS and Linux.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const repo = join(dirname(fileURLToPath(import.meta.url)), '..');
const SKIP = new Set(['node_modules', 'dist', 'target', 'tools', 'release', 'gen', '.git', '.scratch', 'icons']);
const files = (dir = repo) => readdirSync(dir, { withFileTypes: true }).flatMap((e) => SKIP.has(e.name) ? []
  : e.isDirectory() ? files(join(dir, e.name)) : [join(dir, e.name).slice(repo.length + 1).replaceAll('\\', '/')]);
const bytes = (f) => Buffer.byteLength(readFileSync(join(repo, f), 'utf8'));
const all = files();

// Docs, in bytes. Each is its size when the budget landed plus room for a few ordinary changes.
const DOC_BUDGET = new Map([
  ['README.md', 32_000], // the person's guide; a new area of the app gets one section, not a page
  ['.github/README.md', 8_000],
  ['docs/glossary.md', 12_000], // grows by one line per new word the code and docs use
  ['DESIGN.md', 4_000],
  ['CONTRIBUTING.md', 4_000],
  ['decisions/README.md', 7_261], // frozen: the map from the old numbered names
]);
const DOC_DEFAULT = 4_000;
const NOT_DOCS = /^(AGENTS\.md|docs\/done\.md|decisions\/\d|change-logs\/\d|tests\/)/;

test('each doc stays within its size budget', () => {
  const over = all.filter((f) => f.endsWith('.md') && !NOT_DOCS.test(f))
    .map((f) => [f, bytes(f), DOC_BUDGET.get(f) ?? DOC_DEFAULT]).filter(([, size, cap]) => size > cap);
  assert.deepEqual(over.map(([f, s, c]) => `${f}: ${s} bytes, over ${c}`), [],
    'trim the doc, move detail into its own file behind a one-line pointer, or raise its budget here with a reason');
});

// docs/done.md: an area is a line of its own with no "- " in front; its items are the "- " lines.
// A new area of the app is a new area here. An area past its budget splits into two; the file past
// DONE_FILE splits into docs/done/<area>.md, one file per area.
const DONE_ITEM = 3_000; // the detail of an item belongs in its decision record
const DONE_AREA = 20_000;
const DONE_FILE = 80_000;

test('docs/done.md keeps its items, areas and file within budget', () => {
  const text = readFileSync(join(repo, 'docs/done.md'), 'utf8');
  assert.ok(Buffer.byteLength(text) <= DONE_FILE, `docs/done.md is over ${DONE_FILE} bytes: split it into docs/done/<area>.md`);
  const areas = new Map();
  let area = '';
  const long = [];
  for (const line of text.split('\n').slice(1)) {
    if (/^[A-Z]/.test(line)) area = line;
    else if (line.startsWith('- ')) {
      areas.set(area, (areas.get(area) ?? 0) + Buffer.byteLength(line) + 1);
      if (Buffer.byteLength(line) > DONE_ITEM) long.push(`${area}: ${line.slice(0, 60)}…`);
    }
  }
  assert.deepEqual(long, [], `an item is over ${DONE_ITEM} bytes: keep what it does, move the detail to its decision record`);
  const big = [...areas].filter(([, s]) => s > DONE_AREA).map(([a, s]) => `${a}: ${s} bytes`);
  assert.deepEqual(big, [], `an area is over ${DONE_AREA} bytes: split it into two areas`);
});

// One file per decision or change, so the count grows freely and each file stays short.
const RECORD = 6_000; // a longer record is a design doc: keep the decision, link the rest
const CHANGE_LOG = 1_000; // 1-3 sentences and a credit line (change-logs/README.md)

test('each decision record and change log stays within its size budget', () => {
  const over = [
    ...all.filter((f) => /^decisions\/\d.*\.md$/.test(f)).filter((f) => bytes(f) > RECORD).map((f) => `${f}: over ${RECORD}`),
    ...all.filter((f) => /^change-logs\/\d.*\.md$/.test(f)).filter((f) => bytes(f) > CHANGE_LOG).map((f) => `${f}: over ${CHANGE_LOG}`),
  ];
  assert.deepEqual(over, [], 'shorten the file; a decision record links the detail rather than holding it');
});

// Source files, in lines. A file past SOURCE_LINES splits by what its parts do (a module, a
// component). The ones already past it are pinned near their size today, to shrink, not grow.
const SOURCE = /^(src|src-tauri\/src|scripts|tests)\/.*\.(rs|ts|svelte|mjs|js|py|css)$/;
const SOURCE_LINES = 2_000;
const SOURCE_PINNED = new Map([
  ['src-tauri/src/scan.rs', 3_200],
  ['src-tauri/src/lib.rs', 3_100], // the Tauri commands; new ones go in their own module
]);

test('each source file stays within its line budget', () => {
  const over = all.filter((f) => SOURCE.test(f)).map((f) => {
    const lines = readFileSync(join(repo, f), 'utf8').split('\n').length;
    const cap = SOURCE_PINNED.get(f) ?? SOURCE_LINES;
    return lines > cap ? `${f}: ${lines} lines, over ${cap}` : '';
  }).filter(Boolean);
  assert.deepEqual(over, [], 'split the file by what its parts do, or raise its budget here with a reason');
});
