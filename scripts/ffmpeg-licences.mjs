// Collects the licence files of every library BtbN's FFmpeg build can link in, at the exact
// commits its build scripts pin, into scripts/ffmpeg-licences.json. fetch-tools.mjs ships them with
// FFmpeg. Run from the repo root when the pinned FFmpeg build changes:
// `node scripts/ffmpeg-licences.mjs <FFmpeg-Builds commit>`. Needs git and curl.
// The build scripts are read as text, never run. A script that only GPL or nonfree builds use is
// left out, since the app ships the LGPL build.
import { execFileSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { basename, join, relative } from 'node:path'

const commit = process.argv[2]
if (!/^[0-9a-f]{40}$/.test(commit ?? '')) { console.error('Give the full FFmpeg-Builds commit.'); process.exit(1) }
const WORK = join(tmpdir(), 'fvh-ffmpeg-licences')
mkdirSync(WORK, { recursive: true })
const git = (cwd, ...a) => execFileSync('git', a, { cwd, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'], maxBuffer: 64 << 20, timeout: 180_000 })
const LICENCE = /^(licen[cs]e|copying|copyright|notice|unlicense|patents)([._-].*)?$/i
const isLicence = (f) => LICENCE.test(f) && !/\.(cfg|py|sh|cmake|in|am|c|h|json|ya?ml|toml)$/i.test(f)

// Repositories whose licence is not in a file at the top: the files that hold it. `#comment` takes
// a file's first comment, `#from:<text>` the file from the line holding that text.
const ELSEWHERE = {
  'nv-codec-headers': ['include/ffnvcodec/nvEncodeAPI.h#comment', 'include/ffnvcodec/dynlink_loader.h#comment'],
  libglvnd: ['README.md#from:libglvnd itself'],
}
const cut = (t, how) => !how ? t
  : how === 'comment' ? t.match(/\/\*[\s\S]*?\*\//)[0].split('\n').map(l => l.replace(/^\s*\/?\*+\/?\s?/, '')).join('\n').trim() + '\n'
  : t.slice(t.indexOf(how.slice(5)))
// curl, since Node's fetch times out connecting to code.videolan.org where curl does not.
const get = (url) => execFileSync('curl', ['-sfL', '--retry', '3', '--max-time', '120', url], { encoding: 'utf8', maxBuffer: 64 << 20 })
const retry = async (f) => { for (let i = 0; ; i++) { try { return await f() } catch (e) { if (i === 3) throw e; await new Promise(r => setTimeout(r, 10_000 * (i + 1))) } } }

function checkout(name, repo, rev) {
  const d = join(WORK, name.replace(/[^\w.-]/g, '_'))
  if (!existsSync(join(d, '.git'))) { mkdirSync(d, { recursive: true }); git(d, 'init', '-q') }
  try { git(d, 'fetch', '-q', '--depth', '1', '--filter=blob:none', repo, rev) } catch (e) {
    if (/ 50[23]|timed out|ETIMEDOUT/.test(String(e.stderr ?? e))) throw e
    // A server that does not hand out a commit by its hash: fetch everything, then pick it.
    git(d, 'fetch', '-q', '--filter=blob:none', '--tags', repo, '+refs/heads/*:refs/remotes/o/*')
    git(d, 'update-ref', 'FETCH_HEAD', git(d, 'rev-parse', `${rev}^{commit}`).trim())
  }
  return d
}

// code.videolan.org stalls git clients, so its files come through GitLab's API.
async function gitlab(repo, rev) {
  const u = new URL(repo.replace(/\.git$/, '')), project = encodeURIComponent(u.pathname.slice(1))
  const api = `${u.origin}/api/v4/projects/${project}/repository`
  const tree = await retry(async () => JSON.parse(get(`${api}/tree?ref=${rev}&per_page=100`)))
  const out = {}
  for (const f of tree.filter(f => f.type === 'blob' && isLicence(f.name)))
    out[f.name] = await retry(async () => get(`${api}/files/${encodeURIComponent(f.path)}/raw?ref=${rev}`))
  return out
}
// LAME is in Subversion; its licence files are read over HTTP at the pinned revision.
async function svn(repo, rev) {
  const u = new URL(repo), at = u.pathname.indexOf('/svn/') + 5
  const out = {}
  for (const f of ['COPYING', 'LICENSE']) {
    try { out[f] = get(`${u.origin}${u.pathname.slice(0, at)}!svn/bc/${rev}/${u.pathname.slice(at)}/${f}`) } catch { /* not there */ }
  }
  return out
}

const builds = checkout('FFmpeg-Builds', 'https://github.com/BtbN/FFmpeg-Builds', commit)
git(builds, 'checkout', '-q', '-f', 'FETCH_HEAD')
const scripts = []
const walk = d => { for (const n of readdirSync(d).sort()) { const p = join(d, n); statSync(p).isDirectory() ? walk(p) : n.endsWith('.sh') && scripts.push(p) } }
walk(join(builds, 'scripts.d'))

const texts = {}
const parts = []
// A part already collected at the same commit is kept, so a run cut short by a flaky host resumes.
const OUT = join('scripts', 'ffmpeg-licences.json')
const before = existsSync(OUT) ? JSON.parse(readFileSync(OUT, 'utf8')) : null
const keep = t => { const id = createHash('sha256').update(t).digest('hex').slice(0, 16); texts[id] = t; return id }
for (const s of scripts) {
  const sh = readFileSync(s, 'utf8')
  const v = k => sh.match(new RegExp(`^${k}=["']?([^"'\\s]*)`, 'm'))?.[1]
  if (v('SCRIPT_SKIP') || /\$VARIANT == lgpl\*[^\n]*return -1|\$VARIANT == nonfree\*[^\n]*\|\| return -1/.test(sh)) continue
  if (!v('SCRIPT_REPO')) throw new Error(`${s} names no SCRIPT_REPO and has no SCRIPT_SKIP; read it and handle it here`)
  // The systems that build it, from its ffbuild_enabled.
  const on = sh.match(/ffbuild_enabled\(\) \{[\s\S]*?\n\}/)?.[0] ?? ''
  const systems = /\$TARGET != linux\*[^\n]*&& return|\$TARGET == linux\*[^\n]*\|\| return/.test(on) ? ['linux']
    : /\$TARGET == win\*[^\n]*\|\| return|\$TARGET != win\*[^\n]*&& return/.test(on) ? ['windows'] : ['windows', 'linux']
  for (const n of ['', '2', '3']) {
    const repo = v(`SCRIPT_REPO${n}`), rev = v(`SCRIPT_COMMIT${n}`) ?? v(`SCRIPT_REV${n}`)
    if (!repo) continue
    if (!rev) throw new Error(`${s}: SCRIPT_REPO${n} has no commit or revision`)
    let name = basename(s, '.sh').replace(/^\d+-/, '') + (n ? `-${basename(repo).replace(/\.git$/, '')}` : '')
    if (parts.some(p => p.name === name)) name += `-${n}`
    const files = {}
    const had = before?.parts.find(p => p.repo === repo && p.rev === rev && Object.keys(p.files).length)
    if (had) for (const [f, id] of Object.entries(had.files)) files[f] = keep(before.texts[id])
    if (!had) try {
      if (repo.includes('svn.')) {
        for (const [f, t] of Object.entries(await svn(repo, rev))) files[f] = keep(t)
      } else if (repo.includes('code.videolan.org')) {
        for (const [f, t] of Object.entries(await gitlab(repo, rev))) files[f] = keep(t)
      } else {
        const d = await retry(async () => checkout(name, repo, rev))
        const top = git(d, 'ls-tree', '--name-only', 'FETCH_HEAD').split('\n')
        const found = [...top.filter(isLicence),
          ...(top.includes('LICENSES') ? git(d, 'ls-tree', '-r', '--name-only', 'FETCH_HEAD', 'LICENSES/').split('\n').filter(Boolean) : []),
          ...(ELSEWHERE[basename(repo).replace(/\.git$/, '')] ?? [])]
        for (const at of found) {
          const [f, how] = at.split('#')
          files[f] = keep(cut(git(d, 'show', `FETCH_HEAD:${f}`), how))
        }
      }
    } catch (e) { console.error(`${name}: ${String(e.stderr ?? e).trim().split("\n")[0]}${e.cause ? ` (${e.cause.code ?? e.cause})` : ""}`) }
    if (!Object.keys(files).length) console.error(`${name}: no licence file at the top of ${repo}; add where it is to ELSEWHERE`)
    parts.push({ name, systems, script: relative(join(builds, 'scripts.d'), s).replaceAll('\\', '/'), repo, rev, files })
    console.log(`${name}: ${Object.keys(files).join(', ') || '-'}`)
  }
}
writeFileSync(OUT, JSON.stringify({ v: 1, builds: commit, parts, texts }, null, 1) + '\n')
rmSync(join(WORK, 'FFmpeg-Builds'), { recursive: true, force: true })
console.log(`\n${parts.length} parts in ${OUT}`)
const none = parts.filter(p => !Object.keys(p.files).length)
if (none.length) { console.error(`No licence for ${none.map(p => p.name).join(', ')}: run again, or add where it is to ELSEWHERE.`); process.exit(1) }
