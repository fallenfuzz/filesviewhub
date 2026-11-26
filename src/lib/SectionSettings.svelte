<script lang="ts" module>
  import type { Kind } from './api'
  /** What the person chose in Section settings: the name, the kind (null: the app guesses), the Section it shows inside ('' its own), and in a Photos Section the kinds changed of its top-level folders, by path (null: Same as the Section). */
  export interface SectionChange { name: string; kind: Kind | null; merge: string; folders: Record<string, Kind | null>; renamed: boolean; rekind: boolean; remerge: boolean }
  /** What the person chose in Folder settings (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`): its own kind (null: Same as the folder above). */
  export interface FolderChange { kind: Kind | null; rekind: boolean }
</script>

<script lang="ts">
  // A Section's settings, from its menu or Settings, Sections: its name and path, its
  // kinds, and on the right how the kind picked looks at each level, as made-up samples in the
  // views in use; the View menu sets the views (`decisions/2026/10/21/the-view-menu-sets-each-folders-view.md`). A change of kind is counted by the core's read-only dry
  // run (`preview_kind`), stopped when the dialog closes. The primary button applies every
  // change, and Cancel, the X and Esc change nothing.
  // With `folder` it is Folder settings (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`), for a folder inside the Section: no name
  // field, Same as first in the kinds, the levels Inside a folder and One Work, and the dry run of that folder (`preview_folder`).
  // A Photos Section shows no folders (`decisions/2026/10/08/photos-is-a-section-kind-in-the-app.md`), so its Section settings has a tab more, Folders
  // with other things: each top-level folder with a kind of its own, or Make it a Section (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`).
  import { onMount, tick, untrack } from 'svelte'
  import { api, COLORS, natural, place, type Catalog, type KindPreview, type Section } from './api'
  import { KINDS, S } from './strings'
  import { ui } from './ui.svelte'
  import { LEVELS, viewIn, type Level } from './views'
  import { openMenu, menuOpen } from './menus.svelte'
  import { kindFrom, offerable, ownOf, topFoldersOf, viewFrom, yearLike } from './folders.svelte'
  import Icon from './Icon.svelte'
  import Sample from './Sample.svelte'

  let { section, shown = section, catalog, folder, reading, onapply, onfolder, onsplit, onsplittop, onsuggest, onclose }: {
    section: Section; catalog: Catalog
    /** In Folder settings, the Section shown, whose views a folder with none of its own takes: another when `section` shows inside it. */
    shown?: Section
    /** Folder settings for this folder of the Section: its titles, its path in the Section, and the
     * same folder in the Sections shown inside the one shown (`also`), counted with it. */
    folder?: { f: string[]; rel: string; also?: { id: string; rel: string }[] }
    /** Folders are being read now: nothing can be applied until that ends. */
    reading: boolean
    onapply?: (c: SectionChange) => void; onfolder?: (c: FolderChange) => void
    /** Folder settings' Make it a Section (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`), with the kind and the view at each level shown. */
    onsplit?: (kind: Kind, views: Record<string, string>) => void
    /** Make it a Section for a Photos Section's top-level folder, with its kind and views. */
    onsplittop?: (rel: string, kind: Kind, views: Record<string, string>) => void
    /** Suggest kinds for its folders (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`): the offer is made again, at the top of the Section. */
    onsuggest?: () => void
    onclose: () => void
  } = $props()

  // svelte-ignore state_referenced_locally
  const fd = folder
  const kinds = Object.keys(KINDS) as Kind[]
  /** The kinds to pick, Same as (null) first in Folder settings. */
  const choices: (Kind | null)[] = fd ? [null, ...kinds] : kinds
  let dialog: HTMLDialogElement
  let list = $state<HTMLElement>()
  let stage = $state<HTMLElement>()
  /** What had focus before: it gets it back when the dialog closes. */
  const from = document.activeElement as HTMLElement | null
  // The dialog keeps the Section as it was when opened; a read while it is open does not move it.
  // svelte-ignore state_referenced_locally
  const s = section
  /** Whether its drive is connected now: a drive unplugged while the dialog is open turns Make it a
   * Section off at once (`drivesAway` in App.svelte gives a new catalog). */
  const online = $derived(catalog.sections.find(x => x.id === s.id)?.online ?? s.online)
  /** A folder's own kind and views when the dialog opened, and what it takes from above. */
  const own = fd ? ownOf(s, fd.rel) : null
  const above = fd ? kindFrom(s, fd.rel, fd.f, false) : null
  const fname = fd ? place(fd.f.at(-1) ?? '') : ''
  const pathOf = (home: Section, rel: string) => [home.path.replace(/[\\/]+$/, ''), ...rel.split('/')].join(home.path.includes('\\') ? '\\' : '/')
  const fpath = fd ? pathOf(s, fd.rel) : ''
  /** Every folder on disk the setting goes to, for the path's hover: other drives, or another folder with this title. */
  const fpaths = fd ? [fpath, ...(fd.also ?? []).flatMap(p => { const h = catalog.sections.find(x => x.id === p.id); return h ? [pathOf(h, p.rel)] : [] })].join('\n') : ''
  const was: Kind | null = own ? own.kind : s.guessed ? null : s.kind
  /** The views picked, by the View menu: a folder's own, or the Section's. */
  const had = own ? own.views : { ...(ui.views[s.id] ?? {}) }
  let name = $state(s.name)
  let pick = $state<Kind | null>(was)
  let merge = $state(ui.merge[s.id] ?? '')
  let previews = $state<Record<string, KindPreview>>({})
  let error = $state('')
  const key = (k: Kind | null) => k ?? ''
  /** The kind the tabs show: the one picked, the app's guess, or in Folder settings the one above. */
  const showing = $derived<Kind>(pick ?? (above ? above.kind : previews['']?.kind ?? s.kind))
  /** A folder's levels: Inside a folder and One Work, never the Section's own. */
  const levelsOf = (k: Kind) => LEVELS[k].filter(l => !fd || l.id !== 'section')
  const levels = $derived(levelsOf(showing))
  /** In a Photos Section, Folders with other things is the tab open instead of the samples. */
  let onOthers = $state(false)
  /** A Photos Section's top-level folders (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`), each with a kind of its own or Make it a
   * Section; Photos is the Section's own kind, so a folder with it is Same as. A folder split out
   * is listed as a Section of its own. */
  const tops = !fd && s.kind === 'Photos' ? topFoldersOf(s).sort((a, b) => natural(a.name, b.name)) : []
  const linkOf = (n: string) => (s.links ?? []).find(l => l.rel.toLowerCase() === n.toLowerCase())
  const ownKind = (n: string): Kind | null => { const k = ownOf(s, n).kind; return k === 'Photos' ? null : k }
  /** The kind picked for each folder while the dialog is open. */
  let fpicks = $state<Record<string, Kind | null>>({})
  const fkind = (n: string) => (n in fpicks ? fpicks[n] : ownKind(n))
  /** Its tab shows while the kind shown is Photos. */
  const othersShown = $derived(tops.length > 0 && showing === 'Photos')
  const othersOpen = $derived(othersShown && onOthers)
  const withKind = $derived(tops.filter(t => !linkOf(t.name) && fkind(t.name)).length)
  /** The folders whose kind changes, by path. */
  const fchanged = $derived<Record<string, Kind | null>>(othersShown ? Object.fromEntries(Object.entries(fpicks).filter(([n, k]) => k !== ownKind(n))) : {})
  const refold = $derived(Object.keys(fchanged).length > 0)
  const renamed = $derived(!fd && name.trim() !== s.name)
  const rekind = $derived(pick !== was)
  const remerge = $derived(!fd && merge !== (ui.merge[s.id] ?? ''))
  /** A folder's own pick at a level, '' for Same as: one its kind does not offer is none. */
  const ownIn = (p: Record<string, string>, l: Level) => (p[l.id] && l.views.includes(p[l.id]) ? p[l.id] : '')
  /** What a level of the folder shows when it has no pick of its own. */
  // svelte-ignore state_referenced_locally
  const base = shown
  const inherited = (l: Level) => fd ? viewFrom(s, fd.rel, l.id, fd.f, false, showing, base) : { view: l.views[0], from: s.name, own: false, key: '' }
  /** The view a level shows now. */
  const shownAt = (l: Level) => fd ? ownIn(had, l) || inherited(l).view : viewIn(had, l)
  const counted = $derived(previews[key(pick)])
  // The folders' new kinds are counted together, once for each set of changes: their cards and the Marks set aside.
  const fkey = $derived(JSON.stringify(fchanged))
  let fcounted = $state<{ key: string; p: KindPreview } | null>(null)
  const fcount = $derived(fcounted?.key === fkey ? fcounted.p : undefined)
  // Picked a moment after the last pick, so a few picks in a row start one dry run of the Section.
  $effect(() => {
    const k = fkey, [first, ...rest] = Object.entries(fchanged)
    error = ''
    if (!first) return
    const t = setTimeout(() => api.previewFolder(s.id, first[0], first[1], Object.fromEntries(rest))
      .then(p => { fcounted = { key: k, p } }).catch(e => { if (fkey === k && dialog.open) error = String(e) }), 300)
    return () => clearTimeout(t)
  })
  /** A kind or name change waits for a read running now; a Show inside change does not. */
  const waits = $derived(reading && (rekind || renamed || refold))
  /** Something changed, a kind change is counted, and nothing waits for a read. */
  const can = $derived((renamed || rekind || remerge || refold) && !(rekind && !counted) && !(refold && !fcount) && !waits)
  /** Make it a Section leaves the dialog: it waits until the other changes are used or cancelled. */
  const dirty = $derived(renamed || rekind || remerge || refold)
  const color = $derived(COLORS[Math.max(0, catalog.sections.findIndex(x => x.id === s.id)) % COLORS.length])

  // A kind other than the Section's is counted once while the dialog is open: its cards and the
  // Marks set aside. The guess is counted too when the Section's kind is chosen, to tell whether
  // picking the guess changes anything.
  const asked = new Set<string>()
  function count(k: Kind | null) {
    // Read untracked: a preview arriving does not make the effect ask again.
    if (untrack(() => previews[key(k)]) || asked.has(key(k))) return
    asked.add(key(k))
    ;(fd ? previewAll(k) : api.previewKind(s.id, k, 0)).then(p => { previews[key(k)] = p }).catch(e => { asked.delete(key(k)); if (pick === k && dialog.open) error = String(e) })
  }
  /** A folder's dry run on each drive that holds it, added up. */
  async function previewAll(k: Kind | null): Promise<KindPreview> {
    const all = await Promise.all([{ id: s.id, rel: fd!.rel }, ...(fd!.also ?? [])].map(p => api.previewFolder(p.id, p.rel, k)))
    return { ...all[0], total: all.reduce((n, p) => n + p.total, 0), marks: all.reduce((n, p) => n + p.marks, 0), back: all.reduce((n, p) => n + p.back, 0), exact: all.every(p => p.exact) }
  }
  $effect(() => {
    error = ''
    if (pick !== was) count(pick)
    if (!fd && was === null && pick !== null) count(null)
  })

  onMount(() => {
    dialog.showModal()
    tick().then(() => list?.querySelector<HTMLElement>('[aria-checked="true"]')?.focus())
  })
  function close() { if (dialog.open) dialog.close() }
  function closed() {
    api.stopPreview().catch(() => {})
    onclose()
    // Opened from a menu that is gone: the page takes focus, not whatever the browser picks.
    if (from?.isConnected && from !== document.body) from.focus()
    else document.getElementById('works')?.focus()
  }
  function apply() {
    if (fd) onfolder?.({ kind: pick, rekind })
    else onapply?.({ name: name.trim(), kind: pick, merge, folders: fchanged, renamed, rekind, remerge })
    close()
  }
  /** A Photos Section's top-level folder as a Section of its own, with its kind and views. */
  function splitTop(n: string) {
    onsplittop?.(n, fkind(n) ?? 'Photos', ownOf(s, n).views)
    close()
  }
  function kindMenu(e: MouseEvent, n: string) {
    const k = fkind(n), set = (to: Kind | null) => () => { fpicks[n] = to }
    openMenu(e, [
      { label: S.sameAs(s.name), hint: KINDS.Photos.name, checked: k === null, run: set(null) },
      { sep: true },
      ...kinds.filter(x => x !== 'Photos').map(x => ({ label: KINDS[x].name, checked: k === x, run: set(x) })),
    ], S.kindOfFolder(n), 'fk:' + n)
  }
  const colorOf = (id: string) => COLORS[Math.max(0, catalog.sections.findIndex(x => x.id === id)) % COLORS.length]
  /** Makes the folder a Section of its own with the kind and views shown now. */
  function split() {
    onsplit?.(showing, Object.fromEntries(levels.map(l => [l.id, shownAt(l)])))
    close()
  }
  /** Said when no folder looks like another kind, so nothing would be offered. */
  let noOffer = $state(false)
  /** Suggest kinds for its folders: the dialog closes and the Section shows the offer, when there is one. */
  async function suggest() {
    try {
      if (!offerable(catalog, s, await api.suggestKinds(s.id, true)).length) { noOffer = true; return }
      onsuggest?.()
      close()
    } catch (e) { error = String(e) }
  }
  function choose(k: Kind | null) { pick = k }
  const focusPicked = () => tick().then(() => list?.querySelector<HTMLElement>('[aria-checked="true"]')?.focus())
  /** The link goes away with the guess shown, so focus moves to the kind the app guesses. */
  function guessAgain() { pick = null; focusPicked() }
  /** The kind picked is the one the app guesses now: the cards stay, only the guessing stops. */
  const pinning = $derived(!fd && was === null && pick !== null && previews['']?.kind === pick)
  /** The kind shown as picked: in Folder settings Same as is a choice of its own. */
  const picked = (k: Kind | null) => (fd ? pick === k : showing === k)
  /** Up and Down move through the kinds and pick, as a set of radio buttons does. */
  function keys(e: KeyboardEvent) {
    const i = choices.findIndex(picked), n = choices.length
    const to = e.key === 'ArrowDown' || e.key === 'ArrowRight' ? i + 1 : e.key === 'ArrowUp' || e.key === 'ArrowLeft' ? i - 1 : e.key === 'Home' ? 0 : e.key === 'End' ? n - 1 : null
    if (to === null) return
    e.preventDefault()
    choose(choices[(to + n) % n])
    focusPicked()
  }
  /** Left and Right move between the two tabs, as Windows' tabs do. */
  function tabKeys(e: KeyboardEvent) {
    const to = e.key === 'ArrowRight' || e.key === 'End' ? true : e.key === 'ArrowLeft' || e.key === 'Home' ? false : null
    if (to === null) return
    e.preventDefault()
    onOthers = e.key === 'ArrowRight' || e.key === 'ArrowLeft' ? !onOthers : to
    tick().then(() => { const t = stage?.querySelector<HTMLElement>('[role="tab"][aria-selected="true"]'); t?.focus(); t?.scrollIntoView({ block: 'nearest', inline: 'nearest' }) })
  }
  /** The Sections a Split-out Section came from, nearest first: it cannot show inside one, where its folder is (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`). */
  const chain = (id?: string): string[] => { const f = catalog.sections.find(t => t.id === id); return f ? [f.id, ...chain(f.split_from)] : [] }
  const cameFrom = new Set(chain(s.split_from))
  const others = $derived(catalog.sections.filter(t => t.id !== s.id && !ui.merge[t.id] && !cameFrom.has(t.id)))
  /** A Section with others shown inside it stays its own, so no Section is two steps inside another. */
  const holds = $derived(catalog.sections.some(t => ui.merge[t.id] === s.id))
  const insideName = $derived(merge ? catalog.sections.find(t => t.id === merge)?.name ?? '' : '')
  function insideMenu(e: MouseEvent) {
    openMenu(e, [
      { label: S.ownSection, checked: !merge, run: () => (merge = '') },
      ...others.map(t => ({ label: t.name, hint: t.drive, checked: merge === t.id, run: () => (merge = t.id) })),
    ], S.showInside, 'inside')
  }
</script>

<dialog bind:this={dialog} aria-labelledby="ss-title" data-no-undo onclose={closed}>
  <header>
    <span class="dot" style="--c:{color}"></span>
    <div class="nm">
      {#if fd}
        <h2 id="ss-title" class="sr">{S.folderSettingsOf(fname, s.name)}</h2>
        <p class="fname" dir="auto" title={fname} aria-hidden="true">{fname}</p>
        <small dir="auto" title={fpaths}>{fpath}</small>
      {:else}
        <h2 id="ss-title" class="sr">{S.sectionSettingsOf(s.name)}</h2>
        <input bind:value={name} aria-label={S.sectionName} spellcheck="false" dir="auto" onkeydown={e => { if (e.key === 'Enter' && can) { e.preventDefault(); apply() } }} />
        <small dir="auto" title={s.path}>{s.path}</small>
      {/if}
    </div>
    <button class="x" aria-label={S.close} onclick={close}><Icon name="clear" size={14} /></button>
  </header>
  <div class="split">
    <div class="kinds" role="radiogroup" aria-label={S.kindOf(fd ? fname : s.name)} bind:this={list} tabindex="-1" onkeydown={keys}>
      <h3>{S.kindHead}</h3>
      {#each choices as k (k ?? '')}
        {@const on = picked(k)}
        <button class="k" class:on class:same={k === null} role="radio" aria-checked={on} tabindex={on ? 0 : -1} title={k === null ? S.sameAs(above?.from ?? s.name) : undefined} onclick={() => choose(k)}>
          {#if k === null}<b dir="auto">{S.sameAs(above?.from ?? s.name)}</b><small>{KINDS[above?.kind ?? s.kind].name}</small>
          {:else}<b>{KINDS[k].name}{on && pick === null ? S.guessed : ''}</b><small>{KINDS[k].what}</small>{/if}
        </button>
      {/each}
    </div>
    <div class="stage" bind:this={stage}>
      {#if othersShown}
      <div class="pivot" role="tablist" aria-label={S.sectionSettingsOf(s.name)} tabindex="-1" onkeydown={tabKeys}>
        <button role="tab" id="ss-tab-looks" aria-selected={!othersOpen} aria-controls="ss-panel" tabindex={othersOpen ? -1 : 0} onclick={() => (onOthers = false)}>
          {S.kindLooks(KINDS[showing].name)}<small>{S.levelCount(levels.length)}</small></button>
        <button role="tab" id="ss-tab-others" aria-selected={othersOpen} aria-controls="ss-panel" tabindex={othersOpen ? 0 : -1} onclick={() => (onOthers = true)}>
          {S.othersHead}<small>{S.othersSet(withKind)}</small></button>
      </div>
      {:else}
      <h3 class="looks" id="ss-tab-looks">{S.kindLooks(KINDS[showing].name)}</h3>
      {/if}
      {#if othersOpen}
      <div id="ss-panel" role="tabpanel" aria-labelledby="ss-tab-others">
        <p class="cap">{S.othersCap}</p>
        <ul class="others">
          {#each tops as t (t.name)}
            {@const link = linkOf(t.name)}
            {@const to = link && catalog.sections.find(x => x.id === link.id)}
            {@const k = fkind(t.name)}
            <li class="orow" class:changed={t.name in fchanged}>
              {#if link}<span class="oi lk" style="--c:{colorOf(link.id)}"><Icon name="section" size={16} /></span>
              {:else}<span class="oi"><Icon name="folder" size={16} /></span>{/if}
              <span class="onm"><b dir="auto" title={t.name}>{t.name}</b><small>{link ? `${S.ownSectionNow}${to ? ` · ${S.files(to.files)}` : ''}` : S.files(t.files)}</small></span>
              {#if !link}
                {@const year = yearLike(t.name) && !k}
                <button class="drop" disabled={year} title={year ? S.yearLikeWhy : undefined} aria-haspopup="menu" aria-expanded={menuOpen('fk:' + t.name)} aria-label={`${S.kindOfFolder(t.name)}: ${k ? KINDS[k].name : S.sameAs(s.name)}`} onclick={e => kindMenu(e, t.name)}>
                  <span dir="auto">{k ? KINDS[k].name : S.sameAs(s.name)}</span><Icon name="down" size={12} /></button>
                <button class="btn mk" aria-label={S.makeSectionOf(t.name)} disabled={dirty || reading || !online} title={dirty ? S.makeSectionFirst : !online ? S.makeSectionAway : S.makeSectionWhat} onclick={() => splitTop(t.name)}><Icon name="section" />{S.makeSection}</button>
              {/if}
            </li>
          {/each}
        </ul>
      </div>
      {:else}
      <div id="ss-panel" role={othersShown ? 'tabpanel' : 'group'} aria-labelledby="ss-tab-looks">
        <p class="cap">{S.looksCap}</p>
        <ul class="tiles">
          {#each levels as l (l.id)}
            {@const v = shownAt(l)}
            <li class="tile">
              <span class="pic"><Sample kind={showing} view={v} {color} /></span>
              <span class="tn"><b>{S.viewLevels[l.id][0]}</b></span>
              <small>{S.viewNames[v][0]}</small>
            </li>
          {/each}
        </ul>
      </div>
      {/if}
    </div>
  </div>
  {#if fd && onsplit}
  <!-- Off while a new kind is counted, so the note says first which Marks it sets aside; off for a
       folder in more than one place (a merged Section, two folders under one title) and offline. -->
  {@const twice = !!fd.also?.length}
  <div class="foot">
    <button class="btn" disabled={reading || twice || !online || (rekind && !counted)} onclick={split}><Icon name="section" />{S.makeSection}</button>
    <span class="muted what">{twice ? S.makeSectionTwice : !online ? S.makeSectionAway : S.makeSectionWhat}</span>
  </div>
  {/if}
  {#if !fd}
  <div class="foot">
    {#if pick !== null}<button class="link" onclick={guessAgain}>{S.guessAgain}</button>{:else}<span class="muted">{S.kindGuessed}</span>{/if}
    {#if onsuggest}
      <span class="sep" aria-hidden="true"></span>
      {#if noOffer}<span class="muted" role="status" dir="auto">{S.suggestNone(s.name)}</span>
      {:else}<button class="link" disabled={dirty || !online} title={dirty ? S.makeSectionFirst : !online ? S.makeSectionAway : undefined} onclick={suggest}>{S.suggestKinds}</button>{/if}
    {/if}
    <span class="grow"></span>
    {#if catalog.sections.length > 1 && (!holds || merge)}
      <span class="muted" id="ss-inside">{S.showInside}</span>
      <button class="drop" aria-labelledby="ss-inside ss-inside-v" aria-haspopup="menu" aria-expanded={menuOpen('inside')} onclick={insideMenu}>
        <span id="ss-inside-v" dir="auto" title={insideName || undefined}>{insideName || S.ownSection}</span><Icon name="down" size={12} /></button>
    {/if}
  </div>
  {/if}
  <div class="apply">
    <span class="note" class:err={(rekind || refold) && !!error} role="status">{#if waits}{S.waitForRead}{:else if pinning}{S.pinNote}{:else if rekind && error}{error}{:else if rekind && !counted}{S.countWait}{:else if rekind && counted}{S.cardsFrom(KINDS[counted.kind].name, counted.total)} {S.remakeNote(counted.marks, counted.back)}{:else if refold && error}{error}{:else if refold && fcount}{S.foldersCards(fcount.total)} {S.remakeNote(fcount.marks, fcount.back)}{:else if refold}{S.countWait}{/if}</span>
    <button class="btn" onclick={close}>{S.cancel}</button>
    <button class="btn primary" disabled={!can} onclick={apply}>{S.save}</button>
  </div>
</dialog>

<style>
  dialog { border: 0; border-radius: 12px; padding: 18px 22px 16px; width: min(1040px, 94vw); height: min(760px, 92vh); background: var(--bg); color: var(--ink);
    box-shadow: 0 24px 70px rgba(0,0,0,.38); display: flex; flex-direction: column; overflow: hidden; }
  dialog:not([open]) { display: none; }
  dialog[open] { animation: rise .16s ease-out; }
  dialog::backdrop { background: rgba(10, 14, 20, .45); }
  @keyframes rise { from { opacity: 0; transform: translateY(8px) scale(.98) } }
  .sr { position: absolute; width: 1px; height: 1px; overflow: hidden; clip-path: inset(50%); white-space: nowrap; margin: 0; }
  header { display: flex; gap: 12px; align-items: flex-start; }
  .dot { width: 12px; height: 12px; border-radius: 4px; background: var(--c); margin-top: 12px; flex: none; }
  .nm { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 3px; }
  .nm input { font: inherit; font-size: 18px; font-weight: 600; border: 1px solid transparent; border-bottom-color: var(--line); border-radius: 6px 6px 0 0;
    background: none; padding: 4px 8px; color: var(--ink); min-width: 0; text-overflow: ellipsis; transition: background .12s, border-color .12s; }
  .nm input:hover { background: var(--hover); }
  .nm input:focus { outline: none; background: var(--field); border-color: var(--line); border-bottom: 2px solid var(--focus); }
  .fname { margin: 0; font-size: 18px; font-weight: 600; padding: 5px 9px; border-bottom: 1px solid transparent; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .nm small { color: var(--muted); font-size: 12px; padding: 0 9px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .x { flex: none; width: 32px; height: 32px; border: 0; background: none; border-radius: 6px; display: grid; place-items: center; color: var(--soft); }
  .x:hover { background: var(--hover); color: var(--ink); }
  .split { display: grid; grid-template-columns: minmax(170px, 230px) 1fr; grid-template-rows: minmax(0, 1fr); gap: 16px; margin-top: 14px; min-height: 0; flex: 1; }
  .kinds { display: flex; flex-direction: column; gap: 4px; overflow: auto; outline: none; }
  .kinds h3 { margin: 0 0 4px 12px; font-size: 12px; font-weight: 600; color: var(--muted); }
  .k { text-align: left; border: 1px solid transparent; background: none; border-radius: 8px; padding: 8px 12px; display: flex; flex-direction: column; gap: 2px;
    position: relative; transition: background .12s, border-color .12s; }
  .k:hover { background: var(--hover); }
  .k.on { border-color: var(--sel-line); background: var(--sel); }
  .k.on::before { content: ""; position: absolute; left: -1px; top: 12px; bottom: 12px; width: 3px; border-radius: 2px; background: var(--focus); }
  .k b { font-size: 13.5px; }
  .k small { color: var(--soft); font-size: 12.5px; line-height: 1.35; }
  .stage { background: var(--pane); border: 1px solid var(--line); border-radius: 10px; padding: 14px 16px; overflow: auto; min-width: 0; display: flex; flex-direction: column; gap: 12px; }
  [role="tabpanel"] { display: flex; flex-direction: column; gap: 12px; }
  .cap { margin: 0; font-size: 13px; color: var(--soft); overflow-wrap: anywhere; }

  /* The levels as a pivot, as Windows 11's tabs: each with the view picked there. */
  .pivot { display: flex; gap: 4px; border-bottom: 1px solid var(--line); overflow-x: auto; overflow-y: hidden; flex: none; scrollbar-width: thin; outline: none; }
  .pivot button { border: 0; background: none; padding: 6px 12px 9px; display: flex; flex-direction: column; align-items: flex-start; gap: 1px; font-size: 13.5px; font-weight: 600;
    color: var(--soft); position: relative; border-radius: 6px 6px 0 0; white-space: nowrap; transition: background .12s, color .12s; }
  .pivot button small { font-weight: 400; font-size: 12px; color: var(--muted); }
  .pivot button:hover { background: var(--hover); }
  .pivot button[aria-selected="true"] { color: var(--ink); }
  /* Inside the strip, which clips anything outside it. */
  .pivot button:focus-visible, .k:focus-visible { outline: 2px solid var(--focus); outline-offset: -2px; }
  .pivot button[aria-selected="true"]::after { content: ""; position: absolute; left: 12px; right: 12px; bottom: -1px; height: 3px; border-radius: 2px; background: var(--focus); }

  /* The sample tiles, as Windows 11's theme picker: a picture with a ring and a tick when picked. */
  /* The samples of each level, as pictures in the views in use: nothing to pick here (`decisions/2026/10/21/the-view-menu-sets-each-folders-view.md`). */
  .looks { margin: 0; font-size: 13.5px; font-weight: 600; }
  .tiles { display: grid; grid-template-columns: repeat(auto-fill, minmax(220px, 1fr)); gap: 14px; list-style: none; margin: 0; padding: 0; }
  .tile { display: flex; flex-direction: column; gap: 4px; min-width: 0; }
  .pic { display: block; border-radius: 8px; padding: 3px; box-shadow: inset 0 0 0 1px var(--line); background: var(--field); }
  .tn { display: flex; gap: 6px; align-items: center; padding: 0 2px; min-width: 0; }
  .tn b { font-size: 13px; font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .tile small { color: var(--soft); font-size: 12px; padding: 0 2px; }

  /* Folders with other things: a row per folder, as Settings' lists, with the app's own menu button. */
  .others { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 2px; }
  .orow { display: flex; align-items: center; gap: 10px; padding: 7px 8px; border-radius: 8px; flex-wrap: wrap; transition: background .12s; }
  .orow:hover { background: var(--hover); }
  .orow.changed { background: var(--sel); }
  .oi { flex: none; width: 28px; height: 28px; border-radius: 6px; display: grid; place-items: center; color: var(--soft); background: var(--field); box-shadow: inset 0 0 0 1px var(--line); }
  .oi.lk { color: var(--c); background: color-mix(in srgb, var(--c) 14%, transparent); box-shadow: none; }
  .onm { flex: 1 1 160px; min-width: 0; display: flex; flex-direction: column; gap: 1px; }
  .onm b { font-size: 13.5px; font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .onm small { color: var(--muted); font-size: 12px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .orow .drop { flex: none; width: 190px; justify-content: space-between; }
  .orow .drop:disabled { opacity: .45; cursor: default; }
  .orow .mk { flex: none; display: inline-flex; align-items: center; gap: 7px; padding: 4px 10px; font-size: 13px; }

  .foot { display: flex; gap: 10px; align-items: center; margin-top: 12px; padding-top: 12px; border-top: 1px solid var(--line); font-size: 13px; min-width: 0; }
  .grow { flex: 1; }
  .foot .btn { display: inline-flex; align-items: center; gap: 7px; flex: none; }
  .foot .what { min-width: 0; font-size: 12.5px; }
  .link { border: 0; background: none; color: var(--link); padding: 4px 0; font-size: 13px; }
  .link:hover { text-decoration: underline; }
  .link:disabled { color: var(--muted); text-decoration: none; cursor: default; }
  .sep { width: 1px; height: 16px; background: var(--line); flex: none; }
  .drop { display: inline-flex; align-items: center; gap: 6px; border: 1px solid var(--line); background: var(--field); border-radius: 6px; padding: 4px 8px 4px 10px;
    font-size: 13px; max-width: 260px; min-width: 0; }
  .drop span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .drop:hover { background: var(--hover); }
  .apply { display: flex; gap: 8px; align-items: center; margin-top: 12px; }
  .note { flex: 1; font-size: 12.5px; color: var(--soft); min-width: 0; }
  .note.err { color: var(--danger); }
  .apply .btn { min-width: 90px; padding: 6px 14px; flex: none; }

  /* Narrow: the kinds become a row of tabs above the stage. */
  @media (max-width: 820px) {
    .split { grid-template-columns: 1fr; grid-template-rows: auto minmax(0, 1fr); }
    .kinds { flex-direction: row; flex-wrap: wrap; overflow: visible; gap: 4px; padding-bottom: 2px; }
    .kinds h3 { display: none; }
    .k { flex: none; padding: 6px 12px; max-width: 100%; }
    .k b { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
    .k small { display: none; }
    /* Same as keeps the kind it gives, which its name alone does not say. */
    .k.same small { display: block; }
    .k.on::before { left: 12px; right: 12px; top: auto; bottom: -1px; width: auto; height: 3px; }
    .tiles { grid-template-columns: repeat(auto-fill, minmax(170px, 1fr)); }
  }
</style>
