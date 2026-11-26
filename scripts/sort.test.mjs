// The cards' order (sort.ts), run by `npm test`: each pick and way, the length groups, kept
// picks read back, and 150,000 Works sorted inside the 1 s a stall may take.
import { test } from 'node:test'
import assert from 'node:assert/strict'
// Each sort stays under 1 s on a person's PC; CI runners are slower, so three times that there.
const LIMIT = process.env.CI ? 3000 : 1000
import { fileURLToPath } from 'node:url'
import { runnerImport } from 'vite'

const { module: s } = await runnerImport(fileURLToPath(new URL('../src/lib/sort.ts', import.meta.url)))
const item = (path, ext = 'mp4') => ({ name: path, rel: path, path, ext, size: 1, mtime: 0, cloud: false, key: path, title: path, group: '', role: 'main' })
const work = (title, added, paths = [title]) => ({ id: title, title, path: title, folder: [], is_folder: paths.length > 1, items: paths.map(p => item(p)), media: [], size: 1, added, cloud: 0, year: null })
const ids = l => l.map(w => w.id)
const id = w => w

test('kept picks read back, and nothing else does', () => {
  assert.deepEqual(s.readSort('name'), { by: 'name', down: false })
  assert.deepEqual(s.readSort('-length'), { by: 'length', down: true })
  for (const v of [undefined, null, '', '-', 'size', '--name', 3, {}]) assert.equal(s.readSort(v), null)
  for (const v of ['name', '-name', 'added', '-added', 'length', '-length', 'opened', '-opened']) assert.equal(s.writeSort(s.readSort(v)), v)
})

test('no pick keeps the order the cards came in', () => {
  const l = [work('b', 1), work('a', 2)]
  assert.equal(s.sortWorks(l, null, id, {}), l)
})

test('by name reads numbers as a person does, either way', () => {
  const l = [work('10 Advanced', 0), work('2 Basics', 0), work('b', 0), work('A', 0)]
  assert.deepEqual(ids(s.sortWorks(l, { by: 'name', down: false }, id, {})), ['2 Basics', '10 Advanced', 'A', 'b'])
  assert.deepEqual(ids(s.sortWorks(l, { by: 'name', down: true }, id, {})), ['b', 'A', '10 Advanced', '2 Basics'])
  assert.deepEqual(ids(l), ['10 Advanced', '2 Basics', 'b', 'A'], 'the list given is left as it was')
})

test('by date added, equal dates keep their order', () => {
  const l = [work('x', 5), work('y', 1), work('z', 5)]
  assert.deepEqual(ids(s.sortWorks(l, { by: 'added', down: false }, id, {})), ['y', 'x', 'z'])
  assert.deepEqual(ids(s.sortWorks(l, { by: 'added', down: true }, id, {})), ['x', 'z', 'y'])
})

test('by length: times, then pages, then not known, each group turned by the way', () => {
  const l = [work('unknown', 0), work('book', 0), work('short', 0), work('course', 0, ['c1', 'c2']), work('thin', 0)]
  const d = { book: { p: 300 }, short: { s: 60 }, c1: { s: 600 }, c2: { s: 600 }, thin: { p: 20 } }
  assert.deepEqual(ids(s.sortWorks(l, { by: 'length', down: false }, id, d)), ['short', 'course', 'thin', 'book', 'unknown'])
  assert.deepEqual(ids(s.sortWorks(l, { by: 'length', down: true }, id, d)), ['course', 'short', 'book', 'thin', 'unknown'])
})

test('a file Work with no main item takes the length of its first file, as its card', () => {
  const w = work('doc', 0); w.items[0].role = 'doc'
  const l = [work('none', 0), w]
  assert.deepEqual(ids(s.sortWorks(l, { by: 'length', down: false }, id, { doc: { p: 12 } })), ['doc', 'none'])
})

test('each pick starts its own way', () => {
  assert.deepEqual(s.firstWay('name'), { by: 'name', down: false })
  assert.deepEqual(s.firstWay('added'), { by: 'added', down: true })
  assert.deepEqual(s.firstWay('length'), { by: 'length', down: false })
  assert.deepEqual(s.firstWay('opened'), { by: 'opened', down: true })
})

test('by last opened: those never opened come last, either way', () => {
  const l = [work('never', 0), work('old', 0), work('new', 0), work('never2', 0)]
  const at = { old: 5, new: 9 }, opened = w => at[w.id] ?? 0
  assert.deepEqual(ids(s.sortWorks(l, { by: 'opened', down: true }, id, {}, opened)), ['new', 'old', 'never', 'never2'])
  assert.deepEqual(ids(s.sortWorks(l, { by: 'opened', down: false }, id, {}, opened)), ['old', 'new', 'never', 'never2'])
})

test('cards held with their Section sort by their Work', () => {
  const l = [{ w: work('b', 2), s: 1 }, { w: work('a', 1), s: 2 }]
  assert.deepEqual(s.sortWorks(l, { by: 'name', down: false }, x => x.w, {}).map(x => x.s), [2, 1])
})

test('150,000 Works sort in under 1 s each way', () => {
  const l = Array.from({ length: 150000 }, (_, i) => work(`Курс ${(i * 7919) % 150000} - Глава ${i % 37} The Art of ${i % 101}`, (i * 104729) % 150000, [`p${i}`]))
  const d = Object.fromEntries(l.map((w, i) => [w.items[0].path, i % 3 ? { s: (i * 31) % 9000 } : { p: i % 700 }]))
  const opened = w => w.added % 3 ? w.added : 0
  for (const by of ['name', 'added', 'length', 'opened']) for (const down of [false, true]) {
    const t = performance.now()
    s.sortWorks(l, { by, down }, id, d, opened)
    const ms = performance.now() - t
    assert.ok(ms < LIMIT, `${by} ${down ? 'down' : 'up'} took ${Math.round(ms)} ms`)
  }
})
