// A book's name split into its title and its author, for the card: "Title (Author)",
// "Author (Title)", "Author - Title" or "Title - Author", with "Surname, Name" turned round and a
// last " - 2011" kept as the year. A side is an author when it reads as names of two to four
// capitalised words, joined by "&" or "and", with no word a title would have ("The", "Of",
// "Chronicles"). When only one side can be, it is the author; when both can, `byline` weighs how
// the names appear across the Section. Anything else stays whole, with no author.

const WORD = /^\p{Lu}[\p{L}'’.-]*$/u
const INITIAL = /^\p{Lu}\.?$/u
const TITLE_WORDS = new Set(['the', 'a', 'an', 'of', 'for', 'to', 'in', 'on', 'at', 'by', 'with', 'from', 'my', 'your', 'is', 'book', 'books', 'part',
  'vol', 'volume', 'saga', 'series', 'chronicles', 'collection', 'omnibus', 'trilogy', 'edition', 'complete',
  // Words of a course's handouts, now that a loose PDF in any Section gets a byline.
  'lesson', 'module', 'chapter', 'unit', 'week', 'intro', 'introduction', 'guide', 'notes', 'slides', 'handout', 'workbook',
  'worksheet', 'exercises', 'answers', 'summary', 'overview', 'setup', 'manual', 'handbook', 'reference', 'basics', 'cheat', 'sheet',
  'card', 'chart', 'list', 'table', 'tables', 'schedule', 'diagram', 'wheel', 'plan', 'map', 'template', 'checklist', 'recipe',
  'recipes', 'transcript', 'syllabus', 'quiz', 'exam', 'solutions', 'tips'])

function isName(s: string): boolean {
  const w = s.trim().split(/\s+/)
  return w.length >= 2 && w.length <= 4 && !TITLE_WORDS.has(w[0].toLowerCase())
    && w.every(x => INITIAL.test(x) || (WORD.test(x) && !TITLE_WORDS.has(x.toLowerCase()) && !/['’]s$/.test(x)))
}
/** "Heinlein, Robert A" as "Robert A Heinlein"; other names as they are. */
function turned(s: string): string {
  const m = /^([^,]{2,}), ([^,]+)$/.exec(s.trim())
  return m && isName(`${m[2]} ${m[1]}`) ? `${m[2]} ${m[1]}` : s.trim()
}
// A leading number orders the files ("04 Simmons, Dan"); it is not part of the name.
const author = (s: string) => { const t = turned(s.replace(/^\d+\s+/, '')); return t.split(/\s+(?:&|and)\s+/).every(isName) ? t : '' }
/** The same name however it is written: "Robert A. Heinlein" is "Robert A Heinlein". */
const key = (s: string) => s.toLowerCase().replace(/[^\p{L}]+/gu, ' ').trim()

export interface Byline { title: string; by: string; year: string }
interface Way extends Byline { at: 'paren' | 'outer' | 'last' | 'first'; sure: boolean }

/** Every way to read a name as a title and an author, the likelier first when nothing else tells. */
function readings(name: string): { ways: Way[]; whole: Byline } {
  const y = /^(.+?)\s*-\s*((?:19|20)\d\d)$/.exec(name)
  const [rest, year] = y ? [y[1], y[2]] : [name, '']
  const whole = { title: name, by: '', year: '' }
  const ways: Way[] = []
  const add = (title: string, by: string, at: Way['at']) => { const a = author(by); if (a && title.trim()) ways.push({ title: title.trim(), by: a, year, at, sure: a !== by.trim().replace(/^\d+\s+/, '') || a.split(/\s+/).some(x => INITIAL.test(x)) }) }
  const paren = /^(.+?)\s*\(([^()]+)\)$/.exec(rest)
  if (paren) { add(paren[1], paren[2], 'paren'); add(paren[2], paren[1], 'outer') }
  const parts = rest.split(' - ')
  if (parts.length > 1) { add(parts.slice(0, -1).join(' - '), parts.at(-1)!, 'last'); add(parts.slice(1).join(' - '), parts[0], 'first') }
  return { ways, whole }
}

/** Each name that could be an author across a Section's names: how often it appears, and in
 * which places ("- X" last, "X -" first, "(X)", "X (...)"). */
export type Authors = Map<string, { n: number; at: Set<string> }> & {
  /** A book outside a Books Section: only an author after the title, "Title - Author" or
   * "Title (Author)", splits off, so "Pasta Dough - Recipe Card" stays whole. */
  after?: boolean
}

export function authorsIn(names: string[], after = false): Authors {
  const n: Authors = new Map()
  if (after) n.after = true
  for (const name of names) {
    for (const w of readings(name).ways) {
      const e = n.get(key(w.by)) ?? { n: 0, at: new Set<string>() }
      e.n++
      e.at.add(w.at)
      n.set(key(w.by), e)
    }
  }
  return n
}

export function byline(name: string, known: Authors = new Map()): Byline {
  const read = readings(name), whole = read.whole
  const ways = known.after ? read.ways.filter(w => w.at === 'paren' || w.at === 'last') : read.ways
  if (ways.length < 2) return ways[0] ? strip(ways[0]) : whole
  // Both sides could be an author. One written as "Surname, Name", with an initial, or met in more
  // than one place ("Robert A. Heinlein (Red Planet)", "Space Cadet (Robert A. Heinlein)") is. So
  // is one met more than once at the end or in brackets ("Naked Empire - Terry Goodkind"). One met
  // more than once only at the front may as well be a series ("Star Wars - Lost Stars"): nothing is
  // split. Else the one in brackets, or the last.
  const of = (w: Way) => known.get(key(w.by))
  const most = (ws: Way[]) => strip(ws.reduce((a, b) => ((of(b)?.n ?? 0) > (of(a)?.n ?? 0) ? b : a)))
  const sure = ways.filter(w => w.sure || (of(w)?.at.size ?? 0) > 1)
  if (sure.length) return most(sure)
  const again = ways.filter(w => (of(w)?.n ?? 0) > 1)
  const trailing = again.filter(w => w.at === 'paren' || w.at === 'last')
  if (trailing.length) return most(trailing)
  if (again.length) return whole
  return strip(ways[0])
}
const strip = ({ title, by, year }: Way): Byline => ({ title, by, year })
