<script lang="ts">
  import { flushSync, onMount } from 'svelte'
  import { api, ago, firstFile, hasDialog, keyIn, newRound, place, relIn, size, untickIds, markedIds, doneWord, tickAdds, ticksOf, COLORS, type Catalog, type Kind, type Detail, type Item, type Mark, type Marks, type Section, type Suggestion, type Work, type Info, type PhotoPlace, type MoveScope, type Thumb, type Thumbs, type Tab, type CoverTarget, type Chosen } from './lib/api'
  import { KINDS, S } from './lib/strings'
  import TitleBar from './lib/TitleBar.svelte'
  import FolderTree from './lib/FolderTree.svelte'
  import Library from './lib/Library.svelte'
  import WorkDialog from './lib/WorkDialog.svelte'
  import CoverPicker from './lib/CoverPicker.svelte'
  import Settings from './lib/Settings.svelte'
  import MoveAside from './lib/MoveAside.svelte'
  import { inReview, photoKey, placeOf, waitsAgain, type Notes } from './lib/review'
  import { orphansOf } from './lib/orphans'
  import { adoptFolders, loadUi, saveUi, ui } from './lib/ui.svelte'
  import { picksFor, setViews } from './lib/views'
  import { ask } from './lib/ask.svelte'
  import AskBox from './lib/AskBox.svelte'
  import Menu from './lib/Menu.svelte'
  import SectionSettings, { type FolderChange, type SectionChange } from './lib/SectionSettings.svelte'
  import { homesOf, kindAt, loadFolders, ownOf, setCaseDefault, setCaseKept } from './lib/folders.svelte'
  import ToastBox from './lib/ToastBox.svelte'
  import { hideError, hideToast, showError, showToast } from './lib/toast.svelte'

  let ready = $state(false)
  let hasFolders = $state(false)
  // Raw, not a deep proxy: a proxy per Work, folder and file of a 150,000-card Section held the page
  // for seconds after each read. A change is a new catalog, never an edit in place.
  let catalog = $state.raw<Catalog>({ v: 1, sections: [] })
  // Raw too, and for the same reason: a deep proxy made a signal for each Work looked up, 600,000
  // of them in a Section that size. Every change is a new Marks from the core.
  let marks = $state.raw<Marks>({ v: 2, marks: {}, ticks: {} })
  let thumbs = $state<Record<string, Thumb>>({})
  /** What Choose a cover… is open for (`decisions/2026/10/22/covers-are-chosen-never-fetched-by-themselves.md`). */
  let coverFor = $state<CoverTarget | null>(null)
  let details = $state<Record<string, Detail>>({})
  let lib = $state<ReturnType<typeof Library>>()
  let thumbsLeft = $state(0)
  let info = $state<Info | null>(null)
  let query = $state('')
  let reading = $state('')
  /** The last error's text: a failed read hands it to the change that asked for the read. */
  let error = ''
  /** An error shows in full as a toast. */
  function fail(e: unknown) { error = e instanceof Error ? e.message : String(e); showError(error) }
  function clearError() { error = ''; hideError() }
  let opened = $state<{ w: Work; s: Section } | null>(null)
  let settingsTab = $state<Tab | null>(null)
  /** The Section whose settings are open. */
  let sectionOpen = $state<Section | null>(null)
  function sectionSettings(s: Section) { sectionOpen = s }
  /** The folder whose Folder settings are open (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`): its checked folder's Section, its titles and its path there. */
  let folderOpen = $state<{ s: Section; f: string[]; rel: string } | null>(null)
  function folderSettings(s: Section, f: string[], rel: string) { folderOpen = { s, f, rel } }
  let picks = $state(new Map<string, 'on' | 'off'>())
  /** Review's notes on Photos, and pictures still to be read for it. */
  let notes = $state<Notes>({})
  let shotsLeft = $state(0)
  /** The Photos Section whose set-aside photos the Move dialog is open for (`decisions/2026/10/09/review-moves-set-aside-photos-itself-with-undo.md`). */
  let movingAside = $state<{ place: PhotoPlace; scope?: MoveScope } | null>(null)
  /** Opens Review in its own window on these events, of the files `only` keeps, at `start`. */
  async function review(works: Work[], label: string, only?: (i: Item) => boolean, start?: Item) {
    const files = only ? works.flatMap(w => w.items).filter(only).map(i => i.path) : undefined
    try { await api.openReview({ works: works.map(w => w.id), label, files, start: start?.path }) } catch (e) { showError(e) }
  }
  let sortedAt = 0
  async function sort() {
    sortedAt = Date.now()
    const s = await api.photoSort()
    notes = s.notes
    shotsLeft = s.left
  }

  onMount(() => {
    const un = api.onScanProgress(p => (reading = S.reading(p.section, p.files)))
    const unThumbs = api.onThumbs(got)
    const unDetails = api.onDetails(d => Object.assign(details, d))
    // Notes are worked out again as pictures are read: often at first, then every few seconds.
    // A read dropped the kind of folders renamed or moved on disk: said once, as they are gone from the settings.
    const unLost = api.onKindsLost(async lost => {
      showToast(S.kindsLost(lost))
      loadFolders(await api.settings())
    })
    // The Review window's news: choices made, a move, or Review closed with photos marked.
    const unReview = api.onReviewNews(async news => {
      marks = await api.marks()
      // Review's read of the folder said its progress here too: it is over.
      if (news.what === 'moved') { reading = ''; catalog = await api.catalog(); setThumbs(await api.thumbs()); await sort() }
      const w = news.what === 'closed' && news.n ? catalog.sections.flatMap(s => s.works).find(x => x.id === news.work) : undefined
      const p = w && placeOf(catalog, w)
      if (news.what === 'closed' && p) showToast(S.reviewKept(news.n), undefined, { label: S.moveShort, run: () => (movingAside = { place: p }) })
    })
    const unShots = api.onShots(left => {
      shotsLeft = left
      if (!left || Date.now() - sortedAt > 8000) sort()
    })
    ;(async () => {
      const st = await api.settings()
      hasFolders = st.folders.length > 0
      loadUi(st.ui)
      loadFolders(st)
      // How each drive compares folder names, for the folder settings; never waited on.
      api.drives().then(setCaseKept).catch(() => {})
      // A failed save must not stop the app opening: the views apply for this session anyway.
      if (adoptFolders(st.folders.map(folderId))) saveUi().catch(() => {})
      catalog = await api.catalog()
      marks = await api.marks()
      setThumbs(await api.thumbs())
      details = await api.details()
      await sort()
      info = await api.info()
      setCaseDefault(info.keeps_case)
      ready = true
      // A data file that could not be read at start is said once, with the way to its line in the log.
      const aside = await api.setAside()
      if (aside.length) showToast(S.damagedAtStart(aside), undefined, { label: S.seeLog, run: () => (settingsTab = 'log') })
      if (hasFolders) await rescan()
    })()
    // The read state's "N min ago" moves on with time.
    const clockTimer = setInterval(() => (clock = Date.now()), 30_000)
    // A drive unplugged while the app is open greys its Sections at once: the drives are
    // listed, no folder read, when the window gets focus and every 10 s while it is shown.
    const awayTimer = setInterval(() => { if (!document.hidden) drivesAway() }, 10_000)
    window.addEventListener('focus', drivesAway)
    return () => { clearInterval(clockTimer); clearInterval(awayTimer); window.removeEventListener('focus', drivesAway); un.then(f => f()); unThumbs.then(f => f()); unDetails.then(f => f()); unShots.then(f => f()); unLost.then(f => f()); unReview.then(f => f()) }
  })

  function got(t: Thumbs) {
    for (const x of t.done) thumbs[x.id] = x
    if (!t.left && thumbsLeft) api.info().then(i => (info = i)) // the cache grew
    thumbsLeft = t.left
  }
  function setThumbs(t: Thumbs) {
    thumbs = Object.fromEntries(t.done.map(x => [x.id, x]))
    thumbsLeft = t.left
  }

  /** Reads every checked folder again, or one Section or folder; false when the read failed. A
   *  `quiet` read leaves its error in `error` for the change that asked for it, which says it once. */
  async function rescan(only: Section | null = null, under: string[] = [], quiet = false) {
    clearError()
    reading = S.reading(under.at(-1) ?? only?.name ?? 'folders', 0)
    let ok = true
    try { catalog = await api.scan(only?.id ?? null, under) } catch (e) { if (quiet) error = e instanceof Error ? e.message : String(e); else fail(e); ok = false }
    if (ok) api.tellCatalog()
    // A read of one folder leaves the Sections' read times as they were: the status bar names it.
    if (ok) partRead = under.length ? { name: place(under.at(-1)!), at: Date.now() / 1000 } : null
    setThumbs(await api.thumbs())
    await sort()
    reading = ''
    info = await api.info()
    return ok
  }
  async function rethumb(ids: string[]) {
    await api.rebuildThumbs(ids)
    newRound()
    setThumbs(await api.thumbs())
  }
  /** Removes a chosen cover (Remove this cover, Not this cover): the card or tile shows what its
   *  files give again, and Undo puts it back. */
  async function removeCover(ids: string[]) {
    try {
      const gone = await api.removeCover(ids)
      setThumbs(await api.thumbs())
      if (Object.keys(gone).length) showToast(S.coverRemoved, () => restoreCover(gone))
    } catch (e) { fail(e) }
  }
  async function restoreCover(gone: Record<string, Chosen>) {
    try {
      await api.restoreCover(gone)
      setThumbs(await api.thumbs())
    } catch (e) { fail(e) }
  }
  /** A cover was chosen in the picker: its thumbnail arrives with the next batch. */
  async function coverChosen() {
    coverFor = null
    setThumbs(await api.thumbs())
    showToast(S.coverSet)
  }
  async function firstSave() {
    const on = [...picks].filter(([, v]) => v === 'on').map(([k]) => k)
    const off = [...picks].filter(([, v]) => v === 'off').map(([k]) => k)
    hasFolders = (await api.setFolders(on, off)).folders.length > 0
    await rescan()
    // The first new Section opens, so the person sees what was read rather than an empty Next up.
    const first = catalog.sections.find(s => s.works.length) ?? catalog.sections[0]
    if (first) lib?.go({ t: 'section', id: first.id, folder: [] })
  }
  async function changed(again: boolean) {
    const st = await api.settings()
    hasFolders = st.folders.length > 0
    loadFolders(st)
    // A cache move plans the thumbnails again.
    if (again) await rescan()
    else setThumbs(await api.thumbs())
  }
  async function mark(m: 'next' | 'done' | null) {
    if (opened) await markWork(opened.w, m)
  }
  // A folder Work with more than one file opens its dialog; any other opens its file (`decisions/2026/09/29/folder-shows-own-works-and-one-box-shape.md`'s Hub cards).
  function openWork(w: Work, s: Section) {
    if (hasDialog(w)) opened = { w, s }
    else { const f = firstFile(w); if (f) openFile(f.path, w) }
  }
  function openById(id: string) {
    for (const s of catalog.sections) {
      const w = s.works.find(w => w.id === id)
      if (w) { opened = { w, s }; return }
    }
  }
  /** `path` is in folder `dir`, by its text; a Windows path in any case. */
  function isUnder(path: string, dir: string) {
    const win = dir.includes('\\')
    const [p, d] = win ? [path.toLowerCase(), dir.toLowerCase()] : [path, dir]
    const end = win ? '\\' : '/'
    return p === d || p.startsWith(d.endsWith(end) ? d : d + end)
  }
  /** A file or folder on a drive that is not connected cannot open: a toast says so, and true. A
   * connected Section with the same path (another drive given its letter) opens it. */
  function awayToast(path: string) {
    if (catalog.sections.some(s => s.online && isUnder(path, s.path))) return false
    const away = catalog.sections.find(s => !s.online && !s.gone && isUnder(path, s.path))
    if (away) showToast(S.openAway(away.drive))
    return !!away
  }
  async function drivesAway() {
    if (!ready || reading) return
    // Only those Sections change, so a rename or a removal made meanwhile stays.
    const ids = await api.drivesAway().catch(() => [] as string[])
    if (catalog.sections.some(s => s.online && ids.includes(s.id)))
      catalog = { ...catalog, sections: catalog.sections.map(s => (ids.includes(s.id) ? { ...s, online: false } : s)) }
  }
  /** Opens a file; a file of Work `w` puts the Work first on Home's Recently opened. */
  async function openFile(path: string, w?: Work) {
    clearError()
    if (awayToast(path)) return
    try {
      // A file only in the cloud downloads as it opens, so the person is asked first.
      const cloud = await api.open(path)
      if (cloud !== null) {
        if (!await ask(S.downloadSure(path.replace(/^.*[\\/]/, ''), size(cloud)), S.downloadOpen)) return
        await api.open(path, true)
      }
      if (w) marks = await api.setOpened(w.id)
    } catch (e) { fail(e) }
  }
  async function clearOpened() {
    if (!await ask(S.clearOpenedSure, S.clear, true)) return
    try { marks = await api.clearOpened() } catch (e) { fail(e) }
  }
  async function reveal(path: string, open = false) {
    clearError()
    if (awayToast(path)) return
    try { await api.reveal(path, open) } catch (e) { fail(e) }
  }
  /** Forgets a photo's Review choice, so it waits for Review again; Undo puts it back. */
  async function reviewAgain(w: Work, i: Item) {
    const key = photoKey(i, w), old = marks.review?.[key]
    if (!old) return
    // The toast says what the counts will: it waits for Review, or it stays, and only offers Not
    // needed when a step of Review shows it (a lone Junk photo is in none, `decisions/2026/10/21/junk-alone-never-waits-for-review.md`).
    const waits = waitsAgain(i, w, notes, marks), shown = inReview(i, w, notes, marks)
    try { marks = await api.forgetReview([key]) } catch (e) { showError(e); return }
    showToast((waits ? S.reviewAgainDone : shown ? S.reviewAgainKept : S.reviewAgainStays)(i.name), async () => { marks = await api.restoreReview({ [key]: old }) })
  }
  async function forget(s: Section) {
    // The Sections split out of it, at any depth, go with it (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`).
    const inside = (id: string): string[] => catalog.sections.filter(x => x.split_from === id).flatMap(x => [x.name, ...inside(x.id)])
    if (!await ask(S.removeSure(s.name, inside(s.id)), S.removeFolder, true)) return
    catalog = await api.forgetFolder(s.id)
    hasFolders = (await api.settings()).folders.length > 0
  }
  /** A Mark lands on the Work's own id; clearing one on a merged card clears every copy, so the
   * card is left unmarked. A write that fails says so and reloads the Marks. */
  async function markWork(w: Work, m: 'next' | 'done' | null) {
    try {
      for (const id of m ? [w.id] : markedIds(w, marks)) marks = await api.setMark(id, m)
    } catch (e) {
      showError(e)
      marks = await api.marks()
    }
  }
  /** One Mark on several cards at once, as `markWork` sets it on each, saved in one write;
   * one toast's Undo puts back every Mark as it was, its date too. */
  async function markWorks(ws: Work[], m: 'next' | 'done' | null) {
    const at = Math.floor(Date.now() / 1000), now: Record<string, Mark | null> = {}, old: Record<string, Mark | null> = {}
    for (const w of ws) for (const id of m ? [w.id] : markedIds(w, marks)) {
      const was = marks.marks[id] ?? null
      if (m ? was?.m === m : !was) continue
      now[id] = m ? { m, at } : null; old[id] = was
    }
    hideToast()
    if (!Object.keys(now).length) return
    // Cards, not ids: a merged card's copies count once.
    const n = m ? ws.length : ws.filter(w => markedIds(w, marks).some(id => id in now)).length
    const word = new Set(ws.map(doneWord)).size === 1 ? doneWord(ws[0]) : S.done
    try {
      marks = await api.putMarks(now)
      showToast(m ? S.markedMany(n, m === 'next' ? S.next : word) : S.clearedMany(n), async () => {
        // Only the Marks still as this change left them: one changed since keeps its new Mark.
        const back = Object.fromEntries(Object.entries(old).filter(([id]) => {
          const is = marks.marks[id], was = now[id]
          return was ? is?.m === was.m && is.at === was.at : !is
        }))
        try { marks = await api.putMarks(back) } catch (e) { showError(e); marks = await api.marks() }
      })
    } catch (e) {
      showError(e)
      marks = await api.marks()
    }
  }
  /** A Tick lands on the Work's own id, for the items no copy has ticked yet; an untick on a
   * merged card takes the items off every copy that has them. A write that fails says so
   * and reloads the Marks, so the dialog shows what is stored. */
  async function tick(keys: string[], on: boolean) {
    if (!opened) return
    const w = opened.w
    try {
      if (on) {
        const add = tickAdds(w, keys, marks)
        if (add.length) marks = await api.setTicks(w.id, add, true)
      } else for (const id of untickIds(w, keys, marks)) marks = await api.setTicks(id, keys, false)
    } catch (e) {
      showError(e)
      marks = await api.marks()
    }
  }
  const setMerge = (id: string, into: string) => { if (into) ui.merge[id] = into; else delete ui.merge[id] }
  type Kept = { name: string | null; kind: Kind | null; merge: string }
  /** What changes: a kind is read again; a name alone is changed in place, as is Show inside,
   * which only redraws. */
  type What = { read: boolean; rename: boolean; ui: boolean }
  /** Puts a Section's name, kind and Show inside to `to`; when the read a kind change needs
   * fails it puts back `from`, so the settings never claim a kind the cards do not show. */
  async function putSection(s: Section, to: Kept, from: Kept, w: What) {
    // While folders are read, a read or a rename of this Section would be refused: nothing changes yet.
    if ((w.read || w.rename) && reading) throw new Error(S.waitForRead)
    if (w.read) await api.setSection(s.id, to.name, to.kind, true)
    else if (w.rename) catalog = await api.renameSection(s.id, to.name)
    if (w.ui) { setMerge(s.id, to.merge); await saveUi() }
    if (w.read && !(await rescan(s, [], true))) {
      const why = error
      await api.setSection(s.id, from.name, from.kind, true)
      if (w.ui) { setMerge(s.id, from.merge); await saveUi() }
      throw new Error(S.notApplied(why))
    }
  }
  const folderId = (f: { serial: number; rel: string }) => `${f.serial.toString(16).toUpperCase().padStart(8, '0')}:${f.rel}`
  /** Applies Section settings; only a kind change reads the Section again. The toast's Undo puts
   * back the name, kind and Show inside it had. */
  async function applySection(s: Section, c: SectionChange) {
    clearError()
    try {
      const cf = (await api.settings()).folders.find(f => folderId(f) === s.id)
      // Without it, Undo could not put back the name and kind kept now.
      if (!cf) throw new Error(S.notApplied(S.sectionGoneNow))
      const old: Kept = { name: cf.name, kind: cf.kind, merge: ui.merge[s.id] ?? '' }
      // An emptied name gives the Section its folder's name again.
      const want: Kept = { name: c.renamed ? c.name || null : old.name, kind: c.kind, merge: c.merge }
      const w: What = { read: c.rekind, rename: c.renamed && !c.rekind, ui: c.remerge }
      const kinds = c.folders, before = Object.fromEntries(Object.keys(kinds).map(r => [r, ownOf(s, r).kind]))
      const refold = Object.keys(kinds).length > 0
      await putSection(s, want, old, w)
      // Folder kinds that cannot be used take back the rest, so nothing is left half done without Undo.
      if (refold) try { await putKinds(s, kinds, before) } catch (e) { await putSection(s, old, want, w).catch(() => {}); throw e }
      const now = catalog.sections.find(x => x.id === s.id)
      // A Section whose drive is not connected keeps what was read last time until it is read.
      const text = !now?.online && c.rekind ? S.sectionSavedAway(now?.name ?? s.name)
        : S.sectionSaved(now?.name ?? s.name, c.rekind ? KINDS[now?.kind ?? s.kind].name : null)
      showToast(text, async () => { if (refold) await putKinds(s, before, kinds); await putSection(s, old, want, w) })
    } catch (e) { fail(e) }
  }
  /** Gives a Section's top-level folders the kinds `to` (null: Same as the Section), their
   * views kept, and reads it again; when that read fails it puts back `from` (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`). */
  async function putKinds(s: Section, to: Record<string, Kind | null>, from: Record<string, Kind | null>) {
    if (reading) throw new Error(S.waitForRead)
    const done: string[] = []
    const back = async () => { for (const rel of done) loadFolders(await api.setFolder(keyIn(s.id, rel), from[rel] ?? null, ownOf(s, rel).views)) }
    try {
      for (const [rel, k] of Object.entries(to)) { loadFolders(await api.setFolder(keyIn(s.id, rel), k, ownOf(s, rel).views)); done.push(rel) }
    } catch (e) { await back(); throw e }
    if (!(await rescan(s, [], true))) {
      const why = error
      await back()
      throw new Error(S.notApplied(why))
    }
  }
  /** The Section shown that a Section shows inside, or its own id (`decisions/2026/10/05/checked-folders-on-any-drive.md`). */
  const shownOf = (s: Section) => (ui.merge[s.id] && catalog.sections.some(x => x.id === ui.merge[s.id]) ? ui.merge[s.id] : s.id)
  /** The same folder in the other Sections of a merged one, for the dry run. */
  const alsoOf = (o: { s: Section; f: string[]; rel: string }) => homesOf(catalog, shownOf(o.s), o.f).filter(p => p.home.id !== o.s.id || p.rel !== o.rel).map(p => ({ id: p.home.id, rel: p.rel }))
  type Own = { kind: Kind | null; views: Record<string, string> }
  type Place = { home: Section; rel: string }
  /** Puts a folder's own kind and views to `to`, on each drive of a merged Section (`at`); a kind
   * change reads again the top folder holding it, which is all a Folder kind can change
   * (scan.rs `holds_kind`; the core reads the whole Section when the folder above takes its
   * files). When a read fails it puts back `from`, so the settings never claim a kind the cards
   * do not show. */
  async function putFolder(at: Place[], f: string[], to: (p: Place) => Own, from: (p: Place) => Own, read: boolean) {
    if (read && reading) throw new Error(S.waitForRead)
    for (const p of at) loadFolders(await api.setFolder(keyIn(p.home.id, p.rel), to(p).kind, to(p).views))
    // Each Section once: two folders under one title are read by one whole read.
    const homes = [...new Map(at.map(p => [p.home.id, p])).values()]
    for (const p of read ? homes : []) if (!(await rescan(p.home, f.slice(0, 1), true))) {
      const why = error
      for (const q of at) loadFolders(await api.setFolder(keyIn(q.home.id, q.rel), from(q).kind, from(q).views))
      for (const q of homes.filter(q => q !== p)) await rescan(q.home, f.slice(0, 1), true)
      throw new Error(S.notApplied(why))
    }
  }
  /** Applies Folder settings, a kind change, which reads again; the folder keeps its views. The
   * toast's Undo puts back the kind it had (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`). */
  async function applyFolder(s: Section, f: string[], rel: string, c: FolderChange) {
    clearError()
    try {
      const at = homesOf(catalog, shownOf(s), f)
      if (!at.some(p => p.home.id === s.id)) at.unshift({ home: s, rel })
      const olds = new Map(at.map(p => [p, ownOf(p.home, p.rel)]))
      // Views come from the View menu: each write keeps the ones the folder has then, so Undo puts back only the kind.
      const old = (p: Place): Own => ({ kind: olds.get(p)!.kind, views: ownOf(p.home, p.rel).views })
      const want = (p: Place): Own => ({ kind: c.kind, views: ownOf(p.home, p.rel).views })
      await putFolder(at, f, want, old, c.rekind)
      const now = catalog.sections.find(x => x.id === s.id) ?? s
      const name = place(f.at(-1) ?? '')
      const text = !now.online && c.rekind ? S.sectionSavedAway(name) : S.sectionSaved(name, c.rekind ? KINDS[kindAt(now, rel)].name : null)
      showToast(text, () => putFolder(at, f, old, want, c.rekind))
    } catch (e) { fail(e) }
  }
  /** A folder as a Split-out Section (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`): the Section it is in, its path there, its kind, its
   * views, its name, the Section it shows inside and those shown inside it, and its own setting as a
   * folder, which a failed read or Undo puts back. */
  type Split = { home: Section; rel: string; kind: Kind; views: Record<string, string>; name: string | null; merge: string; inside: string[]; own: Own
    /** Its place among the Sections, kept by Undo so the others keep their order and colours. */
    at?: number }
  /** Makes a folder a Section of its own and reads again the Section it is in, which reads the new
   * one too; when that read fails the folder is put back as it was. Returns the new Section's id. */
  async function split(x: Split): Promise<string> {
    if (reading) throw new Error(S.waitForRead)
    const key = keyIn(x.home.id, x.rel)
    const id = await api.splitFolder(key, x.kind, x.at ?? null)
    if (x.name) await api.setSection(id, x.name, x.kind, true)
    setViews(id, picksFor(x.kind, x.views)); setMerge(id, x.merge)
    for (const m of x.inside) ui.merge[m] = id
    await saveUi()
    loadFolders(await api.settings())
    if (!(await rescan(x.home, [], true))) {
      const why = error
      await api.putBack(id, x.kind, {})
      loadFolders(await api.setFolder(key, x.own.kind, x.own.views))
      setViews(id, {}); setMerge(id, ''); await saveUi()
      await rescan(x.home, [], true)
      throw new Error(S.notApplied(why))
    }
    return id
  }
  /** Puts a Split-out Section back into the Section it came from as a folder, with its kind and views
   * as its Folder kind, or with `own` (Undo's: the setting it had before), and reads that Section
   * again; when that read fails it is split out again. Returns what Undo needs to split it again. */
  async function putBack(id: string, own?: Own): Promise<Split> {
    if (reading) throw new Error(S.waitForRead)
    const sec = catalog.sections.find(x => x.id === id)
    const home = catalog.sections.find(x => x.id === sec?.split_from)
    if (!sec || !home) throw new Error(S.notApplied(S.sectionGoneNow))
    const fs = (await api.settings()).folders, cf = fs.find(f => folderId(f) === id)
    const views = { ...(ui.views[id] ?? {}) }
    const x: Split = { home, rel: relIn(id, home.id), kind: sec.kind, views, name: cf?.name ?? null, merge: ui.merge[id] ?? '',
      inside: Object.keys(ui.merge).filter(k => ui.merge[k] === id), at: fs.findIndex(f => folderId(f) === id), own: { kind: sec.kind, views: Object.fromEntries(Object.entries(views).filter(([k]) => k !== 'section')) } }
    await api.putBack(id, x.kind, x.own.views)
    if (own) await api.setFolder(id, own.kind, own.views)
    setViews(id, {}); setMerge(id, '')
    for (const m of x.inside) delete ui.merge[m]
    await saveUi()
    loadFolders(await api.settings())
    if (!(await rescan(home, [], true))) {
      const why = error
      await split(x)
      throw new Error(S.notApplied(why))
    }
    return x
  }
  /** Folder settings' Make it a Section: the folder becomes a Section of its own and opens; the
   * toast's Undo puts it back as it was (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`). */
  async function makeSection(s: Section, rel: string, kind: Kind, views: Record<string, string>) {
    clearError()
    try {
      const own = ownOf(s, rel)
      const id = await split({ home: s, rel, kind, views, name: null, merge: '', inside: [], own })
      const made = catalog.sections.find(x => x.id === id)
      lib?.go({ t: 'section', id, folder: [] })
      showToast(S.madeSection(made?.name ?? rel), async () => { await putBack(id, own) })
    } catch (e) { fail(e) }
  }
  /** Put back into <Section>: the Section it came from shows its folder again, which opens; the
   * toast's Undo makes it a Section again. */
  async function putBackSection(sec: Section) {
    clearError()
    try {
      const titles = catalog.sections.find(x => x.id === sec.split_from)?.links?.find(l => l.id === sec.id)?.folder
      const x = await putBack(sec.id)
      if (titles) lib?.go({ t: 'section', id: x.home.id, folder: titles })
      showToast(S.putBackDone(sec.name, x.home.name), async () => { await split(x) })
    } catch (e) { fail(e) }
  }
  const setOffered = async (s: Section, on: boolean) => loadFolders(await api.setOffered(s.id, on))
  /** The offer's buttons go with it: focus moves to the page, never to nothing. */
  function pageFocus() {
    const a = document.activeElement
    if (!a || a === document.body || a.closest('.offer')) document.getElementById('works')?.focus()
  }
  /** The offer of a kind per folder (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`), answered: each folder takes the kind guessed for
   * it, or becomes a Section of that kind, or the offer is put away. Either way it is not made
   * again, and the toast's Undo puts back the folders and the offer as they were. The toast comes
   * first, so a change made always has its Undo. */
  async function answerOffer(s: Section, list: Suggestion[], how: 'kinds' | 'sections' | 'away') {
    clearError()
    try {
      if (how === 'kinds') {
        const to = Object.fromEntries(list.map(x => [x.rel, x.kind])), before = Object.fromEntries(list.map(x => [x.rel, ownOf(s, x.rel).kind]))
        await putKinds(s, to, before)
        showToast(S.kindsSetEach(list.length, s.name), async () => { await putKinds(s, before, to); await setOffered(s, false); pageFocus() })
      } else if (how === 'sections') {
        // A guessed kind is kept while the folders leave: guessed from what is left, it could change.
        const pin = s.guessed
        const made = await splitEach(s, list, pin)
        const text = S.madeEachSection(made.map(m => catalog.sections.find(x => x.id === m.id)?.name ?? m.rel))
        showToast(pin ? `${text}. ${S.keptKind(s.name, KINDS[s.kind].name)}` : text, async () => { await putBackEach(s, made, pin); await setOffered(s, false); pageFocus() })
      } else {
        showToast(S.offerAway, async () => { await setOffered(s, false); pageFocus() })
      }
      await setOffered(s, true)
      pageFocus()
    } catch (e) { fail(e) }
  }
  /** A folder made a Section by Make each a Section: the new Section's id, the folder's path, its kind and its own setting before. */
  type Made = { id: string; rel: string; kind: Kind; own: Own }
  /** Makes each folder a Section of the kind guessed, then reads the Section they were in once,
   * which reads them too; when a split or that read fails, every one is put back as it was. With
   * `pin`, the Section's guessed kind is kept as its own first. */
  async function splitEach(s: Section, list: { rel: string; kind: Kind }[], pin: boolean): Promise<Made[]> {
    if (reading) throw new Error(S.waitForRead)
    const made: Made[] = []
    try {
      if (pin) await api.setSection(s.id, null, s.kind, false)
      for (const x of list) {
        const own = ownOf(s, x.rel)
        const id = await api.splitFolder(keyIn(s.id, x.rel), x.kind)
        made.push({ id, rel: x.rel, kind: x.kind, own })
        setViews(id, picksFor(x.kind, {}))
      }
      await saveUi()
    } catch (e) { await unsplit(s, made, pin); throw e }
    loadFolders(await api.settings())
    if (!(await rescan(s, [], true))) {
      const why = error
      await unsplit(s, made, pin)
      await rescan(s, [], true)
      throw new Error(S.notApplied(why))
    }
    return made
  }
  /** Puts Sections made by `splitEach` back as the folders they were, with their own settings, and
   * with `pin` lets the app guess the Section's kind again. */
  async function unsplit(s: Section, made: Made[], pin: boolean) {
    for (const m of [...made].reverse()) {
      await api.putBack(m.id, m.kind, {})
      loadFolders(await api.setFolder(m.id, m.own.kind, m.own.views))
      setViews(m.id, {}); setMerge(m.id, '')
      for (const k of Object.keys(ui.merge)) if (ui.merge[k] === m.id) delete ui.merge[k]
    }
    if (pin) await api.setSection(s.id, null, null, false)
    await saveUi()
  }
  /** Undo of Make each a Section: the folders go back into `s`, which is read again; when that read fails they are split out again. */
  async function putBackEach(s: Section, made: Made[], pin: boolean) {
    if (reading) throw new Error(S.waitForRead)
    await unsplit(s, made, pin)
    loadFolders(await api.settings())
    if (!(await rescan(s, [], true))) {
      const why = error
      await splitEach(s, made, pin)
      throw new Error(S.notApplied(why))
    }
  }
  /** Section settings' Suggest kinds for its folders: the offer is made again, at the top of the Section shown. */
  async function suggestAgain(s: Section) {
    clearError()
    try {
      await setOffered(s, false)
      lib?.go({ t: 'section', id: shownOf(s), folder: [] })
    } catch (e) { fail(e) }
  }
  const orphans = $derived.by(() => { const o = orphansOf(catalog, marks); return o.gone.length + o.items.length })
  const gone = $derived(catalog.sections.filter(s => s.gone).length)
  const works = $derived(catalog.sections.reduce((a, s) => a + s.works.length, 0))
  const lastRead = $derived(Math.max(0, ...catalog.sections.filter(s => s.online).map(s => s.scanned_at)))
  const cacheSize = $derived(info?.places.find(p => p.path === info?.cache_dir)?.size ?? 0)
  /** The last read of one folder (its Read this folder again, a kind change, a move): newer than a whole read, the status bar says it. */
  let partRead = $state<{ name: string; at: number } | null>(null)
  let clock = $state(Date.now())
  const wholeState = $derived.by(() => { void clock; return lastRead ? S.readAgo(ago(lastRead)) : '' })
  const partAgo = $derived.by(() => { void clock; return partRead && partRead.at > lastRead ? ago(partRead.at) : '' })
  const partState = $derived(partAgo && partRead ? S.folderReadAgo(partRead.name, partAgo) : '')
  const readState = $derived(reading || partState || wholeState)
  /** Its hover gives both when a folder was read after the rest. */
  const readTip = $derived(reading || [partState, wholeState].filter(Boolean).join('\n'))

  /** How far the status bar gives way so the read state stays whole: 1 shortens the
   *  network line, 2 also leaves out Cache, 3 also the version. */
  let squeeze = $state(0)
  let bar = $state<HTMLElement>()
  let fitting = 0
  const netShort = $derived(squeeze > 0)
  const cut = (e: Element) => e.scrollWidth > e.clientWidth
  function fitBar() {
    cancelAnimationFrame(fitting)
    fitting = requestAnimationFrame(() => {
      const left = bar?.querySelector('.left')
      if (!left) return
      for (let n = 0; n <= 3; n++) {
        squeeze = n
        flushSync()
        // A folder name in the read state is cut inside it: that counts too.
        if (!cut(left) && ![...left.children, ...left.querySelectorAll('.pn')].some(cut)) break
      }
    })
  }
  $effect(() => {
    void [readState, hasFolders, gone, orphans, thumbsLeft, info, cacheSize]
    fitBar()
  })
  $effect(() => {
    if (!bar) return
    const ro = new ResizeObserver(fitBar)
    ro.observe(bar)
    return () => ro.disconnect()
  })
  // F1 opens Settings, Help, unless another dialog is open.
  function help(e: KeyboardEvent) {
    if (e.key !== 'F1' || document.querySelector('dialog[open]:not([aria-labelledby="settings-title"])')) return
    e.preventDefault()
    settingsTab = 'help'
  }
</script>

<!-- WebView2's own menu (Back, Refresh, Print) is no part of the app: a right-click with no menu
     of the app's own shows nothing, except in a text box or on selected text, for Cut, Copy and Paste. -->
<svelte:window oncontextmenu={e => !(e.target as Element).closest?.('input, textarea, [contenteditable]') && !getSelection()?.toString() && e.preventDefault()}
  onkeydown={help} />
{#if hasFolders}<a class="skip" href="#works">{S.skipToWorks}</a>{/if}
<div class="app">
  <TitleBar bind:query search={ready && hasFolders && works > 0} onhome={() => lib?.home()} onsettings={() => (settingsTab = 'folders')} />
  {#if !ready}
    <div></div>
  {:else if !hasFolders}
    <div class="welcome">
      <h1>{S.welcome}</h1>
      <p class="muted">{S.welcomeWhat}</p>
      <FolderTree bind:picks />
      <div class="go"><button class="btn primary" disabled={![...picks.values()].includes('on')} onclick={firstSave}>{S.saveAndRead}</button></div>
    </div>
  {:else}
    <Library bind:this={lib} {catalog} {marks} {thumbs} {details} bind:query onopen={openWork} onmark={markWork} onmarkmany={markWorks} onfile={openFile} onclearopened={clearOpened} onreveal={reveal} onkind={sectionSettings}
      {notes} {shotsLeft} onreview={review} onmove={(p, scope) => (movingAside = { place: p, scope })}
      onfolder={folderSettings} reading={!!reading} onreread={rescan} onrethumb={rethumb} onchoosecover={t => (coverFor = t)} onremovecover={removeCover} onsettings={t => (settingsTab = t)} onforget={forget} onwork={openById} onputback={putBackSection} onoffer={answerOffer} onagain={reviewAgain} />
  {/if}
  <footer bind:this={bar}>
    <div class="left">
    <span class="state" class:live={!!reading} title={readTip}>{#if !reading && partAgo && partRead}<!-- A long folder name is cut, never the time. --><span class="pn" dir="auto">{partRead.name}</span><span class="pa">{S.folderReadAgo('', partAgo)}</span>{:else}{readState}{/if}</span>
    {#if hasFolders}<button onclick={() => rescan()} disabled={!!reading}>{S.refresh}</button>{/if}
    {#if gone && !reading}<button class="warn" onclick={() => (settingsTab = 'folders')}>{S.goneBar(gone)}</button>{/if}
    {#if orphans && !reading}<button class="warn" onclick={() => (settingsTab = 'marks')}>{S.orphansBar(orphans)}</button>{/if}
    {#if thumbsLeft}<span class="live">{S.thumbsLeft(thumbsLeft)}</span>{/if}
    <button onclick={() => (settingsTab = 'about')} title={netShort ? S.network : undefined}>{netShort ? S.networkShort : S.network}</button>
    </div>
    {#if info && squeeze < 2}<span title={info.cache_dir}>{S.cache(size(cacheSize))}</span>{/if}
    {#if info && squeeze < 3}<span class="ver">{S.versionBar(info.version, info.build)}</span>{/if}
  </footer>
</div>

{#if coverFor}
  <CoverPicker target={coverFor} onchosen={coverChosen} onremove={async ids => { coverFor = null; await removeCover(ids) }} onclose={() => (coverFor = null)} />
{/if}
{#if opened}
  <WorkDialog work={opened.w} section={opened.s} color={COLORS[Math.max(0, catalog.sections.findIndex(s => s.id === opened!.s.id)) % COLORS.length]}
    {thumbs} {details} mark={marks.marks[opened.w.id]} ticks={ticksOf(opened.w, marks) ?? {}} onmark={mark} ontick={tick}
    {notes} {marks} onreview={() => { if (opened) review([opened.w], opened.w.title) }}
    onfile={p => openFile(p, opened?.w)} onreveal={reveal} onclose={() => (opened = null)} />
{/if}
<AskBox />
<Menu />
{#if movingAside}
  <MoveAside section={movingAside.place} scope={movingAside.scope} {catalog} {thumbs} onmoved={async () => { marks = await api.marks(); if (movingAside) await rescan(movingAside.place.home, movingAside.place.under) }} onreveal={reveal} onclose={() => (movingAside = null)} />
{/if}
{#if settingsTab}
  <Settings bind:tab={settingsTab} {catalog} bind:marks onchanged={changed} onforget={forget} onrethumb={rethumb} onsection={s => sectionSettings(s)} onclose={() => { settingsTab = null; api.info().then(i => (info = i)) }} />
{/if}

{#if sectionOpen}
  {#key sectionOpen.id}
    <SectionSettings section={sectionOpen} {catalog} reading={!!reading} onapply={c => sectionOpen && applySection(sectionOpen, c)}
      onsplittop={(rel, k, v) => sectionOpen && makeSection(sectionOpen, rel, k, v)} onsuggest={() => sectionOpen && suggestAgain(sectionOpen)} onclose={() => (sectionOpen = null)} />
  {/key}
{/if}
{#if folderOpen}
  {#key keyIn(folderOpen.s.id, folderOpen.rel)}
    <SectionSettings section={folderOpen.s} shown={catalog.sections.find(x => x.id === ui.merge[folderOpen!.s.id]) ?? folderOpen.s} {catalog} folder={{ f: folderOpen.f, rel: folderOpen.rel, also: alsoOf(folderOpen) }} reading={!!reading}
      onfolder={c => folderOpen && applyFolder(folderOpen.s, folderOpen.f, folderOpen.rel, c)} onsplit={(k, v) => folderOpen && makeSection(folderOpen.s, folderOpen.rel, k, v)} onclose={() => (folderOpen = null)} />
  {/key}
{/if}
<ToastBox />

<style>
  .skip { position: absolute; left: -9999px; top: 6px; z-index: 10; background: var(--field); padding: 6px 10px; border-radius: 6px; }
  .skip:focus { left: 8px; }
  .app { height: 100%; display: grid; grid-template-rows: 44px 1fr 30px; }
  .welcome { padding: 28px 40px 20px; display: flex; flex-direction: column; min-height: 0; max-width: 820px; width: 100%; margin: 0 auto; }
  .welcome h1 { margin: 0 0 6px; font-size: 26px; }
  .welcome p { margin: 0 0 16px; max-width: 70ch; }
  .go { display: flex; justify-content: flex-end; margin-top: 12px; }
  /* The read state gives way last: fitBar shortens the network line, then leaves out Cache and
     the version, before the state is cut. */
  footer { display: flex; gap: 18px; align-items: center; padding: 0 14px; font-size: 12px; color: var(--soft); background: var(--bar); border-top: 1px solid var(--line); white-space: nowrap; overflow: hidden; }
  footer .left { flex: 1; min-width: 0; display: flex; gap: 18px; align-items: center; overflow: hidden; }
  footer .left > * { flex: none; }
  footer .left .state { flex: 0 1 auto; min-width: 0; overflow: hidden; text-overflow: ellipsis; }
  footer .left .state:has(.pn) { display: flex; }
  .state .pn { min-width: 0; overflow: hidden; text-overflow: ellipsis; }
  .state .pa { flex: none; white-space: pre; }
  footer button { border: 0; background: none; color: var(--link); padding: 0 2px; min-height: 24px; font-size: 12px; flex: none; }
  footer button:disabled { color: var(--muted); }
  .live::before { content: ""; display: inline-block; width: 7px; height: 7px; border-radius: 50%; background: var(--done); margin-right: 6px; }
  footer .ver { color: var(--muted); }
  footer button.warn { color: var(--next-ink); background: var(--next); border-radius: 4px; padding: 0 6px; }
</style>
