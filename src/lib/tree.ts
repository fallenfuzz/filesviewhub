// The folders of the Section shown, made in one pass over its Works: each folder's Works,
// its own Works and files, and its folders. The folder columns, the rows and the tiles read
// from it, as a Section can hold 600,000 Works in thousands of folders and a pass over every Work
// for each folder froze the page. Kept apart from Library.svelte, with no runes, so `npm test`
// can load it (scripts/tree.test.mjs).
import type { Link, Section, Unlisted, Work } from './api'
import { natural } from './api'

/** A folder of the Section. */
export interface Node {
  /** Every Work in it and below, in the Section's order. */
  every: Work[]
  /** The Works right in it. */
  own: Work[]
  /** The files not on a card right in it. */
  files: Unlisted[]
  /** Some file not on a card is in it or below. */
  hasFiles: boolean
  /** A Split-out Section is further down than its folders. */
  linkBelow: boolean
  /** The place in the list of Split-out Sections of the first one further down, -1 for none. */
  linkFirst: number
  /** The names its folders take from the Split-out Sections in it or below. */
  linkNames: Set<string>
  /** The Split-out Sections right in it, each as the last folder of its path. */
  links: { l: Link; to: Section }[]
  kids: Map<string, Node>
}

/** A folder as the folder columns, the rows and the tiles list it: its name, the Works under it
 * that pass the filters, every Work under it and how many folders it holds. A folder split out as
 * a Section is there as `link`, with that Section's Works as `every` (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`); a filter leaves
 * its `works` empty, as its cards are that Section's. */
export interface Sub { name: string; every: Work[]; works: Work[]; subs: number; link?: Section; lid?: string }

const node = (): Node => ({ every: [], own: [], files: [], hasFiles: false, linkBelow: false, linkFirst: -1, linkNames: new Set(), links: [], kids: new Map() })

export class Tree {
  readonly root = node()

  /** `links`: the Split-out Sections shown in their folders' places, with the Section each opens. */
  constructor(works: Work[], unlisted: Unlisted[], links: { l: Link; to: Section }[]) {
    // A Work in the same folders as the one before it takes the same path without looking it up,
    // as a folder can hold 150,000 Works.
    let path: Node[] = [this.root], last: string[] = []
    for (const w of works) {
      const f = w.folder
      if (f !== last) {
        let k = 0
        const n = Math.min(f.length, last.length)
        while (k < n && f[k] === last[k]) k++
        path.length = k + 1
        for (let i = k; i < f.length; i++) path.push(this.kid(path[i], f[i]))
        last = f
      }
      for (let i = 1; i < path.length; i++) path[i].every.push(w)
      path[path.length - 1].own.push(w)
    }
    for (const u of unlisted) {
      if (u.work) continue
      let at = this.root
      for (const x of u.folder) { at = this.kid(at, x); at.hasFiles = true }
      at.files.push(u)
    }
    links.forEach((x, k) => {
      const f = x.l.folder
      if (!f.length) return
      let at = this.root
      for (let i = 0; i < f.length - 1; i++) {
        at.linkNames.add(f[i])
        at = this.kid(at, f[i])
        at.linkBelow = true
        if (at.linkFirst < 0) at.linkFirst = k
      }
      at.linkNames.add(f[f.length - 1])
      at.links.push(x)
    })
  }

  private kid(at: Node, name: string) {
    let k = at.kids.get(name)
    if (!k) at.kids.set(name, (k = node()))
    return k
  }

  /** Folder `f`, undefined when nothing is under it. */
  at(f: string[]): Node | undefined {
    let n: Node | undefined = this.root
    for (const x of f) if (!(n = n.kids.get(x))) return undefined
    return n
  }

  /** The folders in a folder, each list made once: with unlisted files shown (`unlisted`) a
   * folder that holds only those is one too, as in the Hub; `ok`, with a filter on, holds the
   * Works that pass it. */
  lister(unlisted: boolean, ok: Set<Work> | null): (at: string[]) => Sub[] {
    const made = new Map<Node, Sub[]>()
    const shown = (n: Node) => n.every.length > 0 || n.linkBelow || (unlisted && n.hasFiles)
    return at => {
      const p = this.at(at)
      if (!p) return []
      let list = made.get(p)
      if (list) return list
      // Folders with Works first, then those with only files not on a card, then those only on the
      // way to a Split-out Section, in the order of those Sections: two names can sort as equal
      // ("01 X", "1 X"), and the sort keeps their order.
      let kids = [...p.kids].filter(([, n]) => shown(n))
      if (!unlisted) kids = [...kids.filter(([, n]) => n.every.length), ...kids.filter(([, n]) => !n.every.length).sort(([, a], [, b]) => a.linkFirst - b.linkFirst)]
      list = []
      for (const [name, n] of kids) {
        let subs = 0
        for (const k of n.kids.values()) if (k.every.length) subs++
        for (const x of n.linkNames) if (!n.kids.get(x)?.every.length) subs++
        list.push({ name, every: n.every, works: ok ? n.every.filter(w => ok.has(w)) : n.every, subs })
      }
      for (const { l, to } of p.links) {
        const name = l.folder[l.folder.length - 1], n = p.kids.get(name)
        if (!n || !shown(n)) list.push({ name, every: to.works, works: ok ? [] : to.works, subs: 0, link: to, lid: l.id })
      }
      list.sort((a, b) => natural(a.name, b.name))
      made.set(p, list)
      return list
    }
  }
}
