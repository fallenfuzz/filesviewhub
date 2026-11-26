// What the app is built from that has a newer version, and which locked versions have a known
// security advisory. It changes nothing; run it from WSL (or any system with node, npm and cargo):
//   node scripts/updates.mjs                     everything below
//   node scripts/updates.mjs --advisories        only the security advisories (npm audit, OSV), about 3 s
// It covers the npm packages (npm outdated, npm audit), the Rust crates (cargo update --dry-run,
// and every crate in Cargo.lock against the OSV advisory database), the tools pinned in
// fetch-tools.mjs (FFmpeg, PDFium, the MSYS2 packages of ddjvu) and the Rust and Node toolchains.
// Only package names and versions are sent: to the npm registry, crates.io, api.osv.dev, GitHub
// and repo.msys2.org. Exit code 1 when an advisory has a fixed version not yet used.
import { execSync, spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const app = join(dirname(fileURLToPath(import.meta.url)), '..');
const tauri = join(app, 'src-tauri');
const UA = { 'User-Agent': 'FilesViewHub-dev-updates' };
const env = { ...process.env, PATH: `${process.env.HOME}/.cargo/bin:${process.env.PATH}` };
const sh = (cmd, cwd) => spawnSync(cmd, { cwd, shell: true, encoding: 'utf8', env, maxBuffer: 1 << 26 });
const json = async (url, body) => (await fetch(url, body
  ? { method: 'POST', headers: { ...UA, 'Content-Type': 'application/json' }, body: JSON.stringify(body) }
  : { headers: UA })).json();
const only = process.argv.includes('--advisories');
let fixable = 0;
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
console.log(`  npm audit: ${vulns.length ? vulns.map((v) => `${v.name} (${v.severity})`).join(', ') : 'no advisories'}`);
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

if (!only) {
head('Tools pinned in fetch-tools.mjs');
const pins = readFileSync(join(app, 'scripts', 'fetch-tools.mjs'), 'utf8');
const latest = async (repo) => (await json(`https://api.github.com/repos/${repo}/releases/latest`)).tag_name;
const btbn = pins.match(/autobuild-[\d-]+/)[0];
const ffmpeg = pins.match(/ffmpeg-n([\d.]+)-/)[1];
// Only month-end builds stay (fetch-tools.mjs pins only those): the newest of them.
const dated = (await json('https://api.github.com/repos/BtbN/FFmpeg-Builds/releases?per_page=60'))
  .find((r) => /^autobuild-\d{4}-\d{2}-\d{2}-/.test(r.tag_name) && (([, y, m, d]) => new Date(Date.UTC(y, m - 1, d + 1)).getUTCDate() === 1)(r.tag_name.match(/(\d{4})-(\d{2})-(\d{2})/).map(Number)));
const ffmpegNew = (dated?.assets ?? []).map((a) => a.name.match(/^ffmpeg-n([\d.]+)-\d+-g\w+-win64-lgpl-shared/)?.[1])
  .filter(Boolean).sort((x, y) => y.localeCompare(x, 'en', { numeric: true }))[0];
console.log(`  FFmpeg ${ffmpeg} (${btbn}): newest month-end build ${dated?.tag_name} has ${ffmpegNew ?? 'no release branch build'}`);
console.log(`  PDFium ${pins.match(/chromium\/(\d+)/)[0]}: newest ${await latest('bblanchon/pdfium-binaries')}`);
const repo = await (await fetch('https://repo.msys2.org/mingw/ucrt64/', { headers: UA })).text();
for (const [, pkg, ver] of pins.matchAll(/'mingw-w64-ucrt-x86_64-(.+?)-(\d[^-]*-\d+)-any\.pkg\.tar\.zst'/g)) {
  const esc = encodeURIComponent(pkg).replace(/[+]/g, '\\+');  // the listing writes + as %2B
  const all = [...repo.matchAll(new RegExp(`mingw-w64-ucrt-x86_64-${esc}-(\\d[^-]*-\\d+)-any\\.pkg\\.tar\\.zst"`, 'g'))].map((m) => m[1]);
  const newest = all.sort((a, b) => a.localeCompare(b, 'en', { numeric: true })).at(-1);
  console.log(`  ${pkg} ${ver}${newest && newest !== ver ? ` -> ${newest}` : ' (current)'}`);
}

head('Toolchains');
console.log(`  ${sh('rustup check', tauri).stdout.trim().split('\n').join('\n  ')}`);
const node = process.version;
const lts = (await json('https://nodejs.org/dist/index.json')).find((r) => r.version.startsWith(node.split('.')[0] + '.'));
// From WSL, the Windows node that builds the Windows package is asked too.
const wsl = process.platform === 'linux' && /microsoft/i.test(readFileSync('/proc/version', 'utf8'));
const winNode = wsl ? `; Windows node: ${execSync('/mnt/c/Windows/System32/cmd.exe /d /c node --version', { cwd: '/mnt/c' }).toString().trim()}` : '';
console.log(`  node ${node}: newest ${node.split('.')[0]}.x is ${lts.version}${winNode}`);

}
console.log(fixable ? `\n${fixable} advisories have a fix to take` : '\nno advisory with a fix waiting');
process.exit(fixable ? 1 : 0);
