// The gate before each commit and push: the UI tests, the type check, the Rust tests and clippy,
// with one pass or fail line for each and exit code 1 if any failed.
//   node scripts/check.mjs             runs them here, on Windows, macOS, Linux or WSL
//   node scripts/check.mjs --windows   from WSL: runs them on Windows, through cmd.exe; the default
//                                      from WSL when Windows npm installed node_modules (.cmd files)
//   node scripts/check.mjs --linux     from WSL: runs them in the Linux build folder (linux.sh)
// The output of a step is shown only when the step fails; .scratch/check.log keeps all of it. The slow checks (#[ignore] Rust tests,
// the speed checks of big-folders.sh and stalls.mjs) are not part of it.
import { spawnSync } from 'node:child_process';
import { appendFileSync, existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const app = join(dirname(fileURLToPath(import.meta.url)), '..');
const linux = process.argv.includes('--linux');
const wsl = process.platform === 'linux' && existsSync('/proc/version')
  && /microsoft/i.test(readFileSync('/proc/version', 'utf8'));
// A checkout on a Windows drive whose packages Windows npm installed cannot run them in WSL.
const winBuilt = () => existsSync(join(app, 'node_modules', '.bin'))
  && readdirSync(join(app, 'node_modules', '.bin')).some((f) => f.endsWith('.cmd'));
if (process.argv.includes('--windows') && !wsl) {
  console.log('--windows runs the gate on Windows from WSL only');
  process.exit(2);
}

if (wsl && !linux && (process.argv.includes('--windows') || winBuilt())) {
  // Node, npm and cargo of Windows, run from this folder as Windows sees it.
  const win = spawnSync('wslpath', ['-w', app], { encoding: 'utf8' }).stdout.trim();
  const r = spawnSync('/mnt/c/Windows/System32/cmd.exe',
    ['/d', '/c', `set "PATH=%USERPROFILE%\\.cargo\\bin;%PATH%" && cd /d ${win} && node scripts\\check.mjs`],
    { cwd: '/mnt/c', stdio: 'inherit' });
  process.exit(r.status ?? 1);
}

const linuxSh = (cmd) => `bash scripts/linux.sh run sh -c ${JSON.stringify(cmd)}`;
const steps = [
  ['UI tests', 'npm test'],
  ['Type check', 'npm run check'],
  ['Rust tests', 'cd src-tauri && cargo test --lib'],
  ['Clippy', 'cd src-tauri && cargo clippy --all-targets -- -D warnings'],
].map(([name, cmd]) => [name, linux ? linuxSh(cmd) : cmd]);

// Every step's whole output goes to .scratch/check.log, so a failure seen once can be read later.
const log = join(app, '.scratch', 'check.log');
mkdirSync(dirname(log), { recursive: true });
writeFileSync(log, `check.mjs ${new Date().toISOString()}${linux ? ' --linux' : ''}\n`);
let failed = 0;
for (const [name, cmd] of steps) {
  const start = Date.now();
  const r = spawnSync(cmd, { cwd: app, shell: true, encoding: 'utf8', maxBuffer: 1 << 28 });
  const s = Math.round((Date.now() - start) / 1000);
  appendFileSync(log, `\n== ${name}: ${cmd} (exit ${r.status}, ${s} s)\n${r.stdout}${r.stderr}`);
  if (r.status === 0) {
    console.log(`pass  ${name} (${s} s)`);
  } else {
    failed++;
    console.log(`FAIL  ${name} (${s} s): ${cmd}\n${r.stdout}${r.stderr}`);
  }
}
console.log(failed ? `${failed} of ${steps.length} checks failed (whole output: .scratch/check.log)` : `all ${steps.length} checks passed`);
process.exit(failed ? 1 : 0);
