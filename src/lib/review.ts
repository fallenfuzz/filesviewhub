// Review in a Photos Section (`decisions/2026/10/04/sorting-ends-in-a-move-script-never-a-delete.md`, `decisions/2026/10/04/photos-take-no-marks-review-is-the-progress.md`): which Photos wait for a choice, and the steps that
// go through them. The suggestions come from the Rust core (shots.rs); the choices are kept with
// the Marks.
import { eventPlace, inside, keyIn, place, relIn, type Catalog, type Item, type Marks, type Note, type PhotoPlace, type Work } from './api'

export type Notes = Record<string, Note>

/** Review suggests a Photo may not be needed: an extra Similar shot or a Copy. Junk alone is only
 * shown under Probably not needed and never waits (`decisions/2026/10/21/junk-alone-never-waits-for-review.md`). */
export const suggested = (n?: Note) => !!n && ((!!n.set && !n.best) || !!n.copy)
/** The key a Photo's choice is kept under, as shots.rs makes it: its Photos Section's or Photos
 * folder's id (`eventPlace`), `/`, its path there; that is the Section id and its path in the Section. */
export const photoKey = (i: Item, w: Work): string => keyIn(eventPlace(w), i.rel)
/** The choice made on a Photo; one made for another file at the same path (another size) is none. */
export function choiceOf(i: Item, w: Work, marks: Marks) {
  const c = marks.review?.[photoKey(i, w)]
  return c && (c.size == null || c.size === i.size) ? c : undefined
}
/** A suggested Photo with no choice yet. */
export const waiting = (i: Item, w: Work, notes: Notes, marks: Marks) => suggested(notes[i.path]) && !choiceOf(i, w, marks)
/** Whether a Photo waits for Review once Review again forgets its choice, as the counts will say:
 * only a suggested Photo waits; one Review does not suggest (a Best one) simply stays. */
export function waitsAgain(i: Item, w: Work, notes: Notes, marks: Marks) {
  const review = { ...marks.review }
  delete review[photoKey(i, w)]
  return waiting(i, w, notes, { ...marks, review })
}
/** Whether a step of Review shows this Photo, so a choice can be made on it there. */
export const inReview = (i: Item, w: Work, notes: Notes, marks: Marks) => stepsOf(w, notes, marks, true).some(s => choosable(s, i.path))
export const isSetAside = (i: Item, w: Work, marks: Marks) => choiceOf(i, w, marks)?.keep === false
const each = (works: Work[], f: (i: Item, w: Work) => boolean) => works.reduce((a, w) => a + w.items.reduce((b, i) => b + Number(f(i, w)), 0), 0)
export const toReview = (works: Work[], notes: Notes, marks: Marks) => each(works, (i, w) => waiting(i, w, notes, marks))
/** How many Photos are set aside: what the move script moves. */
export const asideIn = (works: Work[], marks: Marks) => each(works, (i, w) => isSetAside(i, w, marks))
/** "12 to review", "reviewed" or nothing, for a card, a month or a year. */
export function reviewState(works: Work[], notes: Notes, marks: Marks): { n: number; done: boolean } | null {
  const n = toReview(works, notes, marks)
  return n ? { n, done: false } : works.some(w => w.items.some(i => suggested(notes[i.path]))) ? { n: 0, done: true } : null
}

/** One step of Review: a set of Similar shots or a set of Copies, its Photos in time order.
 * `keep` holds the paths suggested to keep. `fixed` holds a Photo shown but chosen in its
 * own step, never here: the file a set of Copies is of when it is in a set of Similar shots or
 * in another event; `from` holds the event of the one from another event. */
export interface Step { work: Work; kind: 'similar' | 'copies'; items: Item[]; call: boolean; keep: Set<string>; fixed?: Set<string>; from?: Map<string, Work> }

/** The event a Photo of a step is in. */
export const workOf = (s: Step, i: Item) => s.from?.get(i.path) ?? s.work
/** Whether a choice on this Photo is made in this step. */
export const choosable = (s: Step, path: string) => s.items.some(i => i.path === path) && !s.fixed?.has(path)

/** Finds a Photo by its path in another event of the same Photos Section or Photos folder. */
export type Elsewhere = (path: string) => { i: Item; w: Work } | undefined

/** How sharp a Photo is against the others of a step of this kind. */
const sharpness = (kind: Step['kind'], notes: Notes, i: Item) => (kind === 'copies' ? notes[i.path]?.copy_detail : notes[i.path]?.detail)
/** The sharpest Photo of a step: the highest detail, the first when equal. */
const sharpest = (kind: Step['kind'], items: Item[], notes: Notes) => items.reduce((a, i) => ((sharpness(kind, notes, i) ?? 0) > (sharpness(kind, notes, a) ?? 0) ? i : a))

/** The steps of one event, in its order, each with a Photo still waiting; with `all`, every step,
 * to go through choices made before. With `elsewhere`, a set of Copies whose file is in another
 * event shows that file too, kept. */
export function stepsOf(w: Work, notes: Notes, marks: Marks, all = false, only?: (i: Item) => boolean, elsewhere?: Elsewhere): Step[] {
  // Each Photo is in one step only: a set of Similar shots with the copies of its Photos first,
  // then Copies.
  const out: Step[] = []
  const sets = new Map<string, Item[]>(), copies = new Map<string, Item[]>()
  for (const i of w.items) {
    const n = notes[i.path]
    if (n?.set) sets.set(n.set, [...(sets.get(n.set) ?? []), i])
  }
  const inSet = new Set([...sets.values()].flat().map(i => i.path))
  // A copy of a Photo in a set of this event (one sent on, saved again) joins that set's step, so
  // one moment is one step.
  const setOf = new Map([...sets].flatMap(([k, items]) => items.map(i => [i.path, k] as const)))
  for (const i of w.items) {
    const n = notes[i.path]
    if (!n?.copy || inSet.has(i.path)) continue
    const set = setOf.get(n.copy)
    if (set) sets.get(set)!.push(i)
    else copies.set(n.copy, [...(copies.get(n.copy) ?? []), i])
  }
  // A set starts with one kept: the Best one, or in a close call the sharpest (shots.rs marks it
  // Best too), so a set never opens with every Photo Not needed.
  for (const items of sets.values()) {
    const call = items.some(i => notes[i.path].call)
    const best = items.find(i => notes[i.path].best) ?? sharpest('similar', items, notes)
    out.push({ work: w, kind: 'similar', items, call, keep: new Set([best.path]) })
  }
  for (const [first, items] of copies) {
    // The file the copies are of is kept. It is chosen here when it is in this event and in no
    // set; in a set, or in another event, it shows beside its copies and is chosen there.
    const own = w.items.find(i => i.path === first)
    const there = own ? undefined : elsewhere?.(first)
    const file = own ?? there?.i
    const step: Step = { work: w, kind: 'copies', items: file ? [file, ...items] : items, call: false, keep: new Set(file ? [first] : []) }
    if (there) step.from = new Map([[there.i.path, there.w]])
    if (there || (own && inSet.has(own.path))) step.fixed = new Set([first])
    out.push(step)
  }
  const order = new Map(w.items.map((i, k) => [i.path, k]))
  const pos = (i: Item) => order.get(i.path) ?? -1
  // In time order, as taken, an unknown time last; the order in the event when they are equal.
  const byTime = (a: Item, b: Item) => (a.at ?? Infinity) - (b.at ?? Infinity) || pos(a) - pos(b)
  // With `only` (Photos only or Clips only), each step keeps the files of that kind.
  const kept = (only ? out.map(s => ({ ...s, items: s.items.filter(only) })).filter(s => s.items.length) : out)
    .map(s => ({ ...s, items: [...s.items].sort(byTime) }))
  // A step is placed by its first Photo of this event.
  const at = (s: Step) => Math.min(...s.items.map(pos).filter(k => k >= 0))
  // A Photo shown here but chosen in its own step does not make this one wait.
  return kept.filter(s => all || s.items.some(i => choosable(s, i.path) && waiting(i, w, notes, marks))).sort((a, b) => at(a) - at(b))
}

/** What a step keeps: each Photo's choice made before, else the suggestion, so a Photo put back
 * to Review shows beside the choices kept for the rest of its set. In a set chosen before,
 * a Photo with no choice that Review does not suggest (the pick of a your-call set) stays, as
 * Review again says. A step with no choice of its own that would keep nothing (the file of a set
 * of Copies set aside in its own step, or not found) keeps its sharpest Photo. */
export function keptIn(s: Step, notes: Notes, marks: Marks): Set<string> {
  const before = s.items.some(i => choiceOf(i, workOf(s, i), marks))
  const out = new Set(s.items.filter(i => choiceOf(i, workOf(s, i), marks)?.keep ?? (s.keep.has(i.path) || (before && !suggested(notes[i.path])))).map(i => i.path))
  const own = s.items.filter(i => !s.fixed?.has(i.path))
  if (!out.size && own.length && !own.some(i => choiceOf(i, workOf(s, i), marks))) out.add(sharpest(s.kind, own, notes).path)
  return out
}

/** Whether Next, and Enter, may go on with what a step keeps: only with one kept, so a whole set
 * is set aside only by its own button, Mark all not needed. */
export const canGoOn = (keep: Set<string>) => keep.size > 0

/** Each Photo's sharpness rank in its step, 1 the sharpest; Photos the app finds equally sharp
 * share one. A Photo with nothing measured has none. */
export function sharpRanks(s: Step, notes: Notes): Map<string, number> {
  const d = (i: Item) => sharpness(s.kind, notes, i)
  const values = [...new Set(s.items.map(d).filter(x => x != null))].sort((a, b) => b - a)
  return new Map(s.items.filter(i => d(i) != null).map(i => [i.path, values.indexOf(d(i)!) + 1]))
}

/** The steps Review goes through, and the one it opens at. Each step with a Photo still waiting,
 * or, with nothing waiting, every step to go through the choices made before. Opened from a Photo
 * (`start`), it opens at that Photo's set, which is among the steps even when chosen; `here`
 * says whether that set is a step at all (none when `only` leaves the Photo out). */
export function stepsAt(works: Work[], notes: Notes, marks: Marks, only?: (i: Item) => boolean, start?: Item, elsewhere?: Elsewhere): { steps: Step[]; at: number; here: boolean } {
  const left = works.flatMap(w => stepsOf(w, notes, marks, false, only, elsewhere))
  const all = works.flatMap(w => stepsOf(w, notes, marks, true, only, elsewhere))
  if (!start) return { steps: left.length ? left : all, at: 0, here: false }
  const here = all.find(s => choosable(s, start.path))
  const first = (s: Step) => `${s.work.id}\n${s.items[0].path}`
  const keys = new Set((left.length ? left : all).map(first))
  const steps = all.filter(s => s === here || keys.has(first(s)))
  return { steps, at: here ? steps.indexOf(here) : 0, here: !!here }
}

/** The Photos Section or Photos folder an event is in (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`), from its id: what is before `#`. */
export function placeOf(catalog: Catalog, w: Work): PhotoPlace | undefined {
  const id = eventPlace(w)
  const home = catalog.sections.filter(s => id === s.id || id.startsWith(inside(s.id))).sort((a, b) => b.id.length - a.id.length)[0]
  if (!home) return undefined
  const parts = id === home.id ? 0 : relIn(id, home.id).split('/').length
  const works = home.works.filter(x => x.event && eventPlace(x) === id)
  return { id, name: parts ? place(w.folder[parts - 1] ?? '') : home.name, home, under: w.folder.slice(0, parts), works }
}
