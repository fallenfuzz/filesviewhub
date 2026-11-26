<script lang="ts">
  // The viewer for an event's photos (`decisions/2026/10/08/photos-is-a-section-kind-in-the-app.md`): one at full size over the dialog, with the
  // arrow keys or the side buttons moving through the event. A clip shows its frame and Enter
  // plays it in the system's player. A type the window cannot draw (HEIC, TIFF) or a file only in
  // the cloud shows its thumbnail and opens in the system's program instead; a file only in the
  // cloud opens after the person agrees to download it (App.svelte openFile).
  import { convertFileSrc } from '@tauri-apps/api/core'
  import { glued, thumbSrc, type Item, type Thumb } from './api'
  import { isClip } from './photos'
  import { S } from './strings'

  let { items, at = $bindable(), thumbOf, onfile, onreveal, note, again }: {
    items: Item[]; at: number; thumbOf: (i: Item) => Thumb | undefined
    onfile: (path: string) => void; onreveal: (path: string) => void
    /** A line about the Photo: when it was taken, and what Review says of it. */
    /** When it was taken, and what Review says of it. */
    note?: (i: Item) => { when: string; what: string }
    /** Review again for a Photo with a Review choice, else none. */
    again?: (i: Item) => (() => void) | undefined
  } = $props()

  const DRAWN = new Set(['jpg', 'jpeg', 'png', 'gif', 'webp', 'bmp', 'insp'])
  const it = $derived(items[at])
  const drawn = $derived(DRAWN.has(it.ext) && !it.cloud)
  const clip = $derived(isClip(it))
  const thumb = $derived(thumbOf(it))
  const reviewAgain = $derived(again?.(it))
  let loaded = $state('')
  let close: HTMLButtonElement
  // Focus goes back to what opened the viewer when it closes, if that is still there.
  const opener = document.activeElement as HTMLElement | null
  $effect(() => { close?.focus() })
  $effect(() => () => { if (opener?.isConnected) opener.focus() })

  const go = (d: number) => { at = (at + d + items.length) % items.length }
  function keys(e: KeyboardEvent) {
    // A held Enter acts once, so no program opens again on each repeat.
    if (e.key === 'Enter' && e.repeat) { e.preventDefault(); e.stopPropagation(); return }
    // Enter plays a clip unless another button has focus.
    const free = !document.activeElement?.matches('button') || document.activeElement === close
    if (e.key === 'Enter' && clip && free) { e.preventDefault(); e.stopPropagation(); onfile(it.path); return }
    const k = { ArrowLeft: -1, ArrowRight: 1, Escape: 0 }[e.key]
    if (k === undefined) return
    // Escape closes the viewer only, not the dialog under it.
    e.preventDefault(); e.stopPropagation()
    if (k) go(k); else at = -1
  }
  // The next one is read ahead, so moving on shows it at once.
  const ahead = $derived(items[(at + 1) % items.length])
</script>

<svelte:window onkeydowncapture={keys} />

<div class="viewer" role="dialog" aria-modal="true" aria-label={S.viewerLabel(it.title, at + 1, items.length)}>
  <div class="stage">
    {#if drawn}
      {#if thumb && loaded !== it.path}<img class="ph" src={thumbSrc(thumb.key)} alt="" />{/if}
      <img src={convertFileSrc(it.path, 'photo')} alt={it.title} onload={() => (loaded = it.path)} class:hide={loaded !== it.path} />
      {#if DRAWN.has(ahead.ext) && !ahead.cloud}<link rel="prefetch" href={convertFileSrc(ahead.path, 'photo')} />{/if}
    {:else}
      {#if thumb}<img class="ph shown" src={thumbSrc(thumb.key)} alt={it.title} />{/if}
      {#if clip}
        {#if it.cloud}<p class="why">{S.onlyInCloud}</p>{/if}
        <button class="btn play" onclick={() => onfile(it.path)}>{S.playClip}</button>
      {:else}
        <p class="why">{it.cloud ? S.onlyInCloud : S.notDrawn}</p>
        <button class="btn" onclick={() => onfile(it.path)}>{S.openInProgram}</button>
      {/if}
    {/if}
  </div>
  {#if items.length > 1}
    <button class="side prev" onclick={() => go(-1)} aria-label={S.previous}>‹</button>
    <button class="side next" onclick={() => go(1)} aria-label={S.nextOne}>›</button>
  {/if}
  <div class="bar">
    <span class="t" dir="auto" title={it.name}>{glued(it.title)}</span>
    <span class="n">{S.nOfM(at + 1, items.length)}</span>
    {#if note}
      {@const nt = note(it)}
      <span class="note" title={[nt.when, nt.what].filter(Boolean).join(' · ')}>{#if nt.what}<span class="when">{`${nt.when} · `}</span>{nt.what}{:else}{nt.when}{/if}</span>
    {/if}
    <span class="grow"></span>
    {#if reviewAgain}<button class="btn" title={S.reviewAgainTip} onclick={() => { reviewAgain?.(); close?.focus() }}>{S.reviewAgain}</button>{/if}
    {#if drawn}<button class="btn" onclick={() => onfile(it.path)}>{S.openInProgram}</button>{/if}
    <button class="btn" onclick={() => onreveal(it.path)}>{S.showInExplorer}</button>
    <button class="btn" bind:this={close} onclick={() => (at = -1)} aria-label={S.close}>×</button>
  </div>
</div>

<style>
  .viewer { position: fixed; inset: 0; z-index: 50; background: rgba(8, 10, 14, .96); color: #fff; display: grid; grid-template-rows: 1fr auto; grid-template-columns: minmax(0, 1fr); }
  .stage { position: relative; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 12px; min-height: 0; padding: 16px 64px 0; }
  .stage img { max-width: 100%; max-height: 100%; object-fit: contain; }
  .stage img.hide { position: absolute; opacity: 0; }
  .stage img.ph { width: min(100%, 1200px); height: 100%; filter: blur(2px); }
  .stage img.ph.shown { filter: none; width: auto; height: auto; max-height: 70%; }
  .why { margin: 0; opacity: .85; }
  .side { position: absolute; top: 50%; transform: translateY(-50%); width: 48px; height: 72px; border: 0; border-radius: 8px; background: rgba(255, 255, 255, .08); color: #fff; font-size: 32px; }
  .side:hover, .side:focus-visible { background: rgba(255, 255, 255, .2); }
  .prev { left: 8px; } .next { right: 8px; }
  .bar { display: flex; align-items: center; gap: 12px; padding: 10px 16px; font-size: 13px; }
  .bar .t { font-weight: 600; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .bar .n { opacity: .75; white-space: nowrap; }
  /* The buttons keep their words on one line; the title and the note give way first. */
  .bar .btn { flex: none; white-space: nowrap; }
  /* The note gives way before the title. */
  .bar .note { flex: 0 100 auto; opacity: .85; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .grow { flex: 1; }
  /* A narrow window puts the buttons on a row of their own, drops the date, and the title gives
     way before what Review says. */
  @media (max-width: 820px) {
    .bar { flex-wrap: wrap; row-gap: 8px; }
    .bar .grow { flex-basis: 100%; }
    .bar .note .when { display: none; }
    .bar .note { flex: 0 1 auto; }
    .bar .t { flex-shrink: 4; min-width: 64px; }
  }
  .btn { border: 1px solid rgba(255, 255, 255, .3); background: rgba(255, 255, 255, .08); color: #fff; border-radius: 6px; padding: 4px 10px; }
  .btn:hover { background: rgba(255, 255, 255, .18); }
  .btn.play { font-size: 15px; padding: 6px 16px; }
</style>
