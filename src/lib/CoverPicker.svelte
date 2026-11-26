<script lang="ts">
  // Choose a cover… (`decisions/2026/10/22/covers-are-chosen-never-fetched-by-themselves.md`): a picture of one's own, picked or pasted, or one found on the web
  // for a Work or a folder tile. Nothing is sent until Search is pressed; the covers found show
  // by their small pictures, and only the one used is fetched in full.
  import { onMount, tick } from 'svelte'
  import { api, type CoverResults, type CoverTarget, type FoundCover } from './api'
  import { S } from './strings'

  let { target, onchosen, onremove, onclose }: {
    target: CoverTarget
    onchosen: () => void
    /** Remove this cover: back to what the files give. */
    onremove: (ids: string[]) => Promise<void>
    onclose: () => void
  } = $props()

  let dialog: HTMLDialogElement
  let field = $state<HTMLInputElement>()
  let words = $state('')
  let results = $state<CoverResults[] | null>(null)
  let searching = $state(false)
  /** Small pictures by address: a `data:` address, or '' when it could not be fetched. */
  let previews = $state<Record<string, string>>({})
  /** A picture of one's own, as the core will keep it: base64 JPEG. */
  let own = $state('')
  let picked = $state<{ own: true } | { own: false; f: FoundCover } | null>(null)
  let busy = $state(false)
  let error = $state('')
  /** A press and its release both on the backdrop close the picker; a drag out of the search
   *  words, or into the picker, keeps it open. */
  let downOut = false
  let upOut = false
  /** Each search's previews land only while it is the last one. */
  let round = 0
  /** Leaves the current search: its previews are dropped here, and the core does not fetch
   *  those still waiting their turn. */
  function leave() {
    round++
    api.coverRound()
  }
  /** A cover was chosen for it: asked of the list itself, as its thumbnail may not be made yet. */
  let chosen = $state(false)
  const ownSrc = $derived(`data:image/jpeg;base64,${own}`)

  onMount(() => {
    dialog.showModal()
    api.hasCover(target.ids).then(v => (chosen = v))
    api.coverWords(target.name).then(w => { words = w; tick().then(() => field?.select()) })
    return leave
  })

  async function search() {
    const w = words.trim()
    if (!w || searching) return
    searching = true; error = ''; results = null; previews = {}
    if (picked && !picked.own) picked = null
    const r = ++round
    try {
      const core = await api.coverRound()
      const got = await api.findCovers(w)
      if (r !== round) return
      results = got
      for (const f of got.flatMap(x => x.found)) {
        api.coverPreview(f.preview, core).then(d => { if (r === round) previews[f.preview] = d }, () => { if (r === round) previews[f.preview] = '' })
      }
    } catch (e) { error = String(e) } finally { if (r === round) searching = false }
  }

  async function pickFile() {
    error = ''
    try {
      const d = await api.pickCoverFile()
      if (d) { own = d; picked = { own: true } }
    } catch (e) { error = String(e) }
  }

  /** A paste that is no picture says so, and takes back a picture pasted or picked before, so
   *  Use cannot set one the person meant to replace. */
  function dropOwn(why: string) {
    error = why
    own = ''
    if (picked?.own) picked = null
  }

  /** A picture pasted anywhere in the dialog, checked by the core as it is pasted; text pasted
   *  into the search words stays text. The clipboard's items, not only its files: WebKit on
   *  macOS and Linux fills only those for a copied picture. */
  function paste(e: ClipboardEvent) {
    const files = [...(e.clipboardData?.items ?? [])].filter(i => i.kind === 'file').map(i => i.getAsFile()).filter(f => !!f)
    if (!files.length) return
    e.preventDefault()
    const file = files.find(f => f.type.startsWith('image/'))
    if (!file) { dropOwn(S.notAPicture); return }
    const r = new FileReader()
    r.onload = async () => {
      try {
        own = await api.readCover(String(r.result).replace(/^data:[^,]*,/, ''))
        picked = { own: true }; error = ''
      } catch (e) { dropOwn(String(e)) }
    }
    r.readAsDataURL(file)
  }

  async function use() {
    if (!picked || busy) return
    busy = true; error = ''
    try {
      if (picked.own) await api.keepCover(target.ids, own)
      else await api.useWebCover(target.ids, picked.f.url, picked.f.from)
      leave()
      dialog.close()
      onchosen()
    } catch (e) { error = String(e) } finally { busy = false }
  }

  async function remove() {
    leave()
    dialog.close()
    await onremove(target.ids)
  }

  const isPicked = (f: FoundCover) => !!picked && !picked.own && picked.f.url === f.url
</script>

<svelte:window onpaste={paste} />

<dialog bind:this={dialog} aria-labelledby="cover-title" onclose={() => { leave(); onclose() }}
  oncancel={e => { if (busy) e.preventDefault() }} onpointerdown={e => (downOut = e.target === dialog)} onpointerup={e => (upOut = e.target === dialog)}
  onclick={e => { if (e.target === dialog && downOut && upOut && !busy) dialog.close() }}>
  <div class="head">
    <h2 id="cover-title">{S.chooseCoverTitle}</h2>
    <p class="name" dir="auto" title={target.name}>{target.name}</p>
    <button class="x" aria-label={S.close} disabled={busy} onclick={() => dialog.close()}><svg viewBox="0 0 10 10" aria-hidden="true"><path d="M1.5 1.5l7 7M8.5 1.5l-7 7" /></svg></button>
  </div>
  <div class="body">
    <h3>{S.ownPicture}</h3>
    <div class="own">
      {#if own}
        <button class="pick" class:on={picked?.own} aria-pressed={!!picked?.own} title={S.pastedOrPicked} onclick={() => (picked = { own: true })} ondblclick={use}>
          <span class="pic"><img src={ownSrc} alt={S.pastedOrPicked} /></span>
        </button>
      {/if}
      <div class="drop">
        <button class="btn" onclick={pickFile}>{S.pickPicture}</button>
        <span class="muted">{S.orPaste}</span>
      </div>
    </div>

    <h3>{S.fromTheWeb}</h3>
    <form class="find" onsubmit={e => { e.preventDefault(); search() }}>
      <!-- Escape closes the picker at once, rather than first clearing the words as a search field does. -->
      <input bind:this={field} bind:value={words} type="search" aria-label={S.searchWords} placeholder={S.searchWords} dir="auto"
        onkeydown={e => { if (e.key === 'Escape') { e.preventDefault(); if (!busy) dialog.close() } }} />
      <!-- Not disabled while searching, so a keyboard press keeps its focus. -->
      <button class="btn primary" type="submit" disabled={!words.trim()} aria-busy={searching}>{searching ? S.searching : S.search}</button>
    </form>
    <p class="muted note">{S.sendsWords}</p>

    {#each results ?? [] as r (r.from)}
      {@const site = S.sites[r.from] ?? r.from}
      <h4>{S.foundOn(site, r.found.length)}</h4>
      {#if r.error}<p class="muted">{S.siteFailed(site, r.error)}</p>{/if}
      {#if r.found.length}
        <div class="grid">
          {#each r.found as f (f.url)}
            {@const text = S.coverResult(f.title, f.by, f.year)}
            <button class="pick" class:on={isPicked(f)} aria-pressed={isPicked(f)} title={text} aria-label={text} onclick={() => (picked = { own: false, f })} ondblclick={use}>
              <span class="pic">
                {#if previews[f.preview]}<img src={previews[f.preview]} alt="" />
                {:else if previews[f.preview] === ''}<span class="none">{S.previewFailed}</span>
                {:else}<span class="wait"></span>{/if}
              </span>
              <span class="t" dir="auto">{f.title}</span>
              <span class="by" dir="auto">{[f.by, f.year].filter(Boolean).join(', ')}</span>
            </button>
          {/each}
        </div>
      {/if}
    {/each}
  </div>
  <div class="foot">
    {#if error}<p class="err" role="alert">{error}</p>{/if}
    {#if chosen}<button class="btn" onclick={remove}>{S.removeCover}</button>{/if}
    <span class="gap"></span>
    <button class="btn" disabled={busy} onclick={() => dialog.close()}>{S.cancel}</button>
    <button class="btn primary" disabled={!picked || busy} onclick={use}>{S.useCover}</button>
  </div>
</dialog>

<style>
  /* As the Work dialog: a raised sheet with a sticky head and foot. */
  dialog { border: 0; border-radius: 12px; padding: 0; width: min(820px, calc(100vw - 24px)); max-height: calc(100vh - 48px);
    background: var(--bg); color: var(--ink); box-shadow: 0 20px 60px rgba(0,0,0,.35); display: none; flex-direction: column; overflow: hidden; }
  dialog[open] { display: flex; animation: rise .14s ease-out; }
  dialog::backdrop { background: rgba(10, 14, 20, .5); }
  @keyframes rise { from { opacity: 0; transform: translateY(6px) scale(.98) } }
  .head { position: relative; padding: 18px 24px 12px; border-bottom: 1px solid var(--line); }
  h2 { margin: 0; font-size: 19px; padding-right: 40px; }
  .name { margin: 2px 0 0; color: var(--muted); font-size: 13px; padding-right: 40px; overflow-wrap: anywhere;
    display: -webkit-box; -webkit-box-orient: vertical; -webkit-line-clamp: 2; line-clamp: 2; overflow: hidden; }
  .x { position: absolute; top: 12px; right: 12px; width: 32px; height: 32px; border: 0; background: none; border-radius: 6px; display: grid; place-items: center; }
  .x:hover { background: var(--hover); }
  .x svg { width: 11px; height: 11px; fill: none; stroke: currentColor; stroke-width: 1.2; }
  .body { padding: 4px 24px 18px; overflow: auto; flex: 1; min-height: 0; }
  h3 { font-size: 14.5px; margin: 16px 0 8px; }
  h4 { font-size: 13px; font-weight: 600; margin: 16px 0 8px; color: var(--soft); }
  .muted { color: var(--muted); font-size: 13px; }
  .own { display: flex; gap: 12px; align-items: stretch; }
  .own .pick { width: 96px; flex: none; }
  .drop { flex: 1; display: flex; gap: 12px; align-items: center; flex-wrap: wrap; padding: 14px 16px; border: 1.5px dashed var(--line); border-radius: 8px; background: var(--pane); }
  .find { display: flex; gap: 8px; }
  .find input { flex: 1; min-width: 0; height: 32px; border: 1px solid var(--line); background: var(--field); border-radius: 6px; padding: 0 12px; }
  .note { margin: 6px 0 0; font-size: 12.5px; }
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(104px, 1fr)); gap: 12px 10px; }
  .pick { border: 0; background: none; padding: 4px; border-radius: 8px; text-align: left; min-width: 0; display: flex; flex-direction: column; gap: 4px; }
  .pick:hover { background: var(--hover); }
  .pick.on { background: var(--sel); box-shadow: inset 0 0 0 2px var(--focus); }
  .pic { aspect-ratio: 2 / 3; border-radius: 5px; background: var(--well); display: grid; place-items: center; overflow: hidden; }
  .pic img { width: 100%; height: 100%; object-fit: contain; display: block; }
  .none { font-size: 11.5px; color: var(--muted); padding: 6px; text-align: center; }
  .wait { width: 100%; height: 100%; background: linear-gradient(100deg, transparent 30%, color-mix(in srgb, var(--field) 60%, transparent) 50%, transparent 70%) var(--well); background-size: 200% 100%; animation: shine 1.2s linear infinite; }
  @keyframes shine { to { background-position: -200% 0; } }
  .t { font-size: 12.5px; font-weight: 600; line-height: 1.25; overflow-wrap: anywhere;
    display: -webkit-box; -webkit-box-orient: vertical; -webkit-line-clamp: 2; line-clamp: 2; overflow: hidden; }
  .by { font-size: 11.5px; color: var(--muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .foot { display: flex; flex-wrap: wrap; gap: 8px; align-items: center; padding: 12px 24px; border-top: 1px solid var(--line); }
  .foot .gap { flex: 1; }
  .foot .btn { min-width: 90px; padding: 6px 14px; }
  .err { flex-basis: 100%; margin: 0 0 4px; color: var(--danger); font-size: 13px; overflow-wrap: anywhere; }
  @media (prefers-reduced-motion: reduce) { .wait { animation: none; } dialog[open] { animation: none; } }
</style>
