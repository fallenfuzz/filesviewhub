// The catalog's Works and unlisted files as the core sends them, compact (from
// src-tauri/src/packed.rs), made whole again by the same rules. A field left out is made again
// from the Section's folder and the Work's or file's other fields; a Work's folder, folder titles
// and media, when left out, are the Work's before it, an item's group the item's before it, and
// an unlisted file's folder titles and reason the file's before it. `scripts/packed.fixture.json`
// holds samples in both forms, written by the core's tests, and `scripts/packed.test.mjs` checks
// this file against it.
import { keyIn, type Item, type Reason, type Role, type Section, type Unlisted, type Work } from './api'

export interface CItem { r: string; n?: string; p?: string; e?: string; s: number; m: number; c?: boolean; k?: string; K?: boolean; t?: string; g?: string; o?: Role; l?: string; at?: number }
export interface CWork { d?: string; p?: string; id?: string; t?: string; f?: string[]; F?: boolean; i: CItem[]; md?: string[]; s?: number; a?: number; c?: number; y?: number; tk?: [number, number]; ev?: boolean; do?: boolean }
export interface CUnlisted { r: string; n?: string; p?: string; f?: string[]; o?: Reason; s: number; m: number; c?: boolean; w?: string }
/** A Section without its Works and unlisted files, and how many of each it has (`w`, `u`). */
export type Head = Omit<Section, 'works' | 'unlisted'> & { w: number; u: number }
/** What the core sends first: every Section's head, and the stamp each part asked for names. */
export interface Heads { v: number; stamp: number; sections: Head[] }

const sepOf = (sp: string) => (sp.startsWith('/') ? '/' : '\\')
const joinPath = (a: string, b: string, sep: string) => {
  const r = sep === '/' ? b : b.replaceAll('/', '\\')
  return a.endsWith(sep) ? a + r : a + sep + r
}
const last = (r: string) => r.slice(r.lastIndexOf('/') + 1)
/** A name without its extension; a name that starts with its only `.` keeps it. */
const stem = (n: string) => { const i = n.lastIndexOf('.'); return i > 0 ? n.slice(0, i) : n }
/** The extension, lower case, of a name whose extension is ASCII. */
function extOf(n: string): string {
  const e = n.slice(n.lastIndexOf('.') + 1 || n.length)
  // eslint-disable-next-line no-control-regex
  return /^[\x00-\x7f]*$/.test(e) ? e.replace(/[A-Z]+/g, x => x.toLowerCase()) : ''
}
/** A lesson's key: its path without the extension. */
function lessonKey(r: string): string {
  const i = r.lastIndexOf('.')
  if (i < 0) return r
  const a = r.slice(0, i)
  return a && !a.endsWith('/') && !r.slice(i).includes('/') ? a : r
}

/** Makes one part of a Section's Works whole, adding them to `out`. */
export function unpack(h: Head, part: CWork[], out: Work[]) {
  const sp = h.path, sep = sepOf(sp)
  let d = '', f: string[] = [], md: string[] = []
  for (const c of part) {
    if (c.d !== undefined) d = c.d
    if (c.f) f = c.f
    if (c.md) md = c.md
    const at = d ? joinPath(sp, d, sep) : sp
    let g = ''
    const items: Item[] = c.i.map(i => {
      const name = i.n ?? last(i.r)
      if (i.g !== undefined) g = i.g
      const it: Item = { name, rel: i.r, path: i.p ?? joinPath(at, i.r, sep), ext: i.e ?? extOf(name), size: i.s, mtime: i.m, cloud: !!i.c,
        key: i.k ?? (i.K ? lessonKey(i.r) : i.r), title: i.t ?? stem(name), group: g, role: i.o ?? 'main' }
      if (i.l) it.lang = i.l
      if (i.at !== undefined) it.at = i.at
      return it
    })
    const folder = !!c.F, first = items[0]?.name
    let size = 0, added = 0, cloud = 0
    for (const i of items) { size += i.size; if (i.mtime > added) added = i.mtime; if (i.cloud) cloud++ }
    const w: Work = {
      id: c.id ?? (folder ? keyIn(h.id, d + '/') : keyIn(h.id, d ? d + '/' + stem(first!) : stem(first!))),
      title: c.t ?? (folder ? last(d) : first === undefined ? '' : stem(first)),
      path: c.p ?? (folder ? at : (items[0]?.path ?? '')),
      // One `folder` and one `media` array are shared by the Works in a row that have them: never changed in place.
      folder: f, is_folder: folder, items, media: md, size: c.s ?? size, added: c.a ?? added, cloud: c.c ?? cloud, year: c.y ?? null,
    }
    if (c.tk) w.taken = c.tk
    if (c.ev) w.event = true
    if (c.do) w.day_only = true
    out.push(w)
  }
}

/** Makes one part of a Section's unlisted files whole, adding them to `out`. */
export function unpackUnlisted(h: Head, part: CUnlisted[], out: Unlisted[]) {
  const sp = h.path, sep = sepOf(sp)
  let f: string[] = [], o: Reason = 'unknown'
  for (const c of part) {
    if (c.f) f = c.f
    if (c.o) o = c.o
    // One `folder` array is shared by the files in a row that have it: never changed in place.
    out.push({ name: c.n ?? last(c.r), rel: c.r, path: c.p ?? joinPath(sp, c.r, sep), folder: f, reason: o, size: c.s, mtime: c.m, cloud: !!c.c, work: c.w ?? null })
  }
}

/** A Section from its head, its Works and its unlisted files. */
export function sectionOf(h: Head, works: Work[], unlisted: Unlisted[]): Section {
  const { w: _, u: __, ...rest } = h
  return { ...rest, works, unlisted }
}
