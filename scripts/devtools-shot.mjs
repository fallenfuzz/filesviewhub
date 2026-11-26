// Screenshots the running app over DevTools, after running optional JavaScript in it, for checks
// by eye. Start the app with WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222;
// run on Windows: `node scripts/devtools-shot.mjs out.png ["js"]`.
import { writeFileSync } from 'node:fs'
const [out, js] = process.argv.slice(2)
const page = (await (await fetch('http://localhost:9222/json/list')).json()).find(p => p.type === 'page')
const ws = new WebSocket(page.webSocketDebuggerUrl)
let id = 0; const wait = new Map()
ws.onmessage = e => { const m = JSON.parse(e.data); wait.get(m.id)?.(m); wait.delete(m.id) }
await new Promise(r => (ws.onopen = r))
const send = (method, params = {}) => new Promise(r => { wait.set(++id, r); ws.send(JSON.stringify({ id, method, params })) })
if (js) { const r = await send('Runtime.evaluate', { expression: js, awaitPromise: true, returnByValue: true }); console.log(JSON.stringify(r.result?.result?.value ?? r.result)) }
await new Promise(r => setTimeout(r, 1500))
const shot = await send('Page.captureScreenshot', { format: 'png' })
writeFileSync(out, Buffer.from(shot.result.data, 'base64'))
ws.close()
