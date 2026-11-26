// A card made of copies of one Work in a merged Section, run by `npm test`: Vite loads
// api.ts as the app does, so the test reads the same code.
import { test } from 'node:test'
import assert from 'node:assert/strict'
import { fileURLToPath } from 'node:url'
import { runnerImport } from 'vite'

const { module: a } = await runnerImport(fileURLToPath(new URL('../src/lib/api.ts', import.meta.url)))

const D = 'AAAA0001:Courses/Guitar/', E = 'BBBB0002:Courses/Guitar/'
const marks = (m, ticks = {}) => ({ v: 2, marks: m, ticks })

test('the card keeps the copy with the newest Mark, else the first', () => {
  assert.equal(a.keptOf([D, E], marks({})), D)
  assert.equal(a.keptOf([D, E], marks({ [E]: { m: 'next', at: 5 } })), E)
  // Done on one copy and a newer Next on the other: the newer Next is what the card shows.
  assert.equal(a.keptOf([D, E], marks({ [D]: { m: 'done', at: 5 }, [E]: { m: 'next', at: 9 } })), E)
  assert.equal(a.keptOf([D, E], marks({ [D]: { m: 'done', at: 9 }, [E]: { m: 'next', at: 5 } })), D)
})

test('a merged card reads the Ticks of every copy', () => {
  // The issue's case: Ticks on the D copy, an older Mark on the E copy, which the card keeps.
  const m = marks({ [E]: { m: 'next', at: 1 } }, { [D]: { '01': 3, '02': 4 }, [E]: { '02': 2, '03': 6 } })
  const w = { id: a.keptOf([D, E], m), copies: [D] }
  assert.equal(w.id, E)
  assert.deepEqual(a.ticksOf(w, m), { '01': 3, '02': 2, '03': 6 })
})

test('a card with no copies reads its own Ticks, the same record', () => {
  const m = marks({}, { [D]: { '01': 3 } })
  assert.equal(a.ticksOf({ id: D }, m), m.ticks[D])
  assert.equal(a.ticksOf({ id: E, copies: [D] }, marks({})), undefined)
})

test('with three copies or equal dates, the newest Mark or else the first copy is kept', () => {
  const F = 'CCCC0003:Courses/Guitar/'
  assert.equal(a.keptOf([D, E, F], marks({ [D]: { m: 'next', at: 2 }, [F]: { m: 'done', at: 7 } })), F)
  assert.equal(a.keptOf([D, E, F], marks({ [E]: { m: 'next', at: 4 }, [F]: { m: 'done', at: 4 } })), E)
  // Every Mark cleared, then the copy other than the first marked: that copy is kept.
  assert.equal(a.keptOf([D, E], marks({ [E]: { m: 'next', at: 8 } })), E)
})

test('an untick writes to the card and to each copy that has the item', () => {
  const F = 'CCCC0003:Courses/Guitar/'
  const w = { id: E, copies: [D, F] }
  const m = marks({}, { [D]: { '01': 1 }, [E]: { '02': 2 }, [F]: { '01': 3, '02': 3 } })
  assert.deepEqual(a.untickIds(w, ['01'], m), [E, D, F])
  assert.deepEqual(a.untickIds(w, ['02'], m), [E, F])
  assert.deepEqual(a.untickIds(w, ['09'], m), [E])
  assert.deepEqual(a.untickIds(w, ['02', '09'], m), [E, F])
  assert.deepEqual(a.untickIds({ id: D }, ['01'], m), [D])
})

test('clearing a Mark writes to the card and to each copy with a Mark', () => {
  const F = 'CCCC0003:Courses/Guitar/'
  const m = marks({ [D]: { m: 'next', at: 2 }, [E]: { m: 'done', at: 5 } })
  assert.deepEqual(a.markedIds({ id: E, copies: [D, F] }, m), [E, D])
  assert.deepEqual(a.markedIds({ id: F }, m), [F])
})

test('a Tick writes only the items no copy has ticked yet', () => {
  const m = marks({}, { [D]: { '01': 1 }, [E]: { '02': 2 } })
  assert.deepEqual(a.tickAdds({ id: E, copies: [D] }, ['01', '02', '03'], m), ['03'])
  assert.deepEqual(a.tickAdds({ id: E }, ['01', '02', '03'], m), ['01', '03'])
})
