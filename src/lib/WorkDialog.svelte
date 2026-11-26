<script lang="ts">
  // A Work's dialog, as the Hub's: its lessons, tracks or titles by folder, numbered, each with a
  // Tick, its length, captions and its own files; then the Work's other files, code files and,
  // with the Settings switch, what is unlisted inside it. The Mark is set here by hand; ticking
  // everything offers it but never sets it (`decisions/2026/09/29/ticks-offer-the-mark-never-set-it.md`).
  import { onMount } from 'svelte'
  import { api, clock, doneWord, glued, grid, keyOf, mains, metaOf, place, shownOf, size, tickable, tickKeys, unit, type Detail, type Item, type Mark, type Marks, type Section, type Thumb, type Work } from './api'
  import { asideIn, choiceOf, isSetAside, reviewState, type Notes } from './review'
  import { ask } from './ask.svelte'
  import { S } from './strings'
  import { ui } from './ui.svelte'
  import Tile from './Tile.svelte'
  import Viewer from './Viewer.svelte'
  import Icon from './Icon.svelte'
  import { isClip } from './photos'

  let { work, section, color, mark, ticks, thumbs, details, notes, marks, onmark, ontick, onclose, onfile, onreveal, onreview }: {
    work: Work; section: Section; color: string; mark?: Mark; ticks: Record<string, number>
    thumbs: Record<string, Thumb>; details: Record<string, Detail>
    onmark: (m: 'next' | 'done' | null) => void; ontick: (keys: string[], on: boolean) => void; onclose: () => void
    onfile: (path: string) => void; onreveal: (path: string) => void
    /** Review in a Photos event (`decisions/2026/10/04/photos-take-no-marks-review-is-the-progress.md`). */
    notes: Notes; marks: Marks; onreview: () => void
  } = $props()

  let dialog: HTMLDialogElement
  /** A press and its release both on the backdrop close the dialog; a drag that selects text
   *  and ends outside, or one that starts outside, keeps it open. */
  let downOut = false
  let upOut = false
  let body: HTMLDivElement
  // Only a folder Work's lessons, tracks or titles take Ticks: a file's formats do not (`decisions/2026/09/29/ticks-offer-the-mark-never-set-it.md`).
  const all = $derived(tickable(work) ? tickKeys(work) : [])
  const n = $derived(all.filter(k => k in ticks).length)
  const word = $derived(doneWord(work))
  const isGrid = $derived(grid(work))
  const dirOf = (p: string) => p.replace(/[\\/][^\\/]*$/, '')
  const thumbId = (i: Item) => `${work.id}#${keyOf(i)}`

  // One group per folder, the Work's own first: its lessons, each with its captions and own files,
  // then the other files kept there.
  type Lesson = { i: Item; no: number; cc: Item[]; files: Item[] }
  const groups = $derived.by(() => {
    const order: string[] = []
    const by = new Map<string, { lessons: Lesson[]; files: Item[]; dir: string }>()
    const at = (g: string, i: Item) => {
      if (!by.has(g)) { order.push(g); by.set(g, { lessons: [], files: [], dir: dirOf(i.path) }) }
      return by.get(g)!
    }
    const items = work.items.map(i => ({ ...i, role: i.role ?? 'main', group: i.group ?? '' }))
    let no = 0
    const lessonOf = new Map<string, Lesson>()
    for (const i of items.filter(i => i.role === 'main')) {
      const l = { i, no: ++no, cc: [], files: [] }
      at(i.group, i).lessons.push(l)
      lessonOf.set(keyOf(i), l)
    }
    for (const i of items.filter(i => i.role === 'cc')) lessonOf.get(keyOf(i))?.cc.push(i)
    for (const i of items.filter(i => i.role === 'doc')) {
      const l = lessonOf.get(keyOf(i))
      if (l && l.i.group === i.group) l.files.push(i)
      else at(i.group, i).files.push(i)
    }
    const sorted = order.sort((a, b) => (a === '' ? -1 : b === '' ? 1 : 0))
    return sorted.map(g => ({ name: g, ...by.get(g)! }))
  })
  // A course's main button: its first lesson with no Tick, by number.
  const lessons = $derived(isGrid || all.length < 2 ? [] : groups.flatMap(g => g.lessons).sort((a, b) => a.no - b.no))
  const resume = $derived(n < all.length ? lessons.find(l => !(keyOf(l.i) in ticks)) : undefined)
  /** A part's heading line: its time, when every lesson's length is known, and how many are
   *  ticked: "57 min · 1 of 4". */
  function sumOf(ls: Lesson[]): string {
    const secs = ls.every(l => details[l.i.path]?.s) ? ls.reduce((a, l) => a + details[l.i.path].s!, 0) : 0
    return [secs && S.minutes(secs), S.partOf(ls.filter(l => keyOf(l.i) in ticks).length, ls.length)].filter(Boolean).join(' · ')
  }
  const goLabel = $derived(resume ? (n ? S.continueAt(resume.no) + glued(resume.i.title) : S.startAt(unit(work), resume.no)) : '')
  const code = $derived(work.items.filter(i => i.role === 'code'))
  const unlisted = $derived(ui.unlisted ? (section.unlisted ?? []).filter(u => u.work === work.id) : [])

  onMount(() => {
    dialog.showModal()
    if (isGrid) api.wantThumbs(mains(work).map(thumbId))
    // Opens at the first item not yet ticked (`decisions/2026/09/29/ticks-offer-the-mark-never-set-it.md`).
    const first = body.querySelector<HTMLElement>('.lesson:not(.done) .tick, .mini:not(.done) .tick')
    first?.scrollIntoView({ block: 'center' })
    first?.focus()
  })
  const markBtn = (m: 'next' | 'done') => () => onmark(mark?.m === m ? null : m)
  /** The event photo open in the viewer, or -1. */
  let viewing = $state(-1)
  const review = $derived(work.event ? reviewState([work], notes, marks) : null)
  const aside = $derived(work.event ? asideIn([work], marks) : 0)
  /** A Photo's tag in the grid: the Best one of its set, an Extra, a Copy, or Junk. */
  function tagOf(i: Item): string {
    if (isSetAside(i, work, marks)) return S.setAside
    const n = notes[i.path]
    if (!n) return ''
    // Once the person chose in a close call, the photo they kept says so.
    if (n.set) return n.call ? (choiceOf(i, work, marks)?.keep ? S.kept : S.yourCall) : n.best ? S.bestOne : S.extra
    return n.copy ? S.copy : n.junk ? S.junkWhy[n.junk] : ''
  }
  const since = $derived(mark?.at ? S.markSince(mark.m === 'next' ? S.next : word, new Date(mark.at * 1000)) : '')
</script>

{#snippet tick(i: Item)}
  {@const on = keyOf(i) in ticks}
  <button class="tick" aria-pressed={on} aria-label={S.tickLabel(i.title, on, word)} onclick={() => ontick([keyOf(i)], !on)}></button>
{/snippet}

<dialog bind:this={dialog} aria-labelledby="work-title" onclose={onclose} onpointerdown={e => (downOut = e.target === dialog)}
  onpointerup={e => (upOut = e.target === dialog)} onclick={e => { if (e.target === dialog && downOut && upOut) dialog.close() }} style="--c:{color}">
  <div class="head">
    <p class="crumbs">{#each [section.name, ...work.folder] as f, i (i)}{i ? ' › ' : ''}<bdi>{place(f)}</bdi>{/each}
      <button class="dir" onclick={() => onreveal(shownOf(work))} title={S.showInExplorer} aria-label={S.showInExplorer}><Icon name="open" /></button></p>
    <h2 id="work-title" dir="auto">{glued(work.title)}{#if work.year}<span class="year"> {work.year}</span>{/if}</h2>
    <p class="muted">{metaOf(work, details)}, {size(work.size)}</p>
    <!-- An event takes no Mark and no Tick; Review is its one action (`decisions/2026/10/04/photos-take-no-marks-review-is-the-progress.md`). -->
    {#if work.event && (review || aside)}
      <div class="tools">
        {#if review && !review.done}<button class="btn primary" onclick={onreview} title={S.reviewTip(review.n)}>{S.reviewN(review.n)}</button>
        {:else if review}<span class="ok">{S.reviewed}</span> <button class="btn" onclick={onreview}>{S.undoReview}</button>{/if}
        {#if aside}<span class="prog" title={S.setAsideTip}>{S.setAsideN(aside)}</span>{/if}
      </div>
    {/if}
    {#if !work.event}
    <div class="tools">
      {#if resume}<button class="btn primary go" onclick={() => onfile(resume.i.path)} title={goLabel}>{#if n}{S.continueAt(resume.no)}<bdi dir="auto">{glued(resume.i.title)}</bdi>{:else}{S.startAt(unit(work), resume.no)}{/if}</button>{/if}
      <button class="btn next" class:on={mark?.m === 'next'} aria-pressed={mark?.m === 'next'} title={since} onclick={markBtn('next')}>{S.next}</button>
      <button class="btn done" class:on={mark?.m === 'done'} aria-pressed={mark?.m === 'done'} title={since} onclick={markBtn('done')}>{word}</button>
      {#if all.length}
        <span class="prog">{S.ofDone(n, all.length, word)}.</span>
        {#if n < all.length}<button class="btn" onclick={() => ontick(all, true)}>{S.tickAll(word)}</button>{/if}
        {#if n}<button class="btn" onclick={async () => await ask(S.clearTicksSure(word, work.title), S.clearTicks, true) && ontick(all, false)}>{S.clearTicks}</button>{/if}
        {#if n === all.length && mark?.m !== 'done'}<button class="btn primary" onclick={() => onmark('done')}>{S.markDone(work.title, word)}</button>{/if}
      {/if}
    </div>
    {/if}
    <button class="x" aria-label={S.close} onclick={() => dialog.close()}><svg viewBox="0 0 10 10" aria-hidden="true"><path d="M1.5 1.5l7 7M8.5 1.5l-7 7" /></svg></button>
  </div>
  <div class="body" bind:this={body}>
    {#each groups as g (g.name)}
      {#if g.name}
        {@const sum = lessons.length && g.lessons.length ? sumOf(g.lessons) : ''}
        <h3><span dir="auto">{glued(g.name)}</span>{#if sum}{' '}<span class="sum">· {sum}</span>{/if} <button class="dir" onclick={() => onreveal(g.dir)} title={S.showInExplorer} aria-label={S.openFolderOf(g.name)}><Icon name="open" /></button></h3>
      {/if}
      {#if isGrid}
        <div class="minigrid">
          {#each g.lessons as { i } (i.path)}
            {@const tag = work.event ? tagOf(i) : ''}
            <div class="mini" class:done={keyOf(i) in ticks} class:aside={work.event && isSetAside(i, work, marks)}>
              {#if tag}<span class="ptag" class:best={tag === S.bestOne}>{tag}</span>{/if}
              {#if !work.event}{@render tick(i)}{/if}
              <button class="hit" onclick={() => work.event && !isClip(i) ? (viewing = work.items.findIndex(x => x.path === i.path)) : onfile(i.path)} title={i.name}>
                <Tile thumb={thumbs[thumbId(i)]} {color} title={i.title} tall={true} meta={i.ext.toUpperCase()} cloud={i.cloud} />
                <span class="t" dir="auto">{glued(i.title)}</span>
              </button>
            </div>
          {/each}
        </div>
      {:else if g.lessons.length}
        <ol class="items">
          {#each g.lessons as l (l.i.path)}
            <li class="lesson" class:done={keyOf(l.i) in ticks}>
              {#if all.length > 1}{@render tick(l.i)}{/if}
              <button class="open" onclick={() => onfile(l.i.path)} title={l.i.name}>
                <span class="no">{l.no}</span><span dir="auto">{glued(l.i.title)}</span>
                <span class="dur">{l.i.cloud ? '☁' : details[l.i.path]?.s ? clock(details[l.i.path].s!) : l.i.ext.toUpperCase()}</span>
              </button>
              {#each l.cc as c (c.path)}<button class="cc" onclick={() => onfile(c.path)} title={S.captions}>{c.lang || 'CC'}</button>{/each}
              {#if l.files.length}
                <ul class="files">{#each l.files as f (f.path)}<li><button class="link" dir="auto" onclick={() => onfile(f.path)}>{f.name}</button></li>{/each}</ul>
              {/if}
            </li>
          {/each}
        </ol>
      {/if}
      {#if g.files.length}
        <ul class="files own">
          {#each g.files as f (f.path)}
            <li><button class="link" dir="auto" onclick={() => onfile(f.path)}>{f.name}</button> <span class="muted">({f.ext.toUpperCase()}{details[f.path]?.p ? `, ${details[f.path].p} pages` : ''}, {size(f.size)})</span></li>
          {/each}
        </ul>
      {/if}
    {/each}
    {#if code.length}
      <details class="code">
        <summary>{S.codeFiles(code.length)}</summary>
        <ul class="files">{#each code as f (f.path)}<li><button class="link" dir="auto" onclick={() => onfile(f.path)}>{f.rel}</button></li>{/each}</ul>
      </details>
    {/if}
    {#if unlisted.length}
      <h3>{S.unlistedHere(unlisted.length)}</h3>
      <ul class="files">
        {#each unlisted as u (u.path)}<li><button class="link" dir="auto" onclick={() => onfile(u.path)}>{u.name}</button> <span class="tag">{S.reason[u.reason]}</span> <span class="muted">{size(u.size)}</span></li>{/each}
      </ul>
    {/if}
    <p class="muted foot">{S.opensWith}</p>
  </div>
  {#if viewing >= 0}<Viewer items={work.items} bind:at={viewing} thumbOf={i => thumbs[thumbId(i)]} {onfile} {onreveal} />{/if}
</dialog>

<style>
  dialog { border: 0; border-top: 4px solid var(--c); border-radius: 12px; padding: 0; width: min(920px, calc(100vw - 24px)); max-height: calc(100vh - 48px);
    background: var(--bg); color: var(--ink); box-shadow: 0 20px 60px rgba(0,0,0,.35); overflow: auto; }
  dialog::backdrop { background: rgba(10, 14, 20, .5); }
  .head { position: sticky; top: 0; z-index: 2; background: var(--bg); padding: 20px 28px 14px; border-bottom: 1px solid var(--line); }
  .crumbs { margin: 0; color: var(--muted); font-size: 12.5px; overflow-wrap: anywhere; padding-right: 40px; }
  h2 { margin: 4px 0 2px; font-size: 23px; padding-right: 40px; overflow-wrap: anywhere; }
  .year { font-weight: 400; color: var(--muted); }
  .head p.muted { margin: 0; font-size: 13px; }
  .tools { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; margin-top: 10px; }
  .tools .next.on { background: var(--next); color: var(--next-ink); border-color: transparent; font-weight: 600; }
  .tools .done.on { background: var(--done); color: var(--done-ink); border-color: transparent; font-weight: 600; }
  .prog { color: var(--soft); font-size: 13.5px; margin: 0 4px 0 6px; }
  .body { padding: 6px 28px 24px; }
  h3 { font-size: 15px; margin: 22px 0 8px; }
  h3 .sum { white-space: nowrap; font-weight: 400; font-size: 13.5px; color: var(--muted); font-variant-numeric: tabular-nums; }
  .go { max-width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .dir { border: 0; background: none; padding: 0; color: var(--muted); display: inline-grid; place-items: center; width: 24px; height: 22px; border-radius: 5px; vertical-align: middle; }
  .dir:hover { background: var(--hover); color: var(--ink); }
  ol.items { list-style: none; margin: 0; padding: 0; }
  .lesson { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; }
  .open { flex: 1; min-width: 0; display: grid; grid-template-columns: 2.4em 1fr auto; gap: 8px; padding: 7px 10px; border-radius: 6px; border: 0; background: none; text-align: left; align-items: baseline; }
  .open:hover { background: var(--hover); }
  .no { color: var(--muted); font-size: 13px; }
  .lesson.done .open { color: var(--muted); }
  .dur { color: var(--muted); font-size: 13px; font-variant-numeric: tabular-nums; }
  .cc { border: 1px solid var(--line); border-radius: 4px; background: var(--field); font-size: 11px; padding: 0 5px; }
  .tick { flex: none; width: 24px; height: 24px; border-radius: 50%; border: 1.5px solid var(--muted); background: var(--field); display: grid; place-items: center; color: transparent; padding: 0; }
  .tick::before { content: ""; width: 10px; height: 5px; border: solid currentColor; border-width: 0 0 2.5px 2.5px; transform: translateY(-2px) rotate(-45deg); }
  .tick:hover { color: var(--muted); border-color: var(--ink); }
  .tick[aria-pressed='true'] { background: var(--done); border-color: var(--done); color: #fff; }
  ul.files { margin: 0; padding-left: 20px; font-size: 13.5px; }
  .lesson ul.files { flex-basis: 100%; margin: -4px 0 6px; padding-left: calc(64px + 2.4em); }
  ul.files li { margin: 4px 0; }
  ul.own { margin-top: 10px; }
  .link { border: 0; background: none; padding: 0; color: var(--link); text-align: left; font-size: inherit; overflow-wrap: anywhere; }
  .link:hover { text-decoration: underline; }
  .minigrid { display: grid; gap: 20px 14px; grid-template-columns: repeat(auto-fill, minmax(140px, 1fr)); }
  .mini { position: relative; }
  .mini .tick { position: absolute; top: 10px; left: 6px; z-index: 2; }
  .mini .hit { display: block; width: 100%; border: 0; background: none; padding: 0; text-align: left; }
  .mini .t { display: block; margin-top: 6px; font-size: 13.5px; font-weight: 600; overflow-wrap: anywhere; }
  .mini.done :global(.tile) { opacity: .7; }
  .mini.aside :global(.tile) { opacity: .45; }
  .ptag { position: absolute; z-index: 2; top: 8px; left: 8px; font-size: 11.5px; font-weight: 700; padding: 1px 8px; border-radius: 999px; background: rgba(22, 32, 42, .85); color: #fff; pointer-events: none; }
  .ptag.best { background: #33d17a; color: #062; }
  .ok { color: var(--done); font-weight: 600; font-size: 13.5px; }
  .code { margin-top: 18px; }
  .code summary { cursor: pointer; font-weight: 600; font-size: 14px; }
  .tag { font-size: 11.5px; border-radius: 4px; padding: 0 6px; background: var(--side); color: var(--soft); }
  .foot { font-size: 12.5px; margin: 18px 0 0; }
  .x { position: absolute; top: 12px; right: 12px; width: 32px; height: 32px; border: 0; background: none; border-radius: 6px; display: grid; place-items: center; }
  .x:hover { background: var(--hover); }
  .x svg { width: 11px; height: 11px; fill: none; stroke: currentColor; stroke-width: 1.2; }
</style>
