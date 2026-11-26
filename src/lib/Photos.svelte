<script lang="ts">
  // A Photos Section as the Photos prototype lays it out (`decisions/2026/10/08/photos-is-a-section-kind-in-the-app.md`): the whole library as a heat
  // calendar with On this day, a year as a strip of its months over each month's events, a month
  // as a calendar with each day's photos and the events as bars, and a day panel with the photos
  // of the day. Pointing at a month skims through its events; pointing at an event skims through
  // its photos. Every count is of files, read through the Lens (photos.ts).
  import { onMount } from 'svelte'
  import { clock, span, thumbSrc, type Detail, type Item, type Marks, type Thumb, type Work } from './api'
  import { choiceOf, isSetAside, stepsAt, type Notes } from './review'
  import { dayKey, isClip, panelWidth, photoThumb, photoThumbId, type Lens } from './photos'
  import { openMenu, type MenuItem } from './menus.svelte'
  import { S } from './strings'
  import { saveUi, ui } from './ui.svelte'
  import Viewer from './Viewer.svelte'

  let { works, folder, day, color, thumbs, details, lens, notes, marks, online, menus, onopen, onday, onreview, onfile, onreveal, onagain, onwant }: {
    /** Every event of the Section. */
    works: Work[]; folder: string[]
    /** The day open in the panel, as 2019-03-21, or `undated`. */
    day: string
    color: string; thumbs: Record<string, Thumb>; details: Record<string, Detail>
    lens: Lens; notes: Notes; marks: Marks
    /** Opens a year, a month or Undated, as folder names; with a day, that day's panel too. */
    onopen: (folder: string[], day?: string, replace?: boolean) => void
    onday: (day: string) => void
    /** Review of some events; with `start`, opened at that Photo's set. */
    onreview: (works: Work[], label: string, only?: (i: Item) => boolean, start?: Item) => void
    onfile: (path: string) => void; onreveal: (path: string) => void
    /** Forgets a photo's Review choice, so it waits for Review again. */
    onagain: (w: Work, i: Item) => void
    /** The thumbnails this view needs first. */
    onwant: (ids: string[]) => void
    /** Whether the Section's drive is here, for Show in File Explorer. */
    online: boolean
    /** The menus Library builds, so they match its own: a year's or a month's, as folder
     * names; an event's, with how this place opens it (none where it is open) and whether Enter
     * does too; Review of events. */
    menus: { place: (folder: string[]) => MenuItem[]; event: (w: Work, open?: () => void, enter?: boolean) => MenuItem[]
      review: (ws: Work[], label: string, only?: (i: Item) => boolean) => MenuItem[] }
  } = $props()

  const M = S.monthsShort, MONTH = S.months
  const HUES = ['#2F5FB3', '#7D4F96', '#B97C0B', '#0E7490', '#BE185D', '#4D7C0F']
  const fmt = (n: number) => n.toLocaleString('en')
  /** An event's times, or that they are not known. */
  const when = (w: Work) => w.day_only ? S.timeUnknown : span(w)
  function groupBy<T>(xs: T[], key: (x: T) => string) {
    const m = new Map<string, T[]>()
    for (const x of xs) { const k = key(x); (m.get(k) ?? m.set(k, []).get(k)!).push(x) }
    return m
  }

  // ---------- events, months and days, as the Lens shows them ----------
  const shown = $derived(works.filter(w => w.event && lens.shown(w)))
  const dated = $derived(shown.filter(w => w.taken && w.folder.length >= 2))
  const undated = $derived(shown.find(w => !w.taken))
  type Cell = { y: string; name: string; k: number; es: Work[]; n: number }
  const months = $derived.by(() => {
    const by = new Map<string, Map<number, Work[]>>()
    for (const w of dated) {
      const [y, m] = w.folder, k = Number(m.slice(0, 2)) - 1
      const row = by.get(y) ?? new Map<number, Work[]>()
      ;(row.get(k) ?? row.set(k, []).get(k)!).push(w)
      by.set(y, row)
    }
    return by
  })
  const monthName = (y: string, k: number) => months.get(y)?.get(k)?.[0]?.folder[1] ?? `${String(k + 1).padStart(2, '0')} ${MONTH[k]}`
  const cellsOf = (y: string): Cell[] => Array.from({ length: 12 }, (_, k) => {
    const es = months.get(y)?.get(k) ?? []
    return { y, name: monthName(y, k), k, es, n: lens.num(es) }
  })
  const years = $derived([...months.keys()].sort((a, b) => b.localeCompare(a)))

  /** Each event's days, and each day's events with the photos taken that day. */
  const days = $derived.by(() => {
    const of = new Map<Work, string[]>(), on = new Map<string, { w: Work; ps: Item[] }[]>()
    for (const w of dated) {
      const by = new Map<string, Item[]>()
      for (const i of lens.items(w)) if (i.at) { const d = dayKey(i.at); (by.get(d) ?? by.set(d, []).get(d)!).push(i) }
      const ks = [...by.keys()].sort()
      of.set(w, ks)
      for (const d of ks) (on.get(d) ?? on.set(d, []).get(d)!).push({ w, ps: by.get(d)! })
    }
    return { of, on, sorted: [...on.keys()].sort() }
  })
  /** Each event's bar colour, by its place in the Section, so a filter does not repaint them. */
  const hue = $derived(new Map(works.filter(w => w.taken).map((w, k) => [w.id, HUES[k % HUES.length]])))

  const countText = (ws: Work[], only?: (i: Item) => boolean) => {
    const c = lens.count(ws, only)
    return lens.f ? S.filterUnit(lens.f, c.photos) : S.countLine(c.photos, c.clips, c.hidden)
  }
  const reviewText = (ws: Work[], only?: (i: Item) => boolean) => {
    const n = lens.toReview(ws, only)
    return n ? S.toReviewN(n) : lens.hadReview(ws, only) ? S.reviewed : ''
  }
  /** The photo that stands for an event: for a filter, one it found; else the first photo shown. */
  const coverOf = (w: Work): Item | undefined => {
    const v = lens.visible(w)
    return v.find(i => !isClip(i)) ?? v[0] ?? lens.items(w)[0]
  }
  const thumbOf = (w: Work, i?: Item) => (i && photoThumb(thumbs, w, i)) || thumbs[w.id]
  /** A thumbnail as a CSS background; a strip of frames shows its first frame. */
  const bg = (t?: Thumb) => t ? `background-image:url('${thumbSrc(t.key)}');${t.n > 1 ? `background-size:auto 100%;background-position:0 50%;` : ''}` : ''
  const shape = (t?: Thumb) => t ? (t.w / Math.max(1, t.n)) / t.h : 4 / 3

  // ---------- the view ----------
  const level = $derived(folder[0] === 'Undated' ? 'undated' : folder.length === 0 ? 'all' : folder.length === 1 ? 'year' : 'month')
  const year = $derived(folder[0] ?? '')
  const monthK = $derived(folder.length >= 2 ? Number(folder[1].slice(0, 2)) - 1 : -1)

  // On this day: the best photos from today's day in past years, or the nearest later day.
  const onDay = $derived.by(() => {
    if (level !== 'all' || lens.f) return null
    const now = new Date()
    for (let off = 0; off < 30; off++) {
      const d = new Date(now.getFullYear(), now.getMonth(), now.getDate() + off)
      const all: { w: Work; i: Item }[] = []
      for (const w of dated) {
        for (const i of lens.items(w)) {
          if (!i.at) continue
          const t = new Date(i.at * 1000)
          if (t.getUTCMonth() === d.getMonth() && t.getUTCDate() === d.getDate() && t.getUTCFullYear() < now.getFullYear()) all.push({ w, i })
        }
      }
      const best = lens.all ? all : all.filter(x => !lens.hidden(x.i, x.w))
      if (!best.length) continue
      best.sort((a, b) => b.i.at! - a.i.at!)
      return { off, d, now, all: all.length, best: best.length, list: best.slice(0, 40) }
    }
    return null
  })
  const dayName = (d: Date) => `${d.getDate()} ${M[d.getMonth()]}`
  // A photo reached with Tab scrolls whole into the strip: the browser leaves one it half shows.
  const seen = (e: FocusEvent) => {
    const b = e.currentTarget as HTMLElement
    if (b.matches(':focus-visible')) b.scrollIntoView({ block: 'nearest', inline: 'nearest' })
  }

  // The overview: one shade scale for the whole library, by the square root of the count.
  const rows = $derived(level === 'all' ? years.map(y => ({ y, cells: cellsOf(y) })) : [])
  const yearCells = $derived(level === 'year' ? cellsOf(year) : [])
  const max = $derived(Math.max(1, ...(level === 'all' ? rows.flatMap(r => r.cells.map(c => c.n)) : yearCells.map(c => c.n))))
  const lv = (n: number) => n ? 12 + Math.round(88 * Math.sqrt(n / max)) : 0

  // ---------- skimming ----------
  let skim = $state<{ x: number; y: number; up: boolean; t?: Thumb; k: number; n: number; title: string; sub: string } | null>(null)
  function skimMonth(e: MouseEvent, c: Cell) {
    const el = e.currentTarget as HTMLElement, r = el.getBoundingClientRect()
    const k = Math.min(c.es.length - 1, Math.max(0, Math.floor((e.clientX - r.left) / r.width * c.es.length)))
    const w = c.es[k]
    const up = r.bottom + 300 > innerHeight
    skim = { x: Math.min(innerWidth - 290, Math.max(8, r.left + r.width / 2 - 140)), y: up ? r.top - 8 : r.bottom + 8, up, t: thumbOf(w, lens.f ? coverOf(w) : undefined), k, n: c.es.length,
      title: S.skimTitle(MONTH[c.k], c.y, k + 1, c.es.length), sub: `${w.title.replace(' ' + c.y, '')}, ${when(w)} · ${countText([w])}` }
  }
  /** An event card flips through its photos under the pointer. */
  let flip = $state<{ id: string; k: number } | null>(null)
  const seqOf = (w: Work) => {
    const first = coverOf(w)
    return first ? [first, ...lens.visible(w).filter(i => i !== first)].slice(0, 12) : []
  }
  function skimCard(e: MouseEvent, w: Work, n: number) {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect()
    flip = { id: w.id, k: Math.min(n - 1, Math.max(0, Math.floor((e.clientX - r.left) / r.width * n))) }
  }
  let hot = $state<string[]>([])
  /** In a year, the month under the strip as the page scrolls. */
  let inMonth = $state(-1)
  $effect(() => {
    if (level !== 'year') return
    const main = host?.closest('main'), strip = host?.querySelector('.ystrip')
    if (!main || !strip) return
    const f = () => {
      const edge = strip.getBoundingClientRect().bottom + 8
      let k = -1
      const ms = host.querySelectorAll<HTMLElement>('.ymon')
      for (const s of ms) if (s.getBoundingClientRect().top <= edge + 40) k = Number(s.id.slice(4))
      // At the end of the page the last month is the one shown, even when too short to reach the strip.
      if (ms.length && main.scrollTop > 0 && main.scrollTop + main.clientHeight >= main.scrollHeight - 2) k = Number(ms[ms.length - 1].id.slice(4))
      inMonth = k
    }
    f()
    main.addEventListener('scroll', f, { passive: true })
    return () => main.removeEventListener('scroll', f)
  })
  // A view that changes under the pointer leaves no preview behind.
  $effect(() => { void folder; skim = null; flip = null; hot = [] })

  // ---------- the month calendar ----------
  const cal = $derived.by(() => {
    if (level !== 'month') return null
    const today = new Date()
    const y = Number(year), first = new Date(Date.UTC(y, monthK, 1)), len = new Date(Date.UTC(y, monthK + 1, 0)).getUTCDate()
    const lead = (first.getUTCDay() - ui.week + 7) % 7
    const cells = Array.from({ length: len }, (_, d) => {
      const key = `${year}-${String(monthK + 1).padStart(2, '0')}-${String(d + 1).padStart(2, '0')}`
      const on = days.on.get(key) ?? []
      const all = on.flatMap(({ w, ps }) => ps.map(i => ({ w, i })))
      const n = lens.f ? all.filter(x => lens.extra(x.i)).length : 0
      const hid = all.filter(x => lens.hidden(x.i, x.w)).length
      let strip = lens.f ? all.filter(x => lens.inF(x.i) && (notes[x.i.path]?.best || !notes[x.i.path]?.set)) : all.filter(x => lens.all || !lens.hidden(x.i, x.w))
      if (lens.f && !strip.length) strip = all.filter(x => lens.extra(x.i))
      if (!strip.length) strip = all
      // Review counts every kind, so a day's own Photos are taken from the whole event.
      const mine = new Set(on.flatMap(({ w }) => w.items.filter(i => i.at && dayKey(i.at) === key)))
      const only = (i: Item) => mine.has(i)
      const ws = on.map(x => x.w)
      return { d: d + 1, key, on, all, n, hid, strip: strip.slice(0, 8), live: all.length > 0 && (!lens.f || n > 0),
        count: countText(ws, i => mine.has(i)), rv: lens.toReview(ws, only), had: lens.hadReview(ws, only),
        today: Number(year) === today.getFullYear() && monthK === today.getMonth() && d + 1 === today.getDate() }
    })
    const names = Array.from({ length: 7 }, (_, k) => S.weekdays[(k + ui.week) % 7])
    // The month's events: those filed under it and those with any day in it, as the calendar draws them.
    const es = [...new Set([...dated.filter(w => w.folder[0] === year && w.folder[1] === folder[1]), ...cells.flatMap(c => c.on.map(x => x.w))])]
    return { lead, cells, names, es, left: lens.toReview(es), had: lens.hadReview(es) }
  })

  // ---------- the day panel ----------
  const panel = $derived.by(() => {
    if (!day) return null
    if (day === 'undated') return undated ? { title: S.undatedTitle, entries: [{ w: undated, ps: lens.items(undated) }] } : null
    const entries = (days.on.get(day) ?? []).filter(x => !lens.f || x.ps.some(lens.extra))
    const title = new Date(day + 'T12:00:00Z').toLocaleDateString('en-GB', { weekday: 'long', day: 'numeric', month: 'long', year: 'numeric', timeZone: 'UTC' })
    return { title, entries }
  })
  /** The day's Photos as the panel shows them, for the viewer to go through every event of the day. */
  const dayList = $derived((panel?.entries ?? []).flatMap(x => x.ps.filter(i => lens.f ? lens.inF(i) : lens.all || !lens.hidden(i, x.w)).map(i => ({ w: x.w, i }))))
  /** The day with photos before (`dir` -1) or after this one, or '' when there is none. */
  function dayTo(dir: number) {
    const list = days.sorted, at0 = list.indexOf(day)
    // A day no longer listed (another kind or filter chosen) steps from where it would sort.
    let i = at0 >= 0 ? at0 + dir : dir > 0 ? list.findIndex(d => d > day) : list.findLastIndex(d => d < day)
    if (i < 0) return ''
    const ok = (d: string) => (days.on.get(d) ?? []).some(x => !lens.f || x.ps.some(lens.extra))
    while (i >= 0 && i < list.length && !ok(list[i])) i += dir
    return i < 0 || i >= list.length ? '' : list[i]
  }
  function stepDay(dir: number) {
    const to = dayTo(dir)
    if (to) openDay(to, true)
  }
  /** Opens a day's panel; a day in another month opens that month's calendar with it. */
  function openDay(d: string, replace = false) {
    const [y, m] = d.split('-')
    if (folder[0] === y && monthK === Number(m) - 1) onday(d)
    else onopen([y, monthName(y, Number(m) - 1)], d, replace)
  }
  /** Opens an event: its month's calendar with its first day's panel. */
  const openEvent = (w: Work) => { const d = days.of.get(w)?.[0]; if (d) openDay(d); else onopen([w.folder[0], w.folder[1]]) }

  // ---------- the menus ----------
  const dayTitle = (d: string) => new Date(d + 'T12:00:00Z').toLocaleDateString('en-GB', { day: 'numeric', month: 'long', year: 'numeric', timeZone: 'UTC' })
  /** Review of a day: its events, only the files taken that day. */
  function dayReview(d: string): MenuItem[] {
    const on = days.on.get(d) ?? [], mine = new Set(on.flatMap(x => x.w.items.filter(i => i.at && dayKey(i.at) === d)))
    return menus.review(on.map(x => x.w), dayTitle(d), i => mine.has(i))
  }
  const dayItems = (d: string): MenuItem[] => [{ label: S.open, icon: 'launch', hint: S.enterKey, run: () => onday(d) }, ...dayReview(d)]
  function panelItems(): MenuItem[] {
    const back = day === 'undated' ? '' : dayTo(-1), next = day === 'undated' ? '' : dayTo(1)
    return [...(day === 'undated' ? [] : dayReview(day)), { sep: true },
      { label: S.previousDay, icon: 'back', hint: S.leftKey, disabled: !back, run: () => stepDay(-1) },
      { label: S.nextDay, icon: 'chev', hint: S.rightKey, disabled: !next, run: () => stepDay(1) },
      { sep: true }, { label: S.close, icon: 'clear', hint: S.escKey, run: () => onday('') }]
  }
  /** A photo's menu; `toDay` adds Go to its day where the photo is away from its day. A photo with
   * a Review choice offers Review again. */
  function photoItems(w: Work, i: Item, open: () => void, toDay = false): MenuItem[] {
    return [{ label: S.open, icon: 'launch', hint: S.enterKey, run: open },
      ...(choiceOf(i, w, marks) ? [{ label: S.reviewAgain, icon: 'review', run: () => onagain(w, i) } as MenuItem] : []),
      { sep: true }, { label: S.showInExplorer, icon: 'open', disabled: !online, run: () => onreveal(i.path) },
      ...(toDay && i.at ? [{ label: S.goToDay, icon: 'folder', run: () => openDay(dayKey(i.at!)) } as MenuItem] : [])]
  }
  /** Sets of Similar shots opened in place under their Best one. */
  let open = $state(new Set<string>())
  /** How many photos each event in the panel draws, a chunk at a time. */
  const CHUNK = 200
  let more = $state<Record<string, number>>({})
  const toggle = (set: string) => { const n = new Set(open); if (n.has(set)) n.delete(set); else n.add(set); open = n }

  let width = $derived(ui.colw.photoday ?? 0)
  let host: HTMLDivElement
  let box = $state({ top: 0, bottom: 0 })
  function place() {
    const m = host?.closest('main')?.getBoundingClientRect()
    if (m) box = { top: m.top, bottom: innerHeight - m.bottom }
  }
  onMount(() => { place() })
  const panelW = $derived(panelWidth(width))
  function grip(e: PointerEvent) {
    const g = e.currentTarget as HTMLElement
    g.setPointerCapture(e.pointerId)
    const move = (m: PointerEvent) => { ui.colw.photoday = Math.round(Math.min(innerWidth * 0.8, Math.max(360, innerWidth - m.clientX))) }
    const up = () => {
      for (const t of ['pointerup', 'pointercancel', 'lostpointercapture']) g.removeEventListener(t, up)
      g.removeEventListener('pointermove', move); saveUi()
    }
    g.addEventListener('pointermove', move)
    for (const t of ['pointerup', 'pointercancel', 'lostpointercapture']) g.addEventListener(t, up)
  }
  function resetGrip() { delete ui.colw.photoday; saveUi() }

  function keys(e: KeyboardEvent) {
    if (!day || viewing || e.defaultPrevented || document.querySelector('.review, dialog[open]')) return
    if ((e.target as HTMLElement)?.matches?.('input, textarea, select, [contenteditable]')) return
    if (e.key === 'Escape') { e.preventDefault(); onday('') }
    else if (day !== 'undated' && (e.key === 'ArrowLeft' || e.key === 'ArrowRight') && !e.altKey) { e.preventDefault(); stepDay(e.key === 'ArrowLeft' ? -1 : 1) }
  }

  // ---------- the viewer ----------
  let viewing = $state<{ list: { w: Work; i: Item }[]; at: number } | null>(null)
  let at = $state(-1)
  /** A clip opens in the system's player, as a lesson does; a Photo in a set of Similar shots or
   * Copies opens Review of its day at that set; anything else opens in the viewer. */
  function view(list: { w: Work; i: Item }[], k: number) {
    const { w, i } = list[k]
    if (isClip(i)) { onfile(i.path); return }
    if (reviewFrom(w, i)) return
    viewing = { list, at: k }; at = k
  }
  /** Review of a Photo's day, as the day's Review does, or of its event when it has no day,
   * through the kind shown, opened at the Photo's set; false, and nothing opens, when no step
   * of it shows the Photo. */
  function reviewFrom(w: Work, i: Item) {
    const d = w.taken && i.at ? dayKey(i.at) : ''
    const on = d ? days.on.get(d) ?? [] : []
    const kind = lens.reviewOnly
    const mine = new Set(on.flatMap(x => x.w.items.filter(x => x.at && dayKey(x.at) === d)))
    const [ws, label, only] = on.length ? [on.map(x => x.w), dayTitle(d), (x: Item) => mine.has(x) && (!kind || kind(x))]
      : [[w], w.taken ? w.title : S.undatedTitle, kind]
    if (!stepsAt(ws, notes, marks, only, i).here) return false
    onreview(ws, label, only, i)
    return true
  }
  $effect(() => { if (at < 0 && viewing) viewing = null })
  const workOf = (i: Item) => viewing?.list.find(x => x.i === i)?.w
  function noteOf(i: Item): { when: string; what: string } {
    const w = workOf(i)
    const when = i.at ? new Date(i.at * 1000).toLocaleString('en-GB', { dateStyle: 'medium', timeStyle: 'short', timeZone: 'UTC' }) : S.undatedTitle
    const n = notes[i.path]
    let what = ''
    if (w && isSetAside(i, w, marks)) what = S.noteAside
    else if (n?.set) {
      const size = w ? w.items.filter(x => notes[x.path]?.set === n.set).length : 0
      what = lens.calls.has(n.set) ? S.noteCall(size) : n.best ? S.noteBest(size, whyOf(n)) : S.noteExtra(size)
    } else if (n?.copy) what = S.noteCopy
    else if (n?.junk) what = S.junkWhy[n.junk]
    return { when, what }
  }
  const whyOf = (n?: { why?: [string, number] }) => !n?.why ? '' : n.why[0] === 'sharp' ? S.sharper(n.why[1]) : S.why[n.why[0]]

  // ---------- thumbnails wanted first ----------
  const baseWant = $derived.by(() => {
    const ids: string[] = []
    if (level === 'all') {
      for (const x of onDay?.list ?? []) ids.push(photoThumbId(x.w, x.i))
      ids.push(...dated.slice(-600).reverse().map(w => w.id))
    } else if (level === 'year') {
      for (const c of yearCells) for (const w of c.es) { const i = coverOf(w); ids.push(i ? photoThumbId(w, i) : w.id) }
    } else if (level === 'month' && cal) {
      for (const c of cal.cells) for (const x of c.strip) ids.push(photoThumbId(x.w, x.i))
    }
    for (const x of panel?.entries ?? []) for (const i of x.ps.slice(0, 400)) ids.push(photoThumbId(x.w, i))
    if (level === 'undated' && undated) for (const i of lens.items(undated).slice(0, 400)) ids.push(photoThumbId(undated, i))
    return ids
  })
  $effect(() => { onwant([...hot, ...baseWant]) })
</script>

<svelte:window onkeydown={keys} onresize={place} />

{#snippet heat(c: Cell, onclick: () => void, small = false)}
  {@const l = lv(c.n)}
  {@const rv = lens.f || !c.n ? '' : reviewText(c.es)}
  {#if c.n}
    <button class="hm" class:small class:here={level === 'year' && c.k === inMonth} style="--lv:{l}%" {onclick} onmousemove={e => skimMonth(e, c)} onmouseleave={() => (skim = null)}
      oncontextmenu={e => { skim = null; openMenu(e, menus.place([c.y, c.name]), `${MONTH[c.k]} ${c.y}`) }}
      aria-label={`${MONTH[c.k]} ${c.y}, ${countText(c.es)}${rv ? `, ${rv}` : ''}`}>
      <span class="mn">{M[c.k]}</span><b>{fmt(c.n)}</b>
      {#if rv}<span class="rv" class:ok={!lens.toReview(c.es)} title={lens.toReview(c.es) ? S.monthReviewTip(lens.toReview(c.es)) : S.reviewDone}>{lens.toReview(c.es) ? fmt(lens.toReview(c.es)) : '✓'}</span>{/if}
    </button>
  {:else}
    <span class="hm empty" class:small><span class="mn">{M[c.k]}</span></span>
  {/if}
{/snippet}

{#snippet tile(w: Work, i: Item, list: { w: Work; i: Item }[], tag = '', cls = '', badge?: { text: string; tip: string; on: () => void }, viewList?: { w: Work; i: Item }[])}
  {@const t = photoThumb(thumbs, w, i)}
  {@const n = notes[i.path]}
  {@const aside = isSetAside(i, w, marks)}
  <!-- Open goes through the whole day in the viewer when it can, else through this list. -->
  {@const open = () => { const all = viewList?.some(x => x.i === i) ? viewList : list; view(all, all.findIndex(x => x.i === i)) }}
  <div class="ph {cls}" class:aside role="presentation" oncontextmenu={e => openMenu(e, photoItems(w, i, open), i.name)}>
    <button class="hit" style={bg(t)} title={i.name} aria-label={i.name} onclick={open}></button>
    {#if isClip(i)}<span class="play">▶ {details[i.path]?.s ? clock(details[i.path].s!) : ''}</span>{/if}
    {#if aside}<span class="tag aside-t">{S.setAside}</span>{:else if tag}<span class="tag" class:best={tag === S.bestOne || tag === S.first}>{tag}</span>{/if}
    {#if n?.junk && !aside}<span class="jk">{S.junkShort[n.junk]}</span>{/if}
    {#if badge}<button class="n" title={badge.tip} aria-label={badge.tip} onclick={badge.on}>{badge.text}</button>{/if}
  </div>
{/snippet}

{#snippet body(w: Work, ps: Item[], day?: { w: Work; i: Item }[])}
  {#if lens.f === 'sim' || lens.f === 'call' || lens.f === 'cp'}
    {@const key = (i: Item) => lens.f === 'cp' ? (notes[i.path]?.copy ?? i.path) : notes[i.path]?.set ?? ''}
    {@const mine = new Set(ps)}
    {@const groups = [...groupBy(lens.items(w).filter(lens.inF), key).values()].filter(s => (s.length > 1 || (lens.f === 'cp' && !!notes[s[0].path]?.copy)) && s.some(i => mine.has(i)))}
    {#each groups as s (key(s[0]))}
      {@const keep = lens.f === 'cp' ? s.find(i => !notes[i.path]?.copy) : s.find(i => notes[i.path]?.best)}
      {@const call = lens.f !== 'cp' && lens.calls.has(notes[s[0].path]?.set ?? '')}
      {@const list = (keep ? [keep, ...s.filter(i => i !== keep)] : s).map(i => ({ w, i }))}
      <div class="set">
        <div>{lens.f === 'cp' ? (keep ? S.setOfCopies(s.length, s.length - 1) : S.copyElsewhere) : call ? S.setOfCall(s.length) : S.setOfSimilar(s.length, s.length - 1, whyOf(notes[keep?.path ?? '']))}</div>
        <div class="row2">
          {#each list as x (x.i.path)}
            {@render tile(w, x.i, list, call ? '' : x.i === keep ? (lens.f === 'cp' ? S.first : S.bestOne) : (lens.f === 'cp' ? S.copy : S.extra), !call && x.i !== keep ? 'extra' : '')}
          {/each}
        </div>
      </div>
    {/each}
  {:else}
    {@const list = ps.filter(i => lens.f ? lens.inF(i) : lens.all || !lens.hidden(i, w) || (!!notes[i.path]?.set && open.has(notes[i.path].set!) && !notes[i.path].junk && !isSetAside(i, w, marks))).map(i => ({ w, i }))}
    <!-- +N on a Best one counts only what opening it shows: the hidden extras that are not Junk or set aside. -->
    {@const folded = groupBy(ps.filter(i => { const n = notes[i.path]; return !!n?.set && !n.best && lens.hidden(i, w) && !n.junk && !isSetAside(i, w, marks) }), i => notes[i.path].set!)}
    {@const cap = more[w.id] ?? CHUNK}
    {#if !list.length && !lens.f}<p class="hint">{S.allHidden(ps.length)}</p>{/if}
    <div class="pgrid">
      {#each list.slice(0, cap) as x (x.i.path)}
        {@const n = notes[x.i.path]}
        {@const extra = n?.set ? folded.get(n.set)?.length ?? 0 : 0}
        {@const fold = !lens.f && !lens.all && !!n?.best && extra > 0}
        {@render tile(w, x.i, list, '', n?.set && !n.best && !lens.calls.has(n.set) ? 'extra' : '',
          fold ? { text: open.has(n!.set!) ? '−' : `+${extra}`, tip: open.has(n!.set!) ? S.hideExtras : S.moreOfMoment(extra), on: () => toggle(n!.set!) } : undefined, day)}
      {/each}
    </div>
    {#if list.length > cap}<button class="btn shown-more" onclick={() => (more[w.id] = cap + CHUNK)}>{S.showMore(Math.min(CHUNK, list.length - cap), list.length - cap)}</button>{/if}
  {/if}
{/snippet}

{#snippet eventHead(w: Work, ps: Item[], label: string)}
  {@const left = lens.toReview([w])}
  {@const mine = new Set(ps)}
  {@const sets = new Set(ps.map(i => notes[i.path]?.set).filter(Boolean)).size}
  <!-- Probably not needed counts as its filter does, Junk in a set left to the set, and once. -->
  {@const junk = lens.f === 'nn' ? 0 : ps.filter(i => notes[i.path]?.junk && !notes[i.path].set).length}
  <!-- Focusable, so the menu key reaches an event's menu when it has no Review button. -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <h3 tabindex="0" oncontextmenu={e => openMenu(e, menus.event(w), w.title)}>{label} <small>{S.eventCounts(countText([w], i => mine.has(i)), sets, junk)}</small>
    {#if left}<button class="btn go" title={S.reviewTip(left)} onclick={() => onreview([w], w.title, lens.reviewOnly)}>{S.reviewN(left, lens.k)}</button>
    {:else if lens.hadReview([w])}<small class="ok">{S.reviewed}</small>{/if}
  </h3>
{/snippet}

<div class="photos" bind:this={host} style="--c:{color}" onmouseleave={() => (skim = null)} role="presentation">
  {#if lens.f && !shown.some(w => level === 'all' || (level === 'undated' ? !w.taken : w.folder[0] === year && (level === 'year' || cal?.es.includes(w))))}
    <p class="hint none">{S.nothingMatches(S.filterName[lens.f])}</p>
  {/if}
  {#if level === 'all'}
    {#if onDay}
      <section class="otd" aria-label={S.onThisDay}>
        <h2>{onDay.off ? S.onNearDay(dayName(onDay.now), dayName(onDay.d)) : S.onThisDayOf(dayName(onDay.now))}
          <small>{lens.all ? S.allOfDay(onDay.list.length, onDay.all) : S.bestOfDay(onDay.list.length, onDay.best, onDay.all)}</small></h2>
        <div class="strip">
          {#each onDay.list as x, k (x.i.path)}
            {@const t = photoThumb(thumbs, x.w, x.i)}
            <button onclick={() => view(onDay.list, k)} onfocus={seen} oncontextmenu={e => openMenu(e, photoItems(x.w, x.i, () => view(onDay.list, k), true), x.i.name)} aria-label={`${x.i.name}, ${new Date(x.i.at! * 1000).getUTCFullYear()}`}>
              {#if t}<span class="im" style="{bg(t)}aspect-ratio:{shape(t)}"></span>{:else}<span class="im ph0"></span>{/if}
              <small>{new Date(x.i.at! * 1000).getUTCFullYear()}</small>
            </button>
          {/each}
        </div>
      </section>
    {/if}
    <div class="heat">
      {#each rows as r (r.y)}
        {@const es = r.cells.flatMap(c => c.es)}
        {@const rv = lens.f ? '' : reviewText(es)}
        <div class="yr">
          <button class="yl" onclick={() => onopen([r.y])} oncontextmenu={e => openMenu(e, menus.place([r.y]), r.y)}>{r.y}<small>{countText(es)}</small>{#if rv}<small class:rvt={lens.toReview(es) > 0} class:okt={!lens.toReview(es)}>{rv}</small>{/if}</button>
          {#each r.cells as c (c.k)}{@render heat(c, () => onopen([c.y, c.name]))}{/each}
        </div>
      {/each}
    </div>
    {#if rows.length}
      <div class="legend">
        <span>{S.legendFewer}<span class="scale">{#each [12, 34, 56, 78, 100] as l (l)}<i style="--lv:{l}%"></i>{/each}</span>{S.legendMore(S.legendWhat(lens.f, lens.all, lens.k), max)}</span>
        {#if !lens.f}<span><i class="rvk" aria-hidden="true"></i> {S.legendReview}</span>{/if}
        <span>{S.legendPoint}</span>
      </div>
    {/if}
    {#if undated && lens.num([undated])}
      <p class="undated"><button class="link" onclick={() => onopen(['Undated'])}>{S.undatedCount(lens.num([undated]))}</button></p>
    {/if}
  {:else if level === 'year'}
    <div class="ystrip">
      <div class="yr12">
        {#each yearCells as c (c.k)}{@render heat(c, () => onopen([c.y, c.name]), true)}{/each}
      </div>
    </div>
    {#each yearCells.filter(c => c.es.length) as c (c.k)}
      {@const left = lens.toReview(c.es)}
      <section class="ymon" id="mon-{c.k}">
        <h3 oncontextmenu={e => openMenu(e, menus.place([c.y, c.name]), `${MONTH[c.k]} ${c.y}`)}><button class="mh" onclick={() => onopen([c.y, c.name])}>{MONTH[c.k]} <small>{S.events(c.es.length)} · {countText(c.es)}</small></button>
          {#if left && !lens.f}<button class="chip rv" title={S.reviewTip(left)} onclick={() => onreview(c.es, `${MONTH[c.k]} ${c.y}`, lens.reviewOnly)}>{S.reviewN(left, lens.k)}</button>{/if}</h3>
        <div class="evs">
          {#each c.es as w (w.id)}
            {@const seq = seqOf(w)}
            {@const k = flip?.id === w.id ? flip.k : 0}
            {@const t = thumbOf(w, seq[k])}
            {@const rv = lens.f ? '' : reviewText([w])}
            <button class="evc" title={`${w.title}, ${when(w)}`} onclick={() => openEvent(w)}
              oncontextmenu={e => { flip = null; openMenu(e, menus.event(w, () => openEvent(w), true), w.title) }}
              onmouseenter={() => (hot = seq.map(i => photoThumbId(w, i)))} onmousemove={e => seq.length > 1 && skimCard(e, w, seq.length)} onmouseleave={() => (flip = null)}>
              <span class="im" style={bg(t)}>
                {#if rv}<span class="rv" class:ok={!lens.toReview([w])}>{rv}</span>{/if}
                {#if seq.length > 1}<span class="dots">{#each seq as _, j (j)}<i class:on={j === k}></i>{/each}</span>{/if}
              </span>
              <b>{w.title.replace(' ' + c.y, '')}</b><small>{when(w)}</small><small>{countText([w])}</small>
            </button>
          {/each}
        </div>
      </section>
    {/each}
  {:else if level === 'month' && cal}
    <div class="mbar">
      {#if cal.left}<button class="btn go" onclick={() => onreview(cal.es.filter(w => lens.toReview([w])), `${MONTH[monthK]} ${year}`, lens.reviewOnly)}>{S.reviewN(cal.left, lens.k)}</button><span class="hint">{S.monthHint}</span>
      {:else if cal.had}<b class="okt">{S.monthReviewed}</b>{:else}<span class="hint">{S.nothingThisMonth}</span>{/if}
    </div>
    <div class="mcal">
      {#each cal.names as n (n)}<div class="dow">{n}</div>{/each}
      {#each Array(cal.lead) as _, k (k)}<div class="day"></div>{/each}
      {#each cal.cells as c (c.key)}
        {@const isToday = c.today}
        {#snippet inner()}
          <div class="dtop"><span class="num">{c.d}</span>
            {#if !lens.f && c.had}<span class="rvp" class:ok={!c.rv} title={c.rv ? `${S.toReviewN(c.rv)}. ${S.dayReviewTip}` : S.reviewDone} aria-label={c.rv ? S.toReviewN(c.rv) : S.reviewDone}>{c.rv ? fmt(c.rv) : '✓'}</span>{/if}</div>
          {#if c.live}
            <div class="pic">
              {#each c.strip.slice(0, 3) as x (x.i.path)}<i style={bg(photoThumb(thumbs, x.w, x.i))}></i>{/each}
              <em><span class:f={!!lens.f} title={!lens.f && c.hid && !lens.all ? `${c.count}. ${S.hiddenTip}` : c.count}>{c.count}</span></em>
            </div>
          {/if}
          <div class="bars">
            {#each c.on as x (x.w.id)}
              {@const ds = days.of.get(x.w) ?? []}
              <!-- A click goes to the day under it; the bar's menu is its event's, so Open opens the event. -->
              <div class="bar" role="presentation" oncontextmenu={e => openMenu(e, menus.event(x.w, () => openEvent(x.w)), x.w.title)} class:s={ds[0] === c.key} class:e={ds.at(-1) === c.key} style="--bc:{hue.get(x.w.id)}" title={`${x.w.title} ${when(x.w)}`}>{ds[0] === c.key ? span(x.w).split(' to ')[0] || '?' : ' '}</div>
            {/each}
          </div>
        {/snippet}
        {#if c.live}
          <div class="day has" class:sel={day === c.key} class:today={isToday} role="button" tabindex="0" aria-label={`${c.d} ${MONTH[monthK]}, ${c.count}`}
            onclick={() => onday(c.key)} onkeydown={e => (e.key === 'Enter' || e.key === ' ') && (e.preventDefault(), onday(c.key))}
            oncontextmenu={e => openMenu(e, dayItems(c.key), dayTitle(c.key))}>{@render inner()}</div>
        {:else}
          <div class="day" class:dim={c.all.length > 0 && !!lens.f} class:today={isToday}>{@render inner()}</div>
        {/if}
      {/each}
    </div>
  {:else if level === 'undated' && undated}
    <section class="ev">
      {@render eventHead(undated, lens.items(undated), S.undatedTitle)}
      <p class="hint">{S.undatedWhat}</p>
      {@render body(undated, lens.items(undated))}
    </section>
  {/if}
</div>

{#if skim}
  <div class="skim" style="left:{skim.x}px;{skim.up ? `bottom:${innerHeight - skim.y}px` : `top:${skim.y}px`}" aria-hidden="true">
    <span class="im" style={bg(skim.t)}></span>
    {#if skim.n > 1}<span class="dots">{#each Array(skim.n) as _, j (j)}<i class:on={j === skim.k}></i>{/each}</span>{/if}
    <div class="t">{skim.title}</div><div class="s">{skim.sub}</div>
  </div>
{/if}

{#if panel && level !== 'undated'}
  <aside class="daypanel" style="--c:{color};top:{box.top}px;bottom:{box.bottom}px;width:{panelW}" aria-label={panel.title}>
    <div class="grip" role="separator" aria-orientation="vertical" title={S.panelGrip} onpointerdown={grip} ondblclick={resetGrip}></div>
    <div class="dh" role="presentation" oncontextmenu={e => openMenu(e, panelItems(), panel.title)}>
      <h2>{panel.title}</h2>
      <p>{S.dayHint(panel.entries.length, lens.f, day === 'undated')}</p>
      <button class="x" aria-label={S.close} onclick={() => onday('')}><svg viewBox="0 0 10 10" aria-hidden="true"><path d="M1.5 1.5l7 7M8.5 1.5l-7 7" /></svg></button>
    </div>
    <div class="db">
      {#if !panel.entries.length}<p class="hint">{S.nothingMatchesDay}</p>{/if}
      {#each panel.entries as x (x.w.id)}
        <section class="ev">
          {@render eventHead(x.w, x.ps, (days.of.get(x.w)?.length ?? 0) > 1 ? x.w.title : when(x.w))}
          {@render body(x.w, x.ps, dayList)}
        </section>
      {/each}
    </div>
  </aside>
{/if}

{#if viewing && at >= 0}
  <Viewer items={viewing.list.map(x => x.i)} bind:at thumbOf={i => { const w = workOf(i); return w ? photoThumb(thumbs, w, i) : undefined }} {onfile} {onreveal} note={noteOf}
    again={i => { const w = workOf(i); return w && choiceOf(i, w, marks) ? () => onagain(w, i) : undefined }} />
{/if}

<style>
  .photos { margin-top: 14px; }
  .hint { color: var(--muted); font-size: 13px; }
  .hint.none { font-size: 15px; margin: 4px 0 16px; }
  .okt { color: var(--done); font-weight: 600; }
  .rvt { color: var(--next-ink); background: var(--next); border-radius: 999px; padding: 0 7px; width: fit-content; font-weight: 600; }
  .link { border: 0; background: none; padding: 0; color: var(--link); }
  .link:hover { text-decoration: underline; }
  .btn.go { background: var(--c); border-color: var(--c); color: #fff; font-weight: 600; }
  .chip { border: 1px solid var(--line); border-radius: 999px; padding: 1px 10px; background: var(--field); font-size: 13px; }
  .chip.rv { background: var(--next); color: var(--next-ink); border-color: transparent; font-weight: 600; }

  /* On this day */
  .otd { margin: 0 0 22px; padding: 14px 16px 16px; background: var(--field); border-radius: 10px; border-left: 6px solid var(--c); }
  .otd h2 { margin: 0 0 6px; font-size: 15px; }
  .otd h2 small { font-weight: 400; font-size: 13px; color: var(--muted); margin-left: 8px; }
  /* a scrolling strip clips on every side: its padding leaves room for the photo's ring, the margin keeps the photos in place */
  .strip { display: flex; gap: 8px; overflow-x: auto; padding: 4px; margin: 0 -4px; scroll-padding-inline: 4px; }
  .strip button { flex: none; border: 0; background: none; padding: 0; text-align: left; color: var(--muted); }
  .strip .im { display: block; height: 120px; border-radius: 6px; background: var(--well) no-repeat 50% 50% / cover; }
  .strip .ph0 { width: 90px; }
  .strip small { font-size: 12px; }
  .strip button:hover .im { outline: 3px solid var(--ink); outline-offset: 1px; }
  .strip button:focus-visible .im { outline: 2px solid var(--focus); outline-offset: 2px; }
  .strip button:focus-visible { outline: none; }

  /* the heat calendar */
  .heat { display: grid; gap: 8px; }
  .yr { display: grid; grid-template-columns: 128px repeat(12, minmax(0, 1fr)); gap: 4px; align-items: stretch; }
  .yl { border: 0; background: none; padding: 0; text-align: left; color: inherit; font-size: 18px; font-weight: 700; display: flex; flex-direction: column; justify-content: center; gap: 1px; }
  .yl small { font-size: 11.5px; font-weight: 400; line-height: 1.35; color: var(--muted); }
  .yl small.rvt { color: var(--next-ink); font-weight: 600; }
  .yl small.okt { color: var(--done); font-weight: 600; }
  .yl:hover { color: var(--c); }
  .hm { position: relative; height: 60px; border-radius: 6px; border: 0; padding: 0; text-align: left; color: var(--ink);
    background: color-mix(in oklab, var(--c) var(--lv), var(--pane)); }
  .hm.here { box-shadow: inset 0 0 0 2px var(--ink); }
  .hm.empty { background: var(--well); color: var(--muted); display: block; }
  button.hm:hover, button.hm:focus-visible { outline: 3px solid var(--ink); outline-offset: 1px; }
  .hm .mn { position: absolute; left: 7px; top: 5px; font-size: 11.5px; font-weight: 600; opacity: .9; }
  .hm b { position: absolute; left: 7px; bottom: 8px; font-size: 17px; }
  .hm .rv { position: absolute; right: 4px; top: 4px; font-size: 10.5px; font-weight: 700; background: var(--next); color: var(--next-ink); padding: 0 5px; border-radius: 999px; }
  .hm .rv.ok { background: var(--field); color: var(--done); }
  /* A narrow month leaves no room beside its name: the review count sits under it. */
  .hm:not(.empty) { container-type: inline-size; }
  @container (max-width: 42px) { .hm .rv { top: 20px; left: 5px; right: auto; } .hm:not(.small) b { font-size: 14px; bottom: 6px; } }
  .legend { display: flex; gap: 6px 18px; align-items: center; flex-wrap: wrap; font-size: 12.5px; color: var(--muted); margin-top: 10px; }
  .legend .scale { display: inline-flex; gap: 2px; vertical-align: middle; margin: 0 6px; }
  .legend .scale i { width: 16px; height: 12px; border-radius: 2px; background: color-mix(in oklab, var(--c) var(--lv), var(--pane)); }
  .legend .rvk { display: inline-block; width: 18px; height: 12px; vertical-align: -1px; font-style: normal; font-size: 10.5px; font-weight: 700; background: var(--next); color: var(--next-ink); padding: 0 5px; border-radius: 999px; }
  .undated { margin: 14px 0 0; }

  /* skimming */
  .skim { position: fixed; z-index: 40; width: 280px; background: var(--field); border-radius: 9px; box-shadow: 0 10px 34px rgb(0 0 0 / .35); padding: 6px; pointer-events: none; }
  .skim .im { display: block; width: 100%; aspect-ratio: 4 / 3; border-radius: 6px; background: var(--well) no-repeat 50% 50% / cover; }
  .skim .t { font-weight: 700; font-size: 13.5px; margin: 6px 3px 0; }
  .skim .s { font-size: 12.5px; color: var(--muted); margin: 1px 3px 3px; }
  .dots { display: flex; gap: 2px; margin: 5px 3px 0; }
  .dots i { flex: 1; height: 3px; background: var(--line); border-radius: 2px; }
  .dots i.on { background: var(--c); }

  /* a year */
  .ystrip { position: sticky; top: -14px; z-index: 3; background: var(--pane); padding: 10px 0; border-bottom: 1px solid var(--line); }
  .yr12 { display: grid; grid-template-columns: repeat(12, minmax(0, 1fr)); gap: 4px; }
  .hm.small { height: 46px; }
  .hm.small b { font-size: 14px; bottom: 6px; }
  /* A narrow box in the strip has no room under its name either: the count sits on its corner, as a badge. */
  @container (max-width: 52px) { .hm.small .rv { top: -7px; right: -5px; left: auto; z-index: 1; box-shadow: 0 0 0 2px var(--pane); } }
  .ymon { margin-top: 22px; }
  /* What Tab reaches in a month lands below the sticky strip, not under it. */
  .ymon button { scroll-margin-top: 80px; }
  .ymon h3 { display: flex; gap: 12px; align-items: baseline; flex-wrap: wrap; margin: 0 0 10px; font-size: 19px; }
  .ymon h3 small { font-size: 13px; font-weight: 400; color: var(--muted); }
  .mh { border: 0; background: none; padding: 0; color: inherit; font: inherit; display: flex; gap: 12px; align-items: baseline; flex-wrap: wrap; text-align: left; }
  .mh:hover { color: var(--c); }
  .evs { display: grid; grid-template-columns: repeat(auto-fill, minmax(170px, 1fr)); gap: 16px 12px; }
  .evc { border: 0; padding: 0; background: none; text-align: left; min-width: 0; color: inherit; }
  .evc .im { position: relative; display: block; aspect-ratio: 4 / 3; border-radius: 7px; overflow: hidden; background: var(--well) no-repeat 50% 50% / cover; }
  .evc:hover .im, .evc:focus-visible .im { outline: 3px solid var(--ink); outline-offset: 2px; }
  .evc .im .dots { position: absolute; left: 4px; right: 4px; bottom: 5px; margin: 0; opacity: 0; transition: opacity .15s; }
  .evc:hover .im .dots { opacity: 1; }
  .evc .im .dots i { background: rgb(255 255 255 / .6); box-shadow: 0 0 0 1px rgb(0 0 0 / .3); }
  .evc .im .dots i.on { background: #FFB020; }
  .evc .rv { position: absolute; left: 6px; top: 6px; background: var(--next); color: var(--next-ink); font-size: 11px; font-weight: 700; padding: 1px 8px; border-radius: 999px; }
  .evc .rv.ok { background: var(--field); color: var(--done); }
  .evc b { display: block; font-size: 14px; font-weight: 600; line-height: 1.3; margin-top: 7px; }
  .evc small { display: block; color: var(--muted); font-size: 12.5px; line-height: 1.35; }

  /* a month */
  .mbar { display: flex; gap: 14px; align-items: center; flex-wrap: wrap; }
  /* Every week as tall as the tallest, so the month keeps its shape. */
  .mcal { display: grid; grid-template-columns: repeat(7, minmax(0, 1fr)); grid-template-rows: auto; grid-auto-rows: 1fr; margin-top: 12px; border-top: 1px solid var(--line); border-left: 1px solid var(--line); }
  .dow { padding: 4px 8px; font-size: 12px; font-weight: 600; color: var(--muted); border-right: 1px solid var(--line); border-bottom: 1px solid var(--line); }
  .day { position: relative; min-height: 128px; min-width: 0; border-right: 1px solid var(--line); border-bottom: 1px solid var(--line); padding: 0 0 6px; display: flex; flex-direction: column; }
  .day.has { cursor: pointer; }
  .day.has:hover .pic, .day.has:focus-visible .pic { outline: 3px solid var(--ink); outline-offset: 1px; }
  .day:focus-visible { outline: 2px solid var(--focus); outline-offset: -2px; }
  .day.dim { opacity: .35; }
  .day.sel { background: color-mix(in oklab, var(--c) 14%, transparent); }
  .num { font-size: 13px; font-weight: 600; padding: 3px 7px; color: var(--muted); }
  .dtop { display: flex; justify-content: space-between; align-items: flex-start; }
  .rvp { margin: 3px 6px 0 0; font-size: 10.5px; font-weight: 700; line-height: 1.6; background: var(--next); color: var(--next-ink); padding: 0 7px; border-radius: 999px; white-space: nowrap; min-width: 0; overflow: hidden; text-overflow: ellipsis; }
  .dtop { min-width: 0; gap: 2px; }
  .rvp.ok { background: none; color: var(--done); }
  .day.today .num { color: #fff; background: var(--c); border-radius: 0 0 6px 0; }
  /* As tall as the day is wide; a narrow day shows its first photo only. */
  .pic { aspect-ratio: 1; max-height: 220px; margin: 0 6px; border-radius: 5px; display: flex; gap: 2px; overflow: hidden; position: relative; }
  .day { container-type: inline-size; }
  @container (max-width: 150px) { .pic i:nth-of-type(n + 2) { display: none; } }
  /* A very narrow day (the day panel open in a small window) puts its badge under the number and
     leaves its count to the day's label, so nothing runs into the next day. */
  @container (max-width: 64px) {
    .dtop { flex-direction: column; align-items: flex-start; }
    .num { padding: 3px 5px; }
    .rvp { margin: 0 0 3px 4px; padding: 0 5px; max-width: calc(100% - 8px); }
    .pic { margin: 0 4px; }
    .pic em { display: none; }
  }
  /* Cropped from near the top, where faces usually are. */
  .pic i { flex: 1 1 0; min-width: 0; height: 100%; background: var(--well) no-repeat 50% 20% / cover; }
  .pic em { position: absolute; right: 3px; bottom: 3px; left: 3px; text-align: right; font-size: 11px; font-weight: 600; font-style: normal; color: #fff; }
  .pic em { line-height: 1.5; }
  .pic em span { background: rgb(22 32 42 / .8); padding: 0 4px; border-radius: 3px; }
  .pic em span.f { background: var(--c); }
  .bars { display: flex; flex-direction: column; gap: 2px; margin-top: 4px; }
  .bar { height: 15px; font-size: 10.5px; font-weight: 600; line-height: 15px; color: #fff; white-space: nowrap; overflow: hidden; padding: 0 5px; margin: 0 -1px; background: var(--bc); }
  .bar.s { margin-left: 6px; border-radius: 8px 0 0 8px; }
  .bar.e { margin-right: 6px; border-radius: 0 8px 8px 0; }
  .bar.s.e { border-radius: 8px; }

  /* the day panel, resizable from its left edge */
  .daypanel { position: fixed; right: 0; z-index: 30; background: var(--bg); box-shadow: -10px 0 30px rgb(0 0 0 / .22); border-left: 6px solid var(--c); display: flex; flex-direction: column; }
  .grip { position: absolute; left: -12px; top: 0; bottom: 0; width: 16px; cursor: col-resize; z-index: 2; touch-action: none; }
  .grip::after { content: ""; position: absolute; left: 3px; top: 50%; width: 6px; height: 48px; margin-top: -24px; border-radius: 3px; background: var(--field); box-shadow: 0 0 0 1px var(--c); }
  .grip:hover::after { background: var(--c); }
  .dh { position: relative; padding: 16px 22px 12px; border-bottom: 1px solid var(--line); }
  .dh h2 { margin: 0; font-size: 21px; padding-right: 44px; }
  .dh p { margin: 4px 0 0; color: var(--muted); font-size: 13px; }
  .x { position: absolute; top: 12px; right: 12px; width: 32px; height: 32px; border: 0; background: none; border-radius: 6px; display: grid; place-items: center; color: var(--ink); }
  .x:hover { background: var(--hover); }
  .x svg { width: 11px; height: 11px; fill: none; stroke: currentColor; stroke-width: 1.2; }
  .db { overflow-y: auto; padding: 4px 22px 40px; }
  .ev { margin-top: 18px; }
  .ev h3 { margin: 0 0 4px; font-size: 15px; display: flex; gap: 10px; align-items: center; flex-wrap: wrap; }
  .ev h3:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; border-radius: 4px; }
  .ev h3 small { font-size: 13px; font-weight: 400; color: var(--muted); }
  .ev h3 small.ok { color: var(--done); font-weight: 600; }
  .ev h3 .btn { font-size: 13px; padding: 3px 10px; }
  .pgrid { display: grid; gap: 6px; grid-template-columns: repeat(auto-fill, minmax(130px, 1fr)); margin-top: 8px; }
  .ph { position: relative; aspect-ratio: 1; border-radius: 6px; overflow: hidden; }
  .ph .hit { position: absolute; inset: 0; width: 100%; border: 0; padding: 0; background: var(--well) no-repeat 50% 50% / cover; }
  .ph:hover .hit, .ph .hit:focus-visible { outline: 3px solid var(--ink); outline-offset: -3px; }
  .ph.extra { outline: 3px dashed var(--muted); outline-offset: -3px; }
  .ph.aside .hit { opacity: .45; }
  .ph .n { position: absolute; right: 5px; top: 5px; background: var(--ink); color: var(--bg); font-size: 12px; font-weight: 700; padding: 1px 7px; border-radius: 999px; border: 2px solid var(--bg); }
  .ph .tag { position: absolute; left: 5px; top: 5px; font-size: 11px; font-weight: 700; padding: 1px 6px; border-radius: 4px; background: rgb(22 32 42 / .85); color: #fff; pointer-events: none; }
  .ph .tag.best { background: #33D17A; color: #062; }
  .ph .jk { position: absolute; left: 5px; bottom: 5px; background: #B42318; color: #fff; font-size: 11px; font-weight: 600; padding: 1px 6px; border-radius: 4px; pointer-events: none; }
  .ph .play { position: absolute; right: 5px; bottom: 5px; background: rgb(22 32 42 / .8); color: #fff; font-size: 11px; padding: 1px 6px; border-radius: 4px; pointer-events: none; }
  .set { margin-top: 10px; padding: 8px 10px 10px; border-radius: 8px; background: var(--side); }
  .set > div:first-child { font-size: 13px; color: var(--muted); }
  .shown-more { margin-top: 10px; }
  .row2 { display: flex; gap: 6px; margin-top: 6px; overflow-x: auto; }
  .row2 .ph { flex: none; width: 140px; }
</style>
