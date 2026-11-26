// Folder kinds and views (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`): a folder inside a Section with a kind or views of its own.
// They are kept in settings.json by Drive and path (store.rs `folder_kinds`), keyed as the
// folder's Section id, `/`, its path in the Section, which is how its Works' ids start. The
// nearest folder with a setting wins, for the kind and for each level's view.
import type { Catalog, Drive, FolderSetting, Kind, Section, Settings, Suggestion, Work } from './api'
import { keyIn, place } from './api'
import { diskOf, relsOf } from './rels'
import { ui } from './ui.svelte'
import { LEVELS, viewIn, type LevelId } from './views'

/** `offered`: the Sections whose offer of a kind per folder was answered or put away. */
export const folderKinds = $state<{ of: Record<string, FolderSetting>; offered: string[] }>({ of: {}, offered: [] })

/** Takes the settings' Folder kinds after they are read or changed. */
export function loadFolders(s: Settings) { folderKinds.of = s.folder_kinds ?? {}; folderKinds.offered = s.offered ?? [] }

/** Whether each listed drive's folder names keep their case (Linux), by serial as keys start
 * (`{serial}:`); a drive not listed (not connected, plugged in later) takes the system's, `caseDefault`. */
let caseKept = $state.raw(new Map<string, boolean>())
let caseDefault = $state(false)
/** Takes the drives listed, for `settingOf`. */
export function setCaseKept(drives: Drive[]) {
  caseKept = new Map(drives.map(d => [d.serial.toString(16).toUpperCase().padStart(8, '0'), d.keeps_case]))
}
/** Takes the system's rule (`Info.keeps_case`), for a drive not listed. */
export function setCaseDefault(keeps: boolean) { caseDefault = keeps }

/** A folder's setting by its key, compared as the core compares paths: as written on a drive whose
 * names keep their case, else case aside. */
function settingOf(key: string): FolderSetting | undefined {
  if (folderKinds.of[key] || (caseKept.get(key.slice(0, 8).toUpperCase()) ?? caseDefault)) return folderKinds.of[key]
  const low = key.toLowerCase()
  return Object.entries(folderKinds.of).find(([k]) => k.toLowerCase() === low)?.[1]
}

export { diskOf, relsOf } from './rels'
/** The first of `relsOf`, or undefined when nothing is under the titles. */
export const relOf = (home: Section, f: string[]): string | undefined => relsOf(home, f)[0]

/** Each folder on disk shown as folder `f` of the Section shown, `shown`, with its checked folder's
 * Section (its own first, then those shown inside it, `decisions/2026/10/05/checked-folders-on-any-drive.md`) and its path there: a folder of a
 * merged Section is one folder on each drive, one title can be two folders, and a setting
 * goes to all of them. */
export function homesOf(catalog: Catalog, shown: string, f: string[]): { home: Section; rel: string }[] {
  return catalog.sections.filter(x => x.id === shown || ui.merge[x.id] === shown)
    .sort((a, b) => Number(b.id === shown) - Number(a.id === shown))
    .flatMap(h => relsOf(h, f).map(rel => ({ home: h, rel })))
}
/** The first of `homesOf`: the one whose setting the folder's menu and dialog show. */
export const homeOf = (catalog: Catalog, s: Section, f: string[]) => homesOf(catalog, s.id, f)[0] as { home: Section; rel: string } | undefined

/** The folders from `rel` up to the Section's, nearest first, as paths. */
function upFrom(rel: string) {
  const parts = rel ? rel.split('/') : []
  return parts.map((_, i) => parts.slice(0, parts.length - i).join('/'))
}
/** The title of folder `r` above or at a folder with titles `f`, as the folder column shows it. */
const titleIn = (f: string[], r: string) => place(f[r.split('/').length - 1] ?? r.split('/').at(-1) ?? '')

/** Where a folder's kind comes from: its own (when `self`), else the nearest folder above with one,
 * else its Section. */
export function kindFrom(home: Section, rel: string, f: string[] = [], self = true): { kind: Kind; from: string; own: boolean } {
  for (const r of upFrom(rel).slice(self ? 0 : 1)) {
    const k = settingOf(keyIn(home.id, r))?.kind
    if (k) return { kind: k, from: titleIn(f, r), own: r === rel }
  }
  return { kind: home.kind, from: home.name, own: false }
}
/** The kind a folder's Works are made as (scan.rs `kind_at`). */
export const kindAt = (home: Section, rel: string) => kindFrom(home, rel).kind

/** The view a folder shows at a level, and where it comes from: its own pick (when `self`), else
 * the nearest folder above with one for that level, else the Section shown, `shown` (the one the
 * View menu sets). `key` is that folder's, '' for the Section. '' as the view when its kind (or
 * `kind`, one it is about to take) has no such level. */
export function viewFrom(home: Section, rel: string, level: LevelId, f: string[] = [], self = true, kind = kindAt(home, rel), shown = home): { view: string; from: string; own: boolean; key: string } {
  const l = LEVELS[kind].find(x => x.id === level)
  if (!l) return { view: '', from: shown.name, own: false, key: '' }
  for (const r of upFrom(rel).slice(self ? 0 : 1)) {
    const v = settingOf(keyIn(home.id, r))?.views?.[level]
    if (v && l.views.includes(v)) return { view: v, from: titleIn(f, r), own: r === rel, key: keyIn(home.id, r) }
  }
  return { view: viewIn(ui.views[shown.id], l), from: shown.name, own: false, key: '' }
}

/** A folder's own setting: its kind and the views picked for it, none when it has none. */
export function ownOf(home: Section, rel: string): { kind: Kind | null; views: Record<string, string> } {
  const s = settingOf(keyIn(home.id, rel))
  return { kind: s?.kind ?? null, views: { ...(s?.views ?? {}) } }
}

/** Each Section's Photos folders, as their titles joined by `/`, from its Events; kept per list
 * of Works, so each read or merge finds them once. */
const roots = new WeakMap<Work[], Set<string>>()
function rootsOf(s: Section) {
  let r = roots.get(s.works)
  if (!r) {
    r = new Set()
    for (const w of s.works) if (w.event) {
      // Below its Photos folder's titles an Event has its year and month, or Undated (scan.rs `events`).
      const parts = w.folder.length - (w.taken ? 2 : 1)
      if (parts > 0) r.add(w.folder.slice(0, parts).join('/'))
    }
    roots.set(s.works, r)
  }
  return r
}
/** The titles of the Photos folder that folder `f` of Section `s` is, or is a year, month or
 * Undated of (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`): from there down it shows as a Photos Section does. Undefined elsewhere,
 * and for a Photos Section's own years, which are no Photos folder. */
export function photoFolderOf(s: Section, f: string[]): string[] | undefined {
  const r = rootsOf(s)
  if (!r.size) return undefined
  for (let n = 1; n <= f.length; n++) if (r.has(f.slice(0, n).join('/'))) return f.slice(0, n)
  return undefined
}

/** An Event of a Photos Section's own calendar: its folders are only its year and month, or
 * Undated (scan.rs `events`); a Photos folder's Events have that folder's titles before them. */
export const ownEvent = (w: Work) => w.event && w.folder.length === (w.taken ? 2 : 1)

/** A Photos Section's Folders with other things (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`): the titles of its top-level folders
 * with another kind, and of those split out as Sections. They show below its years, not in its
 * calendar; none in any other Section. */
export function othersOf(s: Section): Set<string> {
  if (s.kind !== 'Photos') return new Set()
  return new Set([...s.works.filter(w => !ownEvent(w) && w.folder.length).map(w => w.folder[0]), ...(s.links ?? []).map(l => l.folder[0])])
}

/** A top-level folder of a Photos Section titled as a year or Undated cannot take another kind:
 * it would share its place in the first column with that year (scan.rs `title_of`). */
export const yearLike = (name: string) => /^(\d{4}|undated)$/i.test(name.trim())

/** The folder Review's move fills, by its name on disk (shots.rs `NOT_NEEDED`). */
const NOT_NEEDED = 'not needed'
/** A Section's top-level folders on disk with how many files each holds, from what was read last:
 * their real names, as a setting is kept by Drive and path. Review's `Not needed` is left out. */
export function topFoldersOf(s: Section): { name: string; files: number }[] {
  const n = new Map<string, number>()
  const base = s.path.replace(/[\\/]+$/, '')
  const add = (path: string) => {
    if (!path.startsWith(base)) return
    const parts = path.slice(base.length).split(/[\\/]/).filter(Boolean)
    if (parts.length > 1 && parts[0].toLowerCase() !== NOT_NEEDED) n.set(parts[0], (n.get(parts[0]) ?? 0) + 1)
  }
  for (const w of s.works) for (const i of w.items) add(i.path)
  for (const u of s.unlisted ?? []) add(u.path)
  for (const l of s.links ?? []) if (!l.rel.includes('/') && !n.has(l.rel)) n.set(l.rel, 0)
  return [...n].map(([name, files]) => ({ name, files }))
}

/** The folders offered a kind for Section `s` (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`): a folder also in another Section shown
 * with it (Show inside) is one folder on screen, so it is not offered, as Folder settings cannot
 * make it a Section either. */
export function offerable(catalog: Catalog, s: Section, list: Suggestion[]): Suggestion[] {
  const into = ui.merge[s.id]
  const top = into && catalog.sections.some(x => x.id === into) ? into : s.id
  const all = catalog.sections.filter(x => x.id !== s.id && (x.id === top || ui.merge[x.id] === top))
  return list.filter(x => !all.some(h => topFoldersOf(h).some(t => t.name.toLowerCase() === x.rel.toLowerCase()) || relsOf(h, [x.title]).length > 0))
}
