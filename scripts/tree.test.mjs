// The folder tree of the Section shown (tree.ts), run by `npm test`: it must give what the
// passes over every Work gave before it, for every folder, order included. The passes below are
// Library.svelte's and orphans.ts's as they were, word for word. With FVH_DUMP set to a folder
// written by the core's `a_real_catalog_comes_back_whole` (see packed.test.mjs), it checks every
// Section of that catalog too.
import { test } from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync, readdirSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { runnerImport } from 'vite'

const load = async f => (await runnerImport(fileURLToPath(new URL(`../src/lib/${f}`, import.meta.url)))).module
const { Tree } = await load('tree.ts')
const { natural, keyOf } = await load('api.ts')
const { orphansOf, sectionOf } = await load('orphans.ts')
const p = await load('packed.ts')
const { byPlace } = await load('search.ts')

const under = (w, f) => f.every((x, i) => w.folder[i] === x)

// Library.svelte's subsOf in an earlier version: a pass over every Work for each folder asked.
function subsBefore(section, at, { unlisted, linksHere, ok }) {
  const filtered = !!ok
  const linkDirs = at => linksHere.filter(({ l }) => l.folder.length > at.length + 1 && at.every((x, i) => l.folder[i] === x)).map(({ l }) => l.folder[at.length])
  const m = new Map()
  for (const w of section?.works ?? []) if (w.folder.length > at.length && under(w, at)) {
    const n = w.folder[at.length], here = m.get(n)
    if (here) here.push(w)
    else m.set(n, [w])
  }
  if (unlisted) for (const u of section?.unlisted ?? []) if (!u.work && u.folder.length > at.length && at.every((x, i) => u.folder[i] === x)) {
    if (!m.has(u.folder[at.length])) m.set(u.folder[at.length], [])
  }
  for (const n of linkDirs(at)) if (!m.has(n)) m.set(n, [])
  const links = linksHere.filter(({ l }) => l.folder.length === at.length + 1 && at.every((x, i) => l.folder[i] === x) && !m.has(l.folder[at.length]))
  return [...[...m].map(([n, every]) => ({
    name: n, every, works: filtered ? every.filter(w => ok.has(w)) : every,
    subs: new Set([...every.filter(w => w.folder.length > at.length + 1).map(w => w.folder[at.length + 1]), ...linkDirs([...at, n]), ...linksHere.filter(({ l }) => l.folder.length === at.length + 2 && l.folder[at.length] === n && at.every((x, i) => l.folder[i] === x)).map(({ l }) => l.folder[at.length + 1])]).size,
  })), ...links.map(({ l, to }) => ({ name: l.folder[at.length], every: to.works, works: filtered ? [] : to.works, subs: 0, link: to, lid: l.id }))].sort((a, b) => natural(a.name, b.name))
}
// The rest of what read every Work in an earlier version: a folder's Works, a row's own Works and files,
// and whether a folder still holds anything after a read.
const belowBefore = (s, f) => s.works.filter(w => under(w, f))
const mineBefore = (s, f, passes) => s.works.filter(w => w.folder.length === f.length && under(w, f) && passes(w))
const filesBefore = (s, f) => (s.unlisted ?? []).filter(u => !u.work && u.folder.length === f.length && f.every((x, i) => u.folder[i] === x))
const holdsBefore = (s, f) => s.works.some(w => under(w, f)) || (s.unlisted ?? []).some(u => !u.work && f.every((x, i) => u.folder[i] === x))
function orphansBefore(catalog, marks) {
  const items = new Map(catalog.sections.flatMap(s => s.works.map(w => [w.id, new Set(w.items.map(keyOf))])))
  const away = id => sectionOf(catalog, id)?.online === false
  const gone = [...new Set([...Object.keys(marks.marks), ...Object.keys(marks.ticks)])].filter(id => !items.has(id) && !away(id)).sort()
  const lost = Object.entries(marks.ticks).filter(([id]) => items.has(id))
    .flatMap(([id, t]) => Object.keys(t).filter(k => !items.get(id).has(k)).map(k => [id, k]))
  return { gone, items: lost }
}

/** The same list, the same Works in the same order (by identity), the same counts and links. */
function same(a, b, where) {
  assert.equal(a.length, b.length, `${where}: ${a.map(x => x.name)} vs ${b.map(x => x.name)}`)
  a.forEach((x, i) => {
    const y = b[i], at = `${where} › ${y.name}`
    assert.deepEqual([x.name, x.subs, x.link?.id, x.lid], [y.name, y.subs, y.link?.id, y.lid], at)
    sameList(x.every, y.every, at + ' every')
    sameList(x.works, y.works, at + ' works')
  })
}
function sameList(a, b, where) {
  assert.equal(a.length, b.length, where)
  for (let i = 0; i < a.length; i++) if (a[i] !== b[i]) assert.fail(`${where}: item ${i} differs`)
}

/** Every folder of the tree, then some that are not there. */
function foldersOf(tree) {
  const out = [[]]
  const walk = (n, f) => { for (const [k, c] of n.kids) { out.push([...f, k]); walk(c, [...f, k]) } }
  walk(tree.root, [])
  return [...out, ['nowhere'], [...(out.at(-1) ?? []), 'nowhere']]
}

/** Checks Section `s` shown with `linksHere`, every folder or (`cap`) an even spread of them. */
function check(s, linksHere, cap = Infinity) {
  const tree = new Tree(s.works, s.unlisted ?? [], linksHere)
  let all = foldersOf(tree)
  if (all.length > cap) all = Array.from({ length: cap }, (_, i) => all[Math.floor((i * all.length) / cap)])
  // A filter: some Works pass, as a search or a Mark filter leaves them.
  const pass = w => w.id.length % 3 !== 0
  const ok = new Set(s.works.filter(pass))
  for (const unlisted of [false, true]) for (const o of [null, ok]) {
    const list = tree.lister(unlisted, o)
    for (const f of all) same(list(f), subsBefore(s, f, { unlisted, linksHere, ok: o }), `${s.id} [${f.join('/')}] unlisted=${unlisted} filter=${!!o}`)
  }
  for (const f of all) {
    const n = tree.at(f), at = `${s.id} [${f.join('/')}]`
    if (f.length) sameList(n?.every ?? [], belowBefore(s, f), at + ' below')
    sameList((n?.own ?? []).filter(pass), mineBefore(s, f, pass), at + ' own')
    sameList(n?.files ?? [], filesBefore(s, f), at + ' files')
    if (f.length) assert.equal(!!n && (n.every.length > 0 || n.hasFiles), holdsBefore(s, f), at + ' holds')
  }
  return all.length
}

// A Section with what made the passes hard: one title for two folders on disk ("01 Books" and
// "Books"), a renamed folder, number-led and case-only different names, Works at the top,
// a merged Section's Works from two drives with one folder in two runs, files not on a card
// (alone in a folder, beside Works, on a card), and Split-out Sections at each depth, two of
// them in folders whose names sort as equal.
const work = (id, folder, items = ['a.txt']) => ({ id, title: id.split('/').at(-1), path: '', folder, is_folder: false, items: items.map(r => ({ rel: r, key: r })), media: ['Reading'], size: 1, added: 0, cloud: 0, year: null })
const S1 = 'AAAA0001:Lib', S2 = 'BBBB0002:Lib'
const books = ['Books']
const works = [
  work(`${S1}/top.pdf`, []),
  work(`${S1}/01 Books/x.pdf`, books), work(`${S1}/Books/y.pdf`, books),
  work(`${S2}/Books/z.pdf`, ['Books']),
  work(`${S1}/moby_dick/a.pdf`, ['Moby Dick']),
  work(`${S1}/10 Part/a.pdf`, ['Part', '10 Part']), work(`${S1}/2 Part/a.pdf`, ['Part', '2 Part']), work(`${S1}/1 Part/a.pdf`, ['Part', '1 Part']),
  work(`${S1}/a/1.pdf`, ['a']), work(`${S1}/A/1.pdf`, ['A']),
  work(`${S1}/Deep/one/two/three/x.pdf`, ['Deep', 'one', 'two', 'three']), work(`${S1}/Deep/one/y.pdf`, ['Deep', 'one']),
  work(`${S2}/Books/w.pdf`, books), work(`${S2}/Deep/one/two/z.pdf`, ['Deep', 'one', 'two']),
  work(`${S1}/Books/v.pdf`, ['Books']),
]
const unl = (folder, work = null) => ({ name: 'n.bin', rel: [...folder, 'n.bin'].join('/'), path: '', folder, reason: 'unknown', size: 1, cloud: false, work })
const to = (id, n) => ({ id, name: id, path: '', drive: '', kind: 'Books', guessed: false, online: true, works: Array.from({ length: n }, (_, i) => work(`${id}/${i}.pdf`, [])), files: n, scanned_at: 0, unlisted: [] })
const section = {
  id: S1, name: 'Lib', path: '', drive: 'C', kind: 'Books', guessed: false, online: true, files: 0, scanned_at: 0, works,
  unlisted: [unl(['Only files']), unl(['Only files', 'deeper']), unl(['Books']), unl(['Moby Dick'], `${S1}/moby_dick/a.pdf`), unl(['Gone files'], 'x'), unl(['Linked', 'sub']), unl(['1 X'])],
}
const linksHere = [
  { l: { id: 'L1', rel: 'Split', folder: ['Split'] }, to: to('L1', 2) },
  { l: { id: 'L2', rel: 'Books', folder: ['Books'] }, to: to('L2', 1) },
  { l: { id: 'L3', rel: 'Deep/one/other', folder: ['Deep', 'one', 'other'] }, to: to('L3', 3) },
  { l: { id: 'L4', rel: 'Far/away/here', folder: ['Far', 'away', 'here'] }, to: to('L4', 1) },
  { l: { id: 'L5', rel: 'Only files', folder: ['Only files'] }, to: to('L5', 1) },
  { l: { id: 'L6', rel: 'Linked/sub', folder: ['Linked', 'sub'] }, to: to('L6', 1) },
  // Two names that sort as equal, the second with files of its own: the Sections' order holds.
  { l: { id: 'L7', rel: '01 X/y', folder: ['01 X', 'y'] }, to: to('L7', 1) },
  { l: { id: 'L8', rel: '1 X/x', folder: ['1 X', 'x'] }, to: to('L8', 1) },
]

test('the tree gives what the passes over every Work gave, in every folder', () => {
  check(section, linksHere)
  check({ ...section, unlisted: undefined }, [])
  check({ ...section, works: [] }, linksHere)
})

test('a filter that passes nothing leaves every folder with Works listed, as before', () => {
  const tree = new Tree(works, section.unlisted, linksHere)
  for (const f of foldersOf(tree)) same(tree.lister(false, new Set())(f), subsBefore(section, f, { unlisted: false, linksHere, ok: new Set() }), f.join('/'))
})

test('the orphan count is the same as before', () => {
  const catalog = { sections: [section, { ...section, id: 'CCCC0003:Off', online: false, works: [] }] }
  const marks = {
    marks: { [works[1].id]: { m: 'next', at: 1 }, [`${S1}/gone.pdf`]: { m: 'done', at: 2 }, 'CCCC0003:Off/away.pdf': { m: 'next', at: 3 } },
    ticks: { [works[2].id]: { 'a.txt': 1, 'b.txt': 2 }, [`${S2}/old.pdf`]: { 'a.txt': 1 } },
  }
  assert.deepEqual(orphansOf(catalog, marks), orphansBefore(catalog, marks))
})

const dump = process.env.FVH_DUMP
test('a real catalog\'s Sections give what the passes gave', { skip: !dump && 'set FVH_DUMP' }, () => {
  const heads = JSON.parse(readFileSync(`${dump}/heads.json`, 'utf8'))
  const files = readdirSync(dump).filter(f => f.startsWith('part-'))
  const sections = heads.map((h, k) => {
    const works = []
    for (let n = 0; files.includes(`part-${k}-${n}.json`); n++) p.unpack(h, JSON.parse(readFileSync(`${dump}/part-${k}-${n}.json`, 'utf8')), works)
    return { ...h, works }
  })
  for (const s of sections) {
    const linksHere = (s.links ?? []).flatMap(l => { const t = sections.find(x => x.id === l.id); return t && t.id !== s.id ? [{ l, to: t }] : [] })
    // A pass per folder on 600,000 Works takes minutes: a large Section is checked in 40 folders.
    const n = check(s, linksHere, s.works.length > 100000 ? 40 : Infinity)
    console.log(`${s.name}: ${s.works.length} Works, ${n} folders checked`)
  }
  // Sections shown as one (`decisions/2026/10/05/checked-folders-on-any-drive.md`): every Section's Works together, in reading order, as merged
  // Sections show them.
  if (sections.length > 1 && sections.every(s => s.works.length <= 100000)) {
    const one = { ...sections[0], works: sections.flatMap(s => s.works).sort(byPlace), unlisted: sections.flatMap(s => s.unlisted ?? []) }
    console.log(`all as one: ${one.works.length} Works, ${check(one, [])} folders checked`)
  }
})
