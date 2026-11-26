// The compact catalog made whole on the page, run by `npm test`: each sample the
// core's tests wrote to packed.fixture.json, packed and whole, must come back field for field, in
// one part and cut into parts.
import { test } from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync, readdirSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { runnerImport } from 'vite'

const { module: p } = await runnerImport(fileURLToPath(new URL('../src/lib/packed.ts', import.meta.url)))
const { packed, whole, parts, loose } = JSON.parse(readFileSync(new URL('./packed.fixture.json', import.meta.url), 'utf8'))

const head = h => ({ ...h, w: h.w.length, u: h.u.length })

test('every sample comes back whole', () => {
  assert.ok(packed.length > 10)
  packed.forEach((h, n) => {
    const works = [], unlisted = []
    p.unpack(head(h), h.w, works)
    p.unpackUnlisted(head(h), h.u, unlisted)
    assert.deepStrictEqual(p.sectionOf(head(h), works, unlisted), whole[n], `${h.kind} ${h.path}`)
  })
  assert.ok(whole.some(s => s.unlisted.length > 3))
  assert.ok(whole.some(s => s.path.startsWith('/') && s.unlisted.length), 'a Section on a / path with unlisted files')
})

test('unlisted files in parts put together are the Section\'s: each part stands on its own', () => {
  assert.ok(loose.parts.length > 1)
  const h = head(packed[loose.of]), unlisted = []
  for (const part of loose.parts) p.unpackUnlisted(h, part, unlisted)
  assert.deepStrictEqual(unlisted, whole[loose.of].unlisted)
})

test('ids come back byte for byte, so Marks and Ticks stay with their Works', () => {
  packed.forEach((h, n) => {
    const works = []
    p.unpack(head(h), h.w, works)
    assert.deepStrictEqual(works.map(w => w.id), whole[n].works.map(w => w.id))
  })
})

test('parts put together are the Section: each part stands on its own', () => {
  assert.ok(parts.length > 1)
  const h = head(packed[0]), works = []
  for (const part of parts) p.unpack(h, part, works)
  assert.deepStrictEqual(works, whole[0].works)
})

// A real catalog, written by the core's `a_real_catalog_comes_back_whole` with FVH_DUMP set to a
// folder: its parts made whole here give every Work's id and Tick keys byte for byte, and every
// unlisted file field for field.
const dump = process.env.FVH_DUMP
test('a real catalog\'s ids and Tick keys come back byte for byte', { skip: !dump && 'set FVH_DUMP' }, () => {
  const heads = JSON.parse(readFileSync(`${dump}/heads.json`, 'utf8'))
  const files = readdirSync(dump).filter(f => f.startsWith('part-') || f.startsWith('loose-'))
  let lines = ''
  heads.forEach((h, k) => {
    const works = []
    for (let n = 0; files.includes(`part-${k}-${n}.json`); n++) p.unpack(h, JSON.parse(readFileSync(`${dump}/part-${k}-${n}.json`, 'utf8')), works)
    assert.equal(works.length, h.w)
    for (const w of works) lines += w.id + w.items.map(i => '\u0001' + i.key).join('') + '\n'
    let u = 0
    for (let n = 0; files.includes(`loose-${k}-${n}.json`); n++) {
      const got = []
      p.unpackUnlisted(h, JSON.parse(readFileSync(`${dump}/loose-${k}-${n}.json`, 'utf8')), got)
      assert.deepStrictEqual(got, JSON.parse(readFileSync(`${dump}/unlisted-${k}-${n}.json`, 'utf8')), `unlisted part ${k}-${n}`)
      u += got.length
    }
    assert.equal(u, h.u)
  })
  assert.ok(lines === readFileSync(`${dump}/keys.txt`, 'utf8'), 'ids or Tick keys differ')
})
