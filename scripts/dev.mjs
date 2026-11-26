// Runs the dev app with DevTools on port 9222, for checks by agents. Run on Windows, from the repo root:
// `node scripts/dev.mjs run [data folder]` runs `npm run tauri dev` (cargo's folder added to PATH,
// data in .scratch/data-demo unless a folder is given, here or in
// FVH_DEV_DATA) and ends
// with its exit code; its output also goes to .scratch/dev.log. Run it as a background
// command: a child started from WSL ends with its parent. `node scripts/dev.mjs wait` returns once
// DevTools shows the app's page (exit 0, or 1 after 10 min). `node scripts/dev.mjs stop` closes the
// dev window as a person would, so `run` ends with 0; a forced kill (taskkill /F) ends it with 1,
// which looks like a failure but is not one. Only the exe in target\debug is closed, never a
// packaged FilesViewHub.
import { spawn, execFileSync } from 'node:child_process'
import { createWriteStream } from 'node:fs'
import { homedir } from 'node:os'
import { resolve } from 'node:path'

const app = resolve(import.meta.dirname, '..')
const exe = resolve(app, 'src-tauri/target/debug/filesviewhub.exe').toLowerCase()
const sleep = ms => new Promise(r => setTimeout(r, ms))
const page = async () => {
  try { return (await (await fetch('http://localhost:9222/json/list')).json()).find(p => p.type === 'page' && p.url.startsWith('http')) } catch { return undefined }
}
// True once the page has loaded and shows some text: DevTools answers seconds before Vite's
// first load ends, and a screenshot taken then is blank.
const shown = p => new Promise(done => {
  const ws = new WebSocket(p.webSocketDebuggerUrl)
  const end = v => { ws.close(); done(v) }
  ws.onerror = () => end(false)
  ws.onopen = () => ws.send(JSON.stringify({ id: 1, method: 'Runtime.evaluate', params: {
    expression: "document.readyState === 'complete' && document.body.innerText.trim() !== ''", returnByValue: true } }))
  ws.onmessage = e => { const m = JSON.parse(e.data); if (m.id === 1) end(m.result?.result?.value === true) }
})
const devPids = () => {
  const out = execFileSync('powershell', ['-NoProfile', '-Command',
    "Get-CimInstance Win32_Process -Filter \"Name='filesviewhub.exe'\" | ForEach-Object { \"$($_.ProcessId)`t$($_.ExecutablePath)\" }"], { encoding: 'utf8' })
  return out.split(/\r?\n/).filter(Boolean).map(l => l.split('\t')).filter(([, p]) => p?.toLowerCase() === exe).map(([id]) => id)
}

const [cmd, data] = process.argv.slice(2)
if (cmd === 'run') {
  if (await page()) { console.log('already running: DevTools answers on port 9222'); process.exit(1) }
  const env = { ...process.env, WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9222' }
  env.Path = `${resolve(homedir(), '.cargo/bin')};${process.env.Path ?? process.env.PATH}`
  env.FVH_DATA_DIR = resolve(data ?? process.env.FVH_DEV_DATA ?? resolve(app, '.scratch/data-demo'))
  const log = createWriteStream(resolve(app, '.scratch/dev.log'))
  const child = spawn('cmd.exe', ['/d', '/c', 'npm', 'run', 'tauri', 'dev'], { cwd: app, env, stdio: ['ignore', 'pipe', 'pipe'], windowsHide: true })
  for (const s of [child.stdout, child.stderr]) s.on('data', d => { process.stdout.write(d); log.write(d) })
  child.on('exit', code => { console.log(`dev run ended with exit code ${code}`); log.end(() => process.exit(code ?? 1)) })
} else if (cmd === 'wait') {
  for (const t0 = Date.now(); Date.now() - t0 < 10 * 60_000; await sleep(1000)) {
    const p = await page()
    if (p && await shown(p)) { console.log(`ready: ${p.url}`); process.exit(0) }
  }
  console.log('not ready after 10 min; see .scratch/dev.log')
  process.exit(1)
} else if (cmd === 'stop') {
  const pids = devPids()
  if (!pids.length) { console.log('not running'); process.exit(0) }
  // taskkill closes one window at a time: with the Review window open (`decisions/2026/10/27/review-has-its-own-window.md`) it is asked
  // again until the main window is closed too.
  const close = () => { try { execFileSync('taskkill', pids.flatMap(id => ['/PID', id]), { stdio: 'ignore' }) } catch { /* gone */ } }
  close()
  for (let i = 0; i < 30; i++) {
    await sleep(1000)
    if (!devPids().length && !(await page())) { console.log('stopped'); process.exit(0) }
    if (i % 3 === 2) close()
  }
  console.log(`still running after 30 s (PID ${pids.join(', ')}); taskkill /F /PID forces it, and run then ends with 1`)
  process.exit(1)
} else {
  console.log('usage: node scripts/dev.mjs run [data folder] | wait | stop')
  process.exit(2)
}
