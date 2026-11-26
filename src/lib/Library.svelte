<script lang="ts" module>
  export type View =
    | { t: 'next' } | { t: 'progress' } | { t: 'done' } | { t: 'added' } | { t: 'opened' }
    /** Files not on a card: of every Section, or of Section `id` in `folder` and below. */
    | { t: 'unlisted'; id?: string; folder?: string[] }
    | { t: 'section'; id: string; folder: string[]; all?: boolean
        /** In Photos, the day open in the day panel, as 2019-03-21, or `undated`. */
        day?: string }
  // A Books Section's card shows each title with its author apart; the names met across the Section
  // tell which side of "A - B" is the author.
  // A book outside a Books Section reads only "Title - Author" and "Title (Author)".
  const authors = new WeakMap<Section, Authors>(), after = new WeakMap<Section, Authors>()
  function authorsOf(s: Section, books = true) {
    const m = books ? authors : after
    if (!m.has(s)) m.set(s, authorsIn(s.works.map(w => w.title), !books))
    return m.get(s)!
  }
</script>

<script lang="ts">
  // The library as the Hub lays it out (direction A, folder columns made native): Library views
  // and Sections by Drive, a folder column, and the cards. Medium and Mark filters and search
  // narrow any view; Back and Forward walk the views seen (`decisions/2026/09/29/folder-shows-own-works-and-one-box-shape.md`, `decisions/2026/09/30/hub-lists-every-file.md`, `decisions/2026/10/05/checked-folders-on-any-drive.md`).
  import { tick, untrack } from 'svelte'
  import { SvelteSet } from 'svelte/reactivity'
  import type { Catalog, Detail, Item, KindPreview, LastMove, Marks, MoveScope, PhotoPlace, Reason, Section, Suggestion, Tab, Thumb, Unlisted, Work, CoverTarget } from './api'
  import { showError, showToast } from './toast.svelte'
  import { api, COLORS, folderCoverId, doneWord, isBook, eventPlace, glued, keyIn, natural, relIn, place, shownOf, size, tickable, tickKeys, keptOf, ticksOf } from './api'
  import { KINDS, S } from './strings'
  import { saveUi, ui } from './ui.svelte'
  import { LEVELS, picksFor, setViews, viewOf } from './views'
  import { byPlace, matchesFile, score, words } from './search'
  import { firstWay, readSort, sortWorks, writeSort, type Sort, type SortBy } from './sort'
  import Card from './Card.svelte'
  import FolderTile from './FolderTile.svelte'
  import Photos from './Photos.svelte'
  import { FILTERS, Lens, panelWidth, type PhotoFilter } from './photos'
  import Grip from './Grip.svelte'
  import Icon from './Icon.svelte'
  import { asideIn, reviewState, type Notes } from './review'
  import { authorsIn, type Authors } from './byline'
  import { closeMenu, menuOpen, openMenu, type MenuItem } from './menus.svelte'
  import { diskOf as diskRel, folderKinds, homeOf, homesOf, kindAt, loadFolders, othersOf, offerable, ownEvent, ownOf, photoFolderOf, relsOf, viewFrom } from './folders.svelte'
  import { sectionOf } from './orphans'
  import { Tree, type Sub } from './tree'

  let { catalog, marks, thumbs, details, notes, shotsLeft, onreview, query = $bindable(), reading, onopen, onmark, onmarkmany, onfile, onclearopened, onreveal, onkind, onfolder, onreread, onrethumb, onchoosecover, onremovecover, onsettings, onforget, onwork, onmove, onputback, onoffer, onagain }: {
    catalog: Catalog; marks: Marks; thumbs: Record<string, Thumb>; details: Record<string, Detail>; query: string; reading: boolean
    onopen: (w: Work, s: Section) => void; onmark: (w: Work, m: 'next' | 'done' | null) => void
    /** One Mark on several cards, with one Undo. */
    onmarkmany: (ws: Work[], m: 'next' | 'done' | null) => void
    /** Opens a file; `w`, the Work it is in, goes first on Recently opened. */
    onfile: (path: string, w?: Work) => void
    /** Empties Recently opened, once the person agrees. */
    onclearopened: () => void
    onreveal: (path: string, open?: boolean) => void; onkind: (s: Section) => void
    /** Folder settings of folder `f` (titles) of a checked folder's Section, at path `rel` in it (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`). */
    onfolder: (s: Section, f: string[], rel: string) => void
    /** Reads a Section again, or a folder in it; null reads every checked folder. */
    onreread: (s: Section | null, under: string[]) => void
    onrethumb: (ids: string[]) => void; onchoosecover: (t: CoverTarget) => void; onremovecover: (ids: string[]) => Promise<void>; onsettings: (tab: Tab) => void; onforget: (s: Section) => void
    /** Opens the dialog of the Work with this id. */
    onwork: (id: string) => void
    /** Review in Photos (`decisions/2026/10/04/photos-take-no-marks-review-is-the-progress.md`): its notes, pictures still to read, and Review of some events. */
    notes: Notes; shotsLeft: number; onreview: (works: Work[], label: string, only?: (i: Item) => boolean, start?: Item) => void
    /** Opens Review's Move for a Photos Section or Photos folder (`decisions/2026/10/09/review-moves-set-aside-photos-itself-with-undo.md`, `decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`). */
    /** Opens Move for a Photos Section or Photos folder, from the year or month shown. */
    onmove: (s: PhotoPlace, scope?: MoveScope) => void
    /** Puts a Split-out Section back into the Section it came from (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`). */
    onputback: (s: Section) => void
    /** The offer of a kind per folder, answered: a kind for each folder, a Section of each, or put away (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`). */
    onoffer: (s: Section, list: Suggestion[], how: 'kinds' | 'sections' | 'away') => Promise<void>
    /** Forgets a photo's Review choice, so it waits for Review again. */
    onagain: (w: Work, i: Item) => void
  } = $props()

  const COL1 = 236, COL2 = 240, CHUNK = 120
  const MEDIA = ['Watching', 'Reading', 'Listening']

  /** A view as it was left: Back and Forward bring back its scroll and how many cards were shown. */
  /** `anchor`: the first card in sight, as its place among the cards of its folder row (`row`, that
   * row's `data-row`) or among the cards outside the rows, and how far its top is below the pane's. */
  type Visit = { v: View; scroll: number; limit: number; anchor?: { row?: string; j: number; dy: number } }
  let view = $state<View>({ t: 'next' })
  let back = $state<Visit[]>([]), fwd = $state<Visit[]>([])
  let medium = $state(''), markF = $state<'' | 'next' | 'done'>('')
  /** The Unlisted view shows only files with this reason. */
  let reason = $state<Reason | ''>('')
  let limit = $state(CHUNK)
  /** In Photos, one filter on Review's suggestions; what else it shows is kept in settings. */
  let photoF = $state<PhotoFilter>('')
  const lens = $derived(new Lens(notes, marks, ui.photoKind, ui.photoAll, photoF))
  let photoWant = $state<string[]>([])
  /** Works marked while this view is open stay in it until the view changes, at the place they had. */
  let sticky = $state(new Set<string>())
  const pinned = new Map<string, number>()
  let main: HTMLElement
  let narrow = $state(false)
  // With the columns folded, a rail takes their place: Home, a button per Section and, at
  // its foot, Folders, which opens the columns over the page. They stay open while folders are
  // clicked through and close on Folders again, Esc or a click on the cards (not their scroll bar).
  // Pointing opens nothing.
  let peek = $state(false)
  let colsBtn = $state<HTMLButtonElement>()
  let rail = $state<HTMLElement>()
  /** Esc on a rail button pressed with the mouse would draw a focus ring on it, and Esc that closes
   * the columns moves focus to Folders: neither leaves a ring, until focus moves on by Tab. */
  let quiet = $state(false)
  let cols: HTMLDivElement
  /** A rail label past two lines is cut to what fits, measured in bold, and ends in "…". Chrome puts
   * the line clamp's ellipsis after a centred line without making room for it, so it was cut off. */
  function railFit(el: HTMLElement, label: string) {
    const shown = el.firstElementChild as HTMLElement
    const fits = () => shown.scrollHeight <= shown.clientHeight + 1 && shown.scrollWidth <= shown.clientWidth + 1
    const set = (t: string) => { shown.textContent = t; el.dataset.l = t }
    const fit = () => {
      set(label)
      shown.style.fontWeight = '600'
      if (!fits()) {
        const parts = Array.from(new Intl.Segmenter(undefined, { granularity: 'grapheme' }).segment(label), g => g.segment)
        const cut = (n: number) => parts.slice(0, n).join('').trimEnd() + '…'
        let lo = 0, hi = parts.length - 1
        while (lo < hi) { const mid = (lo + hi + 1) >> 1; set(cut(mid)); if (fits()) lo = mid; else hi = mid - 1 }
        set(cut(lo))
      }
      shown.style.fontWeight = ''
    }
    fit()
    document.fonts.ready.then(() => fit())
    return { update(next: string) { label = next; fit() } }
  }

  // Each card and tile sits in a `.cell` (for its menu) that draws no box of its own.
  const CARDS = '.grid > .cell > :first-child, .rows > .cell > :first-child'
  const cardsOf = (el: HTMLElement | undefined = main) => [...(el?.querySelectorAll<HTMLElement>(CARDS) ?? [])]
  /** The cards outside the folder rows, whose tiles and cards are made only near the view. */
  const ownCards = () => cardsOf().filter(c => !c.closest('.exp'))
  const rowEl = (key: string) => [...(main?.querySelectorAll<HTMLElement>('.exp[data-row]') ?? [])].find(e => e.dataset.row === key)
  /** The view now, with the first card in sight and how far its top is below the pane's. */
  function here(): Visit {
    const top = main?.getBoundingClientRect().top ?? 0
    // Halved, not walked: a card read in a folder row out of sight lays that row out, and Expand
    // all opens 2,000 of them.
    const cards = cardsOf()
    let lo = 0, hi = cards.length
    while (lo < hi) { const m = (lo + hi) >> 1; if (cards[m].getBoundingClientRect().bottom > top) hi = m; else lo = m + 1 }
    const c = cards[lo] as HTMLElement | undefined, row = c?.closest<HTMLElement>('.exp[data-row]')?.dataset.row
    const anchor = c && { row, j: (row ? cardsOf(rowEl(row)) : ownCards()).indexOf(c), dy: c.getBoundingClientRect().top - top }
    return { v: view, scroll: main?.scrollTop ?? 0, limit, anchor }
  }
  /** Cards off screen are drawn at an estimated height, so the place is set again for a few
   * frames until the cards above have their real size. */
  function restore(x: Visit) {
    let n = 0
    // The anchor's row makes its cards at once, rather than when it comes near the view.
    if (x.anchor?.row && x.scroll) near.add(x.anchor.row)
    const step = () => {
      if (!main) return
      const a = x.scroll ? x.anchor : undefined
      const c = a ? (a.row ? cardsOf(rowEl(a.row)) : ownCards())[a.j] : undefined
      if (c) main.scrollTop += c.getBoundingClientRect().top - main.getBoundingClientRect().top - x.anchor!.dy
      else main.scrollTo(0, x.scroll)
      if (n++ < 8) requestAnimationFrame(step)
    }
    requestAnimationFrame(step)
  }
  export function go(v: View) {
    back = [...back, here()]; fwd = []
    show({ v, scroll: 0, limit: CHUNK })
  }
  function show(x: Visit) {
    // A Photos filter belongs to its Section's view: leaving the Section clears it.
    if (x.v.t !== 'section' || view.t !== 'section' || x.v.id !== view.id) photoF = ''
    // Lists by Mark hide the Mark filter, so Back and Forward into one clear it, as its row does.
    if (x.v.t === 'next' || x.v.t === 'progress' || x.v.t === 'done' || x.v.t === 'unlisted') markF = ''
    // A reason picked in one list of files not on a card is not carried to another scope.
    const key = (v: View) => v.t === 'unlisted' ? [v.id ?? '', ...v.folder ?? []].join('\n') : null
    if (key(x.v) === null || key(x.v) !== key(view)) reason = ''
    view = x.v; limit = x.limit; sticky = new Set(); pinned.clear(); sel = new Set(); selFrom = ''; selSeen = false
    tick().then(() => restore(x))
  }
  export function goBack() { if (back.length) { fwd = [here(), ...fwd]; show(back.at(-1)!); back = back.slice(0, -1) } }
  export function goForward() { if (fwd.length) { back = [...back, here()]; show(fwd[0]); fwd = fwd.slice(1) } }
  /** A list from the library column. As in the Hub, it shows that list: the search is cleared,
   * and on Next up, In progress and Done the Mark filter too. */
  function list(t: 'next' | 'progress' | 'done' | 'added' | 'opened' | 'unlisted') {
    query = ''
    if (t !== 'added' && t !== 'opened') markF = ''
    go({ t })
  }
  export const home = () => { medium = ''; list('next') }
  /** Home's tabs: the lists by Mark, the newest Works and those opened last, under one rail button. */
  const TABS = [['next', S.nextUp], ['progress', S.inProgress], ['done', S.done], ['added', S.recentlyAdded], ['opened', S.recentlyOpened]] as const
  /** When a Work, or a copy of it on another drive, was last opened from the app; 0 for never. */
  function openedAt(w: Work) {
    const o = marks.opened
    if (!o) return 0
    let t = o[w.id] ?? 0
    for (const c of w.copies ?? []) t = Math.max(t, o[c] ?? 0)
    return t
  }
  const onHome = $derived(TABS.some(([t]) => t === view.t))

  // Merged Sections show as one (`decisions/2026/10/05/checked-folders-on-any-drive.md`): the Works and files of each, under the first one's name.
  const sections = $derived.by(() => {
    const into = (s: Section) => ui.merge[s.id] && catalog.sections.some(t => t.id === ui.merge[s.id] && !ui.merge[t.id]) ? ui.merge[s.id] : ''
    return catalog.sections.filter(s => !into(s)).map(s => {
      const more = catalog.sections.filter(m => into(m) === s.id)
      return more.length ? { ...s, online: s.online || more.some(m => m.online), works: joined([s, ...more]),
        unlisted: [...s.unlisted ?? [], ...more.flatMap(m => m.unlisted ?? [])], links: [...s.links ?? [], ...more.flatMap(m => m.links ?? [])] } : s
    })
  })
  /** The Works of merged Sections, the first one's first. Copies of one Work, at the same place
   * with the same size, are one card on both drives. It keeps the id of the copy with the newest
   * Mark, else the first Section's, so a Mark or Tick set on the card lands where it did; it shows
   * the Ticks of every copy, and opens the copy on a connected drive. A different
   * file at the same place stays a card of its own, with its drive shown. */
  function joined(home: Section[]): Work[] {
    const at = new Map<string, { w: Work; s: Section }[]>()
    for (const s of home) for (const w of s.works) {
      const k = relIn(w.id, s.id)
      at.set(k, [...(at.get(k) ?? []), { w, s }])
    }
    const out: Work[] = []
    for (const here of at.values()) {
      if (here.length === 1) { out.push(here[0].w); continue }
      const bySize = new Map<number, typeof here>()
      for (const x of here) bySize.set(x.w.size, [...(bySize.get(x.w.size) ?? []), x])
      for (const c of bySize.values()) {
        const drives = [...new Set(c.map(x => x.s.drive))]
        if (c.length === 1) { out.push({ ...c[0].w, drives }); continue }
        const id = keptOf(c.map(x => x.w.id), marks)
        const keep = c.find(x => x.w.id === id)!
        const open = keep.s.online ? keep : c.find(x => x.s.online) ?? keep
        out.push({ ...open.w, id: keep.w.id, drives, copies: c.filter(x => x !== keep).map(x => x.w.id) })
      }
    }
    return out.sort(byPlace)
  }
  /** A Work's thumbnail, or a copy's when its own is not made (its drive was never connected). */
  const thumbOf = (w: Work) => thumbs[w.id] ?? w.copies?.map(c => thumbs[c]).find(Boolean)
  /** Choose a cover… for a Work and its copies, or for a folder tile (`decisions/2026/10/22/covers-are-chosen-never-fetched-by-themselves.md`). */
  const coverOfWork = (w: Work): CoverTarget => ({ ids: [w.id, ...w.copies ?? []], name: w.title })
  const coverOfFolder = (s: Section, f: string[]): CoverTarget => ({ ids: [folderCoverId(s.id, f)], name: f.length ? place(f.at(-1)!) : s.name })
  const color = (s: Section) => COLORS[catalog.sections.findIndex(x => x.id === s.id) % COLORS.length]
  const all = $derived(sections.flatMap(s => s.works.map(w => ({ w, s }))))
  /** Each Work's place in reading order, Section by Section: the tiebreak of a search. */
  const rank = $derived(new Map(all.map(({ w }, i) => [w, i])))
  const unlistedAll = $derived(sections.flatMap(s => (s.unlisted ?? []).map(u => ({ u, s }))))

  $effect(() => {
    const v = view
    if (v.t === 'section' && !sections.some(s => s.id === v.id)) view = { t: 'next' }
    if (v.t === 'unlisted' && v.id && !sections.some(s => s.id === v.id)) view = { t: 'unlisted' }
  })

  const byDrive = $derived.by(() => {
    const m = new Map<string, Section[]>()
    for (const s of sections) m.set(s.drive, [...(m.get(s.drive) ?? []), s])
    return [...m]
  })
  /** The Sections in the library column's order, by drive: the rail, Your folders and the address bar's menu follow it. */
  const inOrder = $derived(byDrive.flatMap(([, secs]) => secs))
  const section = $derived(view.t === 'section' ? sections.find(s => s.id === (view as { id: string }).id) : undefined)
  const folder = $derived(view.t === 'section' ? view.folder : [])
  const q = $derived(query.trim())
  const ws = $derived(words(q))
  const filtered = $derived(!!(q || medium || markF))

  const under = (w: Work, f: string[]) => f.every((x, i) => w.folder[i] === x)
  const passes = (w: Work) => (!medium || w.media.includes(medium)) && (!markF || marks.marks[w.id]?.m === markF || sticky.has(w.id))
  /** In progress: a Work with some of its items ticked and no Done mark; one with every item
   * ticked stays until it is marked Done. The time of its latest Tick, or -1 when it is not in
   * progress; `done` false leaves the Done mark out. */
  function lastTick(w: Work, done = true) {
    const t = ticksOf(w, marks)
    if (!t || (done && marks.marks[w.id]?.m === 'done') || !tickable(w)) return -1
    const on = tickKeys(w).filter(k => k in t)
    return on.length ? Math.max(...on.map(k => t[k] ?? 0)) : -1
  }
  /** A Photos Section's Folders with other things (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`). */
  const others = $derived(section ? othersOf(section) : new Set<string>())
  /** In a Photos Section, a folder with another kind or split out as a Section is open, or one
   * inside it: it shows as folders do, not in the calendar (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`). */
  const inOther = $derived(folder.length > 0 && others.has(folder[0]))
  /** A Photos Section's own calendar is open: its years, months and Events. */
  const calTop = $derived(section?.kind === 'Photos' && !inOther)
  // The calendar's counts and Review are of its Events; a search there finds the other folders' Works too.
  const below = $derived.by(() => {
    if (!section) return []
    const all = folder.length ? tree?.at(folder)?.every ?? [] : section.works
    return calTop && !q ? all.filter(ownEvent) : all
  })
  /** The Photos folder open, or the one whose year or month is open (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`): from there down
   * the page is as a Photos Section's. */
  const photoRoot = $derived(section && view.t === 'section' ? photoFolderOf(section, folder) : undefined)
  /** A year, a month or Undated of a Photos folder is open: no folder on disk. */
  const inCalendar = $derived(!!photoRoot && folder.length > photoRoot.length)
  /** The open folder from the Photos Section's top, or from the Photos folder's. */
  const pfolder = $derived(photoRoot ? folder.slice(photoRoot.length) : folder)
  // A folder that is no folder after a read (a kind made it one card, or it is gone from the disk)
  // gives way to the nearest folder above it that still is one, in place (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`).
  $effect(() => {
    const v = view, s = section
    // A year, a month or Undated of a Photos Section stays; a folder that left its other kind gives way.
    if (v.t !== 'section' || !v.folder.length || !s || (s.kind === 'Photos' && !othersOf(s).has(v.folder[0]) && s.works.some(w => ownEvent(w) && w.folder[0] === v.folder[0]))) return
    const t = tree
    const holds = (f: string[]) => { const n = t?.at(f); return (!!n && (n.every.length > 0 || n.hasFiles))
      || (s.links ?? []).some(l => l.folder.length > f.length && f.every((x, i) => l.folder[i] === x)) }
    if (holds(v.folder)) return
    let f = v.folder.slice(0, -1)
    while (f.length && !holds(f)) f = f.slice(0, -1)
    view = { ...v, folder: f, all: undefined }
  })
  /** The Sections split out of the open Section's folders (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`), each shown in its folder's
   * place: one gone from its drive or shown inside another Section has no place to open. */
  const linksHere = $derived((section?.links ?? []).flatMap(l => {
    // One shown inside another Section links to that one, where its cards are.
    const to = sections.find(x => x.id === l.id) ?? sections.find(x => x.id === ui.merge[l.id])
    return to && !to.gone && to.id !== section!.id ? [{ l, to }] : []
  }))
  /** The open Section's folders, made once per Section; the folders in each, once per filter. */
  const tree = $derived(section ? new Tree(section.works, section.unlisted ?? [], linksHere) : undefined)
  const ok = $derived(filtered && section ? new Set(section.works.filter(w => passes(w) && score(w, ws) > 0)) : null)
  const lister = $derived(tree?.lister(ui.unlisted, ok))
  /** The folders in folder `at`, in reading order (`Sub`). */
  const subsOf = (at: string[]): Sub[] => lister?.(at) ?? []
  const subfolders = $derived(subsOf(folder))
  // Folder columns, as the Hub's: one per level of the open folder that has folders inside,
  // stopping at the first without. A folder with none adds no column, so it is only selected in
  // the column above. A level with one folder adds none either: there is nothing beside it to
  // move to, and it shows as a row and in the address bar (`decisions/2026/10/08/a-folder-shows-its-subfolders-as-tiles.md`, `decisions/2026/10/14/a-folder-shows-its-subfolders-as-rows-that-open.md`). The last two columns show; once the folder columns are two, they replace
  // the library column and the first one starts with Next up and Back.
  type Col = { key: string; title: string; at: string[]; list: Sub[] }
  const folderCols = $derived.by((): Col[] => {
    if (!section) return []
    const out: Col[] = []
    for (let d = 0; d <= folder.length; d++) {
      const list = d === folder.length ? subfolders : subsOf(folder.slice(0, d))
      if (!list.length) break
      if (list.length === 1) continue
      out.push({ key: d ? `folders${d + 1}` : 'folders', title: d ? folder[d - 1] : section.name, at: folder.slice(0, d), list })
    }
    return out
  })
  /** A column's folders: in a Photos Section's first column its years, then its Folders with
   * other things, shown under a heading of their own (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`). */
  function colLists(c: Col) {
    if (c.at.length || !others.size) return [c.list]
    return [c.list.filter(f => !others.has(f.name)), c.list.filter(f => others.has(f.name))]
  }
  const deep = $derived(folderCols.length >= 2)
  const shownCols = $derived(folderCols.slice(-2))
  const widths = $derived([...(deep ? [] : [ui.colw.lib ?? COL1]), ...shownCols.map(c => ui.colw[c.key] ?? COL2)].map(w => `${w}px`).join(' '))
  $effect(() => {
    const mq = window.matchMedia('(max-width: 999px)')
    narrow = mq.matches
    const f = (e: MediaQueryListEvent) => (narrow = e.matches)
    mq.addEventListener('change', f)
    return () => mq.removeEventListener('change', f)
  })
  // A folder shows its subfolders as rows that open in place, then its own Works (`decisions/2026/10/14/a-folder-shows-its-subfolders-as-rows-that-open.md`);
  // with a filter on, only the rows with a match. Show all, the Section's Everything below
  // or a search lists every Work below.
  /** The open folder in its checked folder's Section, for its own kind and views (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`);
   * none at a Section's top and in Photos, whose years and months are no folders. */
  const spot = $derived(section && folder.length && !calTop && !inCalendar ? homeOf(catalog, section, folder) : undefined)
  /** What the open folder shows inside: its own view, the nearest folder's above, or the Section's. */
  const folderFrom = $derived(section && spot ? viewFrom(spot.home, spot.rel, 'folder', folder, true, kindAt(spot.home, spot.rel), section) : undefined)
  const folderView = $derived(!section ? '' : folderFrom ? folderFrom.view : viewOf(section, 'folder'))
  const flat = $derived(folderView === 'flat')
  /** The Folder tiles view: subfolders as tiles, as Home's Your folders. */
  const folderTiles = $derived(folderView === 'tiles')
  const everyBelow = $derived(view.t === 'section' && (!!view.all || flat || !!q))
  const own = $derived(subfolders.length && !everyBelow ? below.filter(w => w.folder.length === folder.length) : below)
  const tiles = $derived(view.t === 'section' && subfolders.length && !everyBelow ? subfolders.filter(t => !filtered || t.works.length) : [])
  /** A row's key in `ui.open`: its Section and folder path. Rows start closed. */
  const okey = (s: Section, f: string[]) => [s.id, ...f].join('/')
  const isOpen = (s: Section, f: string[]) => ui.open[okey(s, f)] === true
  function setOpen(s: Section, fs: string[][], on: boolean) {
    for (const f of fs) { if (on) ui.open[okey(s, f)] = true; else delete ui.open[okey(s, f)] }
    if (on) {
      for (const f of fs) fresh.add(f.join('/'))
      setTimeout(() => { for (const f of fs) fresh.delete(f.join('/')) }, 400)
    }
    saveUi()
  }
  /** Rows opened a moment ago: only they unfold, not one whose tiles are made as it comes near. */
  const fresh = new SvelteSet<string>()
  /** Each row, and for an open one its subfolders and its own Works that pass the filters. */
  const rows = $derived(tiles.map(t => {
    const f = [...folder, t.name], open = !t.link && !!section && isOpen(section, f)
    return { t, f, open,
      subs: open ? subsOf(f).filter(u => !filtered || u.works.length) : [],
      mine: open ? sorted((tree?.at(f)?.own ?? []).filter(passes), w => w) : [],
      files: open && !t.every.length ? tree?.at(f)?.files ?? [] : [] }
  }))
  const ROW_CARDS = 12
  // A row out of sight is not laid out; it counts as high as one row open in sight, with as
  // many lines of tiles and cards, so End and the scroll bar reach the last row after Expand all.
  // Not `auto`: a row closed out of sight would keep the height it had open.
  let rowSize = $state<{ base: number; line: number; cols: number; more: number } | null>(null)
  let paneW = $state(0)
  function measureRow() {
    const g = main?.querySelector<HTMLElement>('.exp.open > .expb > .grid, .exp.open > .expb > .rows')
    if (!g || !g.children.length) return
    const cs = getComputedStyle(g), gap = parseFloat(cs.rowGap) || 0
    const cols = cs.display === 'grid' ? cs.gridTemplateColumns.split(' ').length : 1
    const lines = Math.ceil(g.children.length / cols), h = g.offsetHeight
    // "Open the folder for all": counted only for the rows that show it.
    const ln = g.parentElement!.querySelector<HTMLElement>(':scope > .moreln'), more = ln ? ln.offsetHeight + 16 : rowSize?.more ?? 37
    const next = { base: g.closest<HTMLElement>('.exp')!.offsetHeight - h - gap - (ln ? more : 0), line: (h + gap) / lines, cols, more }
    if (JSON.stringify(next) !== JSON.stringify(rowSize)) rowSize = next
  }
  $effect(() => { void rows; void ui.size; void tall; void paneW; tick().then(measureRow) })
  $effect(() => {
    if (!main) return
    const ro = new ResizeObserver(() => (paneW = main.clientWidth))
    ro.observe(main)
    return () => ro.disconnect()
  })
  /** The open rows that came near the view: only their tiles and cards are made, as a Section
   * left with 2,000 rows open made 24,000 of them at once. Made again for each place. */
  const near = new SvelteSet<string>()
  let nearIo: IntersectionObserver | undefined
  // Observed again after the set is cleared: a row already in sight is not reported again otherwise.
  $effect(() => {
    void section?.id; void folder
    untrack(() => near.clear())
    tick().then(() => { for (const e of main?.querySelectorAll<HTMLElement>('.exp[data-row]') ?? []) { nearIo?.unobserve(e); nearIo?.observe(e) } })
  })
  $effect(() => () => nearIo?.disconnect())
  function nearby(el: HTMLElement) {
    tick().then(() => {
      if (!el.isConnected) return
      if (!main) { near.add(el.dataset.row!); return }
      nearIo ??= new IntersectionObserver(es => { for (const e of es) if (e.isIntersecting) near.add((e.target as HTMLElement).dataset.row!) }, { root: main, rootMargin: '2000px 0px' })
      nearIo.observe(el)
    })
    return { destroy() { nearIo?.unobserve(el) } }
  }
  /** The height a row out of sight counts as. */
  function rowGuess(r: (typeof rows)[number]) {
    const n = r.subs.length + Math.min(r.mine.length, ROW_CARDS)
    return r.open && rowSize && n ? Math.round(rowSize.base + Math.ceil(n / rowSize.cols) * rowSize.line + (r.mine.length > ROW_CARDS ? rowSize.more : 0)) : 62
  }
  /** Opens or closes every row shown. */
  const setAll = (on: boolean) => section && setOpen(section, rows.map(r => r.f), on)
  /** Folder `f` of the open Section shows events (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`): it is in Photos, or a Photos folder;
   * `sub` is what its folders are, as a tile says it. */
  function evAt(f: string[]): { ev: boolean; sub: string } {
    const root = ev ? photoRoot ?? [] : section ? photoFolderOf(section, f) : undefined
    return !ev && !root ? { ev: false, sub: 'folder' } : { ev: true, sub: f.length > (root?.length ?? 0) ? 'month' : 'year' }
  }
  /** A row's line: its Works (of how many, with a filter on), its folders (those with a match) and its Marks. */
  function rowLine(t: Sub) {
    // A Photos Section's count elsewhere is its photos: its link says how many of its own Events.
    if (t.link) return S.linkLine(t.link.kind === 'Photos' ? S.works(tileWorks(t.link).length, true) : S.works(countOf(t.link)))
    const d1 = folder.length + 1
    const subs = filtered ? new Set(t.works.filter(w => w.folder.length > d1).map(w => w.folder[d1])).size : t.subs
    const n = t.every.filter(w => marks.marks[w.id]?.m === 'next').length, d = t.every.filter(w => marks.marks[w.id]?.m === 'done').length
    // A Photos folder's years are no subfolders on disk.
    const e = evAt([...folder, t.name]).ev
    return [filtered ? S.someOf(t.works.length, t.every.length, e) : S.works(t.every.length, e), subs && !e ? S.subfolders(subs) : '',
      n ? S.nNext(n) : '', d ? S.nDone(d) : ''].filter(Boolean).join(' · ')
  }

  type Shown = { w: Work; s: Section }
  const listed = $derived.by((): Shown[] => {
    // Search runs inside the folder shown, or everywhere from the lists.
    let base: Shown[]
    if (view.t === 'unlisted') return []
    if (view.t === 'section') base = section ? own.map(w => ({ w, s: section })) : []
    else if (q) base = all
    else if (view.t === 'next' || view.t === 'done') {
      const m = view.t
      base = all.filter(({ w }) => marks.marks[w.id]?.m === m || sticky.has(w.id))
        .map(x => ({ x, t: pinned.get(x.w.id) ?? marks.marks[x.w.id]?.at ?? 0 })).sort((a, b) => b.t - a.t).map(r => r.x)
    } else if (view.t === 'progress') {
      base = all.map(x => ({ x, t: pinned.get(x.w.id) ?? lastTick(x.w) })).filter(r => r.t >= 0 || sticky.has(r.x.w.id))
        .sort((a, b) => b.t - a.t).map(r => r.x)
    } else if (view.t === 'added') base = [...all].sort((a, b) => b.w.added - a.w.added).slice(0, 60)
    else if (view.t === 'opened') base = all.map(x => ({ x, t: openedAt(x.w) })).filter(r => r.t > 0).sort((a, b) => b.t - a.t).slice(0, 60).map(r => r.x)
    else base = []
    const f = base.filter(({ w }) => passes(w))
    if (!q) return sorted(f, x => x.w)
    return f.map(x => ({ x, r: score(x.w, ws) })).filter(r => r.r > 0)
      .sort((a, b) => b.r - a.r || rank.get(a.x.w)! - rank.get(b.x.w)!).map(r => r.x)
  })
  const shown = $derived(listed.slice(0, limit))
  // Unlisted files: their own view, of every Section or of one Section or folder, and in place
  // with View's switch (`decisions/2026/09/30/hub-lists-every-file.md`).
  const scope = $derived.by(() => {
    const v = view
    const s = v.t === 'unlisted' && v.id ? sections.find(x => x.id === v.id) : undefined
    return s ? { s, f: (v as { folder?: string[] }).folder ?? [] } : undefined
  })
  const unlistedFound = $derived(view.t === 'unlisted'
    ? (scope ? unlistedIn(scope.s, scope.f).map(u => ({ u, s: scope.s })) : unlistedAll).filter(({ u }) => !ws.length || matchesFile(u, ws)) : [])
  const REASONS = Object.keys(S.reason) as Reason[]
  const byReason = $derived(REASONS.map(r => ({ r, n: unlistedFound.filter(({ u }) => u.reason === r).length })).filter(x => x.n || x.r === reason))
  /** Where a file not on a card is: its folder below the list's Section or folder, that one itself
   * when the file is in it, else the Section and the whole path. */
  function whereOf(u: Unlisted, s: Section) {
    const sc = scope
    if (!sc) return [glued(s.name), ...u.folder.map(place)].join(' › ')
    const rest = u.folder.slice(sc.f.length)
    return rest.length ? rest.map(place).join(' › ') : sc.f.length ? place(sc.f.at(-1)!) : glued(sc.s.name)
  }
  const workTitle = (id: string) => all.find(({ w }) => w.id === id)?.w.title ?? ''
  const files = $derived.by((): { u: Unlisted; s: Section }[] => {
    // In the order of the table's groups, so Show more adds rows at the end of the last group.
    if (view.t === 'unlisted') return unlistedFound.filter(({ u }) => !reason || u.reason === reason).sort((a, b) => REASONS.indexOf(a.u.reason) - REASONS.indexOf(b.u.reason))
    // A folder that holds no Work lists its unlisted files anyway, so a path to it never ends empty.
    const bare = view.t === 'section' && !!section && !own.length && !tiles.length && !q
    if (!ui.unlisted && !bare) return []
    if (q) return unlistedAll.filter(({ u, s }) => (view.t !== 'section' || (s.id === section?.id && folder.every((x, i) => u.folder[i] === x))) && matchesFile(u, ws))
    if (view.t === 'section' && section) return (section.unlisted ?? []).filter(u => !u.work && u.folder.length === folder.length && folder.every((x, i) => u.folder[i] === x)).map(u => ({ u, s: section }))
    return []
  })

  // Open rows come first on the page, so their tiles and cards get thumbnails first, then the folder's own cards.
  $effect(() => {
    const spread = (w: Work[]) => Array.from({ length: Math.min(8, w.length) }, (_, i) => w[Math.floor((i * w.length) / Math.min(8, w.length))].id)
    api.wantThumbs([...rows.filter(r => r.open).flatMap(r => [...r.subs.flatMap(u => spread(filtered ? u.works : u.every)), ...r.mine.slice(0, ROW_CARDS).map(w => w.id)]), ...shown.map(({ w }) => w.id),
      ...(photosView ? photoWant : []),
      ...(homeFolders ? sections.flatMap(s => spread(s.works)) : []), ...recent.map(({ w }) => w.id)])
  })
  // One box shape per view: portrait when most thumbnails are, or most Works are for reading
  // while their thumbnails are not made yet (`decisions/2026/09/29/folder-shows-own-works-and-one-box-shape.md`).
  const shaped = $derived.by(() => shown.length ? shown.map(({ w }) => w) : bareNext ? newest.map(({ w }) => w) : below.slice(0, 200))
  const tall = $derived.by(() => {
    const t = shaped.filter(w => thumbs[w.id])
    if (t.length && t.length * 2 >= shaped.length) return t.filter(w => thumbs[w.id].h > thumbs[w.id].w).length * 2 >= t.length
    return shaped.filter(w => w.media[0] === 'Reading').length * 2 > shaped.length
  })
  const counts = $derived({
    next: all.filter(({ w }) => marks.marks[w.id]?.m === 'next').length,
    done: all.filter(({ w }) => marks.marks[w.id]?.m === 'done').length,
    progress: all.filter(({ w }) => lastTick(w) >= 0).length,
    unlisted: unlistedAll.length,
  })
  const inFolder = $derived(view.t === 'section' && !!section)
  /** The steps of a Work's path the address bar shows: the Section, then each open folder it is in. */
  function shownSteps(w: Work) {
    let k = 0
    while (k < folder.length && w.folder[k] === folder[k]) k++
    return 1 + k
  }
  /** At the top of a Section, the offer of a kind per folder for it and each Section shown inside
   * it, from the last read: made once per Section, until answered or put away (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`). */
  /** An offer: its Section, its folders, and the dry run's count (undefined while it runs, null when it failed). */
  type Offer = { s: Section; list: Suggestion[]; count?: KindPreview | null }
  let offers = $state<Offer[]>([])
  /** The Section whose top shows `offers`: an offer never shows on another page. */
  let offersAt = $state('')
  /** An answer is being applied: the buttons wait, so a second click does not apply it twice. */
  let answering = $state(false)
  const offerTop = $derived(section && !folder.length ? section.id : '')
  /** What the offers depend on: the Sections at this top, as last read, and whether each was
   * answered. A Mark, a thumbnail or a settings change that alters none of it does not ask again;
   * nothing is asked while folders are read. */
  const offerKey = $derived(offerTop && !reading ? catalog.sections.filter(x => x.id === offerTop || ui.merge[x.id] === offerTop)
    .map(x => [x.id, x.scanned_at, x.files, x.works.length, x.online, folderKinds.offered.includes(x.id)].join(':')).join('|') : '')
  /** Counts what an offer would do, by the core's dry run (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`), once more after a moment when it fails. */
  function countOffer(x: Offer, again: boolean, live: () => boolean) {
    x.count = undefined
    const [first, ...rest] = x.list
    api.previewFolder(x.s.id, first.rel, first.kind, Object.fromEntries(rest.map(r => [r.rel, r.kind])))
      .then(p => { if (live()) x.count = p }, () => {
        if (!live()) return
        if (again) setTimeout(() => { if (live()) countOffer(x, false, live) }, 600)
        else x.count = null
      })
  }
  $effect(() => {
    const key = offerKey
    return untrack(() => {
      offers = []
      offersAt = key ? offerTop : ''
      if (!key) return
      const top = offerTop
      const homes = catalog.sections.filter(x => (x.id === top || ui.merge[x.id] === top) && x.online && !folderKinds.offered.includes(x.id))
      let gone = false
      const live = () => !gone
      Promise.all(homes.map(async s => ({ s, list: offerable(catalog, s, await api.suggestKinds(s.id)).sort((a, b) => natural(a.title, b.title)) })))
        .then(o => {
          if (gone) return
          offers = o.filter(x => x.list.length)
          for (const x of offers) countOffer(x, true, live)
        }, () => { if (!gone) offers = [] })
      return () => { gone = true }
    })
  })
  async function answer(o: { s: Section; list: Suggestion[] }, how: 'kinds' | 'sections' | 'away') {
    if (answering) return
    answering = true
    try { await onoffer(o.s, o.list, how) } finally { answering = false }
  }
  /** Home's Next up ends with a tile per checked folder, columns open or folded. */
  const homeFolders = $derived(view.t === 'next' && !q && sections.length > 0)
  /** Home with nothing marked Next: Your folders lead, then a row of the newest Works, then the backup question. */
  const bareNext = $derived(homeFolders && !medium && !markF && !listed.length)
  let recentW = $state(0), recentEl = $state<HTMLElement>(), recentCols = $state(1)
  // The grid's own column count (auto-fill makes its columns with or without cards), for each card size and width.
  $effect(() => { void [recentW, ui.size, tall]; if (recentEl) recentCols = getComputedStyle(recentEl).gridTemplateColumns.split(' ').length })
  /** The newest Works, in one pass over all of them; `recent` is as many as fill one row of the grid. */
  const newest = $derived.by((): Shown[] => {
    if (!bareNext) return []
    const n = 24, top: Shown[] = []
    for (const x of all) {
      if (top.length === n && x.w.added <= top[n - 1].w.added) continue
      let i = top.length
      while (i > 0 && top[i - 1].w.added < x.w.added) i--
      top.splice(i, 0, x)
      if (top.length > n) top.pop()
    }
    return top
  })
  const recent = $derived(newest.slice(0, ui.size === 'compact' ? 6 : recentCols))
  /** A Photos Section or Photos folder: its Works are events, which take no Mark (`decisions/2026/10/04/photos-take-no-marks-review-is-the-progress.md`). */
  const ev = $derived(calTop || !!photoRoot)
  /** It shows as calendars, as the Photos prototype does (`decisions/2026/10/08/photos-is-a-section-kind-in-the-app.md`); a search lists its events as cards. */
  const photosView = $derived(ev && view.t === 'section' && !q && !(view as { all?: boolean }).all)
  /** Where the cards' order is kept: the open Section, or Home's tabs. A Photos calendar keeps its own. */
  const sortKey = $derived(view.t === 'section' ? (section && !photosView ? section.id : '') : onHome ? 'home' : '')
  const order = $derived(sortKey ? readSort(ui.sort[sortKey]) : null)
  /** `list` in the picked order. Lengths are read as they are now: one that comes in later
   * moves its card when the place is opened again or the pick changes, never under the pointer,
   * and a Section of 150,000 Works is not sorted again for each batch of lengths. */
  function sorted<T>(list: T[], work: (x: T) => Work) {
    // The lengths read as the app starts replace `details` whole, and sort again; a batch fills it in place.
    const o = order, d = details
    return untrack(() => sortWorks(list, o, work, d, openedAt))
  }
  async function sortTo(s: Sort | null) {
    const key = sortKey, was = ui.sort[key]
    if (s) ui.sort[key] = writeSort(s)
    else delete ui.sort[key]
    try { await saveUi() } catch (e) {
      if (was) ui.sort[key] = was
      else delete ui.sort[key]
      showError(S.notApplied(String(e)))
    }
  }
  /** The events the calendars show, their folders from the Photos folder's top. */
  const photoWorks = $derived(!section || !photosView ? [] : !photoRoot ? section.works.filter(ownEvent)
    : section.works.filter(w => w.event && under(w, photoRoot!)).map(w => ({ ...w, folder: w.folder.slice(photoRoot!.length) })))
  const photoDay = $derived(view.t === 'section' ? view.day ?? '' : '')
  /** A day panel open over a month: the columns fold into the rail, as Autohide does. */
  const dayOpen = $derived(photosView && !!photoDay && photoDay !== 'undated' && pfolder.length === 2)
  const hidden = $derived(ui.autohide || narrow || dayOpen)
  // Autohide opens the columns beside the cards, which narrow to make room; a narrow window
  // opens them over the cards, which would be left too thin, and so does a day panel.
  const push = $derived(ui.autohide && !narrow && !dayOpen)
  // Hidden again later, the columns start closed.
  // Focus on the rail as it goes (a day panel closed with Esc) moves to the page.
  let wasHidden = false
  $effect(() => {
    if (!hidden) { peek = false; quiet = false }
    if (wasHidden && !hidden && document.activeElement === document.body) main?.focus()
    wasHidden = hidden
  })
  /** Opens or closes the day panel; it is part of the view, without a step for Back. */
  function setDay(d: string) { if (view.t === 'section') view = { ...view, day: d || undefined } }
  const photoLine = $derived.by(() => {
    if (!photosView) return ''
    const es = below.filter(w => w.event && lens.shown(w))
    const c = lens.count(es)
    return photoF ? S.inEvents(S.filterUnit(photoF, c.photos), es.length) : `${S.events(es.length)}: ${S.countLine(c.photos, c.clips, c.hidden)}`
  })
  /** Each filter's count in the whole Photos Section or Photos folder: sets for Your call, files for the rest. */
  const filterCounts = $derived.by(() => {
    const out: Record<string, number> = {}
    if (!photosView || !section) return out
    for (const f of FILTERS) {
      const l = new Lens(notes, marks, ui.photoKind, true, f)
      const items = photoWorks.flatMap(w => l.items(w))
      out[f] = f === 'call' ? new Set(items.map(i => notes[i.path]?.set).filter(x => x && l.calls.has(x))).size : items.filter(l.extra).length
    }
    return out
  })
  function setPhotos(k: typeof ui.photoKind, all: boolean) { ui.photoKind = k; ui.photoAll = all; saveUi() }
  /** Review in the Photos view: what waits, and what is set aside in the whole Section. */
  // In the Photos view it counts the kind shown, as Review then goes through.
  const reviewHere = $derived(ev && view.t === 'section'
    ? photosView ? (lens.toReview(below) ? { n: lens.toReview(below), done: false } : lens.hadReview(below) ? { n: 0, done: true } : null) : reviewState(below, notes, marks)
    : null)
  /** The Photos Section or Photos folder whose photos Review's move takes (`decisions/2026/10/09/review-moves-set-aside-photos-itself-with-undo.md`, `decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`). */
  const scriptPlace = $derived.by((): PhotoPlace | undefined => {
    if (!ev || !section) return undefined
    if (!photoRoot) return catalog.sections.some(s => s.id === section!.id && s.kind === 'Photos') ? { id: section.id, name: section.name, home: section, under: [], works: section.works.filter(ownEvent) } : undefined
    const at = homeOf(catalog, section, photoRoot)
    // Only its own Events: one title can stand for a Photos folder on each drive of a merged Section.
    const id = at && keyIn(at.home.id, at.rel)
    return at && id ? { id, name: place(photoRoot.at(-1) ?? ''), home: at.home, under: photoRoot, works: section.works.filter(w => w.event && eventPlace(w) === id) } : undefined
  })
  const asideAll = $derived(scriptPlace ? asideIn(scriptPlace.works, marks) : 0)
  // The count says what is shown; Move lists the whole Section's and names this part.
  const asideHere = $derived(asideAll ? asideIn(below, marks) : 0)
  const moveScope = $derived.by((): MoveScope | undefined => {
    if (!pfolder.length) return undefined
    const name = pfolder.length === 2 && /^\d{4}$/.test(pfolder[0]) ? `${place(pfolder[1])} ${pfolder[0]}` : place(pfolder.at(-1)!)
    return { name, works: below }
  })
  /** The last move Undo can take back, read again whenever the catalog is. */
  let lastMove = $state<LastMove | null>(null)
  $effect(() => {
    const id = scriptPlace?.id
    void catalog
    lastMove = null
    if (id) api.lastMove(id).then(l => { if (scriptPlace?.id === id) lastMove = l })
  })
  const listName = $derived(({ next: S.nextUp, progress: S.inProgress, done: S.done, added: S.recentlyAdded, opened: S.recentlyOpened, unlisted: S.unlistedTitle } as Record<string, string>)[view.t] ?? '')
  const heading = $derived(inFolder ? place(folder.at(-1) ?? section!.name) : view.t === 'unlisted' ? (q ? S.unlistedMatching(q) : S.unlistedTitle) : q ? S.results(q) : listName)
  // The open folder on disk. Folder titles drop numbers and marks from the names, so the real
  // names come from a Work under it, and its own Section's path (merged Sections differ).
  const diskPath = $derived.by(() => {
    if (!section) return ''
    const w = below[0]
    const home = w ? sectionOf(catalog, w.id) : section
    // A year or month in Photos is no folder: its Photos folder, or the Photos Section's.
    const f = photoRoot ?? (ev ? [] : folder)
    if (!w || !home || !f.length) return home?.path ?? section.path
    const sep = home.path.includes('\\') ? '\\' : '/'
    return [home.path.replace(/[\\/]+$/, ''), ...diskRel(home, w).split('/').slice(0, f.length)].join(sep)
  })

  /** A folder's full path on disk for its hover: one line per Section merged into it, as each
   * holds its own copy. Real names come from a Work or file under it, as in `diskPath`; years and
   * months in Photos are no folders on disk, so they have none. */
  function pathTip(s: Section, f: string[]): string | undefined {
    if (calendarIn(s, f)) return undefined
    const homes = catalog.sections.filter(x => x.id === s.id || ui.merge[x.id] === s.id)
    const paths = homes.flatMap(h => relsOf(h, f).map(rel => rel ? [h.path.replace(/[\\/]+$/, ''), ...rel.split('/')].join(h.path.includes('\\') ? '\\' : '/') : h.path))
    return paths.join('\n') || undefined
  }
  /** The files not on a card in folder `f` of Section `s` and below; none on a year or month in
   * Photos, which is no folder on disk. */
  function unlistedIn(s: Section, f: string[]) { return calendarIn(s, f) ? [] : (s.unlisted ?? []).filter(u => f.every((x, i) => u.folder[i] === x)) }
  const unlistedHere = $derived(inFolder && section && !q ? unlistedIn(section, folder).length : 0)
  /** Folder `f` of Section `s` is a year, a month or Undated in Photos, no folder on disk; its
   * Photos folder too, none in a Photos Section. */
  function calendarIn(s: Section, f: string[]): { root: string[] } | undefined {
    if (s.kind === 'Photos' && !othersOf(s).has(f[0])) return f.length ? { root: [] } : undefined
    const root = f.length > 1 ? photoFolderOf(s, f) : undefined
    return root && f.length > root.length ? { root } : undefined
  }

  /** A card marked here keeps its place in the list it is on until the list is opened again. */
  function pin(ws: Work[]) {
    for (const w of ws) if (!pinned.has(w.id)) {
      if (view.t === 'progress') pinned.set(w.id, lastTick(w, false))
      else if (view.t === 'next' || view.t === 'done') pinned.set(w.id, marks.marks[w.id]?.at ?? 0)
    }
    sticky = new Set([...sticky, ...ws.map(w => w.id)])
  }
  function mark(w: Work, m: 'next' | 'done' | null) { pin([w]); onmark(w, m) }

  // Several cards selected, as File Explorer selects: Ctrl+click adds or takes away one,
  // Shift+click takes the run from the last one so picked, Ctrl+Space and Shift+Space do the same
  // from the keyboard, Ctrl+A takes the page, and Esc, a plain click or another page clears it.
  let sel = $state.raw(new Set<string>())
  let selFrom = ''
  /** Something was selected on this page: the status then says None selected when it empties. */
  let selSeen = $state(false)
  $effect(() => { if (sel.size) selSeen = true })
  // A new search or filter clears it, as in File Explorer.
  $effect(() => { void q; void medium; void markF; sel = new Set() })
  /** The page's cards that take a Mark, in the order shown: the open folder rows', then the list's. */
  const pageCards = $derived.by(() => {
    const s = section
    const rowCards = !photosView && !(folderTiles && tiles.length) && s ? rows.filter(r => r.open).flatMap(r => r.mine.slice(0, ROW_CARDS).map(w => ({ w, s }))) : []
    return [...rowCards, ...(photosView ? [] : listed)].filter(({ w }) => !w.event)
  })
  /** The selected cards still on the page, in its order. */
  const chosen = $derived(sel.size ? pageCards.filter(({ w, s }) => sel.has(wkey(w, s))) : [])
  function selectRun(k: string, add: boolean) {
    const keys = pageCards.map(({ w, s }) => wkey(w, s))
    if (!keys.includes(selFrom)) selFrom = k
    const a = keys.indexOf(selFrom), b = keys.indexOf(k)
    const run = keys.slice(Math.min(a, b), Math.max(a, b) + 1)
    sel = new Set(add ? [...sel, ...run] : run)
  }
  function selectOne(k: string) {
    const n = new Set(sel)
    if (!n.delete(k)) n.add(k)
    sel = n; selFrom = k
  }
  /** A card the selection takes: one of the page's, not one of Home's Recently added row. */
  const selectable = (w: Work, s: Section) => !w.event && pageCards.some(x => x.w === w && x.s === s)
  /** A click on a card with Ctrl or Shift selects; a plain one clears the selection and goes on. */
  function pick(e: MouseEvent, w: Work, s: Section) {
    if (!selectable(w, s)) return
    const k = wkey(w, s), ctrl = e.ctrlKey || e.metaKey
    if (!ctrl && !e.shiftKey) { if (sel.size) sel = new Set(); selFrom = k; return }
    e.preventDefault(); e.stopPropagation()
    if (e.shiftKey) selectRun(k, ctrl); else selectOne(k)
  }
  function pickKey(e: KeyboardEvent, w: Work, s: Section) {
    if (e.key !== ' ' || !(e.ctrlKey || e.metaKey || e.shiftKey) || !selectable(w, s)) return
    e.preventDefault(); e.stopPropagation()
    const k = wkey(w, s)
    if (e.shiftKey) selectRun(k, e.ctrlKey || e.metaKey); else selectOne(k)
  }
  /** One Mark on every selected card; the selection stays, as File Explorer's does. */
  function markMany(ws: Work[], m: 'next' | 'done' | null) { pin(ws); onmarkmany(ws, m) }
  function manyItems(list: { w: Work }[]): MenuItem[] {
    const ws = list.map(x => x.w), n = ws.length
    const words = new Set(ws.map(doneWord)), word = words.size === 1 ? [...words][0] : S.done
    const marked = ws.filter(w => marks.marks[w.id] || w.copies?.some(c => marks.marks[c])).length
    // Ticked when every card has it already, as one card's menu ticks its Mark.
    const all = (m: string) => ws.every(w => marks.marks[w.id]?.m === m)
    return [
      { label: S.markMany(n, S.next), icon: 'star', checked: all('next'), run: () => markMany(ws, 'next') },
      { label: S.markMany(n, word), icon: 'done', checked: all('done'), run: () => markMany(ws, 'done') },
      ...(marked ? [{ label: S.clearMany(marked), icon: 'clear', run: () => markMany(ws, null) } as MenuItem] : []),
    ]
  }
  /** A card's menu acts on the selection when the card is in it; a card outside it drops it. */
  function cardMenu(e: MouseEvent, w: Work, s: Section) {
    const k = wkey(w, s)
    if (sel.has(k) && chosen.length > 1) return openMenu(e, manyItems(chosen), S.selectedN(chosen.length))
    if (sel.size && !sel.has(k)) sel = new Set()
    openMenu(e, cardItems(w, s), w.title)
  }
  /** Opens a folder; `replace` swaps the view in place, as stepping through days in Photos does, so Back skips those steps. */
  function openFolder(s: Section, f: string[], day?: string, replace = false) {
    const v: View = { t: 'section', id: s.id, folder: f, day }
    if (replace) show({ v, scroll: 0, limit: CHUNK }); else go(v)
  }
  /** The files not on a card in folder `f` of Section `s` and below, from its page's chip or menu. */
  function openUnlisted(s: Section, f: string[]) { query = ''; go({ t: 'unlisted', id: s.id, folder: f }) }
  function clearAll() { query = ''; medium = ''; markF = '' }
  function sizeTo(v: typeof ui.size) { ui.size = v; saveUi() }

  // One chrome surface (title bar, library column, status bar) around one content pane, as
  // Windows 11 apps draw it: the pane's first column, or the cards, takes the rounded corner.
  const paneCol = $derived(hidden && !(push && peek) ? -1 : deep ? (folderCols.slice(-2).length > 1 ? 1 : -1) : folderCols.length ? 0 : -1)
  // The selected folder in each column stays in sight.
  $effect(() => {
    void folder
    // Scrolled by hand: scrollIntoView would also move where the next Tab starts.
    queueMicrotask(() => document.querySelectorAll<HTMLElement>('.cols [aria-current="page"]').forEach(b => {
      const c = b.closest('.col'); if (!c) return
      const r = b.getBoundingClientRect(), cr = c.getBoundingClientRect()
      if (r.top < cr.top) c.scrollTop -= cr.top - r.top
      else if (r.bottom > cr.bottom) c.scrollTop += r.bottom - cr.bottom
    }))
  })

  function keys(e: KeyboardEvent) {
    if (e.altKey && e.key === 'ArrowLeft') { e.preventDefault(); goBack() }
    else if (e.altKey && e.key === 'ArrowRight') { e.preventDefault(); goForward() }
    else if ((e.ctrlKey || e.metaKey) && !e.altKey && !e.shiftKey && e.key.toLowerCase() === 'a' && inCards(e)) {
      e.preventDefault(); sel = new Set(pageCards.map(({ w, s }) => wkey(w, s)))
    } else if (e.key === 'Escape' && sel.size && !e.defaultPrevented && !document.querySelector('dialog[open], [aria-modal="true"], :popover-open:not(.toast)')) {
      e.preventDefault(); sel = new Set()
    }
  }
  /** A plain click on the page outside the cards clears the selection, as in File Explorer. */
  function unselect(e: MouseEvent) {
    const t = e.target as HTMLElement
    if (sel.size && !(e.ctrlKey || e.metaKey || e.shiftKey) && main?.contains(t) && !t.closest('.cell')) sel = new Set()
  }
  /** Ctrl+A selects the cards when the focus is on the page, not in a text field. */
  const inCards = (e: KeyboardEvent) => {
    const t = e.target as HTMLElement
    return !!main?.contains(t) && !t.closest('input, textarea, [contenteditable="true"]') && !!pageCards.length
  }
  /** Esc closes the open columns before anything under them, such as a day panel; in a dialog or
   * a popup it closes that instead. Caught on the way down, so it runs first. */
  function esc(e: KeyboardEvent) {
    if (e.key === 'Escape' && rail?.contains(document.activeElement)) quiet = true
    if (e.key !== 'Escape' || !peek || e.defaultPrevented || document.querySelector('dialog[open], [aria-modal="true"], :popover-open')) return
    e.preventDefault()
    if (cols.contains(document.activeElement)) { quiet = true; colsBtn?.focus() }
    peek = false
  }
  function mouse(e: MouseEvent) {
    if (e.button === 3) { e.preventDefault(); goBack() } else if (e.button === 4) { e.preventDefault(); goForward() }
  }
  const wkey = (w: Work, s: Section) => `${s.id}|${w.id}`
  /** The kind a Work was made as: its folder's own, the nearest folder's above, or its Section's (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`). */
  function kindOfWork(w: Work, s: Section) {
    if (w.event) return 'Photos'
    const home = sectionOf(catalog, w.id) ?? s
    return kindAt(home, relIn(w.id, home.id).split('/').slice(0, w.folder.length).join('/'))
  }

  /** A Section's icon on the rail and in the address bar's menu, by what it holds. */
  const KIND_ICON = { Books: 'book', Courses: 'play', Photos: 'image', Collections: 'folder', Files: 'folder' } as const
  /** Folder `name` in folder `at` of the open Section is a year, month or Undated of Photos: a
   * Photos Section's own (not one of its other folders, `decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`), or as the page shows. */
  const evIn = (at: string[], name: string) => section?.kind === 'Photos' ? !others.has(at[0] ?? name) : ev
  /** How many Works a folder in the open Section shows, as its folder column counts them. */
  const shownIn = (f: Sub, at: string[]) => evIn(at, f.name) && !q ? lens.num(f.every) : f.works.length
  /** A folder's count as its column shows it: a year or month with nothing shown says how many Best only hides. */
  function countLabel(f: Sub, at: string[]) {
    const n = shownIn(f, at), hid = evIn(at, f.name) && !q && !n ? lens.count(f.every).hidden : 0
    return hid ? S.noneShown(hid) : n.toLocaleString('en')
  }
  /** How many Works a Section shows in the library column. */
  const countOf = (s: Section) => s.kind === 'Photos' && !q ? lens.num(s.works.filter(ownEvent)) : filtered ? (s === section && ok ? ok.size : s.works.filter(w => passes(w) && score(w, ws) > 0).length) : s.works.length
  /** How many top folders a Section has, for its tile on Home. */
  const topFolders = (s: Section) => s.kind === 'Photos' ? new Set(tileWorks(s).map(w => w.folder[0])).size
    : new Set([...s.works.filter(w => w.folder.length).map(w => w.folder[0]), ...(s.links ?? []).map(l => l.folder[0])]).size
  /** What a Section's tile shows: a Photos Section's Events and years, its other folders left out (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`). */
  const tileWorks = (s: Section) => s.kind === 'Photos' ? s.works.filter(ownEvent) : s.works
  /** A chevron in the address bar lists what is in that step, as File Explorer's: the Sections
   * after the app's name (`at` null), else the folders in folder `at`; the one open is ticked. */
  const addrKey = (at: string[] | null) => at === null ? 'addr:@' : 'addr:/' + at.join('/')
  function addrMenu(e: MouseEvent, at: string[] | null) {
    const key = addrKey(at)
    if (at === null) openMenu(e, inOrder.map(s => ({ label: s.name, icon: KIND_ICON[s.kind], checked: section?.id === s.id,
      hint: countOf(s).toLocaleString('en'), run: () => { openFolder(s, []); addrPicked() } })), S.insideOf(S.app), key)
    else if (section) {
      const item = (f: Sub): MenuItem => f.link ? { label: place(f.name), icon: KIND_ICON[f.link.kind], section: color(f.link), hint: countOf(f.link).toLocaleString('en'), run: () => { openFolder(f.link!, []); addrPicked() } }
        : { label: place(f.name), icon: 'folder', checked: folder[at.length] === f.name, hint: countLabel(f, at), run: () => { openFolder(section!, [...at, f.name]); addrPicked() } }
      // A Photos Section's years, then its Folders with other things, as its first column shows them (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`).
      const all = subsOf(at), split = !at.length && others.size > 0
      openMenu(e, split ? [...all.filter(f => !others.has(f.name)).map(item), { sep: true }, { head: S.othersHead }, ...all.filter(f => others.has(f.name)).map(item)] : all.map(item),
        S.insideOf(place(at.at(-1) ?? section.name)), key)
    }
  }
  /** The address bar's steps: each goes to its place (none for the open one), and `chev` says
   * what its chevron lists (null the Sections, else the folders in that folder; undefined none). */
  type Step = { key: string; label: string; title?: string; go?: () => void; current?: boolean; chev?: string[] | null
    /** Its menu: the Section's or the folder's, as in the folder columns. */
    menu?: () => MenuItem[]
    /** A plain arrow after it, with no menu: on the path of a list of files not on a card. */
    sep?: boolean }
  const steps = $derived.by((): Step[] => {
    const root = sections.length ? null : undefined
    if (!inFolder || !section) {
      const here = onHome && !q
      // Files not on a card of a Section or folder: its path, then this list.
      const sc = scope
      const up: Step[] = sc ? [{ key: 's', label: glued(sc.s.name), title: pathTip(sc.s, []), go: () => openFolder(sc.s, []), menu: () => placeItems(sc.s, []), sep: true },
        ...sc.f.map((f, i): Step => ({ key: `f${i}`, label: place(f), title: pathTip(sc.s, sc.f.slice(0, i + 1)), go: () => openFolder(sc.s, sc.f.slice(0, i + 1)), menu: () => placeItems(sc.s, sc.f.slice(0, i + 1)), sep: true }))] : []
      return [{ key: 'app', label: S.app, go: here ? undefined : home, current: here, chev: root },
        ...(view.t === 'unlisted' ? [...up, { key: 'unl', label: S.unlistedTitle, current: true }] : [])]
    }
    const s = section, last = folder.length - 1
    return [
      { key: 'app', label: S.app, go: home, chev: root },
      { key: 's', label: glued(s.name), title: pathTip(s, []), go: folder.length ? () => openFolder(s, []) : undefined, current: !folder.length,
        chev: folder.length || subfolders.length ? [] : undefined, menu: () => placeItems(s, [], !folder.length) },
      ...folder.map((f, i): Step => ({ key: `f${i}`, label: place(f), title: pathTip(s, folder.slice(0, i + 1)), go: i < last ? () => openFolder(s, folder.slice(0, i + 1)) : undefined,
        current: i === last, chev: i < last || subfolders.length ? folder.slice(0, i + 1) : undefined, menu: () => placeItems(s, folder.slice(0, i + 1), i === last) })),
    ]
  })
  // A path wider than the bar folds its first steps into a … menu, as File Explorer's; the
  // open step is cut short only when it alone does not fit. A hidden copy of every step gives
  // their widths, so the steps shown are not redrawn to measure them and focus stays put.
  let addr = $state<HTMLElement>(), probe = $state<HTMLElement>()
  let fold = $state(0), tight = $state(false)
  const FOLD_W = 27, DIR_W = 31
  function fit() {
    if (!addr || !probe) return
    const cs = getComputedStyle(addr)
    const room = addr.clientWidth - parseFloat(cs.paddingLeft) - parseFloat(cs.paddingRight) - (inFolder ? DIR_W : 0) - 2
    const ws = [...probe.children].map(c => c.getBoundingClientRect().width + 1)
    let f = 0, total = ws.reduce((a, b) => a + b, 0)
    while (f < ws.length - 1 && total + (f ? FOLD_W : 0) > room) total -= ws[f++]
    fold = f
    tight = total + (f ? FOLD_W : 0) > room
  }
  $effect(() => { void steps.map(x => x.key + x.label).join('/'); tick().then(fit) })
  $effect(() => {
    if (!addr) return
    const ro = new ResizeObserver(() => fit())
    ro.observe(addr)
    return () => ro.disconnect()
  })
  function foldMenu(e: MouseEvent) {
    openMenu(e, steps.slice(0, fold).map((x, i) => ({ label: x.label, icon: i ? 'folder' : 'home', run: () => { x.go?.(); addrPicked() } })), S.earlierSteps, 'addr:…')
  }
  /** A pick in an address bar menu redraws the bar under the focus: it goes to the bar's last button. */
  function addrPicked() {
    tick().then(() => { if (document.activeElement === document.body) [...main.querySelectorAll<HTMLElement>('.addr button')].at(-1)?.focus() })
  }

  // The menus: a right-click on a Section, a folder, a card or the page title, and the ⋯
  // button for the page, open these. Rare actions live here rather than on screen.
  /** A folder's place on disk; years and months in Photos are not folders, so they have none. */
  const diskOf = (s: Section, f: string[]) => f.length ? pathTip(s, f)?.split('\n')[0] : s.path
  /** A Section's menu, or with `f` a folder's in it; `here` leaves out Open, as it is the page shown. */
  function placeItems(s: Section, f: string[], here = false): MenuItem[] {
    const disk = diskOf(s, f)
    const ids = s.works.filter(w => under(w, f)).flatMap(w => [w.id, ...w.copies ?? []])
    // Years and months in Photos are no folders on disk: reading one again reads the Photos
    // Section, or the Photos folder (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`).
    const cal = calendarIn(s, f)
    const again = cal ? cal.root : f, whole = !again.length
    const ym = cal ? f.slice(cal.root.length) : []
    const items: MenuItem[] = [
      ...(here ? [] : [{ label: S.open, icon: 'launch', run: () => openFolder(s, f) } as MenuItem]),
      ...(disk ? [{ label: S.showInExplorer, icon: 'open', run: () => onreveal(disk) } as MenuItem] : []),
      ...(unlistedIn(s, f).length ? [{ label: S.unlistedTitle, icon: 'folder', hint: unlistedIn(s, f).length.toLocaleString('en'), run: () => openUnlisted(s, f) } as MenuItem] : []),
      // A year or a month in Photos offers Review of its events.
      ...(cal ? reviewItems(s.works.filter(w => w.event && under(w, f) && lens.shown(w)), [place(ym[1] ?? ''), ym[0]].filter(Boolean).join(' ')) : []),
    ]
    if (items.length) items.push({ sep: true })
    items.push(
      { label: whole ? S.rereadSection : S.rereadFolder, icon: 'refresh', disabled: reading || s.gone, run: () => onreread(s, again) },
      { label: S.rethumb, icon: 'image', disabled: !s.online || !ids.length, run: () => onrethumb(ids) },
      { label: S.chooseCover, icon: 'image', run: () => onchoosecover(coverOfFolder(s, f)) },
    )
    // Every folder has its own settings (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`); a Section's settings and Remove are on the
    // Section, and a page inside a folder still offers its Section's settings.
    const settings: MenuItem[] = []
    // A year or month page offers its Photos folder's, which set how it shows.
    const fs = !cal ? f : here ? cal.root : []
    const at = fs.length ? homeOf(catalog, s, fs) : undefined
    if (at) settings.push({ label: S.folderSettings, icon: 'gear', hint: KINDS[kindAt(at.home, at.rel)].name, disabled: !at.home.scanned_at, run: () => onfolder(at.home, fs, at.rel) })
    const home = catalog.sections.find(x => x.id === s.id) ?? s
    if (!f.length || here) settings.push({ label: S.sectionSettings, icon: 'gear', hint: KINDS[home.kind].name + (home.guessed ? S.guessed : ''), disabled: !home.scanned_at, run: () => onkind(home) })
    if (settings.length) items.push({ sep: true }, ...settings)
    // A Split-out Section goes back into the Section it came from; one whose folder is gone is removed (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`).
    const from = !f.length && !home.gone ? fromOf(home) : undefined
    if (from) items.push({ sep: true }, { label: S.putBackInto(from.name), icon: 'back', disabled: reading || !home.online, run: () => onputback(home) })
    else if (!f.length) items.push({ sep: true }, { label: S.removeFromApp, icon: 'trash', danger: true, run: () => onforget(home) })
    return items
  }
  /** The Section a Split-out Section came from (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`), from the catalog. */
  const fromOf = (s: Section) => { const id = catalog.sections.find(x => x.id === s.id)?.split_from; return id ? catalog.sections.find(x => x.id === id) : undefined }
  /** A Section's hover: its path, and the Section it was split out of. */
  const secTip = (s: Section) => [pathTip(s, []), fromOf(s) && S.splitOutOf(fromOf(s)!.name), ...awayTips(s)].filter(Boolean).join('\n') || undefined
  /** Section `s` and the ones shown inside it whose drive is not connected. */
  const awayOf = (s: Section) => catalog.sections.filter(x => (x.id === s.id || ui.merge[x.id] === s.id) && !x.online && !x.gone)
  const awayTips = (s: Section) => [...new Set(awayOf(s).map(x => x.id === s.id ? S.driveAway(x.drive) : S.partAway(x.name, x.drive)))]
  /** The hover of a Split-out Section's row or tile in the Section it came from. */
  const linkTip = (to: Section) => [to.name, secTip(to)].filter(Boolean).join('\n')
  /** A Split-out Section's menu where it shows in the Section it came from: open it, its settings, Put back. */
  function linkItems(to: Section, id = to.id): MenuItem[] {
    const home = catalog.sections.find(x => x.id === id) ?? to
    const from = fromOf(home)
    return [
      { label: S.open, icon: 'launch', run: () => openFolder(to, []) },
      { label: S.showInExplorer, icon: 'open', disabled: !home.online, run: () => onreveal(home.path) },
      { label: S.chooseCover, icon: 'image', run: () => onchoosecover(coverOfFolder(to, [])) },
      { sep: true },
      { label: S.sectionSettings, icon: 'gear', hint: KINDS[home.kind].name + (home.guessed ? S.guessed : ''), disabled: !home.scanned_at, run: () => onkind(home) },
      ...(from ? [{ label: S.putBackInto(from.name), icon: 'back', disabled: reading || !home.online, run: () => onputback(home) } as MenuItem] : []),
    ]
  }
  /** A subfolder row's menu: its folder's, with Expand or Collapse and the same for every row. */
  function rowItems(s: Section, f: string[]): MenuItem[] {
    const items = placeItems(s, f), at = items.findIndex(x => 'sep' in x)
    const open = isOpen(s, f)
    items.splice(at < 0 ? items.length : at, 0, { sep: true },
      { label: open ? S.collapseRow : S.expandRow, icon: open ? 'collapse' : 'expand', run: () => setOpen(s, [f], !open) },
      ...allItems())
    return items
  }
  /** Expand all and Collapse all, each off when it would change nothing. */
  const allItems = (): MenuItem[] => [
    { label: S.expandAll, icon: 'expand', disabled: rows.every(r => r.open), run: () => setAll(true) },
    { label: S.collapseAll, icon: 'collapse', disabled: !rows.some(r => r.open), run: () => setAll(false) },
  ]
  /** Review of these events as one item, when any of their files wait; `only` keeps some files,
   * as a day's own. Review goes through the kind shown, as the Review buttons do. */
  function reviewItems(ws: Work[], label: string, only?: (i: Item) => boolean): MenuItem[] {
    const n = lens.toReview(ws, only), kind = lens.reviewOnly
    const both = only && kind ? (i: Item) => only(i) && kind(i) : only ?? kind
    return n ? [{ label: S.reviewN(n, lens.k), icon: 'review', run: () => onreview(ws.filter(w => lens.toReview([w], only)), label, both) }] : []
  }
  /** A card's menu. In Photos (`inPhotos`) an event is in its place: Open is how that place opens
   * it, none where it is open already, `enter` says Enter opens it there too, and there is no Go to its folder. */
  function cardItems(w: Work, s: Section, inPhotos?: { open?: () => void; enter?: boolean }): MenuItem[] {
    const m = marks.marks[w.id]?.m
    const shownHere = !!inPhotos || (view.t === 'section' && view.id === s.id && !view.all && folder.join('/') === w.folder.join('/'))
    const open = inPhotos ? inPhotos.open : () => onopen(w, s)
    return [
      ...(open ? [{ label: S.open, icon: 'launch', hint: !inPhotos || inPhotos.enter ? S.enterKey : undefined, run: open } as MenuItem] : []),
      // Events in Photos take no Mark (`decisions/2026/10/04/photos-take-no-marks-review-is-the-progress.md`); they offer Review instead.
      ...(w.event ? reviewItems([w], w.title) : [{ sep: true }, { label: S.next, icon: 'star', checked: m === 'next', run: () => mark(w, m === 'next' ? null : 'next') },
        { label: doneWord(w), icon: 'done', checked: m === 'done', run: () => mark(w, m === 'done' ? null : 'done') },
        ...(m ? [{ label: S.clearMark, icon: 'clear', run: () => mark(w, null) }] : [])] as MenuItem[]),
      { sep: true },
      { label: S.showInExplorer, icon: 'open', disabled: !s.online, run: () => onreveal(shownOf(w)) },
      ...(shownHere ? [] : [{ label: S.goToFolder, icon: 'folder', run: () => openFolder(s, w.folder) } as MenuItem]),
      { label: S.rethumbOne, icon: 'image', disabled: !s.online, run: () => onrethumb([w.id, ...w.copies ?? []]) },
      { label: S.chooseCover, icon: 'image', run: () => onchoosecover(coverOfWork(w)) },
    ]
  }
  /** The page's own menu: its Section or folder, or on the lists, reading every folder again. */
  const pageItems = (): MenuItem[] => inFolder && section ? placeItems(section, folder, true)
    : scope ? placeItems(scope.s, scope.f).filter(x => !('label' in x) || x.label !== S.unlistedTitle)
    : [{ label: S.refresh, icon: 'refresh', disabled: reading, run: () => onreread(null, []) },
      ...(view.t === 'opened' ? [{ sep: true }, { label: S.clearOpened, icon: 'clear', disabled: !Object.keys(marks.opened ?? {}).length, run: onclearopened }] as MenuItem[] : [])]
  const MARKED = ['next', 'progress', 'done']
  /** What shows, by medium and by Mark, as File Explorer's Filter; the chips under the title say what is on. */
  function filterMenu(e: MouseEvent) {
    openMenu(e, [
      { head: S.filterShow },
      { label: S.everythingShown, checked: !medium, run: () => (medium = '') },
      ...MEDIA.map(m => ({ label: S.mediaName[m], checked: medium === m, run: () => (medium = m) })),
      // Next up, In progress and Done are already lists by Mark.
      ...(MARKED.includes(view.t) ? [] : [{ sep: true }, { head: S.marked },
        { label: S.anyMark, checked: !markF, run: () => (markF = '') },
        { label: S.next, checked: markF === 'next', run: () => (markF = 'next') },
        { label: S.done, checked: markF === 'done', run: () => (markF = 'done') }] as MenuItem[]),
    ], S.filter, 'filter')
  }
  /** Sets what the open place shows inside (`decisions/2026/10/21/the-view-menu-sets-each-folders-view.md`): inside a folder its own view, on each
   * drive of a merged Section, which the folders below it with none of their own take too; ''
   * gives it the view of the folder above or the Section again. At a Section's top the Section's. */
  async function pickFolderView(v: string) {
    if (spot) {
      // A write that fails puts back the drives already written, so they never disagree.
      const done: { key: string; was: ReturnType<typeof ownOf> }[] = []
      try {
        for (const h of homesOf(catalog, section!.id, folder)) {
          const mine = ownOf(h.home, h.rel), views = { ...mine.views }, key = keyIn(h.home.id, h.rel)
          if (v) views.folder = v
          else delete views.folder
          loadFolders(await api.setFolder(key, mine.kind, views))
          done.push({ key, was: mine })
        }
      } catch (e) {
        for (const d of done) await api.setFolder(d.key, d.was.kind, d.was.views).then(loadFolders).catch(() => {})
        showError(S.notApplied(String(e)))
      }
    } else if (section) {
      const was = { ...(ui.views[section.id] ?? {}) }
      setViews(section.id, picksFor(section.kind, { ...was, folder: v }))
      try { await saveUi() } catch (e) { setViews(section.id, was); showError(S.notApplied(String(e))) }
    }
  }
  function viewMenu(e: MouseEvent) {
    const set = (f: () => void) => () => { f(); saveUi() }
    const here = section
    const kind = spot ? kindAt(spot.home, spot.rel) : here?.kind
    const above = spot && kind && here ? viewFrom(spot.home, spot.rel, 'folder', folder, false, kind, here) : undefined
    openMenu(e, [
      { head: S.cardSize },
      ...([['compact', S.compact], ['normal', S.normal], ['large', S.large]] as const).map(([v, label]) => ({ label, checked: ui.size === v, run: () => sizeTo(v) })),
      // The cards' order, as Card size: a search keeps its best match first.
      ...(sortKey && !q ? [{ sep: true }, { head: S.sortBy },
        { label: onHome ? S.sortLatest : S.sortFolder, checked: !order, run: () => sortTo(null) },
        ...([['name', S.sortName], ['added', S.sortAdded], ['length', S.sortLength], ['opened', S.sortOpened]] as [SortBy, string][]).map(([by, label]) =>
          ({ label, checked: order?.by === by, run: () => sortTo(order?.by === by ? order : firstWay(by)) })),
        ...(order ? [{ sep: true }, ...S.sortWays[order.by].map((label, i) => ({ label, checked: order.down === !!i, run: () => sortTo({ ...order, down: !!i }) }))] : [])] as MenuItem[] : []),
      // How the open place shows its subfolders, only where it has some (`decisions/2026/10/21/the-view-menu-sets-each-folders-view.md`): inside a
      // folder its own view, after Same as the view it takes from above.
      // A folder the app cannot place on disk (one holding only Split-out Sections, a Photos
      // folder's year) has no view of its own to set, so it offers none rather than the Section's.
      ...(here && kind && subfolders.length && (spot || !folder.length) && LEVELS[kind].some(l => l.id === 'folder') ? [{ sep: true }, { head: S.subfoldersAs },
        ...(spot && above ? [{ label: S.sameAs(above.from), hint: S.viewNames[above.view][0], checked: !folderFrom?.own, run: () => pickFolderView('') }] : []),
        ...(['rows', 'tiles', 'flat'] as const).map(v => ({ label: S.viewNames[v][0], checked: (!spot || !!folderFrom?.own) && folderView === v, run: () => pickFolderView(v) }))] as MenuItem[] : []),
      ...(rows.length && !folderTiles ? [{ sep: true }, ...allItems()] as MenuItem[] : []),
      { sep: true },
      { label: S.unlistedInPlace, checked: ui.unlisted, toggle: true, run: set(() => (ui.unlisted = !ui.unlisted)) },
    ], S.view, 'view')
  }
  // A menu belongs to the view it was opened in: Back, Forward or a shortcut that changes it closes it.
  $effect(() => { void view; closeMenu() })
  const filterWords = $derived([medium && S.mediaName[medium], markF && (markF === 'next' ? S.next : S.done)].filter(Boolean) as string[])
</script>

<svelte:window onkeydowncapture={esc} onkeydown={keys} onmouseup={mouse} onclick={unselect} onpointerdown={e => peek && main?.contains(e.target as Node) && !(e.target === main && e.offsetX >= main.clientWidth) && (peek = false)} />

<!-- A Split-out Section's icon where it shows in the Section it came from: its kind's, with a shortcut arrow (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`). -->
{#snippet linkIcon(to: Section, size = 16)}<span class="li"><Icon name={KIND_ICON[to.kind]} {size} /><span class="sc"><Icon name="shortcut" size={10} /></span></span>{/snippet}
<div class="body" class:hidden class:push style="--cols:{widths}">
  <div class="side">
  {#if hidden}
    <!-- A rail label is sized for its bold look by a hidden bold copy, so opening a Section does not
         move its lines; two lines at most, broken only between words, then an ellipsis.
         railFit writes its text and the copy's. -->
    {#snippet rl(label: string)}<span class="rl" dir="auto" use:railFit={label}><span></span></span>{/snippet}
    <nav class="rail" class:quiet aria-label={S.rail} bind:this={rail} onfocusout={() => (quiet = false)}>
      <button class="rb" class:on={onHome} aria-current={onHome ? 'page' : undefined} onclick={home}><span class="ri"><Icon name="home" size={18} /></span>{@render rl(S.home)}</button>
      <div class="rsecs">
        {#each inOrder as s (s.id)}
          {@const here = view.t === 'section' && view.id === s.id}
          <button class="rb sec" class:on={here} class:off={!s.online} aria-current={here && !folder.length ? 'page' : here ? 'true' : undefined} style="--c:{color(s)}" aria-label={s.name} title={[s.name, secTip(s)].filter(Boolean).join('\n')}
            onclick={() => openFolder(s, [])} oncontextmenu={e => openMenu(e, placeItems(s, []), s.name)}><span class="ri"><Icon name={KIND_ICON[s.kind]} size={18} /></span>{@render rl(glued(s.name))}</button>
        {/each}
      </div>
      <button class="rb" class:on={peek} bind:this={colsBtn} onclick={() => (peek = !peek)} aria-expanded={peek} aria-controls="folder-cols"
        title={peek ? S.hideColumns : S.showColumns}><span class="ri"><Icon name="columns" size={18} /></span>{@render rl(S.folders)}</button>
    </nav>
  {/if}
  <div class="cols" id="folder-cols" bind:this={cols} class:peek>
    {#if !deep}
    <div class="colwrap">
      <nav class="col col1" aria-label={S.library}>
        <button class="row" class:on={view.t === 'next'} aria-current={view.t === 'next' ? 'page' : undefined} onclick={() => list('next')}>{S.nextUp}<small>{counts.next.toLocaleString('en')}</small></button>
        <button class="row" class:on={view.t === 'progress'} aria-current={view.t === 'progress' ? 'page' : undefined} onclick={() => list('progress')}>{S.inProgress}<small>{counts.progress.toLocaleString('en')}</small></button>
        <button class="row" class:on={view.t === 'done'} aria-current={view.t === 'done' ? 'page' : undefined} onclick={() => list('done')}>{S.done}<small>{counts.done.toLocaleString('en')}</small></button>
        <button class="row" class:on={view.t === 'added'} aria-current={view.t === 'added' ? 'page' : undefined} onclick={() => list('added')}>{S.recentlyAdded}</button>
        <button class="row" class:on={view.t === 'opened'} aria-current={view.t === 'opened' ? 'page' : undefined} onclick={() => list('opened')}>{S.recentlyOpened}</button>
        {#each byDrive as [drive, secs] (drive)}
          <div class="drive">{drive}{secs.every(s => !s.online && !s.gone) ? `, ${S.notConnected}` : ''}</div>
          {#each secs as s (s.id)}
            {@const n = countOf(s)}
            <button class="row" class:off={!s.online} class:none={!n} class:on={view.t === 'section' && view.id === s.id} style="--c:{color(s)}"
              aria-current={view.t === 'section' && view.id === s.id && !folder.length ? 'page' : undefined} title={[s.name, secTip(s)].filter(Boolean).join('\n')} onclick={() => openFolder(s, [])} oncontextmenu={e => openMenu(e, placeItems(s, []), s.name)}>
              <span class="dot"></span><span dir="auto">{glued(s.name)}</span>{#if s.gone}<small class="lost">{S.notFound}</small>{:else}<small>{n.toLocaleString('en')}</small>{/if}
            </button>
          {/each}
        {/each}
        {#if counts.unlisted}
          <button class="row unl" class:on={view.t === 'unlisted' && !view.id} aria-current={view.t === 'unlisted' && !view.id ? 'page' : undefined} onclick={() => list('unlisted')}>{S.unlistedTitle}<small>{counts.unlisted.toLocaleString('en')}</small></button>
        {/if}
      </nav>
      <Grip level="lib" def={COL1} />
    </div>
    {/if}
    {#each shownCols as c, j (c.key)}
      {#if section}
        <div class="colwrap" class:nav={deep && j === 0} class:start={j === paneCol}>
          <nav class="col" aria-label={S.folders}>
            {#if deep && j === 0}
              <button class="row" onclick={() => list('next')}>{S.nextUp}<small>{counts.next.toLocaleString('en')}</small></button>
              <button class="row back" onclick={() => openFolder(section, folder.slice(0, -1))}>‹ {S.back}</button>
            {/if}
            <h4 dir="auto" title={c.title}>{place(c.title)}</h4>
            {#each colLists(c) as list, li (li)}
            {#if li}<h4 class="oh">{S.othersHead}</h4>{/if}
            {#each list as f (f.name)}
              {@const here = folder[c.at.length] === f.name}
              {@const n = shownIn(f, c.at)}
              {#if f.link}
                <!-- A folder split out as a Section (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`): it opens that Section, in its colour. -->
                {@const to = f.link}
                <button class="row slink" class:off={!to.online && !to.gone} style="--c:{color(to)}" title={linkTip(to)} onclick={() => openFolder(to, [])} oncontextmenu={e => openMenu(e, linkItems(to, f.lid), to.name)}>
                  {@render linkIcon(to)}<span dir="auto">{place(f.name)}</span><small>{countOf(to).toLocaleString('en')}</small></button>
              {:else}
              <button class="row" class:none={!n} class:on={here} aria-current={here ? 'page' : undefined} style="--c:{color(section)}"
                title={[f.name, pathTip(section, [...c.at, f.name])].filter(Boolean).join('\n')} onclick={() => openFolder(section, [...c.at, f.name])} oncontextmenu={e => openMenu(e, placeItems(section, [...c.at, f.name]), place(f.name))}><span dir="auto">{place(f.name)}</span><small>{countLabel(f, c.at)}</small></button>
              {/if}
            {/each}
            {/each}
          </nav>
          <Grip level={c.key} def={COL2} />
        </div>
      {/if}
    {/each}
  </div>
  </div>

  <!-- The day panel lies over the pane's right side; the pane's content, toolbar included, keeps clear of it. -->
  <main bind:this={main} id="works" class="v-{ui.size}" class:start={paneCol < 0} tabindex="-1"
    style={photosView && photoDay && photoDay !== 'undated' ? `padding-right:calc(${panelWidth(ui.colw.photoday)} + 16px)` : undefined}>
    <!-- The address bar, as File Explorer's: each chevron lists what is in that step. -->
    {#snippet chev(at: string[] | null, name: string)}
      {@const key = addrKey(at)}
      <button class="chev" aria-haspopup="menu" aria-expanded={menuOpen(key)} aria-label={S.insideOf(name)} title={S.insideOf(name)} onclick={e => addrMenu(e, at)}><Icon name="chev" size={12} /></button>
    {/snippet}
    <nav class="addr" class:tight aria-label={S.address} bind:this={addr}>
      {#if fold}<button class="chev fold" aria-haspopup="menu" aria-expanded={menuOpen('addr:…')} aria-label={S.earlierSteps} title={S.earlierSteps} onclick={foldMenu}><Icon name="more" size={14} /></button>{/if}
      {#each steps.slice(fold) as st (st.key)}
        {#if st.go}<button class="step" dir="auto" title={st.title} onclick={() => { st.go?.(); addrPicked() }} oncontextmenu={e => st.menu && openMenu(e, st.menu(), st.label)}>{st.label}</button>{:else}<span class="step" role="presentation" aria-current={st.current ? 'page' : undefined} dir="auto" title={st.title} oncontextmenu={e => st.menu && openMenu(e, st.menu(), st.label)}>{st.label}</span>{/if}
        {#if st.chev !== undefined}{@render chev(st.chev, st.label)}{:else if st.sep}<span class="chev sep" aria-hidden="true"><Icon name="chev" size={12} /></span>{/if}
      {/each}
      {#if inFolder && section}<button class="dir" onclick={() => onreveal(diskPath, true)} title={S.openInExplorer} aria-label={S.openInExplorer}><Icon name="open" /></button>{/if}
      <div class="probe" aria-hidden="true" bind:this={probe}>
        {#each steps as st (st.key)}<span><span class="step" aria-current={st.current ? 'page' : undefined}>{st.label}</span>{#if st.chev !== undefined || st.sep}<span class="chev"></span>{/if}</span>{/each}
      </div>
    </nav>
    {#if photosView}
    <div class="bar">
        <div class="seg" role="group" aria-label={S.photoKinds}>
          {#each [['', S.photosAndClips], ['p', S.photosOnly], ['c', S.clipsOnly]] as [k, label] (k)}
            <button aria-pressed={ui.photoKind === k} onclick={() => setPhotos(k as typeof ui.photoKind, ui.photoAll)}>{label}</button>
          {/each}
        </div>
        <div class="seg" role="group" aria-label={S.bestOrAll} title={photoF ? S.bestOffInFilter : S.bestOnlyTip}>
          <button aria-pressed={!ui.photoAll} disabled={!!photoF} onclick={() => setPhotos(ui.photoKind, false)}>{S.bestOnly}</button>
          <button aria-pressed={ui.photoAll} disabled={!!photoF} onclick={() => setPhotos(ui.photoKind, true)}>{S.allPhotos}</button>
        </div>
        <div class="chips" role="group" aria-label={S.filters}>
          {#each FILTERS as f (f)}
            {@const n = filterCounts[f]}
            <button class="chip" aria-pressed={photoF === f} title={S.filterHelp[f]} onclick={() => (photoF = photoF === f ? '' : f)}>{S.filterName[f]} <small>{f === 'call' ? S.sets(n) : S.files(n)}</small></button>
          {/each}
        </div>
    </div>
    {/if}

    <!-- The title line: the page's title, then Filter, View and the page's menu, as File Explorer's command bar. -->
    <div class="tline" role="presentation" oncontextmenu={e => openMenu(e, pageItems(), heading)}>
      {#if onHome && !q}
        <!-- Home's title is its tab bar: the open tab is underlined. -->
        <h1 class="sr">{heading}</h1>
        <nav class="tabs" aria-label={S.home}>
          {#each TABS as [t, label] (t)}
            {@const n = t === 'added' || t === 'opened' ? null : counts[t]}
            <button class:on={view.t === t} aria-current={view.t === t ? 'page' : undefined} onclick={() => list(t)}>{label}{#if n !== null}<small>{n.toLocaleString('en')}</small>{/if}</button>
          {/each}
        </nav>
      {:else}
        <h1 dir="auto">{heading}</h1>
      {/if}
      <span class="cmds">
        {#if !ev && view.t !== 'unlisted'}
          <button class="cmd" class:act={filterWords.length > 0} aria-haspopup="menu" aria-expanded={menuOpen('filter')} onclick={filterMenu}>
            <Icon name="filter" /><span class="cl">{filterWords.length ? S.filterOn(filterWords) : S.filter}</span><Icon name="down" size={12} /></button>
        {/if}
        {#if !photosView && view.t !== 'unlisted'}
          <button class="cmd" aria-haspopup="menu" aria-expanded={menuOpen('view')} onclick={viewMenu}><Icon name="view" /><span class="cl">{S.view}</span><Icon name="down" size={12} /></button>
        {/if}
        <button class="cmd icon" aria-haspopup="menu" aria-expanded={menuOpen('page')} aria-label={S.moreFor(scope ? (scope.f.length ? place(scope.f.at(-1)!) : glued(scope.s.name)) : heading)} title={S.more}
          onclick={e => openMenu(e, pageItems(), heading, 'page')}><Icon name="more" /></button>
      </span>
    </div>
    <div class="sub">
      {#if view.t === 'unlisted'}
        <span>{S.files(files.length)}</span>
        {#each byReason as { r, n } (r)}
          <button class="chip" aria-pressed={reason === r} onclick={() => (reason = reason === r ? '' : r)}>{S.reason[r]} <small>{n.toLocaleString('en')}</small></button>
        {/each}
      {:else if photosView}
        <span>{photoLine}</span>
        {#if photoF}<button class="chip on" onclick={() => (photoF = '')}>{S.clearFilter} ×</button>{/if}
      {:else if inFolder && tiles.length && !everyBelow}
        <!-- With a filter on, the counts are of what passes it, as Show all then lists. -->
        {@const here = filtered ? listed.length : own.length}
        {@const rest = filtered ? tiles.reduce((n, t) => n + t.works.length, 0) : below.length - own.length}
        <span>{S.hereAndBelow(here, rest, tiles.length, ev, ev ? (pfolder.length ? 'month' : 'year') : 'subfolder')}</span>
        <button class="chip" onclick={() => section && go({ t: 'section', id: section.id, folder, all: true })}>{S.showAll(here + rest, ev)}</button>
      {:else}
        <span>{bareNext ? S.emptyNext : view.t === 'added' && !filtered ? S.newest(listed.length) : view.t === 'opened' && !filtered && listed.length ? S.openedLast(listed.length) : S.works(listed.length, ev && listed.every(({ w }) => w.event))}</span>
      {/if}
      {#if chosen.length}<button class="chip on" onclick={() => (sel = new Set())} aria-label={S.clearSelection}>{S.selectedN(chosen.length)} ×</button>{/if}
      <span class="vh" role="status">{chosen.length ? S.selectedN(chosen.length) : selSeen ? S.noneSelected : ''}</span>
      {#if view.t === 'section' && view.all && !flat}
        <button class="chip on" onclick={() => section && go({ t: 'section', id: section.id, folder })} aria-label={S.onlyThisFolder}>{S.allWorksX}</button>
      {/if}
      {#if reviewHere && !filtered && !photoF && !(photosView && pfolder.length === 2)}
        {#if !reviewHere.done}<button class="chip rv" title={S.reviewTip(reviewHere.n)} onclick={() => onreview(below, place(folder.at(-1) ?? section?.name ?? ''), photosView ? lens.reviewOnly : undefined)}>{S.reviewN(reviewHere.n, photosView ? lens.k : '')}</button>
        {:else}<span class="ok">{S.reviewDone}</span>{/if}
      {/if}
      {#if asideAll && !filtered}
        {#if asideHere}<span title={S.setAsideTip}>{S.setAsideN(asideHere)}</span>{/if}
        {#if scriptPlace}<button class="chip" title={S.moveAsideTip} onclick={() => onmove(scriptPlace!, moveScope)}>{S.moveAside}</button>{/if}
      {:else if lastMove && scriptPlace && !filtered}
        <button class="chip" title={S.lastMove(lastMove[2], lastMove[1], lastMove[3])} onclick={() => onmove(scriptPlace!, moveScope)}>{S.undoLast}</button>
      {/if}
      {#if unlistedHere && section}
        <button class="chip" onclick={() => openUnlisted(section!, folder)}>{S.notOnCardN(unlistedHere)}</button>
      {/if}
      {#if ev && shotsLeft && !filtered}<span class="live">{S.readingPhotos(shotsLeft)}</span>{/if}
      {#if q}<button class="chip on" onclick={() => (query = '')} aria-label={S.clearSearch}>{S.searchChip(q)} ×</button>{/if}
      {#if q && inFolder}<button class="chip" onclick={() => go({ t: 'next' })}>{S.searchEverywhere}</button>{/if}
      {#if medium}<button class="chip on" onclick={() => (medium = '')} aria-label={S.stopOnly(S.mediaName[medium])}>{S.mediaName[medium]} ×</button>{/if}
      {#if markF}<button class="chip on" onclick={() => (markF = '')} aria-label={S.stopMarkFilter}>{markF === 'next' ? S.next : S.done} ×</button>{/if}
      {#if [q, medium, markF].filter(Boolean).length > 1}<button class="chip" onclick={clearAll}>{S.clearAll}</button>{/if}
    </div>

    {#if inFolder && section && !section.online}
      {@const from = fromOf(section)}
      <div class="notice" role="status">
        <p dir="auto">{section.gone ? from ? S.splitGoneWhat(section.path, from.name) : S.goneWhat(section.path) : S.offlineWhat(section.scanned_at > 0, section.drive)}</p>
        {#if section.gone && !from}<button class="btn" onclick={() => onsettings('folders')}>{S.tickNewPlace}</button>{/if}
        <!-- Its drive back in, the Section comes back by reading it. -->
        {#if !section.gone}<button class="btn" disabled={reading} onclick={() => onreread(section!, [])}>{S.rereadSection}</button>{/if}
        <!-- The note is about the Section, also in one of its folders: so is what it removes. -->
        <button class="btn" onclick={() => onforget(section)}>{S.removeSection}</button>
      </div>
    {/if}
    {#if inFolder && section?.online}
      <!-- Merged Sections (`decisions/2026/10/05/checked-folders-on-any-drive.md`): the one not connected among them says so; the others are read. -->
      {#each awayOf(section) as x (x.id)}
        <div class="notice" role="status"><p dir="auto">{x.id === section.id ? S.offlineWhat(x.scanned_at > 0, x.drive) : S.partAwayWhat(x.name, x.drive)}</p></div>
      {/each}
    {/if}
    {#each inFolder && offersAt === offerTop ? offers : [] as o (o.s.id)}
      {@const wait = reading || answering || !o.count}
      <section class="notice offer" aria-label={S.offerList(o.s.name)} aria-busy={answering}>
        <p dir="auto">{S.offerWhat(o.s.name, KINDS[o.s.kind].name)}</p>
        <ul>
          {#each o.list as x (x.rel)}
            <li><Icon name="folder" size={16} /><b dir="auto" title={x.rel}>{place(x.title)}</b><span>{KINDS[x.kind].name} · {S.files(x.files)}</span></li>
          {/each}
        </ul>
        <p class="count" role="status">{#if o.count === undefined}{S.countWait}{:else if o.count}{S.foldersCards(o.count.total)} {S.remakeNote(o.count.marks, o.count.back)}{:else}{S.countFailed} <button class="again" onclick={() => countOffer(o, false, () => offers.includes(o))}>{S.countAgain}</button>{/if}</p>
        <button class="btn primary" disabled={wait} title={reading ? S.waitForRead : undefined} onclick={() => answer(o, 'kinds')}>{S.setKindEach}</button>
        <button class="btn" disabled={wait} title={reading ? S.waitForRead : undefined} onclick={() => answer(o, 'sections')}>{S.makeEachSection}</button>
        <button class="btn" disabled={answering} onclick={() => answer(o, 'away')}>{S.keepOneKind}</button>
      </section>
    {/each}
    {#snippet backup()}
      <div class="notice" role="status">
        <p>{S.backupAsk}</p>
        <button class="btn" onclick={() => { ui.backupSeen = true; saveUi() }}>{S.backupSeen}</button>
      </div>
    {/snippet}
    {#if view.t === 'next' && !ui.backupSeen && !bareNext}{@render backup()}{/if}
    {#snippet card(w: Work, s: Section)}
      {@const kind = kindOfWork(w, s)}
      <!-- A book outside a Books Section has the Books card too, even in a grid of wide pictures;
           a PDF of wide slides keeps its wide box. -->
      {@const th = thumbOf(w)}
      {@const book = kind !== 'Photos' && isBook(w) && (kind === 'Books' || !th || th.h > th.w)}
      <div class="cell" role="presentation" style:--ratio={book && !tall ? '3/4' : null} oncontextmenu={e => cardMenu(e, w, s)} onclickcapture={e => pick(e, w, s)} onmousedown={e => { if (e.shiftKey) e.preventDefault() }} onkeydown={e => pickKey(e, w, s)}>
      <Card work={w} known={kind === 'Books' ? authorsOf(s) : book ? authorsOf(s, false) : null} color={color(s)} section={s.name} mark={marks.marks[w.id]} review={w.event ? reviewState([w], notes, marks) : null} selected={sel.size > 0 && sel.has(wkey(w, s))} ticks={ticksOf(w, marks)} thumb={th} {details} tall={tall || book}
        compact={ui.size === 'compact'} path={!inFolder || everyBelow} from={inFolder ? shownSteps(w) : 0} band={!inFolder}
        onopen={() => onopen(w, s)} ondialog={() => onwork(w.id)} onmark={m => mark(w, m)} onfile={p => onfile(p, w)} ongo={f => openFolder(s, f)} onchoosecover={() => onchoosecover(coverOfWork(w))} {onremovecover} onreveal={() => onreveal(shownOf(w))} />
      </div>
    {/snippet}
    {#if photosView && section}
      <!-- A Photos folder's events with their folders from its top, so years and months are as a Photos Section's (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`). -->
      {@const root = photoRoot ?? []}
      <Photos works={photoWorks} folder={pfolder} day={photoDay} color={color(section)} {thumbs} {details} {lens} {notes} {marks}
        onopen={(f, d, r) => openFolder(section, [...root, ...f], d, r)} onday={setDay} {onreview} {onfile} {onreveal} {onagain} onwant={ids => (photoWant = ids)}
        online={section.online} menus={{ place: f => placeItems(section!, [...root, ...f]), event: (w, open, enter) => cardItems(w, section!, { open, enter }), review: reviewItems }} />
    {:else if folderTiles && tiles.length && section}
      <div class={ui.size === 'compact' ? 'rows' : 'grid'} class:tall>
        {#each tiles as t (t.name)}
          {@const f = [...folder, t.name]}
          {#if t.link}
            {@const to = t.link}
            <div class="cell" role="presentation" oncontextmenu={e => openMenu(e, linkItems(to, t.lid), to.name)}>
              <FolderTile name={place(t.name)} ev={to.kind === 'Photos'} sub={to.kind === 'Photos' ? 'year' : 'folder'} works={tileWorks(to)} subs={topFolders(to)} color={color(to)} {marks} {thumbs} cover={thumbs[folderCoverId(to.id, [])]} compact={ui.size === 'compact'}
                path={linkTip(to)} link={KIND_ICON[to.kind]} off={!to.online && !to.gone} onopen={() => openFolder(to, [])} />
            </div>
          {:else}
          <div class="cell" role="presentation" oncontextmenu={e => openMenu(e, placeItems(section, f), place(t.name))}>
            <FolderTile name={place(t.name)} {...evAt(f)} works={filtered ? t.works : t.every} subs={t.subs} color={color(section)} {marks} {thumbs} cover={thumbs[folderCoverId(section.id, f)]} compact={ui.size === 'compact'}
              path={pathTip(section, f)} onopen={() => openFolder(section, f)} />
          </div>
          {/if}
        {/each}
      </div>
    {:else if rows.length && section}
      <!-- Subfolders as expander rows, as Windows 11 Settings': Open and ⋯ first, faded until
           pointed at, then the folder; a click anywhere else on the row opens or closes it. -->
      <div class="exps" style="--c:{color(section)}">
        {#each rows as r (okey(section, r.f))}
          {@const name = place(r.t.name)}
          {@const key = 'row:' + r.f.join('/')}
          {#if r.t.link}
            <!-- A folder split out as a Section (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`): a row in that Section's colour with its icon; it opens that Section. -->
            {@const to = r.t.link}
            <div class="exp slink" class:off={!to.online && !to.gone} style="--c:{color(to)}">
              <!-- svelte-ignore a11y_click_events_have_key_events -->
              <div class="exph" role="presentation" oncontextmenu={e => openMenu(e, linkItems(to, r.t.lid), to.name)} onclick={e => !(e.target as HTMLElement).closest('button') && openFolder(to, [])}>
                <button class="ib" title={S.open} aria-label={S.openNamed(to.name)} onclick={() => openFolder(to, [])}><Icon name="launch" /></button>
                <button class="ib" title={S.more} aria-label={S.moreFor(to.name)} aria-haspopup="menu" aria-expanded={menuOpen(key)} onclick={e => openMenu(e, linkItems(to, r.t.lid), to.name, key)}><Icon name="more" /></button>
                <button class="exphit" title={linkTip(to)} onclick={() => openFolder(to, [])}>
                  <span class="fic">{@render linkIcon(to, 18)}</span>
                  <span class="nm"><b dir="auto">{name}</b><small>{rowLine(r.t)}</small></span>
                </button>
              </div>
            </div>
          {:else}
          <div class="exp" class:open={r.open} class:fresh={fresh.has(r.f.join('/'))} style:contain-intrinsic-size="{rowGuess(r)}px"
            data-row={okey(section, r.f)} use:nearby onfocusin={() => near.add(okey(section, r.f))}>
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <div class="exph" role="presentation" oncontextmenu={e => openMenu(e, rowItems(section, r.f), name)}
              onclick={e => !(e.target as HTMLElement).closest('button') && setOpen(section, [r.f], !r.open)}>
              <button class="ib" title={S.open} aria-label={S.openNamed(name)} onclick={() => openFolder(section, r.f)}><Icon name="launch" /></button>
              <button class="ib" title={S.more} aria-label={S.moreFor(name)} aria-haspopup="menu" aria-expanded={menuOpen(key)} onclick={e => openMenu(e, rowItems(section, r.f), name, key)}><Icon name="more" /></button>
              <button class="exphit" aria-expanded={r.open} title={pathTip(section, r.f)} onclick={() => setOpen(section, [r.f], !r.open)}>
                <span class="fic"><Icon name={r.open ? 'open' : 'folder'} size={18} /></span>
                <span class="nm"><b dir="auto">{name}</b><small>{rowLine(r.t)}</small></span>
              </button>
            </div>
            {#if r.open}
              <div class="expb">
                {#if !near.has(okey(section, r.f))}
                  <!-- Made once the row comes near the view. -->
                {:else if r.subs.length || r.mine.length}
                  <div class={ui.size === 'compact' ? 'rows' : 'grid'} class:tall>
                    {#each r.subs as u (u.name)}
                      {@const f = [...r.f, u.name]}
                      <div class="cell" role="presentation" oncontextmenu={e => openMenu(e, placeItems(section, f), place(u.name))}>
                        <FolderTile name={place(u.name)} {...evAt(f)} works={filtered ? u.works : u.every} subs={u.subs} color={color(section)} {marks} {thumbs} cover={thumbs[folderCoverId(section.id, f)]} compact={ui.size === 'compact'}
                          path={pathTip(section, f)} onopen={() => openFolder(section, f)} />
                      </div>
                    {/each}
                    {#each r.mine.slice(0, ROW_CARDS) as w (w.id)}{@render card(w, section)}{/each}
                  </div>
                {:else if r.files.length}
                  <!-- A folder that holds only unlisted files (shown with View's switch) lists them, as its page does. -->
                  <ul class="rowfiles">{#each r.files.slice(0, 500) as u (u.path)}{@render fileRow(u, section, true)}{/each}</ul>
                {:else}
                  <p class="nothing">{S.emptyFolder}</p>
                {/if}
                {#if r.mine.length > ROW_CARDS}<button class="link moreln" onclick={() => openFolder(section, r.f)}>{S.openForAll(r.mine.length)}</button>{/if}
              </div>
            {/if}
          </div>
          {/if}
        {/each}
      </div>
    {/if}
    {#if tiles.length && shown.length && !photosView}<h2 class="here">{S.inThisFolder}</h2>{/if}
    {#if shown.length && !photosView}
      <div class={ui.size === 'compact' ? 'rows' : 'grid'} class:tall>
        {#each shown as { w, s } (wkey(w, s))}
          {@render card(w, s)}
        {/each}
      </div>
      {#if listed.length > limit}<button class="btn more" onclick={() => (limit += CHUNK)}>{S.showMore(Math.min(CHUNK, listed.length - limit), listed.length - limit)}</button>{/if}
    {:else if !tiles.length && !files.length && !photosView && !bareNext}
      <div class="empty">
        {#if filtered}
          <p>{q ? S.noResults(q) : S.nothingFilters}</p>
          {#if q && inFolder}<button class="btn" onclick={() => go({ t: 'next' })}>{S.searchEverywhere}</button>{/if}
          <button class="btn" onclick={clearAll}>{S.clearFilters}</button>
        {:else if view.t === 'next'}
          <p>{S.emptyNext}</p>
          <button class="btn" onclick={() => go({ t: 'added' })}>{S.seeRecent}</button>
        {:else if view.t === 'progress'}
          <p>{S.emptyProgress}</p>
        {:else if view.t === 'done'}
          <p>{S.emptyDone}</p>
        {:else if view.t === 'opened'}
          <p>{S.emptyOpened}</p>
        {:else if view.t === 'unlisted'}
          <p>{S.everyFileOnCard}</p>
        {:else if inFolder && section?.online}
          <p>{S.emptyFolder}</p>
        {:else}
          <p>{S.empty}</p>
        {/if}
      </div>
    {/if}

    {#if homeFolders}
      <h2 class="yf">{S.yourFolders}</h2>
      <div class={ui.size === 'compact' ? 'rows' : 'grid'}>
        {#each inOrder as s (s.id)}
          {@const photos = s.kind === 'Photos'}
          <div class="cell" role="presentation" oncontextmenu={e => openMenu(e, placeItems(s, []), s.name)}>
            <FolderTile name={glued(s.name)} ev={photos} sub={photos ? 'year' : 'folder'} works={tileWorks(s)} subs={topFolders(s)} color={color(s)} {marks} {thumbs} cover={thumbs[folderCoverId(s.id, [])]} compact={ui.size === 'compact'}
              path={secTip(s)} off={!s.online && !s.gone} onopen={() => openFolder(s, [])} />
          </div>
        {/each}
      </div>
    {/if}
    {#if bareNext && recent.length}
      <h2 class="here">{S.recentlyAdded}<button class="chip" onclick={() => list('added')}>{S.seeAll}</button></h2>
      <div class={ui.size === 'compact' ? 'rows' : 'grid'} class:tall bind:clientWidth={recentW} bind:this={recentEl}>
        {#each recent as { w, s } (wkey(w, s))}{@render card(w, s)}{/each}
      </div>
    {/if}
    {#if bareNext && !ui.backupSeen}<div class="later">{@render backup()}</div>{/if}

    {#snippet fileRow(u: Unlisted, s: Section, tag: boolean)}
      <li>
        <button class="link" dir="auto" onclick={() => onfile(u.path)} title={u.path}>{u.name}</button>
        {#if tag}<span class="tag {u.reason}">{S.reason[u.reason]}</span>{/if}
        <span class="muted">{size(u.size)}</span>
        {#if u.work && workTitle(u.work)}<button class="in" dir="auto" onclick={() => onwork(u.work!)}>{S.inWork(workTitle(u.work))}</button>{/if}
        {#if q && calendarIn(s, u.folder)}<span class="where" dir="auto">{[s.name, ...u.folder].join(' › ')}</span>{:else if q}<button class="where" dir="auto" onclick={() => openFolder(s, u.folder)}>{[s.name, ...u.folder].join(' › ')}</button>{/if}
      </li>
    {/snippet}
    {#if view.t === 'unlisted'}
      <!-- A table by reason, as Settings, Programs groups its types: each reason's group says what it means. -->
      <section class="unlisted" aria-label={S.unlistedTitle}>
        {#if files.length}
          <table class="ftable">
            <colgroup><col /><col class="c-size" /><col class="c-where" /></colgroup>
            <thead><tr><th>{S.colFile}</th><th class="num">{S.colSize}</th><th>{S.colFolder}</th></tr></thead>
            {#each REASONS as r (r)}
              {@const group = files.slice(0, limit).filter(({ u }) => u.reason === r)}
              {#if group.length}
                <tbody>
                  <tr class="group"><th colspan="3" scope="rowgroup">{S.reason[r]} <span class="n">{files.filter(({ u }) => u.reason === r).length.toLocaleString('en')}</span>
                    <span class="why">{S.reasonWhat[r]}</span></th></tr>
                  {#each group as { u, s } (u.path)}
                    <tr>
                      <td><button class="link" dir="auto" onclick={() => onfile(u.path)} title={u.path}>{u.name}</button>
                        {#if u.work && workTitle(u.work)}<button class="in" dir="auto" onclick={() => onwork(u.work!)}>{S.inWork(workTitle(u.work))}</button>{/if}</td>
                      <td class="num">{size(u.size)}</td>
                      <!-- A Photos folder shows as years and months, not as a folder of its own: its name is only text. -->
                      <td>{#if calendarIn(s, u.folder)}<span class="where" dir="auto">{whereOf(u, s)}</span>{:else}<button class="where" dir="auto" onclick={() => openFolder(s, u.folder)}>{whereOf(u, s)}</button>{/if}</td>
                    </tr>
                  {/each}
                </tbody>
              {/if}
            {/each}
          </table>
        {/if}
        {#if files.length > limit}<button class="btn more" onclick={() => (limit += CHUNK * 4)}>{S.showMore(Math.min(CHUNK * 4, files.length - limit), files.length - limit)}</button>{/if}
      </section>
    {:else if files.length}
      <section class="unlisted" aria-label={S.unlistedTitle}>
        <h2>{S.unlistedHere(files.length)}</h2>
        <ul>{#each files.slice(0, 500) as { u, s } (u.path)}{@render fileRow(u, s, true)}{/each}</ul>
      </section>
    {/if}
  </main>
</div>

<style>
  .body { display: grid; grid-template-columns: auto 1fr; min-height: 0; position: relative; --rail: 84px; }
  .cols { display: grid; grid-template-columns: var(--cols); min-height: 0; }
  .side { position: relative; display: flex; min-height: 0; }
  .hidden .cols { display: none; }
  .hidden .cols.peek { display: grid; }
  .hidden:not(.push) .cols { position: absolute; top: 0; bottom: 0; left: var(--rail); z-index: 5; box-shadow: 8px 0 24px rgba(0,0,0,.18); }
  /* The rail, as Outlook's and File Explorer's: an icon with its name under it; the open one is raised with an accent bar. */
  .rail { width: var(--rail); display: flex; flex-direction: column; align-items: center; gap: 2px; padding: 6px 0 8px; background: var(--side); min-height: 0; }
  .rsecs { flex: 1; min-height: 0; width: 100%; overflow: hidden auto; scrollbar-width: none; display: flex; flex-direction: column; align-items: center; gap: 2px; padding: 9px 0 6px; position: relative; }
  .rsecs::before { content: ""; flex: none; width: 24px; height: 1px; background: var(--line); margin: -5px 0 4px; }
  .rb { position: relative; flex: none; width: 76px; border: 0; background: none; padding: 4px 5px 5px; border-radius: 7px; display: flex; flex-direction: column; align-items: center; gap: 2px;
    color: var(--soft); transition: background .12s, color .12s, box-shadow .12s; }
  .rb:hover { background: var(--hover); color: var(--ink); }
  .rb:focus-visible { outline-offset: -2px; border-radius: 7px; }
  .rail.quiet .rb:focus-visible { outline: none; }
  .ri { width: 36px; height: 28px; display: grid; place-items: center; border-radius: 6px; }
  .rl { display: grid; max-width: 100%; font-size: 11px; line-height: 1.25; text-align: center; }
  .rl > span, .rl::after { grid-area: 1 / 1; text-overflow: ellipsis; display: -webkit-box; -webkit-box-orient: vertical; -webkit-line-clamp: 2; line-clamp: 2; overflow: hidden; }
  .rl::after { content: attr(data-l); font-weight: 600; visibility: hidden; }
  .rb.sec .ri { color: var(--c); background: color-mix(in srgb, var(--c) 15%, transparent); }
  /* A Section whose drive is not connected: greyed as a button that is off (.btn:disabled);
     what is inside it, so the focus ring keeps its strength. */
  .rb.off > * { opacity: .45; }
  .rb.on { background: var(--field); color: var(--ink); box-shadow: 0 1px 3px rgba(0, 0, 0, .1); }
  .rb.on .rl > span { font-weight: 600; }
  .rb.on::before { content: ""; position: absolute; left: -4px; top: 50%; width: 3px; height: 18px; margin-top: -9px; border-radius: 2px; background: var(--c, var(--focus)); }
  .colwrap { position: relative; min-height: 0; display: flex; border-right: 1px solid var(--line); background: var(--pane); }
  .colwrap:has(.col1), .colwrap.nav { background: var(--side); border-right-color: transparent; }
  .start { border-top-left-radius: 10px; border-top: 1px solid var(--line); border-left: 1px solid var(--line); }
  .hidden:not(.push) .cols .colwrap { border-radius: 0; }
  .col { flex: 1; min-width: 0; overflow: auto; padding: 10px 8px; }
    h4 { margin: 12px 10px 4px; font-size: 12px; font-weight: 600; color: var(--muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .row { display: flex; align-items: center; gap: 9px; width: 100%; border: 0; background: none; text-align: left; padding: 6px 10px; border-radius: 6px; }
  .row:hover { background: var(--hover); }
  .row { position: relative; }
  .row.on { background: var(--field); font-weight: 600; }
  .row.on::before { content: ""; position: absolute; left: 0; top: 50%; width: 3px; height: 16px; margin-top: -8px; border-radius: 2px; background: var(--c, var(--focus)); }
  .colwrap:not(:has(.col1)):not(.nav) .row.on { background: var(--sel); }
  .row small { margin-left: auto; color: var(--muted); font-variant-numeric: tabular-nums; font-weight: 400; }
  .row.none { color: var(--muted); }
  .row.off > *, .exp.off .exph button > * { opacity: .45; }
  /* A name stays on one line and ends in an ellipsis; the row's hover shows it whole. */
  .row span[dir] { min-width: 0; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .row small.lost { color: var(--danger); font-weight: 600; }
  .notice { display: flex; flex-wrap: wrap; gap: 8px 10px; align-items: center; margin-top: 14px; padding: 10px 14px; border-radius: 8px;
    background: color-mix(in srgb, var(--next) 35%, var(--field)); border: 1px solid color-mix(in srgb, var(--next) 70%, var(--line)); }
  .notice p { margin: 0; flex: 1 1 100%; max-width: 80ch; }
  /* The offer of a kind per folder: a row per folder, its name wrapping, its kind and files beside it. */
  .offer ul { list-style: none; margin: 0 0 2px; padding: 0; flex: 1 1 100%; display: flex; flex-direction: column; gap: 4px; }
  .offer li { display: flex; flex-wrap: wrap; align-items: baseline; gap: 2px 10px; padding-left: 24px; position: relative; }
  .offer li > :global(svg) { position: absolute; left: 0; top: 2px; color: var(--soft); }
  .offer li b { flex: 0 1 auto; min-width: 0; font-weight: 600; overflow-wrap: anywhere; }
  .offer li span { color: var(--soft); font-size: 12.5px; white-space: nowrap; }
  .offer .count { color: var(--soft); font-size: 12.5px; }
  .offer .again { border: 0; background: none; padding: 0; color: var(--link); font: inherit; }
  .offer .again:hover { text-decoration: underline; }
  .row.back { color: var(--link); font-weight: 600; }
  /* A Split-out Section in a folder column: its icon in its colour. */
  .row.slink > .li { color: var(--c); }
  .li { position: relative; flex: none; display: inline-grid; place-items: center; }
  .li .sc { position: absolute; right: -5px; bottom: -4px; display: grid; place-items: center; width: 13px; height: 13px; border-radius: 4px; background: var(--field); color: var(--c);
    box-shadow: 0 0 0 1px color-mix(in srgb, var(--c) 45%, var(--line)); }
  .exp.slink .fic { color: var(--c); }
  .row.unl { margin-top: 14px; color: var(--soft); }
  .dot { width: 9px; height: 9px; border-radius: 3px; background: var(--c); flex: none; }
  .drive { margin: 16px 10px 4px; font-size: 12px; color: var(--muted); overflow-wrap: anywhere; }
  main { overflow: auto; padding: 14px 28px 40px; min-width: 0; outline: none; background: var(--pane); }
  .bar { display: flex; gap: 10px; align-items: center; flex-wrap: wrap; margin-bottom: 14px; }
  .seg { display: inline-flex; border: 1px solid var(--line); border-radius: 7px; background: var(--field); }
  .seg button { border: 0; background: none; padding: 3px 11px; font-size: 13px; position: relative; }
  .seg button:first-child { border-radius: 6px 0 0 6px; }
  .seg button:last-child { border-radius: 0 6px 6px 0; }
  .seg button:focus-visible { outline-offset: -2px; z-index: 1; border-radius: 6px; }
  .seg button + button { border-left: 1px solid var(--line); }
  .seg button:disabled { opacity: .5; }
  .seg button[aria-pressed='true'] { background: var(--sel); color: var(--link); font-weight: 600; box-shadow: inset 0 0 0 1px var(--sel-line); }
  /* The address bar, as File Explorer's: one rounded field; the first steps fold into … when the path does not fit. */
  .addr { position: relative; display: flex; align-items: center; gap: 0 1px; min-height: 32px; padding: 0 4px 0 6px; margin-bottom: 12px; border: 1px solid var(--line); border-radius: 6px;
    background: var(--field); color: var(--soft); font-size: 13px; white-space: nowrap; overflow: hidden; }
  .addr :focus-visible { outline-offset: -2px; }
  .step { flex: none; max-width: 22em; overflow: hidden; text-overflow: ellipsis; border: 0; background: none; padding: 3px 6px; border-radius: 4px; color: inherit; font-size: inherit; }
  span.step[aria-current] { max-width: none; }
  .tight span.step[aria-current] { flex: 0 1 auto; min-width: 2em; }
  .probe { position: absolute; left: 0; top: 0; visibility: hidden; pointer-events: none; display: flex; gap: 1px; }
  .probe > span { display: flex; gap: 1px; flex: none; }
  span.step[aria-current] { color: var(--ink); }
  .chev, .addr .dir { flex: none; display: grid; place-items: center; height: 24px; border: 0; background: none; padding: 0; border-radius: 4px; color: var(--muted); }
  .chev { width: 20px; }
  .chev.fold { width: 26px; }
  .chev :global(.icon) { transition: transform .15s; }
  .chev:not(.fold)[aria-expanded='true'] :global(.icon) { transform: rotate(90deg); }
  .addr .dir { width: 26px; margin-left: 4px; }
  .addr button.step:hover, .chev:not(.sep):hover, .chev[aria-expanded='true'], .addr .dir:hover { background: var(--hover); color: var(--ink); }
  /* Home's tab bar: the open tab bold, with the accent line under it. */
  .tabs { display: flex; flex-wrap: wrap; gap: 2px; margin: 0 0 4px -10px; min-width: 0; }
  .tabs button { position: relative; display: inline-flex; align-items: baseline; gap: 6px; border: 0; background: none; padding: 4px 10px 9px; border-radius: 6px;
    font-size: 19px; color: var(--soft); transition: background .12s, color .12s; }
  .tabs button:hover { background: var(--hover); color: var(--ink); }
  .tabs small { font-size: 12.5px; color: var(--muted); font-variant-numeric: tabular-nums; font-weight: 400; }
  .tabs button.on { color: var(--ink); font-weight: 700; }
  .tabs button.on::after { content: ""; position: absolute; left: 50%; bottom: 2px; width: 18px; height: 3px; margin-left: -9px; border-radius: 2px; background: var(--focus); animation: grow .18s ease-out; }
  @keyframes grow { from { width: 6px; margin-left: -3px; } }
  .sr { position: absolute; width: 1px; height: 1px; overflow: hidden; clip-path: inset(50%); white-space: nowrap; }
  .yf { font-size: 15px; color: var(--soft); margin: 34px 0 -4px; }
  .later { margin-top: 34px; }
  .tline { display: flex; align-items: flex-start; gap: 4px 12px; }
  .cmds { margin-left: auto; display: flex; gap: 4px; align-items: center; flex: none; max-width: 60%; padding-top: 2px; }
  .cmd { display: inline-flex; align-items: center; gap: 7px; height: 32px; border: 1px solid transparent; background: none; border-radius: 6px; padding: 0 10px; font-size: 13px; color: var(--ink);
    min-width: 0; transition: background .12s, border-color .12s; }
  .cmd :global(.icon) { color: var(--soft); }
  .cmd .cl { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .cmd:hover, .cmd[aria-expanded='true'] { background: var(--hover); border-color: var(--line); }
  .cmd.icon { width: 32px; padding: 0; justify-content: center; }
  .cmd.act { background: var(--sel); color: var(--link); border-color: var(--sel-line); font-weight: 600; }
  .cmd.act :global(.icon) { color: var(--link); }
  .cell { display: contents; }
  h1 { margin: 2px 0 6px; min-width: 0; font-size: 26px; font-weight: 700; letter-spacing: -.01em; overflow-wrap: anywhere; }
  .vh { position: absolute; width: 1px; height: 1px; overflow: hidden; clip-path: inset(50%); white-space: nowrap; }
  .sub { display: flex; gap: 8px 12px; color: var(--soft); font-size: 13px; align-items: center; flex-wrap: wrap; }
  .link { border: 0; background: none; color: var(--link); padding: 0; font-size: 13px; text-align: left; }
  .link:hover { text-decoration: underline; }
  .chip { border: 1px solid var(--line); border-radius: 999px; padding: 1px 10px; background: var(--field); font-size: 13px; }
  .chip:hover { background: var(--hover); }
  .chips { display: flex; gap: 6px; flex-wrap: wrap; }
  .chip small { font-weight: 400; opacity: .8; }
  .chip.on, .chip[aria-pressed='true'] { background: var(--sel); color: var(--link); border-color: var(--sel-line); font-weight: 600; }
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(200px, 1fr)); gap: 26px 18px; margin-top: 20px; align-items: start; }
  .grid.tall { grid-template-columns: repeat(auto-fill, minmax(150px, 1fr)); --ratio: 3/4; --lines: 5; }
  .v-large .grid { grid-template-columns: repeat(auto-fill, minmax(320px, 1fr)); gap: 34px 24px; --gen: 21px; --lines: 5; }
  .v-large .grid.tall { grid-template-columns: repeat(auto-fill, minmax(230px, 1fr)); }
  .rows { margin-top: 14px; max-width: 1200px; }
  /* Subfolder rows, as Windows 11 Settings' expanders: a raised row; open, its body sits on the page's colour under a line. */
  .exps { display: flex; flex-direction: column; gap: 6px; margin-top: 18px; }
  /* A row out of sight is not laid out: 2,000 rows opened at once with Expand all took seconds. */
  .exp { border: 1px solid var(--line); background: var(--field); border-radius: 8px; transition: box-shadow .15s; content-visibility: auto; contain-intrinsic-size: 62px; }
  .exp:hover { box-shadow: 0 1px 5px rgba(0, 0, 0, .07); }
  .exph { display: flex; align-items: center; gap: 2px; min-height: 60px; padding: 4px 6px 4px 8px; border-radius: 7px; cursor: pointer; transition: background .12s; }
  .exph:hover { background: var(--hover); }
  .exp.open .exph { border-radius: 7px 7px 0 0; }
  .ib { flex: none; display: grid; place-items: center; width: 32px; height: 32px; border: 0; background: none; border-radius: 6px; color: var(--soft); opacity: .6; transition: opacity .12s, background .12s; }
  .exph:hover .ib, .ib:focus-visible, .ib[aria-expanded='true'] { opacity: 1; }
  .ib:hover, .ib[aria-expanded='true'] { background: var(--hover); color: var(--ink); }
  .ib:focus-visible { outline-offset: -2px; }
  /* The row's button rings the whole row, as Settings' expanders do. */
  .exphit:focus-visible { outline: none; }
  .exph:has(.exphit:focus-visible) { outline: 2px solid var(--focus); outline-offset: -2px; }
  .exphit { flex: 1; min-width: 0; display: flex; align-items: center; gap: 14px; border: 0; background: none; text-align: left; padding: 8px 10px; border-radius: 6px; color: inherit; cursor: pointer; }
  .fic { flex: none; display: grid; place-items: center; width: 36px; height: 36px; border-radius: 8px; color: var(--c); background: color-mix(in srgb, var(--c) 15%, transparent); }
  .nm { display: flex; flex-direction: column; min-width: 0; }
  .nm b { font-size: 14.5px; font-weight: 600; overflow-wrap: anywhere; }
  .nm small { color: var(--muted); font-size: 12.5px; }
  .expb { border-top: 1px solid var(--line); padding: 2px 18px 20px; background: var(--pane); border-radius: 0 0 7px 7px; }
  .exp.fresh .expb { animation: unfold .18s ease-out; }
  .expb .grid { margin-top: 18px; }
  @keyframes unfold { from { opacity: 0; transform: translateY(-4px); } }
  .moreln { display: block; margin-top: 16px; }
  .rowfiles { list-style: none; margin: 14px 0 0; padding: 0; max-width: 1100px; }
  .rowfiles li { display: flex; gap: 10px; align-items: baseline; padding: 4px 0; border-bottom: 1px solid var(--line); flex-wrap: wrap; font-size: 13.5px; }
  .nothing { margin: 16px 0 0; color: var(--soft); font-size: 13px; }
  .here { font-size: 15px; margin: 30px 0 -6px; color: var(--soft); }
  .chip.rv { background: var(--next); color: var(--next-ink); border-color: transparent; font-weight: 600; }
  .sub .ok, .here .ok { color: var(--done); font-weight: 600; }
  .here .chip { font-size: 12.5px; margin-left: 6px; vertical-align: 2px; }
  .sub .live::before { content: ""; display: inline-block; width: 7px; height: 7px; border-radius: 50%; background: var(--done); margin-right: 6px; }
  .more { margin: 26px 0; }
  .empty { padding: 36px 0; font-size: 16px; }
  .empty p { margin: 0 0 12px; }
  .empty .btn { margin-right: 8px; }
  .unlisted { margin-top: 28px; }
  .unlisted h2 { font-size: 15px; margin: 0 0 8px; color: var(--soft); }
  .unlisted ul { list-style: none; margin: 0; padding: 0; max-width: 1100px; }
  .unlisted li { display: flex; gap: 10px; align-items: baseline; padding: 4px 0; border-bottom: 1px solid var(--line); flex-wrap: wrap; font-size: 13.5px; }
  .tag { font-size: 11.5px; border-radius: 4px; padding: 0 6px; background: var(--side); color: var(--soft); }
  .tag.unplaced { background: var(--next); color: var(--next-ink); }
  /* As Settings, Programs' table: fixed columns, a shaded row per group. */
  .ftable { border-collapse: collapse; width: 100%; max-width: 1100px; font-size: 13.5px; table-layout: fixed; }
  .ftable .c-size { width: 84px; }
  .ftable .c-where { width: 38%; }
  .ftable th { text-align: left; font-size: 12px; color: var(--muted); font-weight: 600; padding: 6px 8px; border-bottom: 1px solid var(--line); }
  .ftable td { padding: 5px 8px; border-bottom: 1px solid var(--line); vertical-align: top; overflow-wrap: anywhere; }
  .ftable tr.group th { background: var(--side); color: var(--ink); font-size: 12.5px; padding: 6px 8px; }
  .ftable tr.group .n { color: var(--muted); font-weight: 400; margin-left: 4px; }
  .ftable tr.group .why { display: block; color: var(--soft); font-weight: 400; margin-top: 2px; }
  .ftable .num { text-align: right; color: var(--muted); white-space: nowrap; }
  .ftable .link { text-align: left; overflow-wrap: anywhere; }
  .ftable .in { display: block; text-align: left; overflow-wrap: anywhere; }
  .ftable .where { margin: 0; text-align: left; overflow-wrap: anywhere; }
  .in { border: 0; background: none; padding: 0; color: var(--link); font-size: 12.5px; }
  .in:hover { text-decoration: underline; }
  .where { margin-left: auto; border: 0; background: none; color: var(--muted); font-size: 12.5px; padding: 0; }
  button.where:hover { color: var(--ink); text-decoration: underline; }
</style>
