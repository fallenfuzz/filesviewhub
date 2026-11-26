<script lang="ts">
  // A Work as a card (or a row, at the compact size), as the Hub shows it: picture, mark badge,
  // title with its year, how far along it is, its other formats, and where it lives.
  import { tick } from 'svelte'
  import type { Detail, Mark, Thumb, Work } from './api'
  import { doneWord, firstFile, glued, hasDialog, mains, metaOf, place, size, span, tickable, tickKeys } from './api'
  import { S } from './strings'
  import Tile from './Tile.svelte'
  import Icon from './Icon.svelte'
  import { byline, type Authors } from './byline'

  let { work, color, section, known = null, mark, ticks, thumb, details, tall, compact = false, path = false, from = 0, band = false, review = null, selected = false, onopen, ondialog, onmark, onfile, ongo, onreveal, onchoosecover, onremovecover }: {
    work: Work; color: string; section: string
    /** In a Books Section, the authors met across it: the title then shows its author apart. */
    known?: Authors | null
    mark?: Mark; ticks?: Record<string, number>; thumb?: Thumb
    details: Record<string, Detail>; tall: boolean; compact?: boolean
    /** Show where the Work lives: in search, Next up and the other lists. */
    path?: boolean
    /** How many steps of the path, the Section's name first, the address bar shows already: the
     * path line starts below them, and the whole path is its tip. */
    from?: number
    /** Draw the Section's colour on top: in lists where Sections mix. */
    band?: boolean
    /** A Photos event's Review: how many photos wait, or done (`decisions/2026/10/04/photos-take-no-marks-review-is-the-progress.md`). */
    review?: { n: number; done: boolean } | null
    /** One of several cards selected: a ring round the picture and a check in its corner. */
    selected?: boolean
    onopen: () => void
    /** Opens the Work's dialog: a single file's code and exercise files are listed there. */
    ondialog: () => void
    onmark: (m: 'next' | 'done' | null) => void; onfile: (path: string) => void
    ongo: (folder: string[]) => void; onreveal: () => void
    /** Choose a cover… for this Work (`decisions/2026/10/22/covers-are-chosen-never-fetched-by-themselves.md`). */
    onchoosecover: () => void
    /** Removes the cover chosen for these ids: its Work and copies. */
    onremovecover: (ids: string[]) => Promise<void>
  } = $props()

  const keys = $derived(tickable(work) ? tickKeys(work) : [])
  const ticked = $derived(ticks ? keys.filter(k => k in ticks).length : 0)
  const meta = $derived(metaOf(work, details))
  const name = $derived(known ? byline(work.title, known) : { title: work.title, by: '', year: '' })
  const year = $derived(work.year ?? name.year)
  // A Work of one file in several formats opens the first; the others, its captions and its code
  // files are on the Also line.
  const also = $derived(work.is_folder ? [] : mains(work).slice(1))
  const cc = $derived(work.is_folder ? [] : work.items.filter(i => i.role === 'cc'))
  const code = $derived(work.is_folder ? [] : work.items.filter(i => i.role === 'code'))
  const nextMark = $derived(!mark ? 'next' : mark.m === 'next' ? 'done' : null)
  const word = $derived(doneWord(work))
  const label = $derived(mark?.m === 'next' ? S.next : mark?.m === 'done' ? word : '')
  const since = $derived(mark?.at ? S.markSince(mark.m === 'next' ? S.next : word, new Date(mark.at * 1000)) : '')
  const hitLabel = $derived.by(() => { const l = meta ? `${work.title}, ${meta}` : work.title; return selected ? S.selectedCard(l) : l })
  const file = $derived(work.is_folder ? work.path.split(/[\\/]/).pop() : firstFile(work)?.name)
  /** A cover chosen from the web: the globe chip says where from, and its popup offers another or
      Not this cover. A picture of one's own has no chip. */
  const site = $derived(thumb?.from ? (S.sites[thumb.from] ?? '') : '')
  // The Work shown, the one whose thumbnail it shows and its copies: one book, one choice.
  const ids = $derived([...new Set([work.id, ...(thumb ? [thumb.id] : []), ...(work.copies ?? [])])])
  let pop = $state<HTMLElement>()
  let chip: HTMLElement | undefined
  let busy = $state(false)
  /** When a click outside closed the popup: a click on the chip that closed it does not reopen it. */
  let closedAt = 0
  function openCover(e: MouseEvent) {
    e.stopPropagation()
    if (!pop || performance.now() - closedAt < 300) return
    chip = e.currentTarget as HTMLElement
    const r = chip.getBoundingClientRect()
    pop.style.right = `${Math.max(8, innerWidth - r.right)}px`
    pop.showPopover()
    // Above the chip, or below it when the window has no room above.
    const h = pop.offsetHeight
    pop.style.top = `${r.top - h - 6 >= 8 ? r.top - h - 6 : Math.min(r.bottom + 6, innerHeight - h - 8)}px`
    pop.querySelector<HTMLElement>('button')?.focus()
  }
  /** The popup belongs to its card: it closes when the page scrolls under it. */
  const close = () => pop?.hidePopover()
  function toggled(e: ToggleEvent) {
    if (e.newState === 'open') addEventListener('scroll', close, { capture: true, once: true })
    else { removeEventListener('scroll', close, { capture: true }); closedAt = performance.now() }
  }
  async function notThis() {
    // The chip goes with the cover: focus returns to the card's picture.
    const card = chip?.closest('.card, .row')
    busy = true
    try { await onremovecover(ids) } finally { busy = false }
    pop?.hidePopover()
    await tick()
    card?.querySelector<HTMLElement>('.hit')?.focus()
  }
  function another() {
    pop?.hidePopover()
    onchoosecover()
  }
  // The path below the open page; a long one shows its first folder and its last two.
  const full = $derived([section, ...work.folder])
  const trail = $derived.by(() => {
    const n = full.length - from
    return full.map((name, k) => ({ name, k })).slice(from).filter(({ k }) => n <= 4 || k === from || k >= full.length - 2)
  })
</script>

{#snippet badge(flag: boolean)}
  <button class="{flag ? 'flag' : 'pill'} {mark?.m ?? 'none'}" onclick={e => { e.stopPropagation(); onmark(nextMark) }}
    aria-label={S.markLabel(work.title, mark ? label : null)} title={since + S.markCycle(word)}>
    {mark?.m === 'next' ? S.next : mark?.m === 'done' ? (flag ? `✓ ${word}` : word) : flag ? '' : S.mark}{#if flag && !mark?.m}+<span class="more">&nbsp;{S.mark}</span>{/if}
  </button>
{/snippet}
{#snippet where()}
  {#if path && trail.length}
    <div class="path" title={full.map(place).join(' › ')}>
      {#each trail as { name, k }, i (k)}
        {#if i}<span aria-hidden="true">{trail[i - 1].k === k - 1 ? '›' : '› … ›'}</span>{/if}
        <button dir="auto" onclick={e => { e.stopPropagation(); ongo(work.folder.slice(0, k)) }} aria-label={S.showFolder(place(name))}>{place(name)}</button>
      {/each}
    </div>
  {/if}
{/snippet}
{#snippet status()}
  {#if work.taken}<p class="drives">{work.day_only ? S.timeUnknown : span(work)}</p>{/if}
  {#if review}<p class="rv" class:ok={review.done} title={review.done ? S.reviewDone : S.reviewTip(review.n)}>{review.done ? S.reviewed : S.toReview(review.n)}</p>{/if}
  {#if work.drives}<p class="drives">{S.onDrives(work.drives)}</p>{/if}
  {#if ticked}<p class="status">{S.ofDone(ticked, keys.length, word)}</p>{/if}
{/snippet}
{#snippet alsoLine()}
  {#if also.length || cc.length || code.length}
    <p class="also">{S.also}{#each also as it, i (it.path)}{i ? ', ' : ' '}<button class="link" title={it.name} onclick={() => onfile(it.path)}>{it.ext.toUpperCase()}</button>{/each}{#each cc as it, i (it.path)}{also.length || i ? ', ' : ' '}<button class="link" title={it.name} onclick={() => onfile(it.path)}>{S.captionsIn(it.lang)}</button>{/each}{#if code.length}{also.length || cc.length ? ', ' : ' '}<button class="link" onclick={ondialog} aria-haspopup="dialog">{S.codeFiles(code.length)}</button>{/if}</p>
  {/if}
{/snippet}
{#snippet reveal()}
  <button class="dir" onclick={e => { e.stopPropagation(); onreveal() }} title={S.openFolderOf(work.title)} aria-label={S.openFolderOf(work.title)}><Icon name="open" /></button>
{/snippet}
{#snippet coverChip(text: string)}
  {@const what = S.coverFrom(site)}
  <button class="web" onclick={openCover} title={what} aria-label={text ? `${what}, ${text}` : what} aria-haspopup="dialog">
    <Icon name="globe" size={12} />{text}</button>
{/snippet}
{#snippet heading()}
  <span dir="auto">{glued(name.title)}</span>{#if year}<span class="year"> {year}</span>{/if}
{/snippet}

{#if compact}
  <div class="row" class:done={mark?.m === 'done'} class:sel={selected}>
    <button class="hit" onclick={onopen} title={file} aria-label={hitLabel} aria-haspopup={hasDialog(work) ? 'dialog' : undefined}
      style="--ratio:{thumb && thumb.n < 2 ? `${thumb.w}/${thumb.h}` : '16/9'}">
      <Tile {thumb} {color} title={work.title} progress={keys.length ? ticked / keys.length : 0} />
      {#if selected}<span class="chk" aria-hidden="true">✓</span>{/if}
    </button>
    <div class="body">
      <div class="t" role="presentation" onclick={onopen} title={name.by ? work.title : undefined}>{@render heading()}{#if name.by}<span class="by-row" dir="auto"> · {name.by}</span>{/if} {@render reveal()}</div>
      {@render status()}
      {@render alsoLine()}
      {@render where()}
    </div>
    <div class="m">{work.media.join(', ')}<br />{#if site}{@render coverChip('')} {/if}{meta}{work.is_folder ? '' : `, ${size(work.items[0]?.size ?? 0)}`}</div>
    {#if !work.event}{@render badge(false)}{/if}
  </div>
{:else}
  <article class="card" class:done={mark?.m === 'done'} class:sel={selected}>
    {#if !work.event}{@render badge(true)}{/if}
    <div class="pic">
      <button class="hit" onclick={onopen} title={file} aria-label={hitLabel} aria-haspopup={hasDialog(work) ? 'dialog' : undefined}>
        <Tile {thumb} {color} {tall} {band} title={work.title} parent={place(work.folder.at(-1) ?? '')} meta={site ? '' : meta} cloud={work.cloud > 0} />
      </button>
      <!-- The type chip of a web cover is a button of its own, beside the one that opens the book. -->
      {#if site}{@render coverChip(meta)}{/if}
      {#if selected}<span class="chk" aria-hidden="true">✓</span>{/if}
    </div>
    {@render reveal()}
    <!-- A row of cards shares its title height, the tallest title's: the lines below line up. -->
    <div class="head" class:split={!!name.by} title={work.title}>
      <h3 role="presentation" onclick={onopen}>{@render heading()}</h3>
      {#if name.by}<p class="by" dir="auto">{name.by}</p>{/if}
    </div>
    <div class="lines">
      {@render status()}
      {#if ticked}<span class="prog" aria-hidden="true"><i style="width:{ticked / keys.length * 100}%"></i></span>{/if}
      {@render alsoLine()}
      {@render where()}
    </div>
  </article>
{/if}
{#if site}
  <!-- In the top layer, so no card clips it; a click outside or Escape closes it. -->
  <div class="cover-pop" popover="auto" role="dialog" aria-label={S.coverFrom(site)} bind:this={pop} ontoggle={toggled}>
    <p><b>{S.coverFrom(site)}</b></p>
    <p class="what">{S.coverFromWhat}</p>
    <div class="acts">
      <button class="btn" disabled={busy} onclick={another}>{S.chooseAnother}</button>
      <button class="btn" disabled={busy} onclick={notThis}>{S.notThisCover}</button>
    </div>
  </div>
{/if}

<style>
  /* A card takes three rows of its grid, picture, title and the lines below, so each row of cards
     is as tall as its tallest title. Pictures off screen skip layout and paint, so a list of
     thousands stays smooth; the card itself cannot (containment turns its subgrid off). A skipped
     picture keeps its height, as its aspect-ratio sets it. */
  .card { display: grid; grid-template-rows: subgrid; grid-row: span 3; row-gap: 0; position: relative; min-width: 0; padding: 6px; margin: -6px; }
  .card :global(.tile) { content-visibility: auto; }
  .pic { position: relative; }
  .row { content-visibility: auto; contain-intrinsic-size: auto 72px; }
  .hit { display: block; width: 100%; border: 0; background: none; padding: 0; text-align: left; }
  .hit:hover :global(.tile), .hit:focus-visible :global(.tile) { outline: 2px solid var(--focus); outline-offset: 2px; }
  .hit:focus-visible { outline: none; }
  /* Selected: a ring round the picture and a round check in its lower left corner, on any cover. */
  .sel .hit :global(.tile), .sel .hit:hover :global(.tile) { outline: 3px solid var(--focus); outline-offset: 2px; }
  .chk { position: absolute; z-index: 2; left: 6px; bottom: 6px; display: grid; place-items: center; width: 22px; height: 22px; border-radius: 50%;
    background: var(--focus); color: var(--on-focus); font-size: 13px; font-weight: 700; box-shadow: 0 0 0 2px #fff, 0 1px 3px rgba(0, 0, 0, .4); pointer-events: none; }
  .row .hit { position: relative; }
  .row .chk { left: 50%; top: 50%; bottom: auto; translate: -50% -50%; width: 18px; height: 18px; font-size: 11px; }
  .row.sel, .row.sel:hover { background: var(--sel); }
  .done :global(.tile) > :global(:not(.meta, .scrub)) { opacity: .8; filter: grayscale(.35); }
  .head { margin: 9px 0 3px; }
  h3 { margin: 0; font-size: 14.5px; font-weight: 600; line-height: 1.3; overflow-wrap: anywhere; cursor: pointer;
    display: -webkit-box; -webkit-box-orient: vertical; -webkit-line-clamp: 3; line-clamp: 3; overflow: hidden; }
  .split h3 { -webkit-line-clamp: 2; line-clamp: 2; }
  .by { margin: 1px 0 0; font-size: 12.5px; line-height: 1.35; color: var(--muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .by-row { font-weight: 400; color: var(--muted); }
  .year { font-weight: 400; color: var(--muted); margin-left: .3em; }
    /* Show in File Explorer: over the picture's corner on a card, after the title in a row; shown on hover. */
  .dir { border: 0; background: none; padding: 0; color: var(--muted); display: inline-grid; place-items: center; width: 24px; height: 22px; border-radius: 5px; opacity: 0; }
  .card .dir { position: absolute; z-index: 2; top: 12px; right: 12px; background: rgba(22, 32, 42, .75); color: #fff; }
  .card:hover .dir, .card:focus-within .dir, .row:hover .dir, .row:focus-within .dir { opacity: 1; }
  .row .dir:hover { background: var(--hover); color: var(--ink); }
  .flag { z-index: 2; position: absolute; top: 14px; left: 12px; font-size: 12px; font-weight: 700; padding: 2px 10px; border-radius: 999px;
    border: 1px solid rgba(255, 255, 255, .55); color: #fff; box-shadow: 0 1px 3px rgba(0, 0, 0, .3); }
  /* No mark yet: a quiet tinted "+" circle at rest, so a person sees where to mark without the
     pointer and the title printed on a cover stays readable; "+ Mark" at full
     strength under the pointer or keyboard. At rest the white text has over 5:1 contrast even on
     a white cover. No blur: thousands of cards would each get a layer. */
  .flag.none { background: rgba(22, 32, 42, .65); border-color: rgba(255, 255, 255, .7); padding: 2px 7px; }
  .flag.none .more { display: none; }
  .card:hover .flag.none, .card:focus-within .flag.none { background: rgba(22, 32, 42, .85); padding: 2px 10px; }
  .card:hover .flag.none .more, .card:focus-within .flag.none .more { display: inline; }
  .flag.next, .pill.next { background: var(--next); color: var(--next-ink); }
  .flag.done, .pill.done { background: var(--done); color: var(--done-ink); }
  .pill { border: 1px solid var(--line); background: var(--field); border-radius: 999px; padding: 1px 10px; font-size: 13px; font-weight: 600; white-space: nowrap; }
  .pill.none { color: var(--muted); }
  .pill.next, .pill.done { border-color: transparent; }
  .status { margin: 2px 0 4px; font-size: 13px; font-weight: 500; }
  /* A ticked course's count, with a thin bar beside it. */
  .lines { display: grid; grid-template-columns: auto minmax(0, 1fr); column-gap: 10px; align-items: center; align-content: start; min-width: 0; }
  .lines > :global(*) { grid-column: 1 / -1; }
  .lines > .status { grid-column: 1; }
  .prog { grid-column: 2; height: 4px; border-radius: 2px; background: var(--line); overflow: hidden; }
  .prog i { display: block; height: 100%; background: var(--done); }
  .drives { margin: 2px 0; font-size: 12.5px; color: var(--muted); }
  .rv { margin: 3px 0; font-size: 12px; font-weight: 600; width: fit-content; padding: 0 8px; border-radius: 999px; background: var(--next); color: var(--next-ink); }
  .rv.ok { background: none; padding: 0; color: var(--done); }
  .also { margin: 0 0 4px; color: var(--muted); font-size: 13px; }
  .link { border: 0; background: none; padding: 0; color: var(--link); font-size: 13px; }
  .link:hover { text-decoration: underline; }
  .path { font-size: 12.5px; color: var(--muted); display: flex; flex-wrap: wrap; gap: 0 2px; align-items: center; min-width: 0; }
  .path button { background: none; border: 0; padding: 0 2px; color: inherit; text-align: left; font-size: inherit; min-width: 0; overflow-wrap: anywhere; }
  .path button:hover { color: var(--ink); text-decoration: underline; }
  .row { display: grid; grid-template-columns: 96px minmax(0, 1fr) auto auto; gap: 14px; align-items: center; padding: 7px 6px; border-bottom: 1px solid var(--line); }
  .row:hover { background: var(--hover); }
  /* A row's picture takes the cover's own shape at the row's height, centred in a slot as wide
     as a 16:9 frame, so titles line up whatever the shape. */
  .row .hit { width: 96px; height: 54px; display: grid; place-items: center; }
  .row :global(.tile) { height: 100%; max-width: 100%; }
  .row :global(.gen) { padding: 6px; justify-content: center; }
  .row :global(.gen b) { font-size: 10px; }
  .t { font-weight: 600; overflow-wrap: anywhere; cursor: pointer; }
  .m { color: var(--muted); font-size: 12.5px; text-align: right; white-space: nowrap; }
  .body { min-width: 0; }
  /* Drawn as the tile's type chip (Tile.svelte .meta), with a globe. */
  .web { display: inline-flex; align-items: center; gap: 4px; border: 0; font-size: 12px; background: rgba(22, 32, 42, .85); color: #fff; padding: 2px 6px; border-radius: 4px; line-height: inherit; }
  .web:hover, .web:focus-visible { background: rgb(22, 32, 42); outline: 2px solid var(--focus); outline-offset: 1px; }
  .pic .web { position: absolute; z-index: 2; right: 5px; bottom: 5px; }
  /* In a row, a plain globe in the muted text. */
  .m .web { background: none; color: inherit; padding: 1px 3px; margin-right: 3px; vertical-align: -1px; }
  .m .web:hover, .m .web:focus-visible { background: var(--hover); color: var(--ink); outline: none; }
  /* As Settings, Programs' Change menu. */
  .cover-pop { position: fixed; inset: auto; margin: 0; width: 260px; padding: 12px 14px; background: var(--field); color: var(--ink); border: 1px solid var(--line); border-radius: 8px; box-shadow: 0 8px 24px rgba(0,0,0,.18); font-size: 13.5px; }
  .cover-pop p { margin: 0 0 8px; }
  .cover-pop .what { color: var(--muted); font-size: 12.5px; }
  .cover-pop .acts { display: flex; flex-wrap: wrap; gap: 6px; }
</style>
