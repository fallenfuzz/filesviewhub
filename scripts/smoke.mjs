// A smoke check of the running app over DevTools (port 9222): it reloads the page, waits for it
// to settle, and fails on an uncaught error, a console error, an error toast or an empty page.
// Run on Windows, from the repo root, with the dev app up (scripts/dev.mjs):
//   node scripts/smoke.mjs [shot.png]    prints what it saw; exit code 1 on any failure
// The picture, when named, is taken after the checks, for a look by eye.
import { writeFileSync } from 'node:fs'
const out = process.argv[2]
const page = (await (await fetch('http://localhost:9222/json/list')).json()).find(p => p.type === 'page')
const ws = new WebSocket(page.webSocketDebuggerUrl)
let id = 0; const wait = new Map(); const problems = []
ws.onmessage = e => {
  const m = JSON.parse(e.data)
  if (m.id) { wait.get(m.id)?.(m); wait.delete(m.id); return }
  if (m.method === 'Runtime.exceptionThrown') problems.push(`uncaught: ${m.params.exceptionDetails.exception?.description ?? m.params.exceptionDetails.text}`)
  if (m.method === 'Runtime.consoleAPICalled' && m.params.type === 'error')
    problems.push(`console error: ${m.params.args.map(a => a.value ?? a.description).join(' ')}`)
}
await new Promise(r => (ws.onopen = r))
const send = (method, params = {}) => new Promise(r => { wait.set(++id, r); ws.send(JSON.stringify({ id, method, params })) })
const value = async (expression) => (await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true })).result?.result?.value
await send('Runtime.enable'); await send('Page.enable')
problems.length = 0  // Runtime.enable replays what was logged before the reload
await send('Page.reload', { ignoreCache: true })
// Settled: the page shows text and has not changed for 2 s, or 30 s have passed.
let last = '', still = 0
for (let t = 0; t < 60 && still < 4; t++) {
  await new Promise(r => setTimeout(r, 500))
  const now = await value('document.body?.innerText ?? ""') ?? ''
  still = now && now === last ? still + 1 : 0; last = now
}
const seen = await value(`JSON.stringify({
  text: document.body.innerText.trim().length,
  cards: document.querySelectorAll('.card').length,
  alert: [...document.querySelectorAll('[role=alert]')].map(e => e.textContent.trim()).filter(Boolean),
})`).then(JSON.parse)
if (!seen.text) problems.push('the page shows no text')
for (const a of seen.alert) problems.push(`error toast: ${a}`)
console.log(`page: ${seen.text} characters, ${seen.cards} cards`)
if (out) writeFileSync(out, Buffer.from((await send('Page.captureScreenshot', { format: 'png' })).result.data, 'base64'))
ws.close()
for (const p of problems) console.log(`FAIL  ${p}`)
console.log(problems.length ? `${problems.length} problems` : 'pass')
process.exit(problems.length ? 1 : 0)
