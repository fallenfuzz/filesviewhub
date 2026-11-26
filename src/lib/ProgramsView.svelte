<script lang="ts">
  // Settings, Programs: every type in the catalog with the program it opens in (programs.rs), in
  // two views the person picks: Table (by kind) and By program.
  import { onMount, tick } from 'svelte'
  import { api, type Program, type ProgramList, type ProgramRow, type ProgramsView } from './api'
  import { S } from './strings'
  import { saveUi, ui } from './ui.svelte'

  let { onerror, onsaved }: { onerror: (e: string) => void; onsaved: () => void } = $props()

  const GROUPS = ['Video', 'Audio', 'Pictures', 'Books & documents', 'Other types']
  const OTHER = GROUPS[4]
  let list = $state<ProgramList | null>(null)
  let menu = $state<string | null>(null)
  let others = $state(false)
  let busy = $state(false)

  async function load() {
    try { list = await api.programList() } catch (e) { onerror(String(e)) }
  }
  onMount(load)

  const shown = (r: ProgramRow) => r.yours ?? r.default
  const key = (p: Program) => p.id || p.name
  /** A program's place on disk for its hover, as folders show theirs; a Store app has none. */
  const where = (p: Program | null | undefined) => (p && /^([A-Za-z]:\\|\\\\|\/)/.test(p.id) ? p.id : undefined)
  const same = (a: string | undefined, b: string | undefined) => !!a && !!b && a.toLowerCase() === b.toLowerCase()
  const label = (r: ProgramRow) => (r.reveals && !r.yours ? S.runsShown : shown(r)?.name ?? S.noProgram)
  const rowsOf = (g: string) => (list?.rows ?? []).filter(r => r.group === g)
  /** The rows a group shows: all, or in folded Other types only those with a choice, so a
   * choice (or a program since removed) is never hidden. */
  const visible = (g: string) => rowsOf(g).filter(r => g !== OTHER || !folds() || others || r.yours)
  /** Other types folds only when it holds more than three: a click to show fewer costs more. */
  const folds = () => rowsOf(OTHER).length > 3
  /** How many Other types the toggle names: the hidden ones while folded, else all. */
  const otherCount = () => (others ? rowsOf(OTHER).length : rowsOf(OTHER).length - visible(OTHER).length)
  /** The Other types toggle shows while it would show or hide something. */
  const toggles = () => folds() && (others || otherCount() > 0)
  const filesOf = (rs: ProgramRow[]) => rs.reduce((n, r) => n + r.files, 0)
  const byProgram = $derived.by(() => {
    const m = new Map<string, { p: Program | null; runs: boolean; rows: ProgramRow[] }>()
    for (const r of list?.rows ?? []) {
      if (r.group === OTHER && folds() && !others && !r.yours) continue
      const runs = r.reveals && !r.yours, p = runs ? null : shown(r)
      const k = runs ? '\u0000runs' : p ? key(p) : '\u0000none'
      if (!m.has(k)) m.set(k, { p, runs, rows: [] })
      m.get(k)!.rows.push(r)
    }
    // Types with no program come last: they are not a program.
    const none = (b: { p: Program | null; runs: boolean }) => (!b.p && !b.runs ? 1 : 0)
    return [...m.values()].sort((a, b) => none(a) - none(b) || filesOf(b.rows) - filesOf(a.rows))
  })

  function setView(v: ProgramsView) { ui.programsView = v; saveUi(); menu = null }

  async function run(f: () => Promise<void>) {
    busy = true
    try { await f() } catch (e) { onerror(String(e)) } finally { busy = false }
  }
  // Picking the system's program clears the choice, except for a type the app does not start
  // by itself: there the choice is what lets it open at all.
  const pick = (r: ProgramRow, p: Program) => {
    if (same(r.default?.id, p.id) && !r.reveals) return reset(r)
    return run(async () => {
      await api.setOpenWith(r.ext, p.id)
      r.yours = p; r.gone = false
      close()
      onsaved()
    })
  }
  const other = (r: ProgramRow) => run(async () => {
    close()
    const p = await api.pickProgram()
    if (!p) return
    await api.setOpenWith(r.ext, p)
    await load()
    onsaved()
  })
  const reset = (r: ProgramRow) => run(async () => {
    await api.setOpenWith(r.ext, null)
    r.yours = null; r.gone = false
    close()
    onsaved()
    if (!r.files && list) list.rows = list.rows.filter(x => x !== r)
    // A row that folded away or left takes its Change button with it: focus moves to the view.
    await tick()
    if (!opener?.isConnected) document.querySelector<HTMLElement>('[data-others], .seg button[aria-pressed="true"]')?.focus()
  })

  // The Change menu: focus moves into it, arrows move inside it, Escape returns to its button.
  let opener: HTMLElement | null = null
  async function toggle(r: ProgramRow, e: MouseEvent) {
    if (menu === r.ext) return close()
    opener = e.currentTarget as HTMLElement
    menu = r.ext
    await tick()
    // A menu near the bottom of the pane would be cut off: the pane scrolls to show it whole.
    const m = document.querySelector('.menu')
    m?.scrollIntoView({ block: 'nearest' })
    // Focus starts on the current program, else on the first row.
    ;(m?.querySelector<HTMLElement>('[aria-checked="true"]') ?? m?.querySelector<HTMLElement>('button'))?.focus({ preventScroll: true })
  }
  function close() {
    if (menu === null) return
    menu = null
    if (opener?.isConnected) opener.focus()
  }
  function keys(e: KeyboardEvent) {
    const items = [...document.querySelectorAll<HTMLElement>('.menu button')]
    const at = items.indexOf(document.activeElement as HTMLElement)
    if (e.key === 'Escape') { e.preventDefault(); e.stopPropagation(); close() }
    else if (e.key === 'ArrowDown') { e.preventDefault(); items[(at + 1) % items.length]?.focus() }
    else if (e.key === 'ArrowUp') { e.preventDefault(); items[(at - 1 + items.length) % items.length]?.focus() }
    else if (e.key === 'Tab') close()
  }
  function outside(e: MouseEvent) {
    if (menu !== null && !(e.target as HTMLElement).closest('.menu, .change')) menu = null
  }

  /** The system's program is a row of the menu, so picking it is how the choice clears; the
   * extra row stays only where it is not listed, or where picking it keeps a choice (runs). */
  const resetRow = (r: ProgramRow) => !!r.yours && (r.reveals || !r.choices.some(p => same(p.id, r.default?.id)))
  const hue = (s: string) => [...s].reduce((h, c) => (h * 31 + c.charCodeAt(0)) % 360, 7)
</script>

<svelte:window onclick={outside} />

{#snippet icon(p: Program | null, big = false)}
  {#if p && list?.icons[key(p)]}<img class="ico" class:big src={list.icons[key(p)]} alt="" />
  {:else if p}<span class="ico letter" class:big style="background: hsl({hue(p.name)} 45% 42%)" aria-hidden="true">{p.name[0]?.toUpperCase()}</span>
  {:else}<span class="ico blank" class:big aria-hidden="true"></span>{/if}
{/snippet}
{#snippet changeMenu(r: ProgramRow)}
  {#if menu === r.ext}
    <!-- svelte-ignore a11y_interactive_supports_focus -->
    <div class="menu" role="menu" aria-label={S.changeProgramFor(r.ext)} tabindex="-1" onkeydown={keys}>
      {#each r.choices as p (p.id)}
        {@const on = same(shown(r)?.id, p.id) && !(r.reveals && !r.yours)}
        <button role="menuitemradio" title={where(p)} aria-checked={on} disabled={busy} onclick={() => pick(r, p)}>
          <span class="tick" aria-hidden="true">{on ? '✓' : ''}</span>{@render icon(p)}<span class="nm">{p.name}</span>
          {#if same(r.default?.id, p.id)}<span class="def">{S.systemDefault}</span>{/if}
        </button>
      {/each}
      {#if r.choices.length}<hr />{/if}
      <button role="menuitem" disabled={busy} onclick={() => other(r)}>{S.otherProgram}</button>
      {#if resetRow(r)}<button role="menuitem" disabled={busy} onclick={() => reset(r)}>{S.systemProgram}</button>{/if}
    </div>
  {/if}
{/snippet}
{#snippet type(r: ProgramRow)}{S.typeName(r.ext)} <span class="ext-p">(<span class="ext">.{r.ext}</span>)</span>{/snippet}
{#snippet files(r: ProgramRow)}
  {#if r.files}{S.nFiles(r.files)}{:else}<span class="muted">{S.noFilesNow}</span>{/if}
{/snippet}

{#if list?.listed !== false}<div class="bar">
  <p class="muted">{ui.programsView === 'programs' ? S.programsByWhat : S.programsWhat}</p>
  <span class="grow"></span>
  <span class="muted small">{S.view}</span>
  <div class="seg" role="group" aria-label={S.view}>
    {#each ['table', 'programs'] as v (v)}<button aria-pressed={ui.programsView === v} onclick={() => setView(v as ProgramsView)}>{S.programsViews[v]}</button>{/each}
  </div>
</div>{/if}

{#if !list}
  <p class="muted">{S.programsReading}</p>
{:else if !list.listed}
  <p class="muted">{S.programsNotYet}</p>
{:else if !list.rows.length}
  <p class="muted">{S.programsNone}</p>
{:else if ui.programsView === 'table'}
  <table>
    <colgroup><col class="c-type" /><col /><col class="c-files" /><col class="c-act" /></colgroup>
    <thead><tr><th>{S.programsTypes}</th><th>{S.programsOpensIn}</th><th class="num">{S.programsFiles}</th><th></th></tr></thead>
    {#each GROUPS as g (g)}
      {@const rs = rowsOf(g)}
      {#if rs.length}
        <tbody>
          <tr class="group"><th colspan="4" scope="rowgroup">
            {#if g === OTHER && toggles()}<button class="link" data-others aria-expanded={others} onclick={() => (others = !others)}>{others ? '▾' : '▸'} {S.otherTypes(otherCount(), others)}</button>{:else}{g}{/if}
          </th></tr>
            {#each visible(g) as r (r.ext)}
              <tr>
                <td>{@render type(r)}</td>
                <td><span class="prog">{#if !(r.reveals && !r.yours)}{@render icon(shown(r))}{/if}<span class:muted={!shown(r) || (r.reveals && !r.yours)} title={r.reveals && !r.yours ? undefined : where(shown(r))}>{label(r)}</span>{#if r.gone}<span class="gone" title={S.notInstalledWhat}>{S.notInstalled}</span>{:else if r.yours}<span class="yours">{S.yourChoice}</span>{/if}</span></td>
                <td class="num">{@render files(r)}</td>
                <td class="act"><button class="btn sm change" aria-haspopup="menu" aria-expanded={menu === r.ext} aria-label={S.changeProgramFor(r.ext)} onclick={e => toggle(r, e)}>{S.changeProgram}</button>{@render changeMenu(r)}</td>
              </tr>
            {/each}
        </tbody>
      {/if}
    {/each}
  </table>
{:else}
  {#each byProgram as b (b.runs ? 'runs' : b.p ? key(b.p) : 'none')}
    <div class="byp" class:none={!b.p && !b.runs}>
      {@render icon(b.p, true)}
      <div>
        <div class="name" title={where(b.p)}>{b.runs ? S.runsShown : b.p?.name ?? S.noProgram}</div>
        <div class="types">
          {#each b.rows as r (r.ext)}
            <span class="t-wrap">
              <button class="tchip change" class:y={!!r.yours} class:gone={r.gone} title={r.gone ? S.notInstalledWhat : r.yours ? S.yourChoice : S.systemDefault} aria-haspopup="menu" aria-expanded={menu === r.ext} aria-label={S.changeProgramFor(r.ext)} onclick={e => toggle(r, e)}>
                <span>{@render type(r)}</span>{#if b.rows.length > 1 || !r.files}<span class="n">{@render files(r)}</span>{/if}{#if r.gone}<span class="note">{S.notInstalled}</span>{:else if r.yours}<span class="note">{S.yourChoice}</span>{/if}
              </button>
              {@render changeMenu(r)}
            </span>
          {/each}
        </div>
      </div>
      <span class="muted small">{S.nFiles(filesOf(b.rows))}</span>
    </div>
  {/each}
  {#if toggles()}
    <div class="foot"><button class="btn sm" data-others aria-expanded={others} onclick={() => (others = !others)}>{S.otherTypes(otherCount(), others)}</button></div>
  {/if}
{/if}

<style>
  .bar { display: flex; gap: 8px; align-items: flex-start; margin: 4px 0 10px; }
  .bar p { margin: 0; flex: 1 1 auto; }
  .grow { flex: 0; }
  .small { font-size: 12.5px; white-space: nowrap; }
  .bar .small { padding-top: 4px; }
  /* Match Library's segmented control. */
  .seg { display: inline-flex; border: 1px solid var(--line); border-radius: 7px; background: var(--field); flex: none; }
  .seg button { border: 0; background: none; padding: 3px 11px; font-size: 13px; position: relative; }
  .seg button:first-child { border-radius: 6px 0 0 6px; }
  .seg button:last-child { border-radius: 0 6px 6px 0; }
  .seg button:focus-visible { outline-offset: -2px; z-index: 1; border-radius: 6px; }
  .seg button + button { border-left: 1px solid var(--line); }
  .seg button[aria-pressed='true'] { background: var(--sel); color: var(--link); font-weight: 600; box-shadow: inset 0 0 0 1px var(--sel-line); }
  .ext { font-family: ui-monospace, "Cascadia Mono", Consolas, monospace; font-size: 13px; font-weight: 600; }
  .ico { width: 20px; height: 20px; border-radius: 4px; flex: none; object-fit: contain; }
  .ico.big { width: 32px; height: 32px; border-radius: 7px; }
  .letter { display: inline-grid; place-items: center; color: #fff; font-size: 11px; font-weight: 700; }
  .letter.big { font-size: 14px; }
  .blank { display: inline-block; background: var(--well); }
  .yours { font-size: 11.5px; font-weight: 600; color: var(--next-ink); background: var(--next); border-radius: 9px; padding: 0 7px; white-space: nowrap; }
  .btn.sm { padding: 2px 10px; font-size: 13px; }
  .link { border: 0; background: none; padding: 0; font-weight: 600; color: inherit; }
  /* Fixed widths, so Opens in stays put whatever the rows hold. */
  table { border-collapse: collapse; width: 100%; font-size: 13.5px; table-layout: fixed; }
  .c-type { width: 200px; }
  .c-files { width: 110px; }
  .c-act { width: 100px; }
  .ext-p { color: var(--muted); }
  .ext-p .ext { font-weight: 400; }
  th { text-align: left; font-size: 12px; color: var(--muted); font-weight: 600; padding: 6px 8px; border-bottom: 1px solid var(--line); }
  td { padding: 5px 8px; border-bottom: 1px solid var(--line); vertical-align: middle; }
  tr.group th { background: var(--side); color: var(--ink); font-size: 12.5px; padding: 5px 8px; }
  .prog { display: flex; gap: 8px; align-items: center; }
  .num { text-align: right; color: var(--muted); white-space: nowrap; }
  .act { position: relative; text-align: right; white-space: nowrap; }
  .menu { position: absolute; z-index: 5; right: 0; top: calc(100% - 2px); text-align: left; background: var(--field); border: 1px solid var(--line); border-radius: 8px; box-shadow: 0 8px 24px rgba(0,0,0,.18); padding: 4px; min-width: 260px; max-height: 320px; overflow: auto; }
  .menu button { display: flex; gap: 8px; align-items: center; width: 100%; border: 0; background: none; padding: 6px 8px; border-radius: 5px; text-align: left; white-space: nowrap; }
  .menu button:hover, .menu button:focus-visible { background: var(--hover); outline-offset: -2px; }
  .menu button[aria-checked='true'] { font-weight: 600; }
  .menu .tick { width: 12px; flex: none; color: var(--link); font-weight: 700; }
  .menu .nm { overflow: hidden; text-overflow: ellipsis; max-width: 300px; }
  .menu .def { margin-left: auto; padding-left: 10px; font-size: 11.5px; color: var(--muted); font-weight: 400; }
  .menu hr { border: 0; border-top: 1px solid var(--line); margin: 4px 0; }
  .t-wrap { position: relative; }
  .byp { display: grid; grid-template-columns: 32px 1fr auto; gap: 12px; align-items: start; padding: 12px 0; border-bottom: 1px solid var(--line); }
  .byp .name { font-weight: 600; }
  .byp.none .name { color: var(--muted); font-weight: 400; }
  .types { display: flex; flex-wrap: wrap; gap: 6px; margin-top: 6px; }
  .tchip { border: 1px solid var(--line); background: var(--bg); border-radius: 6px; padding: 1px 8px; font-size: 12.5px; display: inline-flex; gap: 6px; align-items: center; }
  .tchip:hover { border-color: var(--focus); }
  .tchip .n { color: var(--muted); }
  .tchip .note { font-size: 11.5px; font-weight: 600; }
  .gone { font-size: 11.5px; font-weight: 600; color: var(--danger); border: 1px solid currentColor; border-radius: 9px; padding: 0 7px; white-space: nowrap; }
  .tchip.y { border-color: var(--next); background: color-mix(in srgb, var(--next) 25%, var(--bg)); }
  .tchip.gone { border-color: var(--danger); color: var(--danger); }
  .types .menu { left: 0; right: auto; }
  .foot { margin-top: 12px; }
</style>
