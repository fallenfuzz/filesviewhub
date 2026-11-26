<script lang="ts">
  // Review (`decisions/2026/10/04/sorting-ends-in-a-move-script-never-a-delete.md`, `decisions/2026/10/04/photos-take-no-marks-review-is-the-progress.md`), drawn as the Viewer is: one step at a time over the window, every
  // Photo of a set of Similar shots or Copies in a grid that fits the window, or one of them large
  // with the set in a strip below. Next keeps the choice, Skip and Back only look; the last
  // step ends on what was set aside, what was skipped and Move (`decisions/2026/10/09/review-moves-set-aside-photos-itself-with-undo.md`).
  import { tick, untrack, type Snippet } from 'svelte'
  import { convertFileSrc } from '@tauri-apps/api/core'
  import { api, keyOf as tickKey, size, thumbSrc, type Catalog, type Item, type Marks, type MoveScope, type PhotoPlace, type Thumb, type Work } from './api'
  import { asideIn, canGoOn, choiceOf, choosable, isSetAside, keptIn, photoKey, placeOf, sharpRanks, stepsAt, waiting, workOf, type Elsewhere, type Notes, type Step } from './review'
  import { openMenu } from './menus.svelte'
  import { S } from './strings'

  let { works, label, catalog, notes, marks = $bindable(), thumbs, onclose, onmove, onagain, only, start, chrome }: {
    works: Work[]; label: string; catalog: Catalog; notes: Notes; marks: Marks; thumbs: Record<string, Thumb>
    /** Review is done; `esc` when by the Esc key. */
    onclose: (how?: 'esc') => void
    /** The window's own buttons, at the right of the top bar, which then moves the window. */
    chrome?: Snippet
    /** Opens Move for the set-aside photos of a Photos Section or Photos folder; `resume`: Review
     * opens again afterwards on the sets still waiting. */
    onmove: (s: PhotoPlace, scope?: MoveScope, resume?: boolean) => void
    /** Forgets a photo's Review choice, so it waits for Review again. */
    onagain: (w: Work, i: Item) => void
    /** Photos only or Clips only: Review goes through files of that kind. */
    only?: (i: Item) => boolean
    /** The Photo Review was opened from: Review opens at its set. */
    start?: Item
  } = $props()

  // The steps are taken once, so a choice made does not move the steps under the person.
  // With nothing waiting, Review goes through the choices made before.
  // A set of Copies whose file is in another event of the same Photos Section or folder shows it.
  const elsewhere: Elsewhere = untrack(() => {
    const by = new Map<string, { i: Item; w: Work }>()
    for (const w of works) for (const x of placeOf(catalog, w)?.works ?? []) for (const i of x.items) by.set(i.path, { i, w: x })
    return path => by.get(path)
  })
  const first = untrack(() => stepsAt(works, notes, marks, only, start, elsewhere))
  const steps: Step[] = first.steps
  let at = $state(first.at)
  let error = $state('')
  const DRAWN = new Set(['jpg', 'jpeg', 'png', 'gif', 'webp', 'bmp', 'insp'])
  const step = $derived(steps[at] as Step | undefined)
  // What is kept in each step, from the choices made before and the suggestion.
  const kept = (s: Step) => keptIn(s, notes, marks)
  let keeps = $state(untrack(() => steps.map(kept)))
  // The choices each step was drawn from. When the marks change by Review again or its Undo, a
  // step with no change of the person's own is drawn again from them, so Next never saves the
  // choice just forgotten.
  let bases = untrack(() => steps.map(kept))
  const same = (a: Set<string>, b: Set<string>) => a.size === b.size && [...a].every(p => b.has(p))
  $effect(() => {
    void marks
    untrack(() => steps.forEach((s, k) => {
      const now = kept(s)
      if (same(keeps[k], bases[k]) && !same(keeps[k], now)) keeps[k] = now
      bases[k] = now
    }))
  })
  const keep = $derived(keeps[at])
  // A step is chosen once no photo in it waits, as the counts on the cards say.
  const chosen = (s: Step) => !s.items.some(i => choosable(s, i.path) && waiting(i, workOf(s, i), notes, marks))
  const left = $derived(steps.filter(s => !chosen(s)).length)
  // Each Photo once: the file of a set of Copies also shows in its own step.
  const asideHere = $derived(new Set(steps.flatMap(s => s.items.filter(i => choosable(s, i.path) && isSetAside(i, workOf(s, i), marks)).map(i => i.path))).size)
  // What the step shows: the suggestion, the choice kept, or a change Next has not kept yet.
  const shown = $derived.by(() => {
    if (!step) return ''
    return !same(keep, kept(step)) ? S.notSaved : chosen(step) ? S.chosenHere : S.notChosen
  })
  // After "Go through the skipped ones", Next and Skip go to the next step not chosen.
  let skippedOnly = $state(false)
  // Focus starts on Review itself, so a first Enter goes to the step, not to a button.
  let root: HTMLDivElement
  $effect(() => { root?.focus() })
  // Focus goes back to what opened Review when it closes, if that is still there.
  const opener = document.activeElement as HTMLElement | null
  $effect(() => () => { if (opener?.isConnected) opener.focus() })

  /** The Photos Sections and Photos folders of the events reviewed, each once (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`). */
  // An event of each, as Review opened; each is looked up again in the catalog, so a move and its read show.
  const firsts = untrack(() => [...new Map(works.map(w => [placeOf(catalog, w)?.id, w] as const).filter(([id]) => !!id)).values()])
  const sections = $derived(firsts.map(w => placeOf(catalog, w)).filter(p => !!p))
  const total = $derived(sections.reduce((a, s) => a + asideIn(s.works, marks), 0))


  /** Keeps a Photo, or marks it Not needed. */
  function choose(i: Item, on: boolean) {
    // A Photo chosen in its own step only shows here.
    if (!step || !choosable(step, i.path)) return
    const k = new Set(keep)
    if (on) k.add(i.path)
    else k.delete(i.path)
    keeps[at] = k
  }
  const flip = (i: Item) => choose(i, !keep.has(i.path))
  async function commit(s: Step, k: Set<string>) {
    const on: [string, number][] = [], off: [string, number][] = []
    for (const i of s.items.filter(i => choosable(s, i.path))) (k.has(i.path) ? on : off).push([photoKey(i, workOf(s, i)), i.size])
    if (on.length) marks = await api.setReview(on, true)
    if (off.length) marks = await api.setReview(off, false)
  }
  function go() {
    big = null
    if (!skippedOnly) { at++; return }
    const n = steps.findIndex((s, k) => k > at && !chosen(s))
    at = n < 0 ? steps.length : n
  }
  // A second Enter while a choice is being saved waits for the next step to show.
  let saving = false
  async function next(k = keep) {
    if (!step || saving) return
    error = ''
    saving = true
    try {
      await commit(step, k)
      keeps[at] = k
      go()
      // Focus leaves the button pressed, so a next Enter is Next again, never that button.
      root?.focus()
    } catch (e) { error = String(e) } finally { saving = false }
  }
  /** Move part way: a change on this step is kept first, as Next would, so the move and
   * the Review that opens after it both have it. */
  async function moveNow(s: PhotoPlace) {
    if (step && canGoOn(keep) && !same(keep, kept(step))) {
      try { await commit(step, keep) } catch (e) { error = String(e); return }
    }
    onmove(s, { name: label, works }, true)
  }
  /** Moves on and keeps nothing: the step stays as it was. */
  function skip() {
    if (!step) return
    error = ''
    go()
    root?.focus()
  }
  /** One step back; from the end, the last step. */
  function back() {
    big = null
    if (at) at--
    root?.focus()
  }
  function goSkipped() {
    skippedOnly = true
    at = steps.findIndex(s => !chosen(s))
    // The button goes with the end screen: focus stays on Review.
    root?.focus()
  }
  // The large view: the index of the Photo shown, or none for the grid.
  let big = $state<number | null>(null)
  let stage = $state<HTMLDivElement>()
  let strip = $state<HTMLDivElement>()
  // The strip keeps the Photo shown in view.
  $effect(() => {
    void big
    strip?.querySelector('.here')?.scrollIntoView({ block: 'nearest', inline: 'nearest' })
  })
  let grid = $state<HTMLDivElement>()
  async function openBig(k: number) {
    big = k
    await tick()
    stage?.focus()
  }
  async function closeBig() {
    const k = big
    big = null
    await tick()
    // Focus goes back to the Photo shown, so the keys go on from there.
    grid?.querySelectorAll<HTMLButtonElement>('.pic')[k ?? 0]?.focus()
  }
  const show = (k: number) => { if (step && big != null) big = Math.max(0, Math.min(step.items.length - 1, k)) }
  /** The Photo a key acts on in the grid: the one with focus, if any. */
  const focused = () => {
    const el = document.activeElement
    return el instanceof HTMLElement && el.dataset.photo != null ? Number(el.dataset.photo) : -1
  }
  /** Tab and Shift+Tab from the last or first button go round to the other end. */
  function tab(e: KeyboardEvent) {
    const all = [...(root?.querySelectorAll<HTMLElement>('button:not([disabled])') ?? [])].filter(b => b.offsetParent)
    if (!all.length) return
    const k = all.indexOf(document.activeElement as HTMLElement)
    const to = e.shiftKey ? (k <= 0 ? all.at(-1) : null) : k < 0 || k === all.length - 1 ? all[0] : null
    if (to) { e.preventDefault(); e.stopPropagation(); to.focus() }
  }
  function keys(e: KeyboardEvent) {
    // Keys in a dialog over Review (Move) are that dialog's.
    if (e.target instanceof Node && e.target !== document.body && !root?.contains(e.target)) return
    // Review covers the window: Tab goes round its own buttons, and the search keys (Ctrl+K, /)
    // would only move focus to the search box hidden under it.
    if (e.key === 'Tab') { tab(e); return }
    if ((e.key.toLowerCase() === 'k' && (e.ctrlKey || e.metaKey)) || (e.key === '/' && !e.ctrlKey && !e.altKey && !e.metaKey)) { e.preventDefault(); e.stopPropagation(); return }
    // Other keys with Ctrl, Alt or the Windows key are the app's.
    if (e.ctrlKey || e.altKey || e.metaKey) return
    // On a Photo, its badge or the strip, Space clicks it and Enter still means Next; on Back, Skip
    // and the other buttons both are theirs.
    const target = e.target instanceof HTMLElement ? e.target : null
    // Enter on Mark all not needed is Next too, so Enter never sets a whole set aside.
    const onPhoto = target?.dataset.photo != null || target?.dataset.next != null
    if (target instanceof HTMLButtonElement && (e.key === ' ' || (e.key === 'Enter' && !onPhoto))) return
    const key = e.key.length === 1 ? e.key.toLowerCase() : e.key
    let used = true
    if (step && big != null) {
      if (key === 'Escape' || key === ' ') closeBig()
      else if (key === 'ArrowLeft') show(big - 1)
      else if (key === 'ArrowRight') show(big + 1)
      else if (key === 'Home') show(0)
      else if (key === 'End') show(step.items.length - 1)
      else if (key === 'k' || key === 'x') choose(step.items[big], key === 'k')
      else used = false
    } else used = false
    if (!used) {
      used = true
      if (key === 'Escape') onclose('esc')
      else if (key === 'ArrowLeft' || key === 'Backspace') back()
      else if (!step) used = false
      else if (key === 'ArrowRight') skip()
      // Enter never sets a whole set aside: that takes its own button.
      // Enter never sets a whole set aside: that takes its own button. The bar says why.
      else if (key === 'Enter') { if (canGoOn(keep)) next(keep) }
      else if (key === ' ') openBig(Math.max(0, step.items.findIndex(i => keep.has(i.path))))
      else if ((key === 'k' || key === 'x') && focused() >= 0) choose(step.items[focused()], key === 'k')
      else if (/^[0-9]$/.test(key) && (Number(key) || 10) <= step.items.length) flip(step.items[(Number(key) || 10) - 1])
      else used = false
    }
    if (used) { e.preventDefault(); e.stopPropagation() }
  }
  const thumbOf = (s: Step, i: Item) => thumbs[`${workOf(s, i).id}#${tickKey(i)}`]
  const time = (i: Item) => (i.at ? new Date(i.at * 1000).toISOString().slice(11, 19) : '')
  /** The key that flips a Photo: 1 to 9, then 0 for the tenth. */
  const keyOf = (k: number) => (k < 9 ? String(k + 1) : k === 9 ? '0' : '')
  const ranks = $derived(step ? sharpRanks(step, notes) : new Map<string, number>())
  /** The same facts for every Photo in every kind of set: when it was taken (with its day
   * when it is from another event), its size in pixels and on disk, and its sharpness rank. */
  function facts(s: Step, i: Item): string[] {
    const n = notes[i.path], w = workOf(s, i), r = ranks.get(i.path)
    const when = w !== s.work && time(i) ? S.dayAndTime(w.title, time(i)) : time(i)
    return [when, n?.px ? S.pixels(n.px[0], n.px[1]) : '', size(i.size), r ? S.sharpRank(r) : ''].filter(Boolean)
  }
  // Where a step holds both photos from the camera and smaller ones without camera details (sent
  // over a messenger), each says which it is.
  const area = (i: Item) => { const px = notes[i.path]?.px; return px ? px[0] * px[1] : 0 }
  function origin(s: Step, i: Item): 'cam' | 'sent' | 'same' | '' {
    const cams = s.items.filter(x => notes[x.path]?.cam)
    const same = notes[i.path]?.version ? 'same' : ''
    if (!cams.length || cams.length === s.items.length) return same
    // Two from the camera taken at the same second are one photo, one of them edited: neither
    // file says which came first.
    if (notes[i.path]?.cam) return same && cams.some(x => x !== i && x.at === i.at) ? 'same' : 'cam'
    // Only a copy or a version of another Photo here: a photo of its own from someone else is no copy.
    const copy = !!(notes[i.path]?.copy || notes[i.path]?.version)
    return copy && area(i) && area(i) < Math.max(...cams.map(area)) ? 'sent' : same
  }
  // Every Photo of the step is one picture: versions of it, or copies of one Photo.
  const versions = (s: Step) => s.kind === 'similar' && s.items.length > 1 && s.items.every(i => notes[i.path]?.version || notes[i.path]?.copy)
  const TAG = { cam: [S.original, S.fromCamera], sent: [S.sent, S.sentCopy], same: [S.samePhoto, S.samePhotoTip] } as const
  const whyBest = (s: Step) => {
    const b = s.items.map(i => notes[i.path]).find(n => n?.best)
    return !b?.why ? '' : b.why[0] === 'sharp' ? S.sharper(b.why[1]) : S.why[b.why[0]]
  }

  // The grid fits the window: as many columns as make the Photos largest, each Photo's box from
  // its own shape, so every Photo of a set of 4, 10 or more shows at once.
  let gw = $state(0), gh = $state(0)
  const GAP = 12, PAD = 16, RING = 6
  const shape = (s: Step, i: Item) => {
    const px = notes[i.path]?.px, th = thumbOf(s, i)
    return px?.[0] && px[1] ? px[0] / px[1] : th?.w && th.h ? th.w / th.h : 4 / 3
  }
  const fit = $derived.by(() => {
    if (!step || !gw || !gh) return { cols: 1, w: 0, h: 0, cap: 0, lines: 1 }
    const n = step.items.length, shapes = step.items.map(i => shape(step, i))
    let out = { cols: 1, w: 0, h: 0, cap: 0, lines: 1, score: -1 }
    // From one row down: another row is taken only when it makes the Photos clearly larger, as
    // one row compares best.
    for (let cols = n; cols >= 1; cols--) {
      const rows = Math.ceil(n / cols)
      const w = (gw - 2 * PAD - (cols - 1) * GAP) / cols - RING
      // Under a narrow Photo its facts take two lines, under a very narrow one a line each.
      const lines = w < 170 ? 4 : w < 300 ? 2 : 1, cap = 8 + 16 * lines
      // Never boxes narrower than the smallest one, which would spill sideways.
      if (w < 80 && cols > 1) continue
      // Two pixels spare, so rounding never makes the grid scroll.
      const h = (gh - 2 - 2 * PAD - (rows - 1) * GAP) / rows - RING - cap
      const score = shapes.reduce((a, r) => a + Math.max(0, Math.min(w, h * r)) * Math.max(0, Math.min(h, w / r)), 0)
      if (score > out.score * 1.15) out = { cols, w, h, cap, lines, score }
    }
    return out
  })
  // The large view's Photo, as large as the space it has.
  let sw = $state(0), sh = $state(0)
  const boxIn = (w: number, h: number, r: number) => {
    const bw = Math.max(0, Math.min(w - 32, (h - 18) * r))
    return `width:${Math.round(bw)}px;height:${Math.round(bw / r)}px`
  }
  // A box never smaller than this; a set too big for the window then scrolls.
  const boxOf = (s: Step, i: Item) => {
    const r = shape(s, i), w = Math.max(80, Math.min(fit.w, fit.h * r))
    return `width:${Math.round(w)}px;height:${Math.round(w / r)}px`
  }
</script>

<svelte:window onkeydowncapture={keys} />

<div class="review" bind:this={root} tabindex="-1" role="dialog" aria-modal="true" aria-label={S.reviewTitle(label)}>
  <div class="top" class:own={!!chrome} data-tauri-drag-region={chrome ? 'deep' : undefined}>
    <b>{S.reviewTitle(label)}</b>
    {#if step}<span class="n">{S.stepOf(at + 1, steps.length)}</span><span class="t">{step.work.title}</span><span class="what">{versions(step) ? S.stepVersions : S.stepWhat[step.kind]}</span><span class="state" class:done={shown === S.chosenHere}>{shown}</span>{/if}
    <span class="grow"></span>
    {#if step}
      {#each sections as s (s.id)}
        {@const n = asideIn(s.works, marks)}
        {#if n}<button class="btn" title={S.moveNowTip} onclick={() => moveNow(s)}>{S.moveNow(n)}{sections.length > 1 ? ` ${s.name}` : ''}</button>{/if}
      {/each}
    {/if}
    {#if chrome}{@render chrome()}{:else}<button class="btn" onclick={() => onclose()} aria-label={S.close}>×</button>{/if}
  </div>
  {#if step}
    {@const menu = (e: MouseEvent, i: Item) => { const w = workOf(step!, i); if (choiceOf(i, w, marks)) openMenu(e, [{ label: S.reviewAgain, icon: 'review', run: () => onagain(w, i) }], i.name) }}
    {#if big == null}
      <div class="cmp" class:tight={fit.w < 190} bind:this={grid} bind:clientWidth={gw} bind:clientHeight={gh} style="--cols:{fit.cols};--cap:{fit.cap}px">
        {#each step.items as i, k (i.path)}
          {@const th = thumbOf(step, i)}
          {@const on = keep.has(i.path)}
          {@const f = facts(step, i)}
          <div class="ph" class:on class:off={!on}>
            <button class="pic" data-photo={k} onclick={() => openBig(k)} oncontextmenu={e => menu(e, i)} title={i.name}
              aria-label={S.photoLabel(i.name, f.join(', '), on ? S.keep : S.setAside)}>
              {#if DRAWN.has(i.ext) && !i.cloud}<img src={convertFileSrc(i.path, 'photo')} alt="" style="{boxOf(step, i)};{th ? `background-image:url(${thumbSrc(th.key)})` : ''}" />
              {:else if th}<img src={thumbSrc(th.key)} alt="" style={boxOf(step, i)} />
              {:else}<span class="none" style={boxOf(step, i)}></span>{/if}
            </button>
            <button class="k" data-photo={k} onclick={() => flip(i)} aria-pressed={on} disabled={!choosable(step, i.path)} title={choosable(step, i.path) ? S.flipTip : S.chosenThere}
              aria-label={S.badgeLabel(i.name, on ? S.keep : S.setAside)}><span class="ic">{on ? '✓' : '✕'}</span><span class="w">{on ? S.keep : S.setAside}</span></button>
            {#if keyOf(k)}<kbd>{keyOf(k)}</kbd>{/if}
            {#if origin(step, i)}{@const o = origin(step, i) as keyof typeof TAG}<span class="src" class:sent={o !== 'cam'} title={TAG[o][1]}>{TAG[o][0]}</span>{/if}
            <!-- On two lines: when and its pixels, then its file size and rank. -->
            <small title={f.join(' · ')}>
              {#if fit.lines === 4}{#each f as x, n (n)}<span>{x}</span>{/each}
              {:else if fit.lines === 2}<span>{f.slice(0, 2).join(' · ')}</span><span>{f.slice(2).join(' · ')}</span>
              {:else}<span>{f.join(' · ')}</span>{/if}
            </small>
          </div>
        {/each}
      </div>
    {:else}
      {@const i = step.items[big]}
      {@const th = thumbOf(step, i)}
      {@const on = keep.has(i.path)}
      <div class="big">
        <div class="stage" class:off={!on} bind:this={stage} bind:clientWidth={sw} bind:clientHeight={sh} tabindex="-1" role="group" aria-label={S.photoLabel(i.name, facts(step, i).join(', '), on ? S.keep : S.setAside)}
          oncontextmenu={e => menu(e, i)}>
          <span class="frame" class:on style={boxIn(sw, sh, shape(step, i))}>
            {#key i.path}
              {#if DRAWN.has(i.ext) && !i.cloud}<img src={convertFileSrc(i.path, 'photo')} alt="" style={th ? `background-image:url(${thumbSrc(th.key)})` : ''} />
              {:else if th}<img src={thumbSrc(th.key)} alt="" />{/if}
            {/key}
            <button class="k" data-photo={big} onclick={() => flip(i)} aria-pressed={on} disabled={!choosable(step, i.path)} title={choosable(step, i.path) ? S.flipKeys : S.chosenThere}
              aria-label={S.badgeLabel(i.name, on ? S.keep : S.setAside)}><span class="ic">{on ? '✓' : '✕'}</span><span class="w">{on ? S.keep : S.setAside}</span></button>
            <span class="of">{S.stepOf(big + 1, step.items.length)}</span>
          </span>
        </div>
        <div class="facts"><b title={i.name}>{i.name}</b><span>{[...facts(step, i), origin(step, i) ? TAG[origin(step, i) as keyof typeof TAG][1] : ''].filter(Boolean).join(' · ')}</span></div>
        <div class="strip" bind:this={strip} role="group" aria-label={S.theSet}>
          {#each step.items as x, k (x.path)}
            {@const tx = thumbOf(step, x)}
            <button class="sp" class:on={keep.has(x.path)} class:here={k === big} data-photo={k} onclick={() => (big = k)} oncontextmenu={e => menu(e, x)} title={x.name}
              aria-current={k === big} aria-label={S.photoLabel(x.name, facts(step, x).join(', '), keep.has(x.path) ? S.keep : S.setAside)}>
              {#if tx}<img src={thumbSrc(tx.key)} alt="" />{:else if DRAWN.has(x.ext) && !x.cloud}<img src={convertFileSrc(x.path, 'photo')} alt="" />{/if}
              <span class="dot">{keep.has(x.path) ? '✓' : '✕'}</span>
              {#if keyOf(k)}<kbd>{keyOf(k)}</kbd>{/if}
            </button>
          {/each}
        </div>
      </div>
    {/if}
    <div class="bar">
      <span class="help">
        {#if big != null}{S.largeHelp}
        {:else if step.kind === 'similar'}{#if step.call}{S.callHelp}{:else}<b>{whyBest(step)}</b> {S.similarHelp}{/if}
        {:else}{S.copiesHelp}{#if step.from}{' '}{S.copyThere}{:else if step.fixed}{' '}{S.copyInSet}{/if}{/if}
        {#if !canGoOn(keep)}<span class="warn" role="status">{S.keepOne}</span>
        {:else if at <= first.at}<span class="once">{S.asideOnce}</span>{/if}
      </span>
      {#if error}<span class="err">{error}</span>{/if}
      <span class="acts">
        <button class="btn" disabled={!at} onclick={back}>{S.stepBack}</button>
        <button class="btn" title={S.skipTip} onclick={skip}>{S.skip}</button>
        <!-- Sets aside every Photo chosen here; one chosen in its own step stays as it is. -->
        <button class="btn" data-next onclick={() => next(new Set([...keep].filter(p => !choosable(step!, p))))}>{S.setAllAside}</button>
        <button class="btn go" disabled={!canGoOn(keep)} title={canGoOn(keep) ? '' : S.keepOne} onclick={() => next()}>{S.nextStep}</button>
      </span>
    </div>
  {:else}
    <div class="sum">
      <h2>{!steps.length ? S.nothingToReview : left ? S.skippedSteps(left) : S.reviewed}</h2>
      {#if steps.length && (asideHere || total || !left)}<p>{S.reviewedHere(asideHere, total, sections.length === 1 ? sections[0].name : '')}</p>{/if}
      {#if left}<button class="btn go" onclick={goSkipped}>{S.goSkipped}</button>{/if}
      {#if total}
        <p class="muted">{S.scriptWhat}</p>
        {#each sections as s (s.id)}
          <button class="btn go" onclick={() => onmove(s, { name: label, works })}>{S.moveAside}{sections.length > 1 ? ` ${s.name}` : ''}</button>
        {/each}
      {/if}
      {#if error}<p class="err">{error}</p>{/if}
      <div>
        {#if steps.length}<button class="btn" onclick={back}>{S.stepBack}</button>{/if}
        <button class="btn" onclick={() => onclose()}>{S.close}</button>
      </div>
    </div>
  {/if}
</div>

<style>
  .review { position: fixed; inset: 0; z-index: 50; background: #0b0d12; color: #fff; display: grid; grid-template-rows: auto 1fr auto; grid-template-columns: minmax(0, 1fr); outline: none; }
  .top, .bar { display: flex; align-items: center; gap: 14px; padding: 10px 16px; font-size: 13px; }
  .top { border-bottom: 1px solid rgba(255, 255, 255, .12); }
  /* In its own window the top bar is the title bar: the window's buttons reach its edge. */
  .top.own { padding-top: 0; padding-bottom: 0; padding-right: 0; min-height: 44px; user-select: none; white-space: nowrap; }
  /* Narrow, the words give way first, the kind of step most; the window's buttons never do. */
  .top.own > * { flex-shrink: 0; }
  .top.own > b, .top.own .what, .top.own .state, .top.own > .btn { flex-shrink: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; }
  .top.own .what { flex-shrink: 100; }
  .top.own .state { flex-shrink: 50; }
  /* Too narrow for it to be read: the step's state goes before it is cut to a stub. */
  @media (max-width: 1100px) { .top.own .state, .top.own .what { display: none; } }
  .top.own > b { flex-shrink: 10; }
  .top.own .btn { white-space: nowrap; }
  .bar { border-top: 1px solid rgba(255, 255, 255, .12); }
  .n, .what { opacity: .75; }
  /* In a narrow window the step count and its state keep to one line; the titles wrap. */
  .n, .state { white-space: nowrap; }
  .state { font-size: 12px; padding: 2px 9px; border-radius: 999px; background: rgba(255, 210, 80, .18); color: #ffd666; }
  .state.done { background: rgba(51, 209, 122, .18); color: #5be39a; }
  .t { font-weight: 600; }
  .grow { flex: 1; }
  /* In a narrow window the buttons go below the help. */
  .bar { flex-wrap: wrap; }
  .help { flex: 1 1 340px; }
  .acts { flex: none; max-width: 100%; display: flex; flex-wrap: wrap; gap: 10px; justify-content: flex-end; margin-left: auto; }
  .help { line-height: 1.4; }
  .cmp { display: grid; grid-template-columns: repeat(var(--cols), minmax(0, 1fr)); gap: 12px; justify-items: center; align-items: center; align-content: safe center;
    padding: 16px; min-height: 0; overflow: auto; }
  /* Each Photo's frame hugs the Photo; its box is sized to fit the window (boxOf). */
  .ph { position: relative; border: 3px solid transparent; border-radius: 10px; display: flex; flex-direction: column; align-items: center; max-width: 100%;
    transition: border-color .12s; }
  .ph.on { border-color: #33d17a; }
  .pic { display: block; border: 0; padding: 0; background: none; border-radius: 7px; cursor: zoom-in; }
  .pic img, .none { display: block; object-fit: contain; border-radius: 7px; background: center / contain no-repeat; transition: opacity .12s, filter .12s; }
  .none { background: rgba(255, 255, 255, .06); }
  /* Not needed stays easy to judge: a little dimmer, never faded out. */
  .off .pic img, .stage.off img { opacity: .82; filter: saturate(.85); }
  .pic:hover img { opacity: 1; filter: none; }
  .pic:focus-visible, .k:focus-visible, .sp:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; }
  .k { position: absolute; top: 8px; left: 8px; font: inherit; font-size: 12px; font-weight: 700; padding: 3px 10px; border-radius: 999px; cursor: pointer;
    color: #fff; background: #c2410c; border: 1px solid rgba(255, 255, 255, .25); box-shadow: 0 1px 4px rgba(0, 0, 0, .4); transition: background .12s, transform .12s; }
  .k:hover:not(:disabled) { background: #dc5a1f; }
  .k:disabled { cursor: default; }
  .k .ic { margin-right: 5px; }
  /* Under a narrow Photo the badge is its mark alone, clear of the key. */
  .tight .k { padding: 3px 7px; }
  .tight .k .ic { margin: 0; }
  .tight .k .w { display: none; }
  .k:active { transform: scale(.96); }
  .on .k { background: #33d17a; color: #052e16; border-color: transparent; }
  .on .k:hover:not(:disabled) { background: #4ade80; }
  /* The key that flips the Photo, drawn as a key so it does not read as a count. */
  kbd { position: absolute; top: 8px; right: 8px; min-width: 22px; padding: 1px 6px; font-family: inherit; font-size: 12px; font-weight: 600; line-height: 18px; text-align: center; color: #fff;
    background: rgba(22, 32, 42, .85); border: 1px solid rgba(255, 255, 255, .35); border-bottom-width: 2px; border-radius: 5px; pointer-events: none; }
  /* Original or Sent copy, on the picture's lower corner. */
  .src { position: absolute; left: 8px; bottom: calc(var(--cap) + 13px); padding: 2px 8px; border-radius: 999px; font-size: 12px; font-weight: 600; pointer-events: auto;
    color: #fff; background: rgba(22, 32, 42, .85); border: 1px solid rgba(255, 255, 255, .35); }
  .src.sent { color: #fde68a; border-color: rgba(253, 230, 138, .5); }
  .once { display: block; opacity: .75; }
  .warn { display: block; color: #fdba74; }
  small { font-size: 12px; opacity: .85; min-height: var(--cap); max-width: 100%; display: flex; flex-direction: column; align-items: center; padding: 5px 4px 0; line-height: 16px; }
  small span { max-width: 100%; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .big { display: grid; grid-template-rows: 1fr auto auto; min-height: 0; animation: in .14s ease-out; }
  @keyframes in { from { opacity: 0; } }
  @media (prefers-reduced-motion: reduce) { .big { animation: none; } }
  .stage { display: flex; align-items: center; justify-content: center; min-height: 0; padding: 12px 16px 6px; outline: none; overflow: hidden; }
  .frame { position: relative; display: block; border-radius: 10px; border: 3px solid transparent; transition: border-color .12s; }
  .frame.on { border-color: #33d17a; }
  .frame img { display: block; width: 100%; height: 100%; object-fit: contain; border-radius: 7px; background: center / contain no-repeat; }
  .frame .k { font-size: 13px; }
  .of { position: absolute; top: 8px; right: 8px; font-size: 12px; padding: 2px 9px; border-radius: 999px; background: rgba(22, 32, 42, .85); }
  .facts { display: flex; gap: 12px; justify-content: center; align-items: baseline; padding: 4px 16px; font-size: 13px; min-width: 0; }
  .facts b { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; min-width: 0; max-width: 50%; }
  .facts span { opacity: .85; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; min-width: 0; }
  .strip { display: flex; gap: 8px; padding: 8px 16px 12px; overflow-x: auto; justify-content: safe center; }
  .sp { position: relative; flex: none; height: 72px; padding: 0; border: 3px solid transparent; border-radius: 8px; background: rgba(255, 255, 255, .06); cursor: pointer; opacity: .8;
    transition: opacity .12s, border-color .12s; }
  .sp img { display: block; height: 100%; width: auto; max-width: 140px; object-fit: cover; border-radius: 5px; }
  .sp.on { border-color: #33d17a; }
  .sp:hover, .sp.here { opacity: 1; }
  .sp.here { outline: 2px solid #fff; outline-offset: 2px; }
  .dot { position: absolute; bottom: 4px; left: 4px; width: 18px; height: 18px; border-radius: 50%; font-size: 11px; font-weight: 700; line-height: 18px; text-align: center;
    background: #c2410c; color: #fff; }
  .sp.on .dot { background: #33d17a; color: #052e16; }
  .sp kbd { top: 4px; right: 4px; min-width: 18px; padding: 0 4px; font-size: 11px; line-height: 15px; }
  .sum { display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 12px; text-align: center; padding: 24px; max-width: 640px; margin: 0 auto; }
  .sum h2 { margin: 0; font-size: 24px; }
  .sum p { margin: 0; }
  .sum div { display: flex; gap: 10px; }
  .muted { opacity: .75; }
  .err { color: #ff8a8a; }
  .btn { border: 1px solid rgba(255, 255, 255, .3); background: rgba(255, 255, 255, .08); color: #fff; border-radius: 6px; padding: 5px 12px; }
  .btn:hover { background: rgba(255, 255, 255, .18); }
  .btn:disabled { opacity: .4; }
  .btn.go:disabled { background: #2f6fde; }
  .btn.go { background: #2f6fde; border-color: #2f6fde; font-weight: 600; }
  .btn.go:hover { background: #3d7cf0; }
</style>
