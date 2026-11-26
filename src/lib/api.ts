// The Rust core's commands and the shapes they return (src-tauri/src/lib.rs, scan.rs, store.rs).
import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import { emit, listen } from '@tauri-apps/api/event'
import { sectionOf, unpack, unpackUnlisted, type CUnlisted, type CWork, type Heads } from './packed'

export type Kind = 'Courses' | 'Books' | 'Collections' | 'Files' | 'Photos'

export interface Drive { path: string; label: string; serial: number; kind: string; free: number; total: number; system: boolean
  /** Its folder names keep their case (Linux): `Books` and `BOOKS` are two folders. */
  keeps_case: boolean }
export interface Folder { name: string; path: string }
export interface CheckedFolder { serial: number; rel: string; last_path: string; unchecked: string[]; name: string | null; kind: Kind | null
  /** A Split-out Section (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`), ticked as part of the folder that holds it. */
  split?: boolean }
/** How the window looks, kept in settings.json. */
export type CardSize = 'compact' | 'normal' | 'large'
export interface Ui {
  colw?: Record<string, number>; size?: CardSize; autohide?: boolean; unlisted?: boolean
  /** How Settings, Log and Settings, Programs show their lists. */
  logView?: LogView; programsView?: ProgramsView
  /** Sections shown inside another, by id (`decisions/2026/10/05/checked-folders-on-any-drive.md`). */
  merge?: Record<string, string>
  /** The Photos view: photos and clips or one of them, Best only or all, and the first day of the week (0 Sunday, 1 Monday). */
  photoKind?: '' | 'p' | 'c'; photoAll?: boolean; week?: number
  /** The reminder to back up the checked folders was put away for good. */
  backupSeen?: boolean
  /** Subfolder rows left open, by Section id and folder path joined with `/`; the rest are closed. */
  open?: Record<string, boolean>
  /** The cards' order per Section id, and `home` for Home's tabs: `name`, `added` or `length`, `-` before it the other way; none is the order they come in. */
  sort?: Record<string, string>
  /** The view picked per Section and level, as `views[section][level] = view`; a missing one is the default. */
  views?: Record<string, Record<string, string>>
  /** Kept before views per Section: 'all' showed every Work below in every folder. Read once, never written. */
  folders?: 'tiles' | 'all'
}
/** A folder's own kind and views inside a Section (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`): none is Same as the folder above. */
export interface FolderSetting { kind?: Kind | null; views?: Record<string, string> }
export interface Settings { v: number; folders: CheckedFolder[]; cache_dir: string | null; ui: Ui | null; open_with: Record<string, string>
  /** The gap in hours that starts a new Photos event. */
  photo_gap: number
  /** Folder kinds and views by Drive and path, `{serial}:{path on the Drive}` (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`). */
  folder_kinds?: Record<string, FolderSetting>
  /** The Sections whose offer of a kind per folder was answered or put away (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`). */
  offered?: string[]
}
/** What an item is to its Work: lessons, tracks and titles take Ticks; the rest do not (`decisions/2026/09/29/ticks-offer-the-mark-never-set-it.md`). */
export type Role = 'main' | 'doc' | 'cc' | 'code'
export interface Item {
  name: string; rel: string; path: string; ext: string; size: number; mtime: number; cloud: boolean; key: string
  title: string; group: string; role: Role; lang?: string
  /** In a Photos Section, when it was taken, as Work.taken. */
  at?: number
}
export interface Work {
  id: string; title: string; path: string; folder: string[]; is_folder: boolean; items: Item[]
  media: string[]; size: number; added: number; cloud: number; year: number | null
  /** Set only in merged Sections. The drives a Work is on: both copies of one, or the one drive
   * of a different file at the same place as another. */
  drives?: string[]
  /** The ids of its other copies on other drives. */
  copies?: string[]
  /** An Event of a Photos Section (`decisions/2026/10/08/photos-is-a-section-kind-in-the-app.md`), and its first and last time taken, as seconds of
   * the camera's clock read as UTC; no times for the Undated Event. */
  event?: boolean
  taken?: [number, number]
  /** An Event of Photos whose names give only the day: `taken` is that day's start, the time is not known. */
  day_only?: boolean
}
export type Reason = 'unplaced' | 'unknown' | 'captions' | 'aside' | 'program' | 'setaside' | 'preview'
export interface Unlisted { name: string; rel: string; path: string; folder: string[]; reason: Reason; size: number; mtime: number; cloud: boolean; work: string | null }
export interface Section {
  id: string; name: string; path: string; drive: string; kind: Kind; guessed: boolean; online: boolean
  /** Its Drive is connected but the folder is not there: moved, renamed or deleted. */
  gone?: boolean
  works: Work[]; files: number; scanned_at: number; unlisted: Unlisted[]
  /** A Split-out Section's: the id of the Section it came from (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`). */
  split_from?: string
  /** The Split-out Sections made from its folders, each shown in its place as a link to it. */
  links?: Link[]
}
/** A folder split out as a Section of its own (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`): that Section's id, the folder's path from
 * the Section's folder, and the titles of the folders down to it, as a Work's `folder`. */
export interface Link { id: string; rel: string; folder: string[] }
/** A top-level folder the app offers a kind of its own (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`): its name on disk, its title, the kind it guesses and its files. */
export interface Suggestion { rel: string; title: string; kind: Kind; files: number }
/** The last move of a Section Undo can take back (`decisions/2026/10/09/review-moves-set-aside-photos-itself-with-undo.md`): its id, when (seconds), how many and where. */
export type LastMove = [number, number, number, string]
/** A Photos Section, or a folder set to Photos inside a Section (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`): what Review's move
 * takes from. `id` is what its Events' ids start with, the Section's id or that and `/` and the
 * folder's path; `home` and `under` are what is read again after a move; `works` are its events. */
/** What an Event's id starts with, its Photos Section's or Photos folder's id (scan.rs `place_of`):
 * the id less `#` and its first Photo's path without the extension, cut off the end, as a folder's
 * name may hold `#`. Any other Work's id is its own. */
export function eventPlace(w: Work): string {
  const r = w.items[0]?.rel
  if (!w.event || r === undefined) return w.id
  const dot = r.lastIndexOf('.'), stem = dot >= 0 ? r.slice(0, dot) : r
  return w.id.endsWith('#' + stem) ? w.id.slice(0, w.id.length - stem.length - 1) : w.id
}
/** The start of every key inside Section or folder `id`: `id/`, or for a whole drive's Section (id
 * `0A1B2C3D:`) the id alone, so a key is its drive and its path on it (store.rs `inside`). */
// The length first: this runs for every Work of a folder that can hold 150,000.
const isDrive = (id: string) => id.length === 9 && /^[0-9A-Fa-f]{8}:$/.test(id)
export const inside = (id: string) => (isDrive(id) ? id : id + '/')
/** The key of path `rel` inside Section or folder `id`; `id` itself when `rel` is empty. */
export const keyIn = (id: string, rel: string) => (rel ? inside(id) + rel : id)
/** The path inside `id` of a key inside it, '' for `id` itself; for an Event of a Photos Section
 * or folder (`{id}#{stem}`), its stem. In a whole drive's Section, where a file's name may start
 * with `#`, an Event's is `#{stem}`. */
export const relIn = (key: string, id: string) =>
  !isDrive(id) && key.startsWith(id + '#') ? key.slice(id.length + 1) : key.startsWith(inside(id)) ? key.slice(inside(id).length) : ''
export interface PhotoPlace { id: string; name: string; home: Section; under: string[]; works: Work[] }
/** Where Move was opened from inside a PhotoPlace (a year, a month, what Review went through): its name and Works. */
export interface MoveScope { name: string; works: Work[] }
/** Where Review's move puts a Section's photos set aside, and what it would take. */
export interface MovePlan {
  count: number; cloud: number; to: string; picked: boolean
  /** Each photo set aside: its path inside the photo folder, the Insta360 previews of its shot
   * (counted on the shot's first video), and the shot, or "". A preview goes only when every video of its shot moves. */
  photos: [string, number, string][]
  /** The Insta360 previews (.lrv) that go with the set-aside videos; not in `count`. */
  previews: number
  /** The synced folder (Dropbox, OneDrive) the photos leave when the folder they go into is outside it. */
  leaves_sync: string | null
  last: LastMove | null
}
/** What a move or its Undo did: how many, and each one skipped with why. */
export interface Moved { id: number; moved: number; skipped: [string, string][] }
/** A file's length in seconds, or a PDF's pages. */
export interface Detail { s?: number; p?: number }
export interface Catalog { v: number; sections: Section[] }
/** A Section's cards as a kind, from `preview_kind`: the first ones, how many in all, every
 * title (Books only, for authors), the thumbnails already made, and the Works whose Marks would be set aside. */
export interface KindPreview { kind: Kind; total: number; works: Work[]; titles: string[]; thumbs: Thumb[]; marks: number
  /** Orphaned Marks this kind brings back: Works with a Mark or Ticks it makes again. */
  back: number
  /** The cards are what a read makes; false until a Section read in an earlier version is read again. */
  exact: boolean }
export interface Mark { m: 'next' | 'done'; at: number }
/** A Review choice on a Photo: kept, or set aside for the move script (`decisions/2026/10/04/photos-take-no-marks-review-is-the-progress.md`). */
export interface Choice { keep: boolean; at: number; size?: number }
export interface Marks { v: number; marks: Record<string, Mark>; ticks: Record<string, Record<string, number>>; review?: Record<string, Choice>
  /** When each Work was last opened from the app, by Work id, in seconds. */
  opened?: Record<string, number> }
export type Why = 'clip' | 'shake' | 'sharp'
export type Junk = 'whatsapp' | 'screenshot' | 'dark' | 'blurred'
/** What Review says about a Photo (shots.rs): its set of Similar shots and whether it is the Best
 * one, a close call (Your call), a Copy of another file, or Junk. */
/** What Review says of a Photo (shots.rs Note); in a set or a group of Copies also its size in
 * pixels and how sharp it is against the others of its set (`detail`) and of its group of Copies
 * (`copy_detail`), 1 being sharpest at every spot. */
export interface Note { set?: string; best?: boolean; call?: boolean; why?: [Why, number]; copy?: string; junk?: Junk; px?: [number, number]; cam?: boolean; version?: boolean; detail?: number; copy_detail?: number }
/** Notes by file path, and pictures still to be read for Review. */
export interface Sorting { notes: Record<string, Note>; left: number }
/** What adding from a backup did: marks and ticks added, settings applied. */
export interface Imported { marks: Marks; added: number; ticks: number; ui: boolean; file: string }
export interface BackupFile { file: string; folder: string }
export interface Place { what: string; path: string; size: number }
export interface Tool { id: string; name: string; version: string; licence: string; files: string[]; source: string; found: boolean }
export interface Info {
  version: string; build: string; places: Place[]; cache_dir: string; cache_default: boolean; tools_dir: string; tools: Tool[]
  /** Folder names keep their case on this system (Linux), for a drive not listed. */
  keeps_case: boolean
}
/** tools/notices.json, made by scripts/notices.mjs. */
export interface NoticeItem { name: string; version: string; licence: string; sources: { what: string; url: string }[]; texts: { file: string; text: string }[] }
export interface NoticeGroup { id: string; items: NoticeItem[]; source?: string }
export interface NoticeList { v: number; version: string; system: string; groups: NoticeGroup[]; texts: Record<string, string> }
export interface Thumb {
  id: string; key: string; w: number; h: number; n: number
  /** The site a book's web cover came from (`openlibrary`, `fantlab`); none for a file's picture. */
  from?: string
}
/** A cover found on the web: the picture to keep, a small one to choose by, and the book. */
export interface FoundCover { url: string; preview: string; from: string; title: string; by: string; year?: number }
export interface CoverResults { from: string; found: FoundCover[]; error?: string }
/** A cover the person chose, kept in StoredData: `own`, or the site. */
export interface Chosen { file: string; from: string }
/** What Choose a cover… is for: a Work and its copies, or a folder tile (`dir:` and its place). */
export interface CoverTarget { ids: string[]; name: string }
/** The id of a folder tile's chosen cover: a Section's, or a folder's in it. */
export const folderCoverId = (sectionId: string, f: string[]) => f.length ? `dir:${sectionId}/${f.join('/')}` : `dir:${sectionId}`

export interface Thumbs {
  done: Thumb[]; left: number
}
/** The Settings pages. */
export type Tab = 'folders' | 'sections' | 'marks' | 'programs' | 'storage' | 'about' | 'log' | 'help' | 'licences'
export type LogView = 'sessions' | 'table' | 'summary'
export type ProgramsView = 'table' | 'programs'
/** A log entry (logs.rs): its message in parts, the paths marked so they can be links. */
export interface LogEntry { date: string; time: string; utc: boolean; level: string; topic: string; parts: { text: string; path: boolean }[] }
/** A program, by what `open_with` stores (programs.rs). An empty id: one the system names but offers no handler for. */
export interface Program { id: string; name: string }
export interface ProgramRow { ext: string; group: string; files: number; reveals: boolean; yours: Program | null; gone: boolean; default: Program | null; choices: Program[] }
/** The rows, and the icons as data URLs by program id (or name, for a program with no id). */
export interface ProgramList { rows: ProgramRow[]; icons: Record<string, string>; listed: boolean }
export interface Picked { checked: string[]; unchecked: string[] }

/** What the Review window shows: its events by id, its title, the files it goes through
 * when only some are (Photos only, Clips only), and the Photo it opens at. */
export interface ReviewRequest { works: string[]; label: string; files?: string[]; start?: string }
/** What the Review window tells the main window: choices made, a move (its Section is read again),
 * or Review closed with photos marked Not needed (`n`, of the event `work`). */
export type ReviewNews = { what: 'marks' } | { what: 'moved' } | { what: 'closed'; n: number; work?: string }

export const api = {
  drives: () => invoke<Drive[]>('drives'),
  /** The app's own licence, and notices.json as text when it is there. */
  notices: () => invoke<{ licence: string; notices: string | null; file: string }>('notices'),
  openWeb: (url: string) => invoke<void>('open_web', { url }),
  folders: (path: string) => invoke<Folder[]>('folders', { path }),
  settings: () => invoke<Settings>('settings'),
  setUi: (ui: Ui) => invoke<void>('set_ui', { ui }),
  picked: () => invoke<Picked>('picked'),
  setFolders: (checked: string[], unchecked: string[]) => invoke<Settings>('set_folders', { checked, unchecked }),
  catalog: async () => whole(await invoke<Heads>('catalog')),
  scan: async (only: string | null = null, under: string[] | null = null) => whole(await invoke<Heads>('scan', { only, under })),
  /** Marks the Sections on a drive unplugged since they were read not connected; their ids. */
  drivesAway: () => invoke<string[]>('drives_away'),
  forgetFolder: async (id: string) => whole(await invoke<Heads>('forget_folder', { id })),
  /** Sets a Section's kind, and its name only with `rename`. */
  setSection: (id: string, name: string | null, kind: Kind | null, rename: boolean) => invoke<Settings>('set_section', { id, name, kind, rename }),
  /** A Section's cards as `kind`, or as the kind the app would guess with null; reads no folder, saves nothing. */
  previewKind: (id: string, kind: Kind | null, show: number) => invoke<KindPreview>('preview_kind', { id, kind, show }),
  stopPreview: () => invoke<void>('stop_preview'),
  /** A folder's cards with `kind` as its own (null: Same as the folder above), and `more` folders of the Section with theirs: the cards in them and the Marks set aside in the Section. Reads no folder, saves nothing. */
  previewFolder: (id: string, rel: string, kind: Kind | null, more: Record<string, Kind | null> | null = null) => invoke<KindPreview>('preview_folder', { id, rel, kind, more }),
  /** Sets a folder's own kind and views, by its id: its Section's id, `/`, its path in the Section. */
  setFolder: (id: string, kind: Kind | null, views: Record<string, string>) => invoke<Settings>('set_folder', { id, kind, views }),
  /** Makes a folder, by its key, a Split-out Section; returns its id (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`). */
  splitFolder: (id: string, kind: Kind, at: number | null = null) => invoke<string>('split_folder', { id, kind, at }),
  /** Puts a Split-out Section back as a folder with this kind and views; returns the Section it went into. */
  putBack: (id: string, kind: Kind, views: Record<string, string>) => invoke<string>('put_back', { id, kind, views }),
  /** A Section's top-level folders that look like another kind, from the last read: none once the offer was answered or put away, unless `again`. */
  suggestKinds: (id: string, again = false) => invoke<Suggestion[]>('suggest_kinds', { id, again }),
  /** Keeps that a Section's offer of a kind per folder was answered or put away, or makes it again. */
  setOffered: (id: string, offered: boolean) => invoke<Settings>('set_offered', { id, offered }),
  /** Folders whose own kind was dropped at a read, renamed or moved on disk: (Section name, folder name). */
  onKindsLost: (f: (lost: [string, string][]) => void) => listen<[string, string][]>('kinds-lost', e => f(e.payload)),
  renameSection: async (id: string, name: string | null) => whole(await invoke<Heads>('rename_section', { id, name })),
  marks: () => invoke<Marks>('marks'),
  setMark: (id: string, mark: 'next' | 'done' | null) => invoke<Marks>('set_mark', { id, mark }),
  /** Many Marks at once, each as given or none. */
  putMarks: (marks: Record<string, Mark | null>) => invoke<Marks>('put_marks', { marks }),
  setTicks: (id: string, keys: string[], on: boolean) => invoke<Marks>('set_ticks', { id, keys, on }),
  moveMark: (from: string, to: string) => invoke<Marks>('move_mark', { from, to }),
  forgetMarks: (ids: string[], items: [string, string][]) => invoke<Marks>('forget_marks', { ids, items }),
  /** Opens a file; a file only in the cloud opens only with `download`, else its size comes back. */
  open: (path: string, download = false) => invoke<number | null>('open', { path, download }),
  /** Shows a file or folder selected in its parent folder; `open` opens a folder itself. */
  reveal: (path: string, open = false) => invoke<void>('reveal', { path, open }),
  info: () => invoke<Info>('info'),
  pickFolder: () => invoke<string | null>('pick_folder'),
  setCacheDir: (path: string | null, carry: boolean) => invoke<string>('set_cache_dir', { path, carry }),
  setAside: () => invoke<string[]>('set_aside'),
  readLog: () => invoke<string>('read_log'),
  readNetworkLog: () => invoke<string>('read_network_log'),
  logEntries: (which: 'app' | 'network') => invoke<LogEntry[]>('log_entries', { which }),
  programList: () => invoke<ProgramList>('program_list'),
  details: () => invoke<Record<string, Detail>>('details'),
  onDetails: (f: (d: Record<string, Detail>) => void) => listen<Record<string, Detail>>('details', e => f(e.payload)),
  exportMarks: () => invoke<BackupFile | null>('export_marks'),
  importMarks: () => invoke<Imported | null>('import_marks'),
  clearMarks: () => invoke<Marks>('clear_marks'),
  setOpened: (id: string) => invoke<Marks>('set_opened', { id }),
  clearOpened: () => invoke<Marks>('clear_opened'),
  setOpenWith: (ext: string, program: string | null) => invoke<Settings>('set_open_with', { ext, program }),
  pickProgram: () => invoke<string | null>('pick_program'),
  thumbs: () => invoke<Thumbs>('thumbs'),
  rebuildThumbs: (ids: string[]) => invoke<void>('rebuild_thumbs', { ids }),
  // Choose a cover… (`decisions/2026/10/22/covers-are-chosen-never-fetched-by-themselves.md`): only findCovers, coverPreview and useWebCover go online.
  coverWords: (name: string) => invoke<string>('cover_words', { name }),
  findCovers: (words: string) => invoke<CoverResults[]>('find_covers', { words }),
  coverRound: () => invoke<number>('cover_round'),
  coverPreview: (url: string, round: number) => invoke<string>('cover_preview', { url, round }),
  useWebCover: (ids: string[], url: string, from: string) => invoke<void>('use_web_cover', { ids, url, from }),
  pickCoverFile: () => invoke<string | null>('pick_cover_file'),
  readCover: (data: string) => invoke<string>('read_cover', { data }),
  keepCover: (ids: string[], data: string) => invoke<void>('keep_cover', { ids, data }),
  hasCover: (ids: string[]) => invoke<boolean>('has_cover', { ids }),
  removeCover: (ids: string[]) => invoke<Record<string, Chosen>>('remove_cover', { ids }),
  restoreCover: (covers: Record<string, Chosen>) => invoke<void>('restore_cover', { covers }),
  wantThumbs: (ids: string[]) => invoke<void>('want_thumbs', { ids }),
  onThumbs: (f: (t: Thumbs) => void) => listen<Thumbs>('thumbs', e => f(e.payload)),
  photoSort: () => invoke<Sorting>('photo_sort'),
  onShots: (f: (left: number) => void) => listen<number>('shots', e => f(e.payload)),
  setReview: (photos: [string, number][], keep: boolean) => invoke<Marks>('set_review', { photos, keep }),
  forgetReview: (keys: string[]) => invoke<Marks>('forget_review', { keys }),
  restoreReview: (choices: Record<string, Choice>) => invoke<Marks>('restore_review', { choices }),
  saveMoveScript: (id: string, files: string[]) => invoke<string | null>('save_move_script', { id, files }),
  movePlan: (id: string) => invoke<MovePlan>('move_plan', { id }),
  setMoveTo: (id: string, path: string | null) => invoke<MovePlan>('set_move_to', { id, path }),
  moveSetAside: (id: string, files: string[]) => invoke<Moved>('move_set_aside', { id, files }),
  undoMove: (moveId: number) => invoke<Moved>('undo_move', { moveId }),
  lastMove: (id: string) => invoke<LastMove | null>('last_move', { id }),
  onMoveProgress: (f: (p: { done: number; total: number }) => void) => listen<{ done: number; total: number }>('move-progress', e => f(e.payload)),
  setPhotoGap: (hours: number) => invoke<Settings>('set_photo_gap', { hours }),
  openReview: (request: ReviewRequest) => invoke<void>('open_review', { request }),
  reviewRequest: () => invoke<ReviewRequest | null>('review_request'),
  onReviewRequest: (f: () => void) => listen('review-request', () => f()),
  tellReview: (news: ReviewNews) => emit('review-news', news),
  onReviewNews: (f: (news: ReviewNews) => void) => listen<ReviewNews>('review-news', e => f(e.payload)),
  /** The main window read folders again: the Review window takes its steps again when it comes back. */
  tellCatalog: () => emit('catalog-news'),
  onCatalogNews: (f: () => void) => listen('catalog-news', () => f()),
  onScanProgress: (f: (p: { section: string; files: number }) => void) => listen<{ section: string; files: number }>('scan-progress', e => f(e.payload)),
}

/** What a part asked for says when the catalog changed since its heads were sent (lib.rs `CHANGED`). */
const CHANGED = 'The catalog changed while it was sent.'

/** The catalog from its heads: each Section's Works and unlisted files are asked for in parts and
 * made whole, so no reply nears the longest string the page holds. A catalog changed
 * meanwhile starts again. */
async function whole(heads: Heads): Promise<Catalog> {
  for (let tries = 0; ; tries++) {
    try {
      const sections: Section[] = []
      for (const h of heads.sections) {
        const works: Work[] = []
        while (works.length < h.w) {
          const part = await invoke<CWork[]>('catalog_part', { stamp: heads.stamp, id: h.id, from: works.length })
          if (!part.length) throw CHANGED
          unpack(h, part, works)
        }
        const unlisted: Unlisted[] = []
        while (unlisted.length < h.u) {
          const part = await invoke<CUnlisted[]>('catalog_unlisted', { stamp: heads.stamp, id: h.id, from: unlisted.length })
          if (!part.length) throw CHANGED
          unpackUnlisted(h, part, unlisted)
        }
        sections.push(sectionOf(h, works, unlisted))
      }
      return { v: heads.v, sections }
    } catch (e) {
      if (String(e) !== CHANGED || tries >= 5) throw e
      heads = await invoke<Heads>('catalog')
    }
  }
}

// Thumbnails are served as never changing; one made again under the same key needs a new address.
let round = 0
export const newRound = () => { round++ }

/** A thumbnail from the cache, served by the app's `thumb` protocol. */
export const thumbSrc = (key: string) => convertFileSrc(key, 'thumb') + (round ? `?r=${round}` : '')

/** A catalog read before items had roles: every item counts as a lesson, track or title. */
const roleOf = (i: Item): Role => i.role ?? 'main'
/** The lessons, tracks or titles of a Work: what takes Ticks. */
export const mains = (w: Work) => w.items.filter(i => roleOf(i) === 'main')
/** A Work takes Ticks when it has items of its own: a course's lessons, an album's tracks (`decisions/2026/09/29/ticks-offer-the-mark-never-set-it.md`). */
export const tickable = (w: Work) => w.is_folder && !w.event && mains(w).length > 1
/** A folder Work with more than one file opens its dialog, even with one lesson: its handouts,
 * captions and code are there (the Hub's rule). Any other Work opens its file. */
export const hasDialog = (w: Work) => w.is_folder && w.items.length > 1
/** The file a card opens: its first lesson, track or title, never a handout. */
export const firstFile = (w: Work): Item | undefined => mains(w)[0] ?? w.items[0]
const PICTURE_OR_BOOK = new Set(['jpg', 'jpeg', 'png', 'gif', 'webp', 'bmp', 'tif', 'tiff', 'heic', 'insp', 'pdf', 'epub', 'djvu', 'djv', 'fb2', 'mobi',
  'azw3', 'kfx', 'cbz', 'cbr', 'doc', 'docx', 'rtf', 'txt', 'odt', 'chm', 'ppt', 'pptx', 'ppsx', 'pps', 'odp', 'xls', 'xlsx', 'md'])
const BOOK = new Set(['pdf', 'epub', 'djvu', 'djv', 'fb2', 'mobi', 'azw3', 'kfx', 'cbz', 'cbr'])
/** A Work of one book file: in any kind of Section it gets the Books card, portrait with its author apart. */
export const isBook = (w: Work) => !w.is_folder && !w.event && BOOK.has(firstFile(w)?.ext ?? '')
/** A folder Work of pictures or books shows its titles as a grid with thumbnails (thumbs.rs `grid`). */
export const grid = (w: Work) => { const m = mains(w); return w.is_folder && m.length > 1 && m.length <= GRID_MAX && m.every(i => PICTURE_OR_BOOK.has(i.ext) || (w.event && VIDEO.has(i.ext))) }
/** What Show in File Explorer selects for a Work: an Event's first photo, else its file or folder. */
export const shownOf = (w: Work) => (w.event && w.items[0]?.path) || w.path
const GRID_MAX = 1000
/** The word for done, by what a Work is mostly for. */
export const doneWord = (w: Work) => ({ Watching: 'Watched', Reading: 'Read', Listening: 'Listened' } as Record<string, string>)[w.media[0]] ?? 'Done'
/** A length as 3:20 or 1:02:05. */
export function clock(s: number): string {
  s = Math.round(s)
  const two = (n: number) => String(n).padStart(2, '0')
  return s >= 3600 ? `${Math.floor(s / 3600)}:${two(Math.floor(s / 60) % 60)}:${two(s % 60)}` : `${Math.floor(s / 60)}:${two(s % 60)}`
}
/** Text folded for search: case, accents and ё aside. */
export const fold = (s: string) => s.normalize('NFKD').replace(/[\u0300-\u036f]/g, '').toLowerCase().replace(/ё/g, 'е').replace(/й/g, 'и')

/** Folder names in the order a person reads them: "2 Basics" before "10 Advanced". One shared
 * collator: `localeCompare` with options builds one per call, 30 times slower on a large library. */
export const natural = new Intl.Collator(undefined, { numeric: true, sensitivity: 'base' }).compare

/** An item's Tick key; a catalog read before items had keys uses the path. */
export const keyOf = (i: Item) => i.key || i.rel
/** A Work's Tick keys, once each: a lesson's video and handout share one. */
export const tickKeys = (w: Work) => [...new Set(mains(w).map(keyOf))]
/** Of copies of one Work in a merged Section, the id a card keeps: the copy with the newest Mark,
 * else the first, so the last Mark the person set is the one it shows. */
export function keptOf(ids: string[], m: Marks): string {
  let kept = ids[0], at = -Infinity
  for (const id of ids) { const x = m.marks[id]; if (x && x.at > at) { kept = id; at = x.at } }
  return kept
}
/** A Work's Ticks: in a merged Section, those of every copy of it, each at its earliest date as
 * a backup's are. A Work with no copies gets its own record, not a new one. */
export function ticksOf(w: Work, m: Marks): Record<string, number> | undefined {
  if (!w.copies?.length) return m.ticks[w.id]
  let out: Record<string, number> | undefined
  for (const id of [w.id, ...w.copies]) for (const [k, at] of Object.entries(m.ticks[id] ?? {})) {
    out ??= {}
    out[k] = k in out ? Math.min(out[k], at) : at
  }
  return out
}
/** The ids clearing a Work's Mark writes to: its own, and each copy with a Mark. */
export const markedIds = (w: Work, m: Marks) => [w.id, ...(w.copies ?? []).filter(c => m.marks[c])]
/** The items a Tick writes to a Work's own id: those it and its copies have not ticked yet. */
export const tickAdds = (w: Work, keys: string[], m: Marks) => { const t = ticksOf(w, m) ?? {}; return keys.filter(k => !(k in t)) }
/** The ids an untick of these items writes to: the Work's own, and each copy that has any of them. */
export const untickIds = (w: Work, keys: string[], m: Marks) =>
  [w.id, ...(w.copies ?? []).filter(c => keys.some(k => k in (m.ticks[c] ?? {})))]

export function size(n: number): string {
  if (n < 1024) return `${n} B`
  const u = ['KB', 'MB', 'GB', 'TB']
  let i = -1
  do { n /= 1024; i++ } while (n >= 1024 && i < u.length - 1)
  return `${n < 10 ? n.toFixed(1) : Math.round(n)} ${u[i]}`
}

export function ago(secs: number): string {
  const d = Math.max(0, Date.now() / 1000 - secs)
  if (d < 60) return 'just now'
  if (d < 3600) return `${Math.floor(d / 60)} min ago`
  if (d < 86400) return `${Math.floor(d / 3600)} h ago`
  return `${Math.floor(d / 86400)} days ago`
}

export const COLORS = ['#D9822B', '#3A7BD5', '#8A63C2', '#2E9E6A', '#C2456B', '#2A9BA8', '#9C7B2E', '#5B6BD5']

const VIDEO = new Set(['mp4', 'mkv', 'avi', 'mov', 'webm', 'm4v', 'wmv', 'flv', 'ts', 'mpg', 'mpeg', '3gp', 'vob', 'insv', 'lrv'])
const AUDIO = new Set(['mp3', 'm4a', 'm4b', 'aac', 'flac', 'ogg', 'opus', 'wav', 'wma'])
/** A title as drawn: a leading number ("1. ", "02 ", "3) ", "4 - "), kept or a title's own, stays on the
 * line of the word after it, so a long word never leaves it alone on the first line. */
export const glued = (t: string) => t.replace(/^(\d{1,3})(\.|\)| -)? (?=\S)/, (_, n: string, m = '') => `${n}${m.replace(' ', '\u00a0')}\u00a0`)
/** A folder name as shown: a Photos month folder "02 February" reads "February"; a number kept glued. */
export const place = (name: string) => glued(name.replace(/^(0[1-9]|1[0-2]) (?=(January|February|March|April|May|June|July|August|September|October|November|December)$)/, ''))

/** An Event's time span: "17:44", or "17:44 to 19:54"; none when the time is not known. */
export function span(w: Work): string {
  if (!w.taken || w.day_only) return ''
  const hm = (t: number) => new Date(t * 1000).toISOString().slice(11, 16)
  const [a, b] = w.taken
  return b - a >= 60 ? `${hm(a)} to ${hm(b)}` : hm(a)
}

export function noun(w: Work, n: number): string {
  if (w.event) {
    const clips = w.items.filter(i => VIDEO.has(i.ext)).length, photos = w.items.length - clips
    return [photos && `${photos} photo${photos === 1 ? '' : 's'}`, clips && `${clips} clip${clips === 1 ? '' : 's'}`].filter(Boolean).join(', ')
  }
  return `${n} ${unit(w)}${n === 1 ? '' : 's'}`
}
/** What one main item of a Work is called: lesson, track, title or file. */
export function unit(w: Work): string {
  const m = mains(w), all = (s: Set<string>) => m.length > 0 && m.every(i => s.has(i.ext))
  return all(VIDEO) ? 'lesson' : all(AUDIO) ? 'track' : grid(w) ? 'title' : 'file'
}

/** The line on a Work's picture: "12 lessons, 3:20:00", "PDF, 240 pages", "42:10". */
export function metaOf(w: Work, details: Record<string, Detail>): string {
  const m = mains(w)
  if (w.event) return noun(w, m.length)
  if (w.is_folder) {
    const secs = m.reduce((a, i) => a + (details[i.path]?.s ?? 0), 0)
    return secs ? `${noun(w, m.length)}, ${clock(secs)}` : noun(w, m.length)
  }
  const first = m[0] ?? w.items[0]
  if (!first) return ''
  const d = details[first.path]
  if (d?.s) return clock(d.s)
  if (d?.p) return `${first.ext.toUpperCase()}, ${d.p} ${d.p === 1 ? 'page' : 'pages'}`
  return first.ext.toUpperCase()
}
