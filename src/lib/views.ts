// The view levels of each Section kind and the views each level offers. The view picked is
// kept per Section and level in `ui.views`; a missing one is the level's first view, its default.
// Only views the app draws are listed: a new view goes in here when it is built.
import type { Kind, Section } from './api'
import { ui } from './ui.svelte'

export type LevelId = 'section' | 'folder' | 'work' | 'years' | 'year' | 'month' | 'day'
export interface Level { id: LevelId; views: string[] }

const section: Level = { id: 'section', views: ['cards'] }
const folder: Level = { id: 'folder', views: ['rows', 'tiles', 'flat'] }
const work = (v: string): Level => ({ id: 'work', views: [v] })

export const LEVELS: Record<Kind, Level[]> = {
  Courses: [section, folder, work('lessons')],
  Books: [section, folder, work('formats')],
  Collections: [section, folder, work('files')],
  Files: [section, folder, work('details')],
  Photos: [{ id: 'years', views: ['heat'] }, { id: 'year', views: ['months'] }, { id: 'month', views: ['calendar'] }, { id: 'day', views: ['grid'] }],
}

/** The view a level shows with these picks: the one picked if the level offers it, else its default. */
export const viewIn = (picks: Record<string, string> | undefined, l: Level) => {
  const v = picks?.[l.id]
  return v && l.views.includes(v) ? v : l.views[0]
}

/** The view a Section shows at a level; '' when its kind has no such level. */
export function viewOf(s: Section, level: LevelId) {
  const l = LEVELS[s.kind].find(x => x.id === level)
  return l ? viewIn(ui.views[s.id], l) : ''
}

/** Keeps only the picks that differ from a default of `kind`'s levels. */
export function picksFor(kind: Kind, picks: Record<string, string>) {
  return Object.fromEntries(LEVELS[kind].map(l => [l.id, viewIn(picks, l)] as const).filter(([id, v]) => v !== LEVELS[kind].find(l => l.id === id)!.views[0]))
}

/** Puts a Section's picks in `ui.views`; none leaves no entry. The caller saves. */
export function setViews(id: string, picks: Record<string, string>) {
  if (Object.keys(picks).length) ui.views[id] = { ...picks }
  else delete ui.views[id]
}
