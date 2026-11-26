// The order of the cards: by name, date added, length or last opened, either way, picked per Section and
// for Home in View. No runes, so `npm test` can load it (scripts/sort.test.mjs).
import type { Detail, Work } from './api'
import { mains, natural } from './api'

export type SortBy = 'name' | 'added' | 'length' | 'opened'
/** A pick as kept in settings.json: `name`, `added`, `length` or `opened`, with `-` before it the other way. */
export interface Sort { by: SortBy; down: boolean }

const BY: SortBy[] = ['name', 'added', 'length', 'opened']

/** A kept pick, or null for the order the cards come in. */
export function readSort(v: unknown): Sort | null {
  if (typeof v !== 'string') return null
  const down = v.startsWith('-'), by = (down ? v.slice(1) : v) as SortBy
  return BY.includes(by) ? { by, down } : null
}
export const writeSort = (s: Sort) => (s.down ? '-' : '') + s.by
/** The way each pick starts: names A to Z, the newest first, the shortest first, the last opened first. */
export const firstWay = (by: SortBy): Sort => ({ by, down: by === 'added' || by === 'opened' })

/** A Work's length: its time in seconds, else its pages. Works with a time come first, then those
 * with pages, then those whose length is not known yet; the way picked turns each group round,
 * never the order of the groups. */
function lengthOf(w: Work, details: Record<string, Detail>): [number, number] {
  // As the card's line (`metaOf`): a folder's lessons added up, a file Work's first file.
  const m = mains(w), of = w.is_folder ? m : [m[0] ?? w.items[0]].filter(Boolean)
  let s = 0, p = 0
  for (const i of of) { const d = details[i.path]; if (d) { s += d.s ?? 0; p += d.p ?? 0 } }
  return s ? [0, s] : p ? [1, p] : [2, 0]
}

/** `list` in the order `s` picks; null keeps the order it came in. Equal ones keep theirs too (the sort is stable).
 * Each key is read once, as a Section can hold 150,000 Works. `opened`: when a Work was last
 * opened, 0 for never; those never opened come after the rest, either way. */
export function sortWorks<T>(list: T[], s: Sort | null, work: (x: T) => Work, details: Record<string, Detail>, opened: (w: Work) => number = () => 0): T[] {
  if (!s || list.length < 2) return list
  const k = s.down ? -1 : 1
  if (s.by === 'name') return [...list].sort((a, b) => k * natural(work(a).title, work(b).title))
  if (s.by === 'added') {
    const at = list.map(x => ({ x, v: work(x).added }))
    return at.sort((a, b) => k * (a.v - b.v)).map(r => r.x)
  }
  if (s.by === 'opened') {
    const at = list.map(x => ({ x, v: opened(work(x)) }))
    return at.sort((a, b) => +!a.v - +!b.v || k * (a.v - b.v)).map(r => r.x)
  }
  const at = list.map(x => { const [g, v] = lengthOf(work(x), details); return { x, g, v } })
  return at.sort((a, b) => a.g - b.g || k * (a.v - b.v)).map(r => r.x)
}
