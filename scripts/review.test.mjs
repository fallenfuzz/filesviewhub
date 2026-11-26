// Review's counts, Review again's toast and Junk that never waits, run by `npm test`: Vite loads review.ts as the
// app does, so the test reads the same code.
import { test } from 'node:test'
import assert from 'node:assert/strict'
import { fileURLToPath } from 'node:url'
import { runnerImport } from 'vite'

const { module: r } = await runnerImport(fileURLToPath(new URL('../src/lib/review.ts', import.meta.url)))

const SEC = '0000ABCD:Photos'
const item = (rel, size = 10) => ({ name: rel, rel, path: `D:\\Photos\\${rel}`, ext: 'jpg', size, mtime: 0, cloud: false, key: '', title: rel, group: '', role: 'main' })
// One Event: a set of Similar shots (a Best one and two more), a set where it is your call, a Copy
// of a file elsewhere, a Junk screenshot and a photo Review says nothing of.
const items = ['best.jpg', 'extra1.jpg', 'extra2.jpg', 'call1.jpg', 'call2.jpg', 'copy.jpg', 'junk.jpg', 'plain.jpg'].map(n => item(n))
const w = { id: `${SEC}#best`, title: 'Day', path: '', folder: ['2019', '06 June'], is_folder: false, items, media: [], size: 0, added: 0, cloud: 0, year: 2019, event: true, taken: [0, 0] }
const notes = {
  [items[0].path]: { set: 's', best: true }, [items[1].path]: { set: 's' }, [items[2].path]: { set: 's' },
  [items[3].path]: { set: 'c', call: true }, [items[4].path]: { set: 'c', call: true },
  [items[5].path]: { copy: 'elsewhere' }, [items[6].path]: { junk: 'screenshot' },
}
// Every photo has a choice: the Best one kept, the rest Not needed.
const marks = { v: 2, marks: {}, ticks: {}, review: Object.fromEntries(items.map((i, k) => [r.photoKey(i, w), { keep: k === 0, at: 1, size: i.size }])) }

test('every photo has a choice, so none waits', () => {
  assert.equal(r.toReview([w], notes, marks), 0)
})

test('Review again says it waits exactly when the count goes up by it', () => {
  for (const i of items) {
    const review = { ...marks.review }
    delete review[r.photoKey(i, w)]
    const up = r.toReview([w], notes, { ...marks, review }) - r.toReview([w], notes, marks)
    assert.equal(r.waitsAgain(i, w, notes, marks), up === 1, i.name)
    assert.ok(up === 0 || up === 1, i.name)
  }
})

test('the suggested photos wait again; the Best one, Junk and a photo with no suggestion stay', () => {
  const waits = items.filter(i => r.waitsAgain(i, w, notes, marks)).map(i => i.name)
  assert.deepEqual(waits, ['extra1.jpg', 'extra2.jpg', 'call1.jpg', 'call2.jpg', 'copy.jpg'])
})

test('a lone photo that looks like a screenshot does not wait and is no step of Review', () => {
  const none = { v: 2, marks: {}, ticks: {}, review: {} }
  const lone = { ...w, items: [items[6], items[7]] }
  assert.equal(r.toReview([lone], notes, none), 0)
  assert.equal(r.reviewState([lone], notes, none), null)
  assert.deepEqual(r.stepsOf(lone, notes, none, true), [])
})

test('a lone Junk photo set aside before stays set aside, with nothing to review', () => {
  const lone = { ...w, items: [items[6], items[7]] }
  const aside = { v: 2, marks: {}, ticks: {}, review: { [r.photoKey(items[6], lone)]: { keep: false, at: 1, size: items[6].size } } }
  assert.equal(r.asideIn([lone], aside), 1)
  assert.equal(r.toReview([lone], notes, aside), 0)
  assert.equal(r.reviewState([lone], notes, aside), null)
  // So Review again's toast offers no Not needed for it; a Best one is in its set's step.
  assert.equal(r.inReview(items[6], lone, notes, aside), false)
  assert.equal(r.inReview(items[0], w, notes, marks), true)
})

test('a screenshot in a set of Similar shots or Copies still waits with its set', () => {
  const none = { v: 2, marks: {}, ticks: {}, review: {} }
  const shots = { ...notes, [items[1].path]: { set: 's', junk: 'screenshot' }, [items[5].path]: { copy: 'elsewhere', junk: 'screenshot' } }
  const waits = items.filter(i => r.waiting(i, w, shots, none)).map(i => i.name)
  assert.deepEqual(waits, ['extra1.jpg', 'extra2.jpg', 'call1.jpg', 'call2.jpg', 'copy.jpg'])
  assert.deepEqual(r.stepsOf(w, shots, none).map(s => s.kind), ['similar', 'similar', 'copies'])
})

test('another file at the same path (another size) has no choice to forget', () => {
  const other = { ...items[1], size: 99 }
  const ow = { ...w, items: [items[0], other] }
  assert.equal(r.choiceOf(other, ow, marks), undefined)
})

test('opened from a photo, Review opens at its set and goes on through the sets that wait', () => {
  const none = { v: 2, marks: {}, ticks: {}, review: {} }
  const names = s => s.items.map(i => i.name).join(',')
  // From the your-call set: the steps that wait, at that set; Back reaches the one before.
  const from = r.stepsAt([w], notes, none, undefined, items[3])
  assert.deepEqual(from.steps.map(names), ['best.jpg,extra1.jpg,extra2.jpg', 'call1.jpg,call2.jpg', 'copy.jpg'])
  assert.equal(from.at, 1)
  // A Best one opens at its own set; a Copy at its step.
  assert.equal(r.stepsAt([w], notes, none, undefined, items[0]).at, 0)
  assert.equal(r.stepsAt([w], notes, none, undefined, items[5]).at, 2)
  // With no photo, Review opens at the first step, as before.
  assert.deepEqual(r.stepsAt([w], notes, none), { steps: r.stepsOf(w, notes, none), at: 0, here: false })
  assert.equal(from.here, true)
})

test('a set already chosen opens with its choices, among the sets that still wait', () => {
  const k = i => r.photoKey(i, w)
  // The first set chosen, the rest waiting.
  const some = { v: 2, marks: {}, ticks: {}, review: { [k(items[0])]: { keep: true, at: 1, size: 10 }, [k(items[1])]: { keep: false, at: 1, size: 10 }, [k(items[2])]: { keep: false, at: 1, size: 10 } } }
  const from = r.stepsAt([w], notes, some, undefined, items[1])
  assert.equal(from.steps.length, 3)
  assert.equal(from.at, 0)
  assert.ok(from.steps[0].items.every(i => !r.waiting(i, w, notes, some)))
  // Opened from a waiting set, the chosen one is left out, as the Review chip leaves it.
  assert.equal(r.stepsAt([w], notes, some, undefined, items[3]).steps.length, 2)
  // Everything chosen: every set, at the photo's.
  const all = r.stepsAt([w], notes, marks, undefined, items[5])
  assert.equal(all.steps.length, 3)
  assert.equal(all.at, 2)
})

test('a photo in no set gives no step to open at', () => {
  const none = { v: 2, marks: {}, ticks: {}, review: {} }
  for (const i of [items[6], items[7]]) assert.equal(r.inReview(i, w, notes, none), false, i.name)
  assert.equal(r.stepsAt([w], notes, none, undefined, items[7]).here, false)
})

test('a kind left out by `only` keeps the photo\'s set out too', () => {
  const none = { v: 2, marks: {}, ticks: {}, review: {} }
  const from = r.stepsAt([w], notes, none, i => i.name !== 'copy.jpg', items[3])
  assert.deepEqual(from.steps.map(s => s.kind), ['similar', 'similar'])
  assert.equal(from.at, 1)
  // The Copy itself, left out, has no step to open at, so the viewer opens instead.
  assert.equal(r.stepsAt([w], notes, none, i => i.name !== 'copy.jpg', items[5]).here, false)
})

test('the pick of a your-call set, its choice forgotten, stays beside the choices of its set', () => {
  const k = i => r.photoKey(i, w)
  // The app's pick in a your-call set is best and call; the other one was set aside.
  const picked = { ...notes, [items[3].path]: { set: 'c', call: true, best: true } }
  const step = r.stepsOf(w, picked, marks, true).find(s => s.call)
  const forgot = { ...marks, review: { ...marks.review } }
  delete forgot.review[k(items[3])]
  assert.equal(r.waiting(items[3], w, picked, forgot), false)
  assert.deepEqual([...r.keptIn(step, picked, forgot)], ['call1.jpg'].map(n => items.find(i => i.name === n).path))
  // With no choice made in the set, a your-call set keeps the app's pick, never nothing.
  const none = { v: 2, marks: {}, ticks: {}, review: {} }
  assert.deepEqual([...r.keptIn(step, picked, none)], [items[3].path])
  // An extra Similar shot forgotten waits again and shows Not needed.
  const sim = r.stepsOf(w, notes, marks, true)[0]
  const extra = { ...marks, review: { ...marks.review } }
  delete extra.review[k(items[1])]
  assert.deepEqual([...r.keptIn(sim, notes, extra)], [items[0].path])
})

const { module: str } = await runnerImport(fileURLToPath(new URL('../src/lib/strings.ts', import.meta.url)))

test('how much sharper the Best one is reads as a percent, then as times, up to more than 10 times', () => {
  assert.equal(str.S.sharper(34), 'Sharper where the detail is: about 34% more fine detail than the next one.')
  assert.equal(str.S.sharper(99), 'Sharper where the detail is: about 99% more fine detail than the next one.')
  assert.equal(str.S.sharper(100), 'Sharper where the detail is: about 2 times the fine detail of the next one.')
  assert.equal(str.S.sharper(230), 'Sharper where the detail is: about 3.3 times the fine detail of the next one.')
  assert.equal(str.S.sharper(899), 'Sharper where the detail is: about 10 times the fine detail of the next one.')
  assert.equal(str.S.sharper(900), 'Sharper where the detail is: more than 10 times the fine detail of the next one.')
  assert.equal(str.S.sharper(22901), 'Sharper where the detail is: more than 10 times the fine detail of the next one.')
})

// no set opens with nothing kept, Enter never sets a set aside, the same facts in every set.
const none = { v: 2, marks: {}, ticks: {}, review: {} }

test('a your-call set starts with one kept: the sharpest, or the first when equal', () => {
  const call = (a, b) => ({ ...notes, [items[3].path]: { set: 'c', call: true, detail: a }, [items[4].path]: { set: 'c', call: true, detail: b } })
  const kept = n => [...r.keptIn(r.stepsOf(w, n, none).find(s => s.call), n, none)].map(p => p.split('\\').pop())
  assert.deepEqual(kept(call(0.97, 1)), ['call2.jpg'])
  assert.deepEqual(kept(call(1, 1)), ['call1.jpg'])
  // The app's pick (shots.rs marks it Best) wins, as the sharpest it found.
  assert.deepEqual(kept({ ...call(1, 1), [items[4].path]: { set: 'c', call: true, best: true, detail: 1 } }), ['call2.jpg'])
})

test('Enter never sets all aside: Next goes on only with one kept, and every step opens with one', () => {
  assert.equal(r.canGoOn(new Set()), false)
  assert.equal(r.canGoOn(new Set(['x'])), true)
  const there = { ...items[0], name: 'there.jpg', rel: 'there.jpg', path: 'D:\\Photos\\there.jpg' }
  const other = { ...w, id: `${SEC}#there`, items: [there] }
  const copies = { ...notes, [items[5].path]: { copy: there.path } }
  const elsewhere = p => (p === there.path ? { i: there, w: other } : undefined)
  for (const s of r.stepsOf(w, copies, none, true, undefined, elsewhere)) assert.ok(r.canGoOn(r.keptIn(s, copies, none)), s.items.map(i => i.name).join())
})

test('a set of Copies whose file is in another event shows that file, kept, from its own event', () => {
  const there = { ...items[0], name: 'there.jpg', rel: 'there.jpg', path: 'D:\\Photos\\there.jpg', at: 5 }
  const other = { ...w, id: `${SEC}#there`, items: [there] }
  const copies = { ...notes, [items[5].path]: { copy: there.path } }
  const step = r.stepsOf(w, copies, none, false, undefined, p => (p === there.path ? { i: there, w: other } : undefined)).find(s => s.kind === 'copies')
  // copy.jpg has no time, so it comes after there.jpg.
  assert.deepEqual(step.items.map(i => i.name), ['there.jpg', 'copy.jpg'])
  assert.deepEqual([...step.keep], [there.path])
  assert.equal(r.workOf(step, there), other)
  assert.equal(r.workOf(step, items[5]), w)
  // It is chosen in its own event, never here, so Mark all not needed leaves it.
  assert.equal(r.choosable(step, there.path), false)
  assert.equal(r.choosable(step, items[5].path), true)
})

const ev = (id, list) => ({ ...w, id: `${SEC}#${id}`, items: list })
const timed = (rel, at) => ({ ...item(rel), at })

test('a copy of a Photo in a set joins that set\'s step, so one moment is one step', () => {
  const best = timed('b.jpg', 1), x = timed('x.jpg', 2), d = timed('d.jpg', 3)
  const e = ev('a', [best, x, d])
  const n = { [best.path]: { set: 's', best: true, detail: 1, copy_detail: 0.4 }, [x.path]: { set: 's', detail: 0.9 }, [d.path]: { copy: best.path, copy_detail: 1 } }
  const by = new Map(e.items.map(i => [i.path, { i, w: e }]))
  const steps = r.stepsOf(e, n, none, true, undefined, p => by.get(p))
  assert.equal(steps.length, 1)
  assert.equal(steps[0].kind, 'similar')
  assert.deepEqual(steps[0].items.map(i => i.name), ['b.jpg', 'x.jpg', 'd.jpg'])
  // Each Photo is chosen there; the Best one starts kept, its copy and the extra shot Not needed.
  assert.ok([best, x, d].every(i => r.choosable(steps[0], i.path)))
  assert.deepEqual([...r.keptIn(steps[0], n, none)], [best.path])
  // The copy waits for Review with its set.
  assert.equal(r.stepsOf(e, n, none, false, undefined, p => by.get(p)).length, 1)
  assert.equal(r.stepsAt([e], n, none, undefined, d, p => by.get(p)).steps[0].kind, 'similar')
})

test('a copy of a Photo in a set of another event stays a step of its own, with that Photo shown', () => {
  const best = timed('b.jpg', 1), x = timed('x.jpg', 2), d = timed('d.jpg', 90000)
  const e1 = ev('a', [best, x]), e2 = ev('b', [d])
  const n = { [best.path]: { set: 's', best: true, detail: 1 }, [x.path]: { set: 's', detail: 0.9 }, [d.path]: { copy: best.path } }
  const by = new Map([...e1.items.map(i => [i.path, { i, w: e1 }]), ...e2.items.map(i => [i.path, { i, w: e2 }])])
  const [s] = r.stepsOf(e2, n, none, true, undefined, p => by.get(p))
  assert.equal(s.kind, 'copies')
  assert.ok(!r.choosable(s, best.path) && r.choosable(s, d.path))
})

test('every Photo that waits is in a step that waits, once copies join their sets', () => {
  const best = timed('b.jpg', 1), x = timed('x.jpg', 2), d = timed('d.jpg', 3), y = timed('y.jpg', 4), z = timed('z.jpg', 5)
  const e = ev('a', [best, x, d, y, z])
  const n = { [best.path]: { set: 's', best: true, detail: 1 }, [x.path]: { set: 's', detail: 0.9 }, [d.path]: { copy: best.path }, [z.path]: { copy: y.path } }
  const steps = r.stepsOf(e, n, none, false)
  const inSteps = new Set(steps.flatMap(s => s.items.filter(i => r.choosable(s, i.path) && r.waiting(i, e, n, none)).map(i => i.path)))
  assert.equal(inSteps.size, r.toReview([e], n, none))
  // Each Photo is chosen in one step only.
  const all = steps.flatMap(s => s.items.filter(i => r.choosable(s, i.path)).map(i => i.path))
  assert.equal(all.length, new Set(all).size)
})

test('a set of Copies whose file is not found keeps its sharpest copy', () => {
  const c1 = timed('c1.jpg', 1), c2 = timed('c2.jpg', 2)
  const n = { [c1.path]: { copy: 'D:\\Other\\o.jpg', copy_detail: 0.5 }, [c2.path]: { copy: 'D:\\Other\\o.jpg', copy_detail: 1 } }
  const [s] = r.stepsOf(ev('a', [c1, c2]), n, none, false, undefined, () => undefined)
  assert.deepEqual([...r.keptIn(s, n, none)], [c2.path])
})

test('a set chosen before as all Not needed shows that choice', () => {
  const a = timed('a.jpg', 1), b = timed('b.jpg', 2)
  const e = ev('a', [a, b])
  const n = { [a.path]: { set: 's', best: true }, [b.path]: { set: 's' } }
  const all = { ...none, review: { [r.photoKey(a, e)]: { keep: false, at: 1, size: 10 }, [r.photoKey(b, e)]: { keep: false, at: 1, size: 10 } } }
  assert.equal(r.keptIn(r.stepsOf(e, n, all, true)[0], n, all).size, 0)
})

test('photos taken at the same time keep the event order; one with no time comes last', () => {
  const a = timed('a.jpg', 5), b = timed('b.jpg', 5), c = timed('c.jpg', undefined)
  const n = { [c.path]: { set: 's', best: true }, [a.path]: { set: 's' }, [b.path]: { set: 's' } }
  assert.deepEqual(r.stepsOf(ev('a', [c, a, b]), n, none)[0].items.map(i => i.name), ['a.jpg', 'b.jpg', 'c.jpg'])
})

test('a set shows its photos in time order', () => {
  const at = [30, 10, 20]
  const timed = { ...w, items: w.items.map((i, k) => (k < 3 ? { ...i, at: at[k] } : i)) }
  const step = r.stepsOf(timed, notes, none)[0]
  assert.deepEqual(step.items.map(i => i.name), ['extra1.jpg', 'extra2.jpg', 'best.jpg'])
})

test('the sharpness rank: 1 the sharpest, equal ones share a rank, unmeasured ones have none', () => {
  const ranked = { ...notes, [items[0].path]: { set: 's', best: true, detail: 1 }, [items[1].path]: { set: 's', detail: 0.9 }, [items[2].path]: { set: 's', detail: 1 } }
  const step = r.stepsOf(w, ranked, none)[0]
  assert.deepEqual([...r.sharpRanks(step, ranked).values()], [1, 2, 1])
  assert.equal(r.sharpRanks(step, notes).size, 0)
  assert.deepEqual([1, 2, 3, 4, 11, 12, 13, 21, 22].map(n => str.S.sharpRank(n)),
    ['Sharpest', '2nd sharpest', '3rd sharpest', '4th sharpest', '11th sharpest', '12th sharpest', '13th sharpest', '21st sharpest', '22nd sharpest'])
})
