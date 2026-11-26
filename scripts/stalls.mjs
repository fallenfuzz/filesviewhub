// Measures how long the running app's window stops answering, over DevTools (port 9222), for the
// speed checks (every stall under 1 s). Run on Windows, from the repo root:
//   node scripts/stalls.mjs start    starts recording every task of the page longer than 50 ms
//   node scripts/stalls.mjs report   prints the longest, their count and the total, in ms
// A reload of the page ends the recording. Work run from DevTools itself is not counted.
const what = process.argv[2]
const js = {
  start: `window.__fvhStalls = []; new PerformanceObserver(l => l.getEntries().forEach(e =>
    __fvhStalls.push(Math.round(e.duration)))).observe({ type: 'longtask' }); 'recording'`,
  report: `window.__fvhStalls ? JSON.stringify({ longest: Math.max(0, ...__fvhStalls),
    count: __fvhStalls.length, total: __fvhStalls.reduce((a, b) => a + b, 0) })
    : 'not recording: run start first (a reload ends it)'`,
}[what]
if (!js) { console.log('usage: node scripts/stalls.mjs start | report'); process.exit(2) }
const page = (await (await fetch('http://localhost:9222/json/list')).json()).find(p => p.type === 'page')
const ws = new WebSocket(page.webSocketDebuggerUrl)
await new Promise(r => (ws.onopen = r))
const reply = new Promise(r => (ws.onmessage = e => r(JSON.parse(e.data))))
ws.send(JSON.stringify({ id: 1, method: 'Runtime.evaluate', params: { expression: js, returnByValue: true } }))
const m = await reply
console.log(m.result?.result?.value ?? JSON.stringify(m.result))
ws.close()
