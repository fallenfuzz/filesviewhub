// What the Photos view shows (`decisions/2026/10/08/photos-is-a-section-kind-in-the-app.md`), as the Photos prototype does: photos and clips or one of
// them, Best only or All, and one filter on Review's suggestions. Every count, shade and list in
// the view reads it, so they all count files, never events.
import { keyOf, type Item, type Marks, type Thumb, type Work } from './api'
import { choiceOf, suggested, waiting, type Notes } from './review'

const CLIPS = new Set(['mp4', 'mov', 'm4v', '3gp', 'avi', 'mkv', 'webm', 'wmv', 'mpg', 'mpeg', 'ts', 'flv', 'vob', 'insv', 'lrv'])
export const isClip = (i: Item) => CLIPS.has(i.ext)
/** Photos and clips, photos only, or clips only. */
export type PhotoKind = '' | 'p' | 'c'
/** A filter on Review's suggestions: Your call, Similar shots, Copies, Probably not needed. */
export type PhotoFilter = '' | 'call' | 'sim' | 'cp' | 'nn'
export const FILTERS: PhotoFilter[] = ['call', 'sim', 'cp', 'nn']

/** A Photo's thumbnail: each Photo of an event with more than one has its own (thumbs.rs `grid`);
 * an event of one Photo has only the event's. */
export const photoThumbId = (w: Work, i: Item) => w.items.length > 1 ? `${w.id}#${keyOf(i)}` : w.id
export const photoThumb = (thumbs: Record<string, Thumb>, w: Work, i: Item) => thumbs[photoThumbId(w, i)]

/** The day panel's width: the one dragged to, or 44% of the window up to 640 px; never over 80%. */
export const panelWidth = (dragged?: number) => dragged ? `min(${dragged}px, 80vw)` : 'min(640px, 44vw)'

/** The day a Photo was taken, as 2019-03-21, by the camera's clock (read as UTC). */
export const dayKey = (t: number) => new Date(t * 1000).toISOString().slice(0, 10)

export class Lens {
  /** Sets of Similar shots the app could not judge (Your call): every shot in them stays in view. */
  readonly calls = new Set<string>()
  /** Files that Copies are copies of. */
  readonly firsts = new Set<string>()
  constructor(readonly notes: Notes, readonly marks: Marks, readonly k: PhotoKind, readonly all: boolean, readonly f: PhotoFilter) {
    for (const n of Object.values(notes)) {
      if (n.call && n.set) this.calls.add(n.set)
      if (n.copy) this.firsts.add(n.copy)
    }
  }
  kindOK = (i: Item) => !this.k || (this.k === 'c') === isClip(i)
  /** An event's Photos of the kind shown. */
  items = (w: Work) => w.items.filter(this.kindOK)
  /** What Best only leaves out: the extra Similar shots of a set with a Best one, Copies, Junk and
   * what was set aside in Review. A Photo kept in Review stays in view. */
  hidden = (i: Item, w: Work) => {
    const c = choiceOf(i, w, this.marks)
    if (c) return !c.keep
    const n = this.notes[i.path]
    if (!n) return false
    // A set comes first, as in Review's steps: its Best one stays, whatever else is said of it.
    if (n.set) return !n.best && !this.calls.has(n.set)
    return !!n.copy || !!n.junk
  }
  /** In the filter: the whole set, so there is something to compare. */
  inF = (i: Item) => {
    const n = this.notes[i.path]
    switch (this.f) {
      case 'call': return !!n?.set && this.calls.has(n.set)
      case 'sim': return !!n?.set
      case 'cp': return !n?.set && (!!n?.copy || this.firsts.has(i.path))
      case 'nn': return !n?.set && !!n?.junk
      default: return true
    }
  }
  /** What the filter counts: the extras of each set, the copies, the Junk. */
  extra = (i: Item) => {
    const n = this.notes[i.path]
    switch (this.f) {
      case 'call': return !!n?.set && !n.best && this.calls.has(n.set)
      case 'sim': return !!n?.set && !n.best
      case 'cp': return !n?.set && !!n?.copy
      case 'nn': return !n?.set && !!n?.junk
      default: return true
    }
  }
  /** The Photos an event shows: the filter's extras, or what Best only or All shows. */
  visible = (w: Work) => this.items(w).filter(i => this.f ? this.extra(i) : this.all || !this.hidden(i, w))
  /** An event is shown when it has Photos of the kind and, with a filter, something it counts. */
  shown = (w: Work) => this.f ? this.items(w).some(this.extra) : this.items(w).length > 0
  /** A bare number for tight places: what is shown. */
  num = (ws: Work[]) => ws.reduce((a, w) => a + this.visible(w).length, 0)
  /** Photos, clips and how many Best only hides, of some events or of some Photos of one. */
  count(ws: Work[], only?: (i: Item) => boolean) {
    let photos = 0, clips = 0, hidden = 0
    for (const w of ws) for (const i of this.items(w)) {
      if (only && !only(i)) continue
      if (this.f) { if (this.extra(i)) photos++; continue }
      if (!this.all && this.hidden(i, w)) { hidden++; continue }
      if (isClip(i)) clips++; else photos++
    }
    return { photos, clips, hidden }
  }
  /** Files of the kind shown waiting for a Review choice; Review goes through the same kind (`kindOK`). */
  toReview = (ws: Work[], only?: (i: Item) => boolean) =>
    ws.reduce((a, w) => a + this.items(w).filter(i => (!only || only(i)) && waiting(i, w, this.notes, this.marks)).length, 0)
  /** Whether any of them had a suggestion at all: "✓ reviewed" then means something. */
  hadReview = (ws: Work[], only?: (i: Item) => boolean) => ws.some(w => this.items(w).some(i => (!only || only(i)) && suggested(this.notes[i.path])))
  /** What Review goes through: everything, or the kind shown. */
  get reviewOnly() { return this.k ? this.kindOK : undefined }
}
