// Orphaned Marks (`decisions/2026/09/30/work-id-is-its-path.md`): Marks and Ticks whose Work or item is gone from the catalog, usually
// after a rename or move. Never matched automatically: Works that may be the same are only offered.
import { inside, keyOf, type Catalog, type Marks, type Section, type Work } from './api'

export interface Orphans {
  /** Works gone with their Mark or Ticks. */
  gone: string[]
  /** Ticked items gone from Works that are still here, as [Work id, item]. */
  items: [string, string][]
}

export const sectionOf = (catalog: Catalog, id: string): Section | undefined =>
  catalog.sections.filter(s => id.startsWith(inside(s.id)) || id.startsWith(s.id + '#')).sort((a, b) => b.id.length - a.id.length)[0]

export function orphansOf(catalog: Catalog, marks: Marks): Orphans {
  // One pass over every Work, looking up only the marked ones: a catalog can hold 600,000.
  const want = new Set([...Object.keys(marks.marks), ...Object.keys(marks.ticks)])
  const here = new Map<string, Work>()
  for (const s of catalog.sections) for (const w of s.works) if (want.has(w.id)) here.set(w.id, w)
  // A Work in a Section that is not connected is not orphaned: it is only away.
  const away = (id: string) => sectionOf(catalog, id)?.online === false
  const gone = [...want].filter(id => !here.has(id) && !away(id)).sort()
  const lost = Object.entries(marks.ticks).filter(([id]) => here.has(id)).flatMap(([id, t]) => {
    const keys = new Set(here.get(id)!.items.map(keyOf))
    return Object.keys(t).filter(k => !keys.has(k)).map(k => [id, k] as [string, string])
  })
  return { gone, items: lost }
}


/** The last part of a Work id: its file or folder name, without the drive at the top of a whole
 * drive's Section. */
export const nameOf = (id: string) => (id.replace(/\/$/, '').split('/').pop() ?? id).replace(/^[0-9A-Fa-f]{8}:/, '')

const fold = (s: string) => s.normalize('NFC').toLowerCase()

/** Works an orphan may have become: same Section, no Mark of their own, and the same name, or a
 * title holding `query` when the person searches (a rename). */
export function candidates(catalog: Catalog, marks: Marks, id: string, query = ''): Work[] {
  // A Section no longer checked (its folder renamed or moved and ticked again) is looked for everywhere.
  const s = sectionOf(catalog, id)
  const free = (s ? s.works : catalog.sections.flatMap(x => x.works)).filter(w => !marks.marks[w.id])
  const q = fold(query.trim())
  if (q) return free.filter(w => fold(w.title).includes(q) || fold(nameOf(w.id)).includes(q)).slice(0, 8)
  return free.filter(w => fold(nameOf(w.id)) === fold(nameOf(id)))
}
