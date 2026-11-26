<script lang="ts">
  // Settings: Folders, Sections, Marks, Programs, Storage, "What this app does", Log, Help and Licences (`decisions/2026/09/30/work-id-is-its-path.md`, `decisions/2026/10/05/checked-folders-on-any-drive.md`, `decisions/2026/10/05/nothing-hidden-no-network-by-default.md`, `decisions/2026/10/05/app-data-local-cache-movable.md`, `decisions/2026/10/05/section-kind-guessed-and-changeable.md`).
  import { onMount, tick } from 'svelte'
  import { api, relIn, size, type Catalog, type Info, type Marks, type Section, type Tab } from './api'
  import { KINDS, S, TOOL_USE } from './strings'
  import { candidates, nameOf, orphansOf, sectionOf } from './orphans'
  import FolderTree from './FolderTree.svelte'
  import LogView from './LogView.svelte'
  import Help from './Help.svelte'
  import Licences from './Licences.svelte'
  import ProgramsView from './ProgramsView.svelte'
  import { loadUi, saveUi, ui } from './ui.svelte'
  import { menuOpen, openMenu } from './menus.svelte'
  import Icon from './Icon.svelte'
  import { ask } from './ask.svelte'

  let { tab = $bindable(), catalog, marks = $bindable(), onchanged, onrethumb, onforget, onsection, onclose }: {
    tab: Tab; catalog: Catalog; marks: Marks; onforget: (s: Section) => Promise<void>
    /** Opens a Section's settings. */
    onsection: (s: Section) => void
    onchanged: (rescan: boolean) => void; onrethumb: (ids: string[]) => Promise<void>; onclose: () => void
  } = $props()

  let dialog: HTMLDialogElement
  let heading = $state<HTMLElement>()
  let picks = $state(new Map<string, 'on' | 'off'>())
  let info = $state<Info | null>(null)
  let error = $state('')
  let busy = $state(false)
  let carry = $state(true)
  /** A setting changed: "Saved" shows by the close button for a few seconds. */
  let savedSign = $state(0)
  let signTimer: ReturnType<typeof setTimeout> | undefined
  // Each change draws the sign anew, so a second one soon after fades from the start again.
  function saved() {
    clearTimeout(signTimer)
    savedSign++
    signTimer = setTimeout(() => (savedSign = 0), 3000)
  }
  /** What a button did, and where its green line shows, as the Marks line. */
  let note = $state<{ at: string; text: string } | null>(null)
  const say = (at: string, text: string) => (note = { at, text })
  // A line below the fold, as under a Section's kinds, comes into sight.
  $effect(() => {
    const at = note?.at
    if (at) tick().then(() => dialog?.querySelector(`.ok[data-at="${CSS.escape(at)}"]`)?.scrollIntoView({ block: 'nearest', behavior: 'smooth' }))
  })
  // A line belongs to the visit to its tab.
  $effect(() => { void tab; note = null })
  // A tab opened from elsewhere (F1, a link to Licences) takes the focus to its heading; one picked in
  // the list keeps it there.
  let byList = false
  $effect(() => {
    void tab
    if (byList) { byList = false; return }
    tick().then(() => heading?.focus())
  })
  let gap = $state(4)

  onMount(async () => {
    dialog.showModal()
    heading?.focus()
    const p = await api.picked()
    picks = new Map([...p.checked.map(c => [c, 'on'] as const), ...p.unchecked.map(u => [u, 'off'] as const)])
    info = await api.info()
    const st = await api.settings()
    gap = st.photo_gap ?? 4
  })

  async function run(f: () => Promise<void>) {
    error = ''; busy = true
    try { await f() } catch (e) { error = String(e) } finally { busy = false }
  }
  const saveFolders = () => run(async () => {
    const on = [...picks].filter(([, v]) => v === 'on').map(([k]) => k)
    const off = [...picks].filter(([, v]) => v === 'off').map(([k]) => k)
    await api.setFolders(on, off)
    onchanged(true); dialog.close()
  })
  function gapMenu(e: MouseEvent) {
    openMenu(e, [1, 2, 3, 4, 6, 8, 12, 24].map(h => ({ label: S.hours(h), checked: gap === h, run: () => { if (h !== gap) setGap(h) } })), S.photoGap, 'gap')
  }
  function weekMenu(e: MouseEvent) {
    openMenu(e, ([[1, S.monday], [0, S.sunday]] as const).map(([d, label]) => ({ label, checked: ui.week === d, run: () => { ui.week = d; setUi() } })), S.weekStart, 'week')
  }
  const setGap = (h: number) => run(async () => {
    gap = (await api.setPhotoGap(h)).photo_gap
    onchanged(true)
    saved()
  })
  const moveCache = (to: string | null) => run(async () => {
    note = null
    const at = await api.setCacheDir(to, carry)
    info = await api.info()
    onchanged(false)
    say('cache', S.cacheMoved(at, carry))
  })
  const pickCache = () => run(async () => {
    note = null
    const p = await api.pickFolder()
    if (p) { const at = await api.setCacheDir(p, carry); info = await api.info(); onchanged(false); say('cache', S.cacheMoved(at, carry)) }
  })
  const rethumbAll = () => run(async () => {
    if (!(await ask(S.rethumbAllSure, S.rethumbAllGo))) return
    note = null
    await onrethumb(catalog.sections.flatMap(s => s.works.map(w => w.id))); info = await api.info()
    say('thumbs', S.rethumbAllSaved)
  })
  // Saved shows once the setting is kept; a failure shows as the error instead.
  const setUi = () => run(async () => { await saveUi(); saved() })
  const orphans = $derived(orphansOf(catalog, marks))
  const live = $derived(new Set(catalog.sections.flatMap(s => s.works.map(w => w.id))))
  const away = $derived(Object.keys(marks.marks).filter(id => !live.has(id) && sectionOf(catalog, id)?.online === false).length)
  const counts = $derived({
    next: Object.entries(marks.marks).filter(([id, m]) => live.has(id) && m.m === 'next').length,
    done: Object.entries(marks.marks).filter(([id, m]) => live.has(id) && m.m === 'done').length,
    ticks: Object.entries(marks.ticks).filter(([id]) => live.has(id)).reduce((n, [, t]) => n + Object.keys(t).length, 0) - orphans.items.length,
  })
  let finds = $state<Record<string, string>>({})
  /** A Work id as a person reads it: the Section's name and the path inside it. */
  function shown(id: string) {
    const s = sectionOf(catalog, id)
    return s ? `${s.name}/${relIn(id, s.id)}` : id.slice(id.indexOf(':') + 1)
  }
  function what(id: string) {
    const m = marks.marks[id]?.m, n = Object.keys(marks.ticks[id] ?? {}).length
    return [m === 'next' ? S.next : m === 'done' ? S.done : '', n ? S.nTicks(n) : ''].filter(Boolean).join(', ')
  }
  const titleOf = (id: string) => catalog.sections.flatMap(s => s.works).find(w => w.id === id)?.title ?? nameOf(id)
  const move = (from: string, to: string) => run(async () => {
    note = null
    marks = await api.moveMark(from, to)
    say('marks', S.markMoved(shown(to)))
  })
  const forget = () => run(async () => {
    const n = orphans.gone.length + orphans.items.length
    if (!(await ask(S.forgetSure(n), S.forget, true))) return
    note = null
    marks = await api.forgetMarks(orphans.gone, orphans.items)
    say('marks', S.forgotten(n))
  })
  const held = (m: Marks) =>
    S.marksHeld(Object.keys(m.marks).length, Object.values(m.ticks).reduce((n, t) => n + Object.keys(t).length, 0), Object.keys(m.review ?? {}).length)
  const exportMarks = () => run(async () => {
    note = null
    const what = held(marks)
    const f = await api.exportMarks()
    if (f) say('marks', S.backupSaved(what, f.file, f.folder))
  })
  const importMarks = () => run(async () => {
    if (!(await ask(S.importSure, S.importGo))) return
    note = null
    const r = await api.importMarks()
    if (!r) return
    marks = r.marks
    say('marks', S.imported(r.added, r.ticks, r.ui, r.file))
    // A backup saved before the backup reminder was put away does not bring it back.
    const seen = ui.backupSeen
    if (r.ui) loadUi((await api.settings()).ui)
    if (seen && !ui.backupSeen) { ui.backupSeen = true; saveUi() }
    onchanged(false)
  })
  const clearMarks = () => run(async () => {
    if (!(await ask(S.clearMarksSure, S.clearMarks, true))) return
    note = null
    const what = held(marks)
    marks = await api.clearMarks()
    say('marks', S.marksCleared(what))
  })
</script>

{#snippet ok(at: string)}
  <p class="ok" role="status" data-at={at}>{#if note?.at === at}✓ <span dir="auto">{note.text}</span>{/if}</p>
{/snippet}

<dialog bind:this={dialog} aria-labelledby="settings-title" onclose={onclose}>
  <div class="frame">
    <nav>
      <h2 id="settings-title">{S.settings}</h2>
      <button class:on={tab === 'folders'} aria-current={tab === 'folders' ? 'page' : undefined} onclick={() => { if (tab !== 'folders') { byList = true; tab = 'folders' } }}>{S.tabFolders}</button>
      <button class:on={tab === 'sections'} aria-current={tab === 'sections' ? 'page' : undefined} onclick={() => { if (tab !== 'sections') { byList = true; tab = 'sections' } }}>{S.tabSections}</button>
      <button class:on={tab === 'marks'} aria-current={tab === 'marks' ? 'page' : undefined} onclick={() => { if (tab !== 'marks') { byList = true; tab = 'marks' } }}>{S.tabMarks}{#if orphans.gone.length + orphans.items.length}<small class="warn">{orphans.gone.length + orphans.items.length}</small>{/if}</button>
      <button class:on={tab === 'programs'} aria-current={tab === 'programs' ? 'page' : undefined} onclick={() => { if (tab !== 'programs') { byList = true; tab = 'programs' } }}>{S.tabPrograms}</button>
      <button class:on={tab === 'storage'} aria-current={tab === 'storage' ? 'page' : undefined} onclick={() => { if (tab !== 'storage') { byList = true; tab = 'storage' } }}>{S.tabStorage}</button>
      <button class:on={tab === 'about'} aria-current={tab === 'about' ? 'page' : undefined} onclick={() => { if (tab !== 'about') { byList = true; tab = 'about' } }}>{S.tabAbout}</button>
      <button class:on={tab === 'log'} aria-current={tab === 'log' ? 'page' : undefined} onclick={() => { if (tab !== 'log') { byList = true; tab = 'log' } }}>{S.tabLog}</button>
      <button class:on={tab === 'help'} aria-current={tab === 'help' ? 'page' : undefined} onclick={() => { if (tab !== 'help') { byList = true; tab = 'help' } }}>{S.tabHelp}</button>
      <button class:on={tab === 'licences'} aria-current={tab === 'licences' ? 'page' : undefined} onclick={() => { if (tab !== 'licences') { byList = true; tab = 'licences' } }}>{S.tabLicences}</button>
      <span class="grow"></span>
      {#if info}<p class="ver muted">{S.version(info.version, info.build)}</p>{/if}
    </nav>
    <section>
      {#if tab === 'folders'}
        <h3 tabindex="-1" bind:this={heading}>{S.tabFolders}</h3>
        <p class="muted">{S.welcomeWhat}</p>
        <FolderTree bind:picks />
        {#if catalog.sections.some(s => !s.online)}
          <h4>{S.awayTitle}</h4>
          <p class="muted">{S.awayWhat}</p>
          {#each catalog.sections.filter(s => !s.online) as s (s.id)}
            <div class="place"><div><b dir="auto">{s.name}</b><span class="muted" dir="auto">{s.path}, {s.gone ? S.notFound : S.notConnected}</span></div><span></span>
              <button class="btn" disabled={busy} onclick={() => run(() => onforget(s))}>{S.removeFolder}</button></div>
          {/each}
        {/if}
        <div class="foot"><button class="btn primary" disabled={busy} onclick={saveFolders}>{S.saveAndRead}</button></div>
      {:else if tab === 'sections'}
        <h3 tabindex="-1" bind:this={heading}>{S.tabSections}</h3>
        <fieldset class="choices">
          <legend>{S.unlistedTitle}</legend>
          <label><input type="checkbox" checked={ui.unlisted} onchange={e => { ui.unlisted = e.currentTarget.checked; setUi() }} />
            <span>{S.showUnlisted}<small class="muted">{S.showUnlistedWhat}</small></span></label>
        </fieldset>
        {#if catalog.sections.some(s => s.kind === 'Photos' || s.works.some(w => w.event))}
          <fieldset class="choices">
            <legend>{S.photoGapTitle}</legend>
            <!-- The app's own dropdown, as Section settings' Show inside: a button that opens the one menu. -->
            <div class="pick"><span id="set-gap">{S.photoGap}</span>
              <button class="drop" aria-labelledby="set-gap set-gap-v" aria-haspopup="menu" aria-expanded={menuOpen('gap')} aria-disabled={busy} onclick={e => { if (!busy) gapMenu(e) }}>
                <span id="set-gap-v">{S.hours(gap)}</span><Icon name="down" size={12} /></button></div>
            <small class="muted">{S.photoGapWhat}</small>
            <div class="pick"><span id="set-week">{S.weekStart}</span>
              <button class="drop" aria-labelledby="set-week set-week-v" aria-haspopup="menu" aria-expanded={menuOpen('week')} onclick={weekMenu}>
                <span id="set-week-v">{ui.week === 0 ? S.sunday : S.monday}</span><Icon name="down" size={12} /></button></div>
            <small class="muted">{S.weekStartWhat}</small>
          </fieldset>
        {/if}
        <p class="muted">{S.sectionsWhat}</p>
        {#each catalog.sections as s (s.id)}
          {@const off = !s.online && !s.gone}
          <div class="place sec" class:off>
            <div><b dir="auto">{s.name}</b><span class="muted" dir="auto">{s.path}</span>{#if off}<span class="away">{S.driveAway(s.drive)}</span>{/if}
              <span class="muted">{S.kindShown(KINDS[s.kind].name, s.guessed)}{s.split_from ? ` · ${S.splitOutOf(catalog.sections.find(t => t.id === s.split_from)?.name ?? '')}` : ''}{ui.merge[s.id] ? ` · ${S.shownInside(catalog.sections.find(t => t.id === ui.merge[s.id])?.name ?? '')}` : ''}</span></div>
            <span></span>
            <button class="btn" aria-label={S.sectionSettingsFor(s.name)} disabled={!s.scanned_at} onclick={() => onsection(s)}>{S.sectionSettingsBtn}</button>
          </div>
        {:else}
          <p>{S.noSections}</p>
        {/each}
      {:else if tab === 'marks'}
        <h3 tabindex="-1" bind:this={heading}>{S.tabMarks}</h3>
        <p class="muted">{S.marksWhat(counts.next, counts.done, counts.ticks)}{away ? ` ${S.marksAway(away)}` : ''}</p>
        <div class="foot left">
          <button class="btn" disabled={busy} onclick={exportMarks}>{S.exportMarks}</button>
          <button class="btn" disabled={busy} onclick={importMarks}>{S.importMarks}</button>
          <button class="btn danger" disabled={busy} onclick={clearMarks}>{S.clearMarks}</button>
        </div>
        {@render ok('marks')}
        <p class="muted small0">{S.backupWhat}</p>
        {#if orphans.gone.length + orphans.items.length}
          <h4>{S.orphaned(orphans.gone.length + orphans.items.length)}</h4>
          <p class="muted">{S.orphanedWhat}</p>
          {#each orphans.gone as id (id)}
            {@const same = candidates(catalog, marks, id)}
            {@const found = finds[id] ? candidates(catalog, marks, id, finds[id]) : []}
            <div class="orphan">
              <div><b>{shown(id)}</b> <span class="muted">({what(id)})</span></div>
              {#each [...same, ...found.filter(w => !same.includes(w))] as w (w.id)}
                <div class="move"><span>{shown(w.id)}</span><button class="btn" disabled={busy} onclick={() => move(id, w.id)}>{S.moveHere}</button></div>
              {/each}
              {#if !sectionOf(catalog, id)}<span class="muted small">{S.sectionGone}</span>{/if}
              <input class="find" placeholder={S.findWork} aria-label={S.findWorkFor(shown(id))} bind:value={finds[id]} />
              {#if finds[id]?.trim() && !found.length}<span class="muted small">{S.noResults(finds[id].trim())}</span>{/if}
            </div>
          {/each}
          {#each orphans.items as [id, k] (id + '|' + k)}
            <div class="orphan"><b>{titleOf(id)}: {k}</b> <span class="muted">({S.tickedItem})</span></div>
          {/each}
          <div class="foot left"><button class="btn danger" disabled={busy} onclick={forget}>{S.forget}</button></div>
        {:else}
          <p>{S.noOrphans}</p>
        {/if}
      {:else if tab === 'programs'}
        <h3 tabindex="-1" bind:this={heading}>{S.tabPrograms}</h3>
        <ProgramsView onerror={e => (error = e)} onsaved={saved} />
      {:else if tab === 'storage'}
        <h3 tabindex="-1" bind:this={heading}>{S.tabStorage}</h3>
        <p class="muted">{S.storageWhat}</p>
        {#if info}
          <div class="where"><span>{info.cache_dir}</span><button class="btn" onclick={() => api.reveal(info!.cache_dir, true)}>{S.openFolder}</button></div>
          <label class="carry"><input type="checkbox" bind:checked={carry} /> {S.carry}</label>
          <div class="foot left">
            <button class="btn" disabled={busy} onclick={pickCache}>{S.chooseFolder}</button>
            {#if !info.cache_default}<button class="btn" disabled={busy} onclick={() => moveCache(null)}>{S.defaultPlace}</button>{/if}
          </div>
          {@render ok('cache')}
          <p class="muted small">{S.defaultWhere}</p>
        {/if}
        <h4>{S.rethumbAllTitle}</h4>
        <p class="muted">{S.rethumbAllWhat}</p>
        <div class="foot left"><button class="btn" disabled={busy} onclick={rethumbAll}>{S.rethumbAll}</button></div>
        {@render ok('thumbs')}
      {:else if tab === 'about'}
        <h3 tabindex="-1" bind:this={heading}>{S.tabAbout}</h3>
        <p class="muted">{S.aboutWhat}</p>
        <h4>{S.stores}</h4>
        {#if info}
          {#each info.places as p (p.path)}
            <div class="place"><div><b>{p.what}</b><span class="muted">{p.path}</span></div><span class="muted">{size(p.size)}</span>
              <button class="btn" onclick={() => api.reveal(p.path, !p.path.endsWith('.json'))}>{S.openFolder}</button></div>
          {/each}
        {/if}
        <h4>{S.readsTitle}</h4>
        <p>{S.reads}</p>
        <h4>{S.runsTitle}</h4>
        <p>{S.runs}</p>
        {#if info}
          {#each info.tools as t (t.id)}
            <div class="place"><div><b>{t.name} {t.version}</b><span class="muted">{TOOL_USE[t.id] ?? ''} {S.licence(t.licence)}. <button class="link" onclick={() => (tab = 'licences')}>{S.licenceAndSource}</button></span></div>
              <span class="muted" class:err={!t.found}>{t.found ? '' : S.toolMissing}</span>
              <button class="btn" onclick={() => api.reveal(info!.tools_dir, true)}>{S.openFolder}</button></div>
          {/each}
        {/if}
        <h4>{S.networkTitle}</h4>
        <p>{S.networkWhat}</p>
        <ul class="hosts">
          <li><b>{S.openLibrary}</b><span class="muted">{S.openLibraryWhat}</span></li>
          <li><b>{S.fantLab}</b><span class="muted">{S.fantLabWhat}</span></li>
        </ul>
        <p class="muted">{S.networkLog}</p>
        <p>{S.webview}</p>
        <h4>{S.logTitle}</h4>
        <p class="muted">{S.logWhat}</p>
        <div class="foot left"><button class="btn" onclick={() => (tab = 'log')}>{S.seeLog}</button></div>
      {:else if tab === 'log'}
        <h3 tabindex="-1" bind:this={heading}>{S.tabLog}</h3>
        <LogView onerror={e => (error = e)} />
      {:else if tab === 'help'}
        <h3 tabindex="-1" bind:this={heading}>{S.tabHelp}</h3>
        <Help />
      {:else if tab === 'licences'}
        <h3 tabindex="-1" bind:this={heading}>{S.tabLicences}</h3>
        <Licences onerror={e => (error = e)} />
      {/if}
      {#if error}<p class="err">{error}</p>{/if}
    </section>
  </div>
  <span class="saved" role="status">{#key savedSign}{#if savedSign}<span class="sign">✓ {S.savedSign}</span>{/if}{/key}</span>
  <button class="x" aria-label={S.close} onclick={() => dialog.close()}><svg viewBox="0 0 10 10" aria-hidden="true"><path d="M1.5 1.5l7 7M8.5 1.5l-7 7" /></svg></button>
</dialog>

<style>
  dialog { border: 0; border-radius: 12px; padding: 0; width: min(980px, 94vw); height: min(720px, 90vh); background: var(--bg); color: var(--ink); box-shadow: 0 20px 60px rgba(0,0,0,.35); }
  dialog::backdrop { background: rgba(10, 14, 20, .45); }
  .frame { display: grid; grid-template-columns: 220px 1fr; height: 100%; }
  nav { background: var(--side); padding: 18px 10px; display: flex; flex-direction: column; gap: 2px; }
  nav h2 { font-size: 18px; margin: 0 10px 12px; }
  nav button { border: 0; background: none; text-align: left; padding: 7px 10px; border-radius: 6px; position: relative; }
  nav button:hover { background: var(--hover); }
  nav button.on { background: var(--field); font-weight: 600; }
  nav button.on::before { content: ""; position: absolute; left: 0; top: 50%; width: 3px; height: 16px; margin-top: -8px; border-radius: 2px; background: var(--focus); }
  h3:focus { outline: none; }
  .grow { flex: 1; }
  .ver { font-size: 12px; margin: 0 10px; }
  section { padding: 22px 28px; overflow: auto; background: var(--pane); display: flex; flex-direction: column; min-height: 0; }
  h3 { margin: 0 0 6px; font-size: 20px; }
  h4 { margin: 18px 0 4px; font-size: 14px; }
  section p { margin: 0 0 10px; max-width: 70ch; }
  .small { font-size: 12.5px; }
  .hosts { list-style: none; margin: 4px 0 8px; padding: 0; }
  .hosts li { display: flex; flex-direction: column; margin-bottom: 8px; }
  .hosts .muted { font-size: 12.5px; }
  .foot { display: flex; justify-content: flex-end; gap: 8px; margin-top: 12px; }
  .foot.left { justify-content: flex-start; }
  .where { display: flex; gap: 12px; align-items: center; background: var(--side); border-radius: 8px; padding: 10px 12px; overflow-wrap: anywhere; }
  .where span { flex: 1; }
  .carry { display: flex; gap: 8px; align-items: center; margin-top: 12px; }
  .place { display: grid; grid-template-columns: 1fr auto auto; gap: 14px; align-items: center; padding: 8px 0; border-bottom: 1px solid var(--line); }
  .place div { display: flex; flex-direction: column; min-width: 0; }
  .place div span { font-size: 12.5px; overflow-wrap: anywhere; }
  .place b { overflow-wrap: anywhere; }
  /* A Section whose drive is not connected: greyed as a button that is off, but the line that
     says so; its Settings… still opens. */
  .place.off > div > :not(.away) { opacity: .45; }
  .err { color: var(--danger); }
  .choices { border: 1px solid var(--line); border-radius: 10px; padding: 10px 14px 12px; margin: 6px 0 16px; }
  .choices legend { font-weight: 600; padding: 0 4px; }
  .choices label { display: flex; gap: 10px; align-items: flex-start; padding: 5px 0; }
  .choices input { margin-top: 3px; }
  .choices small { display: block; font-size: 12.5px; }
  .pick { display: flex; gap: 10px; align-items: center; padding: 5px 0; }
  /* As Section settings' Show inside button. */
  .drop { display: inline-flex; align-items: center; gap: 6px; border: 1px solid var(--line); background: var(--field); border-radius: 6px; padding: 4px 8px 4px 10px;
    font-size: 13px; max-width: 260px; min-width: 0; }
  .drop span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .drop:hover { background: var(--hover); }
  /* Kept focusable while the folders are read again, so the focus stays on it after a pick. */
  .drop[aria-disabled='true'] { opacity: .6; }
  .small0 { font-size: 12.5px; }
  .ok { color: var(--done); font-weight: 600; font-size: 13.5px; overflow-wrap: anywhere; }
  .ok:empty { margin: 0; }
  nav small.warn { margin-left: 8px; background: var(--next); color: var(--next-ink); border-radius: 9px; padding: 0 7px; font-size: 11.5px; font-weight: 600; }
  .orphan { padding: 10px 0; border-bottom: 1px solid var(--line); display: flex; flex-direction: column; gap: 6px; overflow-wrap: anywhere; }
  .move { display: flex; gap: 12px; align-items: center; padding-left: 16px; }
  .move span { flex: 1; }
  .move .btn { padding: 2px 10px; font-size: 13px; }
  .find { margin-left: 16px; max-width: 360px; border: 1px solid var(--line); border-radius: 6px; padding: 4px 8px; background: var(--field); }
  .small { font-size: 12.5px; padding-left: 16px; }
  .btn.danger { color: var(--danger); }
  .link { border: 0; background: none; padding: 0; font: inherit; color: var(--link); text-decoration: underline; cursor: pointer; }
  .x { position: absolute; z-index: 2; top: 12px; right: 12px; width: 32px; height: 32px; border: 0; background: var(--pane); border-radius: 6px; display: grid; place-items: center; }
  .x svg { width: 11px; height: 11px; fill: none; stroke: currentColor; stroke-width: 1.2; }
  .x:hover { background: var(--hover); }
  .saved { position: absolute; top: 18px; right: 54px; color: var(--done); font-weight: 600; font-size: 13.5px; }
  .sign { display: inline-block; background: var(--pane); border-radius: 6px; padding: 1px 8px; animation: fade 3s forwards; }
  @keyframes fade { 80% { opacity: 1 } 100% { opacity: 0 } }
</style>
