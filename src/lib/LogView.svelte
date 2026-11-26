<script lang="ts">
  // Settings, Log: the app log and the network log (logs.rs), in three views the person picks:
  // Sessions (a block per app start), Table (a row per entry) and Summary (counts first).
  import { onMount } from 'svelte'
  import { api, fold, type LogEntry, type LogView } from './api'
  import { S } from './strings'
  import { saveUi, ui } from './ui.svelte'

  let { onerror }: { onerror: (e: string) => void } = $props()

  let app = $state<LogEntry[] | null>(null)
  let net = $state<LogEntry[] | null>(null)
  let which = $state<'app' | 'network'>('app')
  let q = $state('')
  let level = $state('')
  let tile = $state('')
  let limit = $state(1000)
  let copied = $state(false)
  /** Sessions opened or closed by hand, and how far each one is shown, by its start. */
  let open = $state<Record<string, boolean>>({})
  let shownTo = $state<Record<string, number>>({})

  async function load() {
    try {
      ;[app, net] = await Promise.all([api.logEntries('app'), api.logEntries('network')])
    } catch (e) { onerror(String(e)) }
  }
  onMount(load)

  const text = (e: LogEntry) => e.parts.map(p => p.text).join('')
  const problem = (e: LogEntry) => e.level === 'WARN' || e.level === 'ERROR'
  /** A file opened or shown; one that failed counts as a problem instead. */
  const opened = (e: LogEntry) => e.topic === 'open' && !problem(e)
  const ws = $derived(fold(q).split(/\s+/).filter(Boolean))
  const hit = (e: LogEntry) => ws.every(w => fold(text(e)).includes(w))
  const list = $derived(which === 'app' ? app : net)
  const count = (l: string) => (app ?? []).filter(e => e.level === l).length

  function setView(v: LogView) { ui.logView = v; saveUi(); limit = 1000; level = '' }
  async function copy() {
    await navigator.clipboard.writeText(await (which === 'network' && ui.logView !== 'summary' ? api.readNetworkLog() : api.readLog()))
    copied = true; setTimeout(() => (copied = false), 2000)
  }
  async function folder() {
    try { await api.reveal((await api.info()).places.find(p => p.path.endsWith('Logs'))?.path ?? '', true) } catch (e) { onerror(String(e)) }
  }
  async function reveal(p: string) {
    try { await api.reveal(p) } catch (e) { onerror(String(e)) }
  }

  // Table: newest first, filtered by level and search.
  const rows = $derived((list ?? []).filter(e => (!level || e.level === level) && hit(e)).reverse())

  // Sessions: a block per app start, newest first.
  type Session = { start: LogEntry | null; entries: LogEntry[] }
  const sessions = $derived.by(() => {
    const out: Session[] = []
    for (const e of app ?? []) {
      if (e.topic === 'start' || !out.length) out.push({ start: e.topic === 'start' ? e : null, entries: [] })
      out[out.length - 1].entries.push(e)
    }
    return out.reverse()
  })
  const filtering = $derived(!!ws.length || !!level)
  const shown = (s: Session) => s.entries.filter(e => (!level || problem(e)) && hit(e)).reverse()
  const keyOf = (s: Session) => (s.start ? `${s.start.date} ${s.start.time}` : '')
  const isOpen = (s: Session, i: number) => open[keyOf(s)] ?? (filtering || i === 0)
  function day(d: string) {
    const at = new Date(), iso = (x: Date) => `${x.getFullYear()}-${String(x.getMonth() + 1).padStart(2, '0')}-${String(x.getDate()).padStart(2, '0')}`
    if (d === iso(at)) return S.today
    at.setDate(at.getDate() - 1)
    return d === iso(at) ? S.yesterday : d
  }

  // Summary: counts, each a filter.
  const TILES: [string, (e: LogEntry) => boolean][] = [
    ['problems', problem], ['open', opened], ['scan', e => e.topic === 'scan'], ['thumbs', e => e.topic === 'thumbs'], ['net', () => true],
  ]
  const tileCount = (k: string, f: (e: LogEntry) => boolean) => (k === 'net' ? (net ?? []).length : (app ?? []).filter(f).length)
  const summary = $derived.by(() => {
    const t = TILES.find(([k]) => k === tile)
    return ((tile === 'net' ? net : app) ?? []).filter(e => (!t || t[1](e)) && hit(e)).reverse()
  })
</script>

{#snippet when(e: LogEntry, date: boolean)}
  <time>{date && e.date ? `${e.date} ` : ''}{e.time}{e.utc && e.time ? ' UTC' : ''}</time>
{/snippet}
{#snippet msg(e: LogEntry)}
  <span class="msg">{#each e.parts as p, i (i)}{#if p.path}<button class="path" title={p.text} aria-label={S.showPath(p.text)} onclick={() => reveal(p.text)}>{p.text}</button>{:else}{p.text}{/if}{/each}</span>
{/snippet}
{#snippet badge(e: LogEntry)}
  {#if problem(e)}<span class="lvl {e.level}">{e.level === 'WARN' ? 'WARN' : 'ERROR'}</span>{/if}
{/snippet}
{#snippet topic(t: string)}
  <svg class="topic" viewBox="0 0 16 16" aria-hidden="true">
    {#if t === 'start'}<path d="M5 3.5v9l7-4.5z" />
    {:else if t === 'scan'}<circle cx="7" cy="7" r="4" /><path d="M10 10l3.5 3.5" />
    {:else if t === 'open'}<path d="M1.5 12.5v-8a1 1 0 0 1 1-1h4L8 5h5a1 1 0 0 1 1 1v1" /><path d="M1.5 12.5 3.6 7.5h11l-2.1 5z" />
    {:else if t === 'thumbs'}<rect x="2" y="3" width="12" height="10" rx="1" /><path d="M2.5 11.5l3.5-3.5 3 3 2-2 2.5 2.5" />
    {:else if t === 'net'}<circle cx="8" cy="8" r="6" /><path d="M2 8h12M8 2c2 2 2 10 0 12M8 2c-2 2-2 10 0 12" />
    {:else if t === 'save'}<path d="M3 2.5h8l2 2v9H3z" /><path d="M5.5 2.5v3h5v-3M5 13.5v-4h6v4" />
    {:else}<circle cx="8" cy="8" r="1.5" />{/if}
  </svg>
{/snippet}
{#snippet more(n: number)}
  {#if n > limit}<button class="btn more" onclick={() => (limit += 1000)}>{S.showMore(Math.min(1000, n - limit), n - limit)}</button>{/if}
{/snippet}

<div class="bar">
  {#if ui.logView !== 'summary'}
    <div class="seg" role="group" aria-label={S.tabLog}>
      <button aria-pressed={which === 'app'} onclick={() => { which = 'app'; limit = 1000 }}>{S.logApp}</button>
      <button aria-pressed={which === 'network'} onclick={() => { which = 'network'; level = ''; limit = 1000 }}>{S.logNetwork}</button>
    </div>
  {/if}
  <input class="find" type="search" placeholder={S.searchLabel} aria-label={S.searchLabel} bind:value={q} />
  <span class="grow"></span>
  <span class="muted small">{S.view}</span>
  <div class="seg" role="group" aria-label={S.view}>
    {#each ['sessions', 'table', 'summary'] as v (v)}<button aria-pressed={ui.logView === v} onclick={() => setView(v as LogView)}>{S.logViews[v]}</button>{/each}
  </div>
</div>
<p class="muted">{ui.logView === 'summary' ? S.summaryWhat : which === 'app' ? S.logAppWhat : S.logNetworkWhat}</p>

{#if !app || !net}
  <p class="muted">…</p>
{:else if ui.logView === 'table'}
  {#if which === 'app'}
    <div class="chips">
      {#each ['ERROR', 'WARN', 'INFO'] as l (l)}<button class="chip" aria-pressed={level === l} onclick={() => (level = level === l ? '' : l)}>{S.levels[l]} <b>{count(l)}</b></button>{/each}
    </div>
  {/if}
  <div class="rows" role="list">
    {#each rows.slice(0, limit) as e, i (i)}
      <div class="row {e.level}" role="listitem">{@render when(e, true)}<span>{@render badge(e)}</span>{@render msg(e)}</div>
    {:else}
      <p class="muted pad">{list?.length ? S.logNoMatch : S.logEmpty}</p>
    {/each}
  </div>
  {@render more(rows.length)}
{:else if ui.logView === 'sessions'}
  {#if which === 'app'}
    <div class="chips">
      <button class="chip" aria-pressed={!!level} onclick={() => (level = level ? '' : 'problems')}>{S.problemsOnly} <b>{count('ERROR') + count('WARN')}</b></button>
    </div>
    {#each sessions as s, i (i)}
      {@const es = shown(s)}
      {#if es.length || !filtering}
        {@const errs = s.entries.filter(e => e.level === 'ERROR').length}
        {@const warns = s.entries.filter(e => e.level === 'WARN').length}
        <section class="session">
          <button class="head" aria-expanded={isOpen(s, i)} onclick={() => (open[keyOf(s)] = !isOpen(s, i))}>
            <span class="tri" aria-hidden="true">{isOpen(s, i) ? '▾' : '▸'}</span>
            <b>{s.start ? `${day(s.start.date)}, ${s.start.time.slice(0, 5)}` : S.beforeStart}</b>
            <span class="muted">{S.session(s.entries.length, s.entries.filter(opened).length)}</span>
            <span class="grow"></span>
            {#if errs}<span class="lvl ERROR">{S.nErrors(errs)}</span>{/if}
            {#if warns}<span class="lvl WARN">{S.nWarnings(warns)}</span>{/if}
          </button>
          {#if isOpen(s, i)}
            {@const to = shownTo[keyOf(s)] ?? 1000}
            {#each es.slice(0, to) as e, j (j)}
              <div class="ev {e.level}">{@render topic(e.topic)}{@render when(e, false)}{@render msg(e)}</div>
            {/each}
            {#if es.length > to}<button class="btn more" onclick={() => (shownTo[keyOf(s)] = to + 1000)}>{S.showMore(Math.min(1000, es.length - to), es.length - to)}</button>{/if}
          {/if}
        </section>
      {/if}
    {:else}
      <p class="muted">{S.logEmpty}</p>
    {/each}
    {#if filtering && sessions.every(s => !shown(s).length)}<p class="muted">{S.logNoMatch}</p>{/if}
  {:else}
    <div class="rows" role="list">
      {#each rows.slice(0, limit) as e, i (i)}
        <div class="ev" role="listitem">{@render topic('net')}{@render when(e, true)}{@render msg(e)}</div>
      {:else}
        <p class="muted pad">{net.length ? S.logNoMatch : S.logEmpty}</p>
      {/each}
    </div>
    {@render more(rows.length)}
  {/if}
{:else}
  <div class="tiles">
    {#each TILES as [k, f] (k)}
      <button class="tile" class:err={k === 'problems' && tileCount(k, f) > 0} aria-pressed={tile === k} onclick={() => { tile = tile === k ? '' : k; limit = 1000 }}>
        <b>{tileCount(k, f)}</b><span>{S.tiles[k]}</span>
      </button>
    {/each}
  </div>
  <h4>{tile ? S.tiles[tile] : S.everything} <span class="muted">{summary.length}</span>
    {#if tile}<button class="chip" onclick={() => (tile = '')}>{S.logShowAll}</button>{/if}</h4>
  <div class="rows" role="list">
    {#each summary.slice(0, limit) as e, i (i)}
      <div class="row {e.level}" role="listitem">{@render when(e, true)}<span>{@render badge(e)}</span>{@render msg(e)}</div>
    {:else}
      <p class="muted pad">{S.logNoMatch}</p>
    {/each}
  </div>
  {@render more(summary.length)}
{/if}

<div class="foot">
  <button class="btn" onclick={load}>{S.logRefresh}</button>
  <button class="btn" onclick={copy}>{copied ? S.copied : which === 'network' && ui.logView !== 'summary' ? S.copyNetworkLog : S.copyLog}</button>
  <button class="btn" onclick={folder}>{S.openLogFolder}</button>
</div>

<style>
  .bar { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; margin: 4px 0 10px; }
  .find { flex: 1; min-width: 140px; max-width: 320px; border: 1px solid var(--line); border-radius: 6px; padding: 4px 8px; background: var(--field); }
  .grow { flex: 1; }
  .small { font-size: 12.5px; }
  .pad { padding: 10px 6px; }
  /* Match Library's segmented control and chips. */
  .seg { display: inline-flex; border: 1px solid var(--line); border-radius: 7px; background: var(--field); }
  .seg button { border: 0; background: none; padding: 3px 11px; font-size: 13px; position: relative; }
  .seg button:first-child { border-radius: 6px 0 0 6px; }
  .seg button:last-child { border-radius: 0 6px 6px 0; }
  .seg button:focus-visible { outline-offset: -2px; z-index: 1; border-radius: 6px; }
  .seg button + button { border-left: 1px solid var(--line); }
  .seg button[aria-pressed='true'] { background: var(--sel); color: var(--link); font-weight: 600; box-shadow: inset 0 0 0 1px var(--sel-line); }
  .chips { display: flex; gap: 6px; margin-bottom: 10px; }
  .chip { border: 1px solid var(--line); border-radius: 999px; padding: 1px 10px; background: var(--field); font-size: 13px; }
  .chip:hover { background: var(--hover); }
  .chip[aria-pressed='true'] { background: var(--sel); color: var(--link); border-color: var(--sel-line); font-weight: 600; }
  .chip b { font-weight: 600; }
  .lvl { font-size: 11px; font-weight: 700; letter-spacing: .03em; border-radius: 4px; padding: 0 5px; white-space: nowrap; }
  .lvl.WARN { color: var(--next-ink); background: color-mix(in srgb, var(--next) 55%, transparent); }
  .lvl.ERROR { color: var(--danger); background: color-mix(in srgb, var(--danger) 14%, transparent); }
  time { font-family: ui-monospace, "Cascadia Mono", Consolas, monospace; font-size: 12px; color: var(--muted); white-space: nowrap; }
  .msg { overflow-wrap: anywhere; white-space: pre-wrap; }
  .path { display: inline; border: 0; background: none; padding: 0; color: var(--link); text-align: left; font: inherit; overflow-wrap: anywhere; }
  .path:hover { text-decoration: underline; }
  .rows { font-size: 13px; border-top: 1px solid var(--line); }
  .row { display: grid; grid-template-columns: 150px 52px 1fr; gap: 10px; padding: 4px 6px; border-bottom: 1px solid var(--line); align-items: baseline; }
  .row.WARN { background: color-mix(in srgb, var(--next) 14%, transparent); }
  .row.ERROR { background: color-mix(in srgb, var(--danger) 9%, transparent); }
  /* Settings' pane is a scrolling flex column, and overflow: hidden would let a block shrink to its border. */
  .session { border: 1px solid var(--line); border-radius: 10px; margin-bottom: 10px; background: var(--field); overflow: hidden; flex: none; }
  .head { display: flex; gap: 10px; align-items: center; width: 100%; border: 0; background: none; padding: 10px 14px; text-align: left; }
  .head:hover { background: var(--hover); }
  .tri { color: var(--muted); width: 10px; }
  .ev { display: grid; grid-template-columns: 18px 64px 1fr; gap: 8px; padding: 5px 14px; border-top: 1px solid var(--line); font-size: 13px; align-items: baseline; }
  .rows .ev { grid-template-columns: 18px 172px 1fr; border-top: 0; border-bottom: 1px solid var(--line); padding: 5px 6px; }
  .ev.WARN { box-shadow: inset 3px 0 var(--next); }
  .ev.ERROR { box-shadow: inset 3px 0 var(--danger); }
  .topic { width: 14px; height: 14px; fill: none; stroke: var(--muted); stroke-width: 1.3; stroke-linejoin: round; stroke-linecap: round; align-self: center; }
  .tiles { display: grid; grid-template-columns: repeat(5, 1fr); gap: 8px; margin-bottom: 4px; }
  .tile { border: 1px solid var(--line); border-radius: 10px; background: var(--field); padding: 10px 12px; text-align: left; display: flex; flex-direction: column; gap: 2px; }
  .tile:hover { background: var(--hover); }
  .tile b { font-size: 22px; line-height: 1.1; }
  .tile span { font-size: 12.5px; color: var(--muted); }
  .tile[aria-pressed='true'] { border-color: var(--focus); box-shadow: inset 0 0 0 1px var(--focus); background: var(--sel); }
  .tile.err b { color: var(--danger); }
  h4 { display: flex; gap: 8px; align-items: center; margin: 14px 0 6px; font-size: 14px; }
  h4 .muted { font-weight: 400; }
  .more { margin-top: 10px; align-self: flex-start; }
  .foot { display: flex; gap: 8px; margin-top: 14px; }
</style>
