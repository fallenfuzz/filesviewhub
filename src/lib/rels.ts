// Where the folders shown in a Section are on disk: titles drop numbers and marks, so a folder
// shown is found by the Works and unlisted files under it. Kept apart from folders.svelte.ts,
// with no runes, so `npm test` can load it (scripts/folders.test.mjs).
import type { Section, Work } from './api'
import { eventPlace, relIn } from './api'

/** The folder on disk a Work is in, as a path from its Section's folder: an Event's is its Photos
 * folder (scan.rs `place_of`), '' in a Photos Section, whose years and months are no folders. */
export const diskOf = (home: Section, w: Work) => relIn(eventPlace(w), home.id)

/** The paths in its Section of the folders shown under titles `f`: the real names come from the
 * Works and unlisted files under them. Titles drop numbers and marks, so one title can stand for
 * two folders on disk ("01 Books" and "Books"); each is listed once. `home` is a Section of
 * the catalog, never a merged view rebuilt on each change, or its index is made again each time. */
export function relsOf(home: Section, f: string[]): string[] {
  if (!f.length) return ['']
  return [...(relsIndex(home).get(f.join('\0')) ?? [])]
}

/** Every Section's `relsOf`, by its titles joined with NUL, made in one pass over the Section: a
 * folder column asks for each folder it lists, and a Section can hold 600,000 Works in thousands
 * of folders. Kept while the Section is the same object; the catalog is replaced, never
 * changed in place. */
const relsMade = new WeakMap<Section, Map<string, Set<string>>>()
function relsIndex(home: Section): Map<string, Set<string>> {
  let made = relsMade.get(home)
  if (made) return made
  const index = new Map<string, Set<string>>()
  made = index
  let titles: string[] = [], last = ''
  // A path's first n parts for each title n of `folder`, until the path has fewer parts; cut
  // without splitting it, as a folder can hold 150,000 Works.
  const add = (folder: string[], r: string) => {
    let key = '', i = -1, h = '', n = 0
    for (; n < folder.length && i !== -2; n++) {
      key = n ? key + '\0' + folder[n] : folder[n]
      const j = r.indexOf('/', i + 1)
      h = j < 0 ? r : r.slice(0, j)
      i = j < 0 ? -2 : j
      let rels = index.get(key)
      if (!rels) index.set(key, (rels = new Set()))
      rels.add(h)
    }
    titles = folder
    // A path that ran out before its titles did proves nothing about the next one.
    last = n === folder.length && h ? h + '/' : ''
  }
  // A path under the deepest folder found last, with the same titles, is in the same folders at
  // every level: it needs no cutting.
  const same = (folder: string[], r: string) => last !== '' && r.startsWith(last) && folder.length === titles.length && folder.every((x, i) => titles[i] === x)
  for (const w of home.works) {
    if (!w.folder.length) continue
    const r = diskOf(home, w)
    if (w.event || !same(w.folder, r)) add(w.folder, r)
  }
  last = ''
  for (const u of home.unlisted ?? []) if (u.folder.length && !same(u.folder, u.rel)) add(u.folder, u.rel)
  relsMade.set(home, made)
  return made
}
