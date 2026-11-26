// A book outside a Books Section has the Books card: its title and author apart, run by
// `npm test`: Vite loads byline.ts and api.ts as the app does, so the test reads the same code.
import { test } from 'node:test'
import assert from 'node:assert/strict'
import { fileURLToPath } from 'node:url'
import { runnerImport } from 'vite'

const load = f => runnerImport(fileURLToPath(new URL(`../src/lib/${f}`, import.meta.url))).then(r => r.module)
const b = await load('byline.ts')
const a = await load('api.ts')

test('a book name splits into its title and its author', () => {
  const names = ['Learning to See - J. Albescu', 'Networks at Home - R. Okafor', 'The Patient Gardener - Mira Solberg']
  const known = b.authorsIn(names, true)
  assert.deepEqual(names.map(n => b.byline(n, known)), [
    { title: 'Learning to See', by: 'J. Albescu', year: '' },
    { title: 'Networks at Home', by: 'R. Okafor', year: '' },
    { title: 'The Patient Gardener', by: 'Mira Solberg', year: '' },
  ])
})

test('a lesson name stays whole', () => {
  for (const n of ['Watercolor for Beginners', 'The Night Sky, Explained', 'Spanish in 30 Days'])
    assert.deepEqual(b.byline(n), { title: n, by: '', year: '' })
})

test('a course handout named as a lesson stays whole among its Section names', () => {
  const names = ['Lesson 1 - Intro', 'Lesson 2 - Intro', 'Week 2 - Slides', 'Module 3 - Setup Guide', 'Python Basics - Cheat Sheet',
    'Pasta Dough - Recipe Card', 'Grilling - Temperature Chart', 'Knife Skills - Shopping List', 'Spanish - Verb Tables',
    'Sourdough - Feeding Schedule', 'Guitar - Chord Chart', 'Home Lab - Network Diagram', 'Watercolor - Color Wheel']
  const known = b.authorsIn(names, true)
  for (const n of names) assert.deepEqual(b.byline(n, known), { title: n, by: '', year: '' }, n)
})

test('outside a Books Section an author splits off only after the title', () => {
  assert.deepEqual(b.byline('Mira Solberg - The Patient Gardener', b.authorsIn([], true)).by, '')
  assert.deepEqual(b.byline('Mira Solberg - The Patient Gardener').by, 'Mira Solberg', 'a Books Section reads either side')
  assert.deepEqual(b.byline('The Patient Gardener (Mira Solberg)', b.authorsIn([], true)).by, 'Mira Solberg')
})

const work = (ext, more = {}) => ({ id: 'x', title: 't', path: `t.${ext}`, folder: [], is_folder: false, media: [],
  items: [{ name: `t.${ext}`, path: `t.${ext}`, ext, size: 1, role: 'main' }], ...more })

test('a Work of one book file is a book', () => {
  for (const ext of ['pdf', 'epub', 'djvu', 'fb2', 'mobi', 'cbz']) assert.equal(a.isBook(work(ext)), true, ext)
})

test('a lesson, a document, a folder or a photo event is not', () => {
  for (const ext of ['mp4', 'docx', 'pptx', 'txt', 'jpg']) assert.equal(a.isBook(work(ext)), false, ext)
  assert.equal(a.isBook(work('pdf', { is_folder: true })), false, 'folder')
  assert.equal(a.isBook(work('pdf', { event: {} })), false, 'event')
})
