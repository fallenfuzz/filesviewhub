<script lang="ts">
  // Review's move (`decisions/2026/10/09/review-moves-set-aside-photos-itself-with-undo.md`): every photo set aside in the photo folder, by month, each ticked
  //; where they go, a warning when that takes them out of a synced folder, then Move with
  // its progress, what it did, and Undo. Save as a script stays for a person who wants to read it
  // first (`decisions/2026/10/08/the-app-saves-the-move-script-for-its-own-system.md`). Both take only the photos ticked.
  import { onMount } from 'svelte'
  import { SvelteSet } from 'svelte/reactivity'
  import { api, eventPlace, thumbSrc, type Catalog, type Item, type MoveScope, type Moved, type MovePlan, type PhotoPlace, type Thumb, type Work } from './api'
  import { dayKey, photoThumb, photoThumbId } from './photos'
  import { S } from './strings'

  let { section, scope, catalog, thumbs, onmoved, onreveal, onclose }: {
    section: PhotoPlace; scope?: MoveScope; catalog: Catalog; thumbs: Record<string, Thumb>
    onmoved: () => Promise<void>; onreveal: (path: string, open?: boolean) => void; onclose: () => void
  } = $props()

  let dialog: HTMLDialogElement
  let plan = $state<MovePlan | null>(null)
  let error = $state('')
  let busy = $state<{ done: number; total: number; undo: boolean } | null>(null)
  /** What the last press did: a move, or its Undo. */
  let done = $state<{ out: Moved; undo: boolean; to: string } | null>(null)
  let saved = $state('')
  /** The photos unticked, by their path inside the photo folder; they stay where they are. */
  const off = new SvelteSet<string>()

  /** Each photo of the plan with its Event and file, from the catalog as it is now (a move reads
   * the Section again). */
  const byRel = $derived.by(() => {
    const live = catalog.sections.flatMap(s => s.works).filter(w => w.event && eventPlace(w) === section.id)
    return new Map((live.length ? live : section.works).flatMap(w => w.items.map(i => [i.rel, { w, i }] as const)))
  })
  const rows = $derived((plan?.photos ?? []).map(([rel, previews, shot]) => {
    const x = byRel.get(rel)
    const dir = rel.includes('/') ? rel.slice(0, rel.lastIndexOf('/')) : ''
    // The month is its Event's, as the calendar places it; the photo's own time sorts it.
    return { rel, previews, shot, w: x?.w, i: x?.i, month: x?.w.taken ? dayKey(x.w.taken[0]).slice(0, 7) : '', at: x?.i.at ?? x?.w.taken?.[0] ?? Infinity,
      name: rel.slice(rel.lastIndexOf('/') + 1), folder: [section.name, ...dir.split('/').filter(Boolean)].join(' › ') }
  }))
  /** The photos by month, oldest first, Undated last. */
  const months = $derived.by(() => {
    const g = new Map<string, typeof rows>()
    for (const r of [...rows].sort((a, b) => (a.month || '~').localeCompare(b.month || '~') || a.at - b.at || a.rel.localeCompare(b.rel))) {
      const l = g.get(r.month)
      if (l) l.push(r); else g.set(r.month, [r])
    }
    return [...g].map(([k, list]) => ({ k, name: k ? `${S.months[Number(k.slice(5)) - 1]} ${k.slice(0, 4)}` : S.undatedTitle, list }))
  })
  const ticked = $derived(rows.filter(r => !off.has(r.rel)))
  const inScope = $derived(scope ? new Set(scope.works.map(w => w.id)) : null)
  const scopeN = $derived(inScope ? rows.filter(r => r.w && inScope.has(r.w.id)).length : 0)
  const cloud = $derived(ticked.filter(r => r.i?.cloud).length)
  // A shot's previews go only when all its videos do (two on a dual-lens camera).
  const previews = $derived(rows.reduce((a, r) => a + (r.previews && !rows.some(o => o.shot === r.shot && off.has(o.rel)) ? r.previews : 0), 0))
  const left = $derived(ticked.length - cloud)
  /** Ticks or unticks these photos: all of them when any is unticked, else none. */
  function tick(list: typeof rows) {
    const on = list.some(r => off.has(r.rel))
    for (const r of list) { if (on) off.delete(r.rel); else off.add(r.rel) }
  }
  const tickState = (list: typeof rows) => { const n = list.filter(r => off.has(r.rel)).length; return n === 0 ? 'on' : n === list.length ? 'off' : 'some' }
  const bg = (w?: Work, i?: Item) => { const t = w && i && photoThumb(thumbs, w, i); return t ? `background-image:url('${thumbSrc(t.key)}');${t.n > 1 ? 'background-size:auto 100%;background-position:0 50%;' : ''}` : '' }

  onMount(() => {
    dialog.showModal()
    load()
    const unlisten = api.onMoveProgress(p => { if (busy) busy = { ...busy, ...p } })
    return () => { unlisten.then(f => f()) }
  })
  async function load() {
    try { plan = await api.movePlan(section.id) } catch (e) { error = String(e) }
    // A move drops the thumbnails of the photos it moved, so they are asked for after each one.
    api.wantThumbs(months.flatMap(m => m.list).filter(r => r.w && r.i).slice(0, 400).map(r => photoThumbId(r.w!, r.i!)))
  }
  async function pick() {
    error = ''
    const p = await api.pickFolder()
    if (!p) return
    try { plan = await api.setMoveTo(section.id, p) } catch (e) { error = String(e) }
  }
  async function useDefault() {
    error = ''
    try { plan = await api.setMoveTo(section.id, null) } catch (e) { error = String(e) }
  }
  async function move() {
    if (!plan) return
    error = ''; saved = ''
    const to = plan.to
    busy = { done: 0, total: left + previews, undo: false }
    try { done = { out: await api.moveSetAside(section.id, ticked.map(r => r.rel)), undo: false, to } } catch (e) { error = String(e) }
    await after()
  }
  async function undo(id: number, to: string) {
    error = ''
    busy = { done: 0, total: 0, undo: true }
    try { done = { out: await api.undoMove(id), undo: true, to } } catch (e) { error = String(e) }
    await after()
  }
  /** The Section is read again, then the plan, and the dialog can always be closed after. */
  async function after() {
    try { await onmoved(); await load() } catch (e) { error = String(e) } finally { busy = null }
  }
  async function script() {
    error = ''
    saved = ''
    try { const p = await api.saveMoveScript(section.id, ticked.map(r => r.rel)); if (p) saved = S.savedTo(p) } catch (e) { error = String(e) }
  }
</script>

<dialog bind:this={dialog} aria-labelledby="move-title" onclose={onclose} oncancel={e => busy && e.preventDefault()}>
  <h2 id="move-title">{plan?.count ? S.moveTitle : done ? (done.undo ? S.moveUndone : S.photosMoved) : S.moveAside.replace('…', '')}</h2>

  {#if done}
    <p class="ok" role="status">✓
      {done.undo ? S.putBack(done.out.moved) : S.movedInto(done.out.moved, done.to)}
      {#if done.out.skipped.length}{done.undo ? S.nLeft(done.out.skipped.length) : S.nSkipped(done.out.skipped.length)}.{/if}
    </p>
    {#if done.out.skipped.length}
      <details><summary>{done.undo ? S.nLeft(done.out.skipped.length) : S.nSkipped(done.out.skipped.length)}</summary>
        <ul>{#each done.out.skipped as [name, why] (name + why)}<li><span dir="auto">{name}</span>: {why}</li>{/each}</ul>
      </details>
    {/if}
  {/if}

  {#if plan && plan.count}
    {@const all = tickState(rows)}
    <div class="head">
      <input type="checkbox" aria-label={S.moveAll} title={S.moveAll} checked={all === 'on'} indeterminate={all === 'some'} disabled={!!busy} onchange={() => tick(rows)} />
      <span class="scope">{S.moveScope(rows.length, section.name, scope && scope.name !== section.name ? [scopeN, scope.name] : undefined)}</span>
      <span class="muted">{S.moveUntick}</span>
    </div>
    <div class="list" role="group" aria-label={S.moveScope(rows.length, section.name)}>
      {#each months as m (m.k)}
        {@const st = tickState(m.list)}
        <label class="month">
          <input type="checkbox" aria-label={S.moveMonth(m.name)} checked={st === 'on'} indeterminate={st === 'some'} disabled={!!busy} onchange={() => tick(m.list)} />
          <span>{m.name}</span><span class="n">{m.list.length - m.list.filter(r => off.has(r.rel)).length} / {m.list.length}</span>
        </label>
        {#each m.list as r (r.rel)}
          <label class="photo" class:out={off.has(r.rel)}>
            <input type="checkbox" checked={!off.has(r.rel)} disabled={!!busy} onchange={() => tick([r])} />
            <span class="th" style={bg(r.w, r.i)}></span>
            <span class="txt"><span class="name" dir="auto" title={r.name}>{r.name}</span><span class="folder" dir="auto" title={r.i ? r.i.path.slice(0, r.i.path.length - r.name.length - 1) : r.folder}>{r.folder}</span></span>
          </label>
        {/each}
      {/each}
    </div>
    <p class="into">{S.moveInto}</p>
    {#if !plan.picked}<p class="what">{S.moveDefault}</p>{/if}
    <p class="path" dir="auto">{plan.to}</p>
    <div class="row">
      <button class="btn" onclick={pick} disabled={!!busy}>{S.movePick}</button>
      {#if plan.picked}<button class="btn" onclick={useDefault} disabled={!!busy}>{S.moveUseDefault}</button>{/if}
    </div>
    {#if plan.leaves_sync}<p class="warn">{S.moveLeavesSync(plan.leaves_sync)}</p>{/if}
    {#if previews}<p class="muted">{S.movePreviews(previews)}</p>{/if}
    {#if cloud}<p class="muted">{S.moveCloud(cloud)}</p>{/if}
    <p class="muted">{S.moveSafe}</p>
  {/if}

  {#if plan?.last && !busy}
    <p class="last">{#if done?.out.id !== plan.last[0]}{S.lastMove(plan.last[2], plan.last[1], plan.last[3])}{/if}
      <button class="btn" onclick={() => undo(plan!.last![0], plan!.last![3])}>{S.undoMove}</button>
      <button class="btn" onclick={() => onreveal(plan!.last![3], true)}>{S.openFolder}</button></p>
  {/if}

  {#if busy}<p role="status">{busy.undo ? S.undoing(busy.done, busy.total) : S.moving(busy.done, busy.total)}</p>{/if}
  {#if error}<p class="err" role="alert">{error}</p>{/if}
  <p class="ok" role="status">{#if saved}✓ <span dir="auto">{saved}</span>{/if}</p>

  <div class="bar">
    {#if plan && plan.count}
      <button class="btn" onclick={script} disabled={!!busy || !ticked.length}>{S.saveScript}</button>
      <span class="grow"></span>
      <button class="btn primary" onclick={move} disabled={!!busy || !left}>{S.moveGo(left)}</button>
    {:else}<span class="grow"></span>{/if}
    <button class="btn" onclick={() => dialog.close()} disabled={!!busy}>{plan?.count ? S.cancel : S.close}</button>
  </div>
</dialog>

<style>
  dialog { border: 0; border-radius: 12px; padding: 20px 22px; width: min(620px, 94vw); background: var(--bg); color: var(--ink); box-shadow: 0 20px 60px rgba(0,0,0,.35); }
  dialog::backdrop { background: rgba(10, 14, 20, .45); }
  h2 { font-size: 18px; margin: 0 0 12px; }
  p { margin: 8px 0; line-height: 1.45; }
  .into { margin-bottom: 2px; font-weight: 600; }
  .what { margin: 0; }
  .path { margin-top: 2px; color: var(--muted); font-size: 12.5px; word-break: break-all; }
  .row, .bar { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; }
  .bar { margin-top: 16px; }
  .grow { flex: 1; }
  .muted { color: var(--muted); font-size: 13px; }
  .warn { padding: 8px 10px; border-radius: 8px; background: color-mix(in srgb, var(--next) 35%, transparent); font-size: 13px; }
  .err { color: var(--danger); }
  .ok { color: var(--done); font-weight: 600; font-size: 13.5px; overflow-wrap: anywhere; }
  .ok:empty { margin: 0; }
  .last { font-size: 13px; display: flex; gap: 8px; align-items: center; flex-wrap: wrap; }
  details { font-size: 13px; }
  ul { margin: 6px 0; padding-left: 20px; max-height: 160px; overflow: auto; }
  input[type=checkbox] { width: 16px; height: 16px; margin: 0; flex: none; accent-color: var(--focus); cursor: pointer; }
  .head { display: flex; gap: 10px; align-items: baseline; flex-wrap: wrap; margin: 4px 0 6px; padding-left: 9px; }
  .head input { align-self: center; }
  .scope { font-weight: 600; }
  .list { max-height: min(340px, 42vh); overflow: auto; scroll-padding-top: 36px; border: 1px solid var(--line); border-radius: 8px; background: var(--field); margin-bottom: 12px; overscroll-behavior: contain; }
  .month { position: sticky; top: 0; z-index: 1; display: flex; gap: 10px; align-items: center; padding: 7px 8px; font-weight: 600; font-size: 13px; background: var(--side); border-bottom: 1px solid var(--line); cursor: pointer; }
  .month .n { margin-left: auto; color: var(--muted); font-weight: 400; font-variant-numeric: tabular-nums; }
  .photo { display: flex; gap: 10px; align-items: center; padding: 5px 8px; cursor: pointer; border-radius: 6px; margin: 2px 4px; transition: background-color .12s, opacity .12s; }
  .photo:hover { background: var(--hover); }
  .photo.out { opacity: .55; }
  .photo.out .th { filter: grayscale(1); }
  .photo:has(input:focus-visible), .month:has(input:focus-visible) { outline: 2px solid var(--focus); outline-offset: -2px; }
  .list input:focus-visible { outline: none; }
  .th { width: 48px; height: 36px; flex: none; border-radius: 4px; background: color-mix(in srgb, var(--ink) 10%, transparent) center / cover no-repeat; }
  .txt { display: flex; flex-direction: column; min-width: 0; }
  .name, .folder { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .name { font-size: 13.5px; }
  .folder { font-size: 12px; color: var(--muted); }
</style>
