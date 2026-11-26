// Search as the Hub does it: every word must match, case, accents and ё aside. A match on the
// title ranks first, then on the Work's own text (year, formats, Media, its lessons and tracks,
// its file names), then on its folders. Folded text is kept per Work, so a key press folds nothing.
import { fold, mains, natural, type Unlisted, type Work } from './api'

export const words = (q: string) => fold(q).split(/\s+/).filter(Boolean)

type Folded = { title: string; own: string; path: string }
const folded = new WeakMap<Work, Folded>()
const textOf = (w: Work): Folded => {
  let t = folded.get(w)
  if (!t) {
    const exts = [...new Set(w.items.map(i => i.ext))].join(' ')
    t = {
      title: fold(w.title),
      own: fold([w.title, w.year ?? '', exts, w.media.join(' '), ...mains(w).map(i => i.title), ...w.items.map(i => i.name)].join(' ')),
      path: fold(w.folder.join(' ')),
    }
    folded.set(w, t)
  }
  return t
}

/** 3: every word in the title; 2: in the Work's own text; 1: some only in its folders; 0: no match. */
export function score(w: Work, ws: string[]): number {
  if (!ws.length) return 2
  const { title, own: mine, path } = textOf(w)
  if (ws.every(x => title.includes(x))) return 3
  let ownOnly = true
  for (const x of ws) {
    if (mine.includes(x)) continue
    if (!path.includes(x)) return 0
    ownOnly = false
  }
  return ownOnly ? 2 : 1
}

export function matchesFile(u: Unlisted, ws: string[]): boolean {
  const t = fold(`${u.name} ${u.folder.join(' ')}`)
  return ws.every(x => t.includes(x))
}

/** Folder, then title, in reading order. */
export function byPlace(a: Work, b: Work): number {
  for (let i = 0; i < Math.min(a.folder.length, b.folder.length); i++) {
    const o = natural(a.folder[i], b.folder[i])
    if (o) return o
  }
  return a.folder.length - b.folder.length || natural(a.title, b.title)
}
