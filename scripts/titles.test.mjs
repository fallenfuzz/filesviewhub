// A kept leading number stays on the line of the word after it, run by `npm test`: Vite
// loads api.ts as the app does, so the test reads the same code.
import { test } from 'node:test'
import assert from 'node:assert/strict'
import { fileURLToPath } from 'node:url'
import { runnerImport } from 'vite'

const { module: a } = await runnerImport(fileURLToPath(new URL('../src/lib/api.ts', import.meta.url)))
const NB = '\u00a0'

test('a kept number is glued to the word after it', () => {
  assert.equal(a.glued('4. Supercalifragilistic'), `4.${NB}Supercalifragilistic`)
  assert.equal(a.glued('02 Next steps'), `02${NB}Next steps`)
  assert.equal(a.glued('3) Wet on wet'), `3)${NB}Wet on wet`)
  assert.equal(a.glued('4 - Intro'), `4${NB}-${NB}Intro`)
  assert.equal(a.glued('2.1 Что такое DevOps'), '2.1 Что такое DevOps', 'only the first space after a bare number or marker')
})

test('other titles are left as they are', () => {
  for (const t of ['Docker', '2001 A Space Odyssey', '1984', '3D Printing', '3-D Printing', '10.', ''])
    assert.equal(a.glued(t), t)
})

test('a month folder still reads as its month, and a kept number is glued', () => {
  assert.equal(a.place('02 February'), 'February')
  assert.equal(a.place('1. Getting started'), `1.${NB}Getting started`)
})
