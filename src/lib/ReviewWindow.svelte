<script lang="ts">
  // Review in its own window: it moves, goes full screen (F11), sits on another screen or
  // steps aside for the main window. It reads the catalog, the choices, the notes and the
  // thumbnails itself, and tells the main window what changed (`api.tellReview`).
  import { onMount } from 'svelte'
  import { getCurrentWindow, Window } from '@tauri-apps/api/window'
  import { api, type Catalog, type Item, type Marks, type MoveScope, type PhotoPlace, type ReviewRequest, type Thumb, type Thumbs, type Work } from './api'
  import { asideIn, photoKey, placeOf, waitsAgain, inReview, type Notes } from './review'
  import Review from './Review.svelte'
  import MoveAside from './MoveAside.svelte'
  import ToastBox from './ToastBox.svelte'
  import Menu from './Menu.svelte'
  import AskBox from './AskBox.svelte'
  import { hideError, showError, showToast } from './toast.svelte'
  import { S } from './strings'

  const win = getCurrentWindow()
  let catalog = $state<Catalog>({ v: 1, sections: [] })
  let marks = $state<Marks>({ v: 2, marks: {}, ticks: {} })
  let notes = $state<Notes>({})
  let thumbs = $state<Record<string, Thumb>>({})
  let request = $state<ReviewRequest | null>(null)
  // Review takes its steps once: a new request, or a move it started, opens it again.
  let run = $state(0)
  let ready = $state(false)
  let full = $state(false)
  let moving = $state<{ place: PhotoPlace; scope?: MoveScope; resume?: boolean } | null>(null)

  // The events asked for, found by their files once taken, as an event's id changes when a move
  // takes its first photo.
  let files = new Set<string>()
  const works = $derived.by(() => {
    const all = catalog.sections.flatMap(s => s.works).filter(w => w.event)
    if (!files.size) {
      const ids = new Set(request?.works ?? [])
      files = new Set(all.filter(w => ids.has(w.id)).flatMap(w => w.items.map(i => i.path)))
    }
    return all.filter(w => w.items.some(i => files.has(i.path)))
  })
  const only = $derived.by(() => {
    if (!request?.files) return undefined
    const files = new Set(request.files)
    return (i: Item) => files.has(i.path)
  })
  const start = $derived(request?.start ? works.flatMap(w => w.items).find(i => i.path === request!.start) : undefined)

  const setThumbs = (t: Thumbs) => { thumbs = Object.fromEntries(t.done.map(x => [x.id, x])) }
  async function sort() { notes = (await api.photoSort()).notes }
  /** Reads the catalog, choices, thumbnails and notes, as the main window may have read folders again. */
  async function fresh() {
    catalog = await api.catalog()
    marks = await api.marks()
    setThumbs(await api.thumbs())
    await sort()
  }
  async function load() {
    request = await api.reviewRequest()
    files = new Set()
    run++
  }
  onMount(() => {
    const unThumbs = api.onThumbs(t => { for (const x of t.done) thumbs[x.id] = x })
    const unRequest = api.onReviewRequest(async () => { await fresh(); await load() })
    // Notes follow photos still being measured.
    const unShots = api.onShots(left => { if (!left) sort() })
    // Full screen left by the system (a key, the taskbar) shows on the button.
    const unSize = win.onResized(async () => { full = await win.isFullscreen() })
    // Choices made in the main window (Review again) show when this window comes back; after the
    // main window read folders again (a move's Undo) Review takes its steps again.
    let stale = false
    const unCatalog = api.onCatalogNews(() => { stale = true })
    const back = async () => {
      if (!stale) { marks = await api.marks(); return }
      stale = false
      await fresh()
      run++
    }
    window.addEventListener('focus', back)
    // Closed by its button, Esc or the system: the main window says what is kept and offers Move.
    const unClose = win.onCloseRequested(async () => {
      // Never in the way of closing.
      try {
        const places = [...new Map(works.map(w => [placeOf(catalog, w)?.id, w] as const).filter(([id]) => !!id)).values()]
        const n = places.reduce((a, w) => a + asideIn(placeOf(catalog, w)!.works, marks), 0)
        await api.tellReview({ what: 'closed', n, work: places.find(w => asideIn(placeOf(catalog, w)!.works, marks))?.id })
      } catch { /* the window closes anyway */ }
    })
    ;(async () => {
      await fresh()
      await load()
      full = await win.isFullscreen()
      ready = true
    })()
    return () => { window.removeEventListener('focus', back); unThumbs.then(f => f()); unRequest.then(f => f()); unShots.then(f => f()); unSize.then(f => f()); unClose.then(f => f()); unCatalog.then(f => f()) }
  })

  /** Marks as Review keeps them, said to the main window so its counts follow. */
  function setMarks(m: Marks) {
    marks = m
    api.tellReview({ what: 'marks' })
  }
  async function toggleFull() {
    full = !(await win.isFullscreen())
    await win.setFullscreen(full)
  }
  /** Steps aside: this window goes down to the taskbar and the main window comes up. */
  async function toApp() {
    await win.minimize()
    const main = await Window.getByLabel('main')
    await main?.unminimize()
    await main?.show()
    await main?.setFocus()
  }
  function keys(e: KeyboardEvent) {
    if (e.key === 'F11') { e.preventDefault(); toggleFull() }
  }
  /** Esc leaves full screen first; otherwise Review closes with its window. */
  async function close(how?: 'esc') {
    if (how === 'esc' && full) { full = false; await win.setFullscreen(false); return }
    await win.close()
  }
  /** Forgets a photo's Review choice, so it waits for Review again; Undo puts it back. */
  async function again(w: Work, i: Item) {
    const key = photoKey(i, w), old = marks.review?.[key]
    if (!old) return
    const waits = waitsAgain(i, w, notes, marks), shown = inReview(i, w, notes, marks)
    hideError()
    try { setMarks(await api.forgetReview([key])) } catch (e) { showError(e); return }
    showToast((waits ? S.reviewAgainDone : shown ? S.reviewAgainKept : S.reviewAgainStays)(i.name), async () => { setMarks(await api.restoreReview({ [key]: old })) })
  }
  /** After a move: its folder read again, the main window told, and Review opened again at the
   * sets still waiting when the move was started from its top. */
  async function moved() {
    const m = moving
    if (!m) return
    marks = await api.marks()
    // While the main window reads folders a second read is refused: Review then shows what it
    // has, and the main window's read brings the rest.
    let read = true
    hideError()
    try { catalog = await api.scan(m.place.home.id, m.place.under) } catch (e) { read = false; showError(e) }
    setThumbs(await api.thumbs())
    await sort()
    await api.tellReview({ what: read ? 'moved' : 'marks' })
    if (m.resume && request) { request = { ...request, start: undefined }; run++ }
  }
</script>

<svelte:window onkeydown={keys} />

{#if ready && request}
  {#key run}
    <Review works={works} label={request.label} {only} {start} {catalog} {notes} bind:marks={() => marks, setMarks} {thumbs} onagain={again}
      onmove={(p, scope, resume) => (moving = { place: p, scope, resume })} onclose={close}>
      {#snippet chrome()}
        <span class="win">
          <button class="btn" title={S.toAppTip} onclick={toApp}>{S.toApp}</button>
          <button class="btn" title={full ? S.fullOffTip : S.fullTip} aria-pressed={full} onclick={toggleFull}>{full ? S.fullOff : S.full}</button>
          <button class="ctl" aria-label={S.minimize} onclick={() => win.minimize()}><svg viewBox="0 0 10 10"><path d="M1 5h8" /></svg></button>
          <button class="ctl" aria-label={S.maximize} onclick={() => win.toggleMaximize()}><svg viewBox="0 0 10 10"><rect x="1.5" y="1.5" width="7" height="7" /></svg></button>
          <button class="ctl close" aria-label={S.close} onclick={() => win.close()}><svg viewBox="0 0 10 10"><path d="M1.5 1.5l7 7M8.5 1.5l-7 7" /></svg></button>
        </span>
      {/snippet}
    </Review>
  {/key}
{/if}
{#if moving}
  <MoveAside section={moving.place} scope={moving.scope} {catalog} {thumbs} onmoved={moved} onreveal={(p, open) => { hideError(); api.reveal(p, open).catch(e => showError(e)) }} onclose={() => (moving = null)} />
{/if}
<AskBox />
<Menu />
<ToastBox />

<style>
  .win { display: flex; align-items: center; gap: 8px; align-self: stretch; }
  /* As Review's own buttons. */
  .btn { border: 1px solid rgba(255, 255, 255, .3); background: rgba(255, 255, 255, .08); color: #fff; border-radius: 6px; padding: 5px 12px; white-space: nowrap; }
  .btn:hover { background: rgba(255, 255, 255, .18); }
  .btn[aria-pressed='true'] { background: rgba(255, 255, 255, .22); }
  .ctl { width: 46px; align-self: stretch; border: 0; background: none; color: #fff; display: grid; place-items: center; }
  .ctl:hover { background: rgba(255, 255, 255, .1); }
  .ctl.close:hover { background: #C42B1C; }
  .ctl:first-of-type { margin-left: 4px; }
  svg { width: 10px; height: 10px; fill: none; stroke: currentColor; stroke-width: 1; }
</style>
