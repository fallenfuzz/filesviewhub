// What the app is built from that has a newer version, and which locked versions have a known
// security advisory. It changes nothing; run it from WSL (or any system with node, npm and cargo):
//   node scripts/updates.mjs                     everything below
//   node scripts/updates.mjs --advisories        only the security advisories (npm audit, OSV) and the
//                                                pinned tools' age, about 5 s; the release workflow runs it
// It covers the npm packages (npm outdated, npm audit), the Rust crates (cargo update --dry-run,
// and every crate in Cargo.lock against the OSV advisory database), the tools pinned in
// fetch-tools.mjs (FFmpeg, PDFium, the MSYS2 packages of ddjvu) and the Rust and Node toolchains.
// Only package names and versions are sent: to the npm registry, crates.io, api.osv.dev, GitHub
// and repo.msys2.org. Exit code 1 when an advisory has a fixed version not yet used, or when a pinned
// tool is stale: a newer upstream release has been out for more than STALE_DAYS.
// No advisory database covers these builds, so their age stands in for one.
import { execSync, spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const app = join(dirname(fileURLToPath(import.meta.url)), '..');
const tauri = join(app, 'src-tauri');
const UA = { 'User-Agent': 'FilesViewHub-dev-updates' };
// The job's token stays with this script: no child command gets it.
const { GITHUB_TOKEN: _token, ...parent } = process.env;
const env = { ...parent, PATH: `${process.env.HOME}/.cargo/bin:${process.env.PATH}` };
const sh = (cmd, cwd) => spawnSync(cmd, { cwd, shell: true, encoding: 'utf8', env, maxBuffer: 1 << 26 });
// On GitHub Actions the job's token lifts GitHub's limit of 60 requests an hour per address.
const auth = (url) => (process.env.GITHUB_TOKEN && url.startsWith('https://api.github.com/') ? { Authorization: `Bearer ${process.env.GITHUB_TOKEN}` } : {});
// A source that answers with an error stops the script (exit 1), so the check never passes unread.
const get = async (url, init) => {
  const res = await fetch(url, init);
  if (!res.ok) throw new Error(`${url}: HTTP ${res.status}`);
  return res;
};
const json = async (url, body) => (await get(url, body
  ? { method: 'POST', headers: { ...UA, 'Content-Type': 'application/json' }, body: JSON.stringify(body) }
  : { headers: { ...UA, ...auth(url) } })).json();
const only = process.argv.includes('--advisories');
let fixable = 0;
let stale = 0;
// Checks that could not be made: a source that changed its form or gave no date. Counted as failed.
let unread = 0;
const failed = (what) => { unread++; return ` [stale check failed: ${what}]`; };
// A pinned tool counts as stale once a newer upstream release has been out this many days.
const STALE_DAYS = 90;
// One line for a pin, given the date of the first release newer than it (none: current).
const behind = (since) => {
  if (since === undefined) return '';
  if (!since || isNaN(new Date(since))) return failed('no date for the newer release');
  const d = Math.round((Date.now() - new Date(since)) / 864e5);
  if (d > STALE_DAYS) stale++;
  return d > STALE_DAYS ? ` [stale: a newer release has been out ${d} days, over ${STALE_DAYS}]` : ` (a newer release has been out ${d} days)`;
};
const first = (dates) => dates.filter(Boolean).sort((a, b) => new Date(a) - new Date(b))[0];
// Advisories with a fix the app cannot take yet, each with why and until when (the same expiry as
// osv-scanner.toml). Not counted toward the exit code until then; each is still listed, and after
// its date it counts again, so the reason is checked once more.
const WAITING = new Map([
  ['RUSTSEC-2024-0429', { why: 'glib 0.18 comes with gtk 0.18, which Tauri uses on Linux; the app calls no glib Variant API', until: '2027-02-23' }],
]);
const today = new Date().toISOString().slice(0, 10);
const waiting = (id) => WAITING.has(id) && today <= WAITING.get(id).until;
const head = (t) => console.log(`\n== ${t}`);

head('npm packages (package.json)');
// New majors not taken yet, each with why; the report names the reason next to the version.
const HELD = new Map([
  ['typescript', 'TypeScript 7 stops svelte-check from loading; take it once svelte-check supports it'],
  ['@types/node', 'the app is built with Node 24, so its types stay on 24'],
]);
if (!only) {
const outdated = JSON.parse(sh('npm outdated --json', app).stdout || '{}');
for (const [name, v] of Object.entries(outdated)) {
  const major = v.latest.split('.')[0] !== v.current.split('.')[0];
  console.log(`  ${name} ${v.current} -> ${v.wanted}${v.latest !== v.wanted ? ` (latest ${v.latest}${major ? ', a new major' : ''})` : ''}${HELD.has(name) ? ` [held: ${HELD.get(name)}]` : ''}`);
}
if (!Object.keys(outdated).length) console.log('  all current');
}
const audit = JSON.parse(sh('npm audit --json', app).stdout || '{}');
const vulns = Object.values(audit.vulnerabilities ?? {});
if (audit.error || !audit.metadata) console.log(`  npm audit:${failed('npm audit gave no report')}`);
else console.log(`  npm audit: ${vulns.length ? vulns.map((v) => `${v.name} (${v.severity})`).join(', ') : 'no advisories'}`);
fixable += vulns.filter((v) => v.fixAvailable).length;

head('Rust crates (Cargo.lock)');
if (!only) {
const dry = sh('cargo update --dry-run --verbose', tauri).stderr;
const direct = new Set([...readFileSync(join(tauri, 'Cargo.toml'), 'utf8').matchAll(/^([a-z0-9_-]+)\s*=/gm)].map((m) => m[1]));
const ups = [...dry.matchAll(/Updating (\S+) v(\S+) -> v(\S+)/g)];
console.log(`  ${ups.length} crates have a compatible update (cargo update); the app's own:`);
for (const [, n, a, b] of ups) if (direct.has(n)) console.log(`    ${n} ${a} -> ${b}`);
for (const [, n, a, b] of dry.matchAll(/Unchanged (\S+) v(\S+) \(available: v(\S+)\)/g)) {
  if (direct.has(n)) console.log(`  ${n} ${a}: ${b} is out, a new major (edit Cargo.toml)`);
}
}
const lock = readFileSync(join(tauri, 'Cargo.lock'), 'utf8').split('[[package]]').slice(1)
  .map((p) => ({ name: p.match(/name = "(.+)"/)[1], version: p.match(/version = "(.+)"/)[1], reg: p.includes('source = "registry+') }))
  .filter((p) => p.reg);
const osv = await json('https://api.osv.dev/v1/querybatch',
  { queries: lock.map((p) => ({ package: { ecosystem: 'crates.io', name: p.name }, version: p.version })) });
if (osv.results?.length !== lock.length) console.log(`  OSV:${failed(`${osv.results?.length ?? 0} answers for ${lock.length} crates`)}`);
const ids = [...new Set(osv.results.flatMap((r, i) => (r.vulns ?? []).map((v) => `${lock[i].name}@${lock[i].version}|${v.id}`)))]
  .filter((k) => k.split('|')[1].startsWith('RUSTSEC'));
console.log(`  advisories (RustSec, ${lock.length} crates checked): ${ids.length || 'none'}`);
for (const k of ids) {
  const [crate, id] = k.split('|');
  const v = await json(`https://api.osv.dev/v1/vulns/${id}`);
  const fixed = v.affected?.flatMap((a) => a.ranges?.flatMap((r) => r.events.map((e) => e.fixed).filter(Boolean)) ?? []) ?? [];
  if (fixed.length && !waiting(id)) fixable++;
  console.log(`    ${crate} ${id}: ${v.summary}${fixed.length ? ` (fixed in ${fixed.join(', ')})` : ' (no fix: unmaintained or unsound)'}${waiting(id) ? ` [waiting until ${WAITING.get(id).until}: ${WAITING.get(id).why}]` : WAITING.has(id) ? ` [no longer waiting: ${WAITING.get(id).until} has passed]` : ''}`);
}

head('Tools pinned in fetch-tools.mjs');
const pins = readFileSync(join(app, 'scripts', 'fetch-tools.mjs'), 'utf8');
const btbn = pins.match(/autobuild-[\d-]+/)[0];
const ffmpeg = pins.match(/ffmpeg-n([\d.]+)-/)[1];
// Only month-end builds stay (fetch-tools.mjs pins only those): the newest of them.
const builds = await json('https://api.github.com/repos/BtbN/FFmpeg-Builds/releases?per_page=60');
const tagDate = (t) => t?.match(/(\d{4}-\d{2}-\d{2})/)?.[1];
const monthEnds = builds.filter((r) => /^autobuild-\d{4}-\d{2}-\d{2}-/.test(r.tag_name) && (([, y, m, d]) => new Date(Date.UTC(y, m - 1, d + 1)).getUTCDate() === 1)(r.tag_name.match(/(\d{4})-(\d{2})-(\d{2})/).map(Number)));
const dated = monthEnds[0];
const ffmpegNew = (dated?.assets ?? []).map((a) => a.name.match(/^ffmpeg-n([\d.]+)-\d+-g\w+-win64-lgpl-shared/)?.[1])
  .filter(Boolean).sort((x, y) => y.localeCompare(x, 'en', { numeric: true }))[0];
if (!builds.some((r) => r.tag_name === btbn)) console.log(`  FFmpeg:${failed(`${btbn} is not in the newest 60 builds`)}`);
const newerBuilds = monthEnds.filter((r) => tagDate(r.tag_name) > tagDate(btbn));
console.log(`  FFmpeg ${ffmpeg} (${btbn}): ${!monthEnds.length ? failed('no month-end build listed') : `newest month-end build ${dated.tag_name} has ${ffmpegNew ?? 'no release branch build'}${behind(newerBuilds.length ? first(newerBuilds.map((r) => r.published_at)) ?? null : undefined)}`}`);
const pdfium = pins.match(/chromium\/(\d+)/)[1];
const pdfiums = (await json('https://api.github.com/repos/bblanchon/pdfium-binaries/releases?per_page=30'))
  .map((r) => ({ n: Number(r.tag_name.match(/^chromium\/(\d+)$/)?.[1]), at: r.published_at })).filter((r) => r.n);
if (!pdfiums.some((r) => r.n === Number(pdfium))) console.log(`  PDFium:${failed(`chromium/${pdfium} is not in the newest 30 releases`)}`);
const newerPdfium = pdfiums.filter((r) => r.n > Number(pdfium));
console.log(`  PDFium chromium/${pdfium}: ${!pdfiums.length ? failed('no chromium/<n> release listed') : `newest chromium/${Math.max(...pdfiums.map((r) => r.n))}${behind(newerPdfium.length ? first(newerPdfium.map((r) => r.at)) ?? null : undefined)}`}`);
const repo = await (await get('https://repo.msys2.org/mingw/ucrt64/', { headers: UA })).text();
const msysPins = [...pins.matchAll(/'mingw-w64-ucrt-x86_64-(.+?)-(\d[^-]*-\d+)-any\.pkg\.tar\.zst'/g)];
const msysCalls = pins.split("msys('").length - 1;
if (!msysPins.length || msysPins.length !== msysCalls) console.log(`  MSYS2:${failed(`${msysPins.length} of ${msysCalls} pins read from fetch-tools.mjs`)}`);
for (const [, pkg, ver] of msysPins) {
  const esc = encodeURIComponent(pkg).replace(/[+]/g, '\\+');  // the listing writes + as %2B
  // Each listing line ends with the file's date: `...pkg.tar.zst">...</a> 17-Dec-2022 08:22`.
  const all = new Map([...repo.matchAll(new RegExp(`mingw-w64-ucrt-x86_64-${esc}-(\\d[\\w.+~]*-\\d+)-any\\.pkg\\.tar\\.zst">[^<]*</a>\\s+(\\d{2}-\\w{3}-\\d{4} \\d{2}:\\d{2})`, 'g'))].map((m) => [m[1], `${m[2]} UTC`]));
  const order = (a, b) => a.localeCompare(b, 'en', { numeric: true });
  const newer = [...all.keys()].filter((v) => order(v, ver) > 0);
  const newest = newer.sort(order).at(-1);
  if (!all.has(ver)) { console.log(`  ${pkg} ${ver}${failed('the pinned version is not in the MSYS2 listing')}`); continue; }
  console.log(`  ${pkg} ${ver}${newest ? ` -> ${newest}${behind(first(newer.map((v) => all.get(v))) ?? null)}` : ' (current)'}`);
}

if (!only) {
head('Toolchains');
console.log(`  ${sh('rustup check', tauri).stdout.trim().split('\n').join('\n  ')}`);
const node = process.version;
const lts = (await json('https://nodejs.org/dist/index.json')).find((r) => r.version.startsWith(node.split('.')[0] + '.'));
// From WSL, the Windows node that builds the Windows package is asked too.
const wsl = process.platform === 'linux' && /microsoft/i.test(readFileSync('/proc/version', 'utf8'));
const winNode = wsl ? `; Windows node: ${execSync('/mnt/c/Windows/System32/cmd.exe /d /c node --version', { cwd: '/mnt/c', env }).toString().trim()}` : '';
console.log(`  node ${node}: newest ${node.split('.')[0]}.x is ${lts.version}${winNode}`);

}
console.log(fixable ? `\n${fixable} advisories have a fix to take` : '\nno advisory with a fix waiting');
if (stale) console.log(`${stale} pinned tools are stale: update them in fetch-tools.mjs`);
if (unread) console.log(`${unread} checks could not be made (stale check failed): the gate fails until they can`);
process.exit(fixable || stale || unread ? 1 : 0);
