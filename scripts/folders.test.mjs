// The paths on disk of the folders shown under titles (`relsOf`), run by `npm test`: Vite loads
// rels.ts as the app does. Now they come from one index per Section; this checks it gives
// what the full pass made before it gives, for every folder, order included.
import { test } from 'node:test'
import assert from 'node:assert/strict'
import { fileURLToPath } from 'node:url'
import { runnerImport } from 'vite'

const { module: f } = await runnerImport(fileURLToPath(new URL('../src/lib/rels.ts', import.meta.url)))
const { module: api } = await runnerImport(fileURLToPath(new URL('../src/lib/api.ts', import.meta.url)))

// relsOf as it was in an earlier version, word for word: one pass over the Section per folder asked.
function before(home, f2) {
  if (!f2.length) return ['']
  const under = folder => f2.length <= folder.length && f2.every((x, i) => folder[i] === x)
  const head = r => {
    let i = -1
    for (let k = 0; k < f2.length; k++) {
      i = r.indexOf('/', i + 1)
      if (i < 0) return k === f2.length - 1 ? r : undefined
    }
    return r.slice(0, i)
  }
  const rels = new Set()
  let last = ''
  const add = r => {
    const h = head(r)
    if (h === undefined) return
    rels.add(h)
    last = h ? api.keyIn(home.id, h) + '/' : ''
  }
  for (const w of home.works) if (under(w.folder) && (w.event || !last || !w.id.startsWith(last))) add(f.diskOf(home, w))
  for (const u of home.unlisted ?? []) if (under(u.folder)) add(u.rel)
  return [...rels]
}

const work = (sec, rel, folder, extra = {}) => ({ id: api.keyIn(sec, rel), title: rel, path: '', folder, is_folder: false, items: [{ rel: rel.split('/').at(-1) }], media: [], size: 0, added: 0, cloud: 0, year: null, ...extra })
// An Event: its id is its Photos folder's key (the Section's id in a Photos Section), `#`, the
// stem of its first photo.
const event = (place, stem, folder) => ({ id: `${place}#${stem}`, title: stem, path: '', folder, is_folder: false, items: [{ rel: `${stem}.jpg` }], media: [], size: 0, added: 0, cloud: 0, year: null, event: true })

// Every folder the Works and unlisted files of `home` are under, at every depth.
const foldersOf = home => new Set([...home.works.map(w => w.folder), ...(home.unlisted ?? []).map(u => u.folder)]
  .flatMap(t => t.map((_, i) => t.slice(0, i + 1).join('\0'))))
function same(home) {
  const all = foldersOf(home)
  for (const k of all) {
    const t = k.split('\0')
    assert.deepEqual(f.relsOf(home, t), before(home, t), `${home.id}: ${t.join(' / ')}`)
  }
  for (const t of [[], ['Nowhere'], ['Nowhere', 'else']]) assert.deepEqual(f.relsOf(home, t), before(home, t))
  return all.size
}

test('each folder of a nested Section gives what the full pass gave', () => {
  const SEC = '0000ABCD:Books'
  const works = []
  // "01 Fiction" and "Fiction" show as one title; a folder Work sits next to file Works.
  for (const top of ['01 Fiction', 'Fiction', 'Science']) {
    const title = top.replace(/^\d+ /, '')
    for (const series of ['Series 1', 'Series 2']) {
      for (let b = 0; b < 3; b++) works.push(work(SEC, `${top}/${series}/book ${b}.epub`, [title, series]))
      works.push(work(SEC, `${top}/${series}/Box set`, [title, series], { is_folder: true }))
    }
    works.push(work(SEC, `${top}/loose.pdf`, [title]))
  }
  works.push(work(SEC, 'top.pdf', []))
  // One title on two folders, in turns: the folder found last changes at each Work.
  for (const top of ['01 Poems', 'Poems', '01 Poems', 'Poems']) works.push(work(SEC, `${top}/x ${works.length}.pdf`, ['Poems']))
  // A Work whose id is another Section's, as a merged list holds.
  works.push(work('0000ABCD:Other', 'Science/Series 1/far.pdf', ['Science', 'Series 1']))
  const home = { id: SEC, works, unlisted: [
    { rel: 'Atlas/maps/a.txt', folder: ['Atlas', 'maps'] },
    { rel: 'Science/Series 1/notes.txt', folder: ['Science', 'Series 1'] },
    { rel: 'Fiction/Series 2/notes.txt', folder: ['Fiction', 'Series 2'] },
    { rel: 'Zoo/z.txt', folder: ['Zoo'] },
  ] }
  assert.ok(same(home) > 8)
  assert.deepEqual(f.relsOf(home, ['Fiction']), ['01 Fiction', 'Fiction'])
  assert.deepEqual(f.relsOf(home, ['Poems']), ['01 Poems', 'Poems'])
  assert.deepEqual(f.relsOf(home, ['Atlas', 'maps']), ['Atlas/maps'])
})

test('Events, whose paths can be shorter than their titles, give what the full pass gave', () => {
  // A Photos Section: an Event's folder on disk is the Section's own ('').
  const PH = '0000ABCD:Photos'
  const photos = { id: PH, works: [event(PH, 'a', ['2024', 'May']), event(PH, 'b', ['2024', 'May']), event(PH, 'c', ['2024', 'June']), event(PH, 'd', ['2023', 'May'])], unlisted: [] }
  same(photos)
  assert.deepEqual(f.relsOf(photos, ['2024', 'May']), [])
  assert.deepEqual(f.relsOf(photos, ['2024']), [''])
  // A Photos folder in a Books Section: its Events are under three titles, their path is one part;
  // a Work after an Event under the same titles, and files around them.
  const SEC = '0000ABCD:Mixed'
  const mixed = { id: SEC, works: [
    work(SEC, 'Pics/old.pdf', ['Pics']),
    event(`${SEC}/Pics`, 'p1', ['Pics', '2024', 'May']),
    event(`${SEC}/01 Pics`, 'p2', ['Pics', '2024', 'May']),
    work(SEC, 'Pics/2024/May/note.pdf', ['Pics', '2024', 'May']),
    work(SEC, 'Pics/2024/May/note2.pdf', ['Pics', '2024', 'May']),
    event(`${SEC}/Pics`, 'p3', ['Pics', '2024', 'June']),
  ], unlisted: [{ rel: 'Pics/2024/x.txt', folder: ['Pics', '2024'] }, { rel: '01 Pics/y.txt', folder: ['Pics'] }] }
  same(mixed)
  assert.deepEqual(f.relsOf(mixed, ['Pics', '2024', 'May']), ['Pics/2024/May'])
  assert.deepEqual(f.relsOf(mixed, ['Pics']), ['Pics', '01 Pics'])
  // A Work right after an Event whose path ran out before its titles: it is not in a folder found.
  const after = { id: SEC, works: [event(`${SEC}/Pics`, 'p1', ['Pics', '2024', 'May']), work(SEC, 'Pics/2024/May/note.pdf', ['Pics', '2024', 'May'])], unlisted: [] }
  same(after)
  assert.deepEqual(f.relsOf(after, ['Pics', '2024', 'May']), ['Pics/2024/May'])
})

test("a whole drive's Section gives what the full pass gave", () => {
  const D = '0A1B2C3D:'
  const drive = { id: D, works: [
    work(D, 'Books/a.pdf', ['Books']), work(D, 'Books/b.pdf', ['Books']), work(D, '01 Books/c.pdf', ['Books']),
    work(D, 'Books/Sub/d.pdf', ['Books', 'Sub']), event(D, 'e', ['2024', 'May']), event(`${D}Pics`, 'f', ['Pics', '2024']),
  ], unlisted: [{ rel: 'Books/n.txt', folder: ['Books'] }] }
  same(drive)
  assert.deepEqual(f.relsOf(drive, ['Books']), ['Books', '01 Books'])
})

test('a Section is read once, however many folders are asked for', () => {
  const SEC = '0000ABCD:Big'
  const works = []
  for (let a = 0; a < 1000; a++) for (let b = 0; b < 20; b++) works.push(work(SEC, `A${a}/S${b % 2}/${b}.pdf`, [`A${a}`, `S${b % 2}`]))
  let read = 0
  const home = { id: SEC, get works() { read++; return works }, unlisted: [] }
  for (let a = 0; a < 1000; a++) assert.deepEqual(f.relsOf(home, [`A${a}`, 'S1']), [`A${a}/S1`])
  assert.equal(read, 1)
})

test('a new Section object is read again', () => {
  const SEC = '0000ABCD:Books'
  const a = { id: SEC, works: [work(SEC, 'A/x.pdf', ['A'])], unlisted: [] }
  assert.deepEqual(f.relsOf(a, ['A']), ['A'])
  const b = { ...a, works: [work(SEC, 'B/x.pdf', ['A'])] }
  assert.deepEqual(f.relsOf(b, ['A']), ['B'])
})
