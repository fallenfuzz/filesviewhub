// How the window looks: column widths, each Section's view at each level, card size, the cards' order per Section and on Home, autohide, unlisted files,
// merged Sections, the subfolder rows left open, how the Log and Programs lists show, what the Photos view shows and whether the backup reminder was put away. Kept in settings.json, so it travels with StoredData (`decisions/2026/10/07/app-data-in-storeddata-next-to-the-app.md`).
import { readSort } from './sort'
import { api, type CardSize, type LogView, type ProgramsView, type Ui } from './api'

export const ui = $state<{
  colw: Record<string, number>; views: Record<string, Record<string, string>>; size: CardSize; autohide: boolean; unlisted: boolean; merge: Record<string, string>
  logView: LogView; programsView: ProgramsView
  photoKind: '' | 'p' | 'c'; photoAll: boolean; week: number; backupSeen: boolean; open: Record<string, boolean>; sort: Record<string, string>
}>({ colw: {}, views: {}, size: 'normal', sort: {}, autohide: false, unlisted: false, merge: {}, logView: 'sessions', programsView: 'table', photoKind: '', photoAll: false, week: 1, backupSeen: false, open: {} })

const isMap = (v: unknown): v is Record<string, unknown> => !!v && typeof v === 'object' && !Array.isArray(v)

/** Set by settings kept before views per Section, with Everything below picked for every folder:
 *  each Section gets that view once the catalog is known (`adoptFolders`). */
let allBelow = false

export function loadUi(u: Ui | null) {
  ui.colw = Object.fromEntries(Object.entries(isMap(u?.colw) ? u.colw : {}).filter(([, w]) => Number.isFinite(w))) as Record<string, number>
  ui.views = Object.fromEntries(Object.entries(isMap(u?.views) ? u.views : {}).filter(([, v]) => isMap(v))
    .map(([id, v]) => [id, Object.fromEntries(Object.entries(v as Record<string, unknown>).filter(([, x]) => typeof x === 'string'))])) as Record<string, Record<string, string>>
  allBelow = u?.views === undefined && u?.folders === 'all'
  ui.size = u?.size && ['compact', 'normal', 'large'].includes(u.size) ? u.size : 'normal'
  ui.autohide = u?.autohide === true
  ui.unlisted = u?.unlisted === true
  ui.logView = u?.logView === 'table' || u?.logView === 'summary' ? u.logView : 'sessions'
  ui.programsView = u?.programsView === 'programs' ? 'programs' : 'table'
  ui.photoKind = u?.photoKind === 'p' || u?.photoKind === 'c' ? u.photoKind : ''
  ui.photoAll = u?.photoAll === true
  ui.week = u?.week === 0 ? 0 : 1
  ui.backupSeen = u?.backupSeen === true
  ui.open = Object.fromEntries(Object.entries(isMap(u?.open) ? u.open : {}).filter(([, v]) => v === true)) as Record<string, boolean>
  ui.sort = Object.fromEntries(Object.entries(isMap(u?.sort) ? u.sort : {}).filter(([, v]) => readSort(v))) as Record<string, string>
  ui.merge = Object.fromEntries(Object.entries(isMap(u?.merge) ? u.merge : {}).filter(([, v]) => typeof v === 'string')) as Record<string, string>
}

export const saveUi = () => api.setUi($state.snapshot(ui))

/** Gives each Section Everything below inside a folder, once, when the settings were kept with that
 *  picked for every folder; true when it changed `ui`. */
export function adoptFolders(ids: string[]) {
  if (!allBelow) return false
  allBelow = false
  for (const id of ids) ui.views[id] = { ...ui.views[id], folder: 'flat' }
  return true
}
