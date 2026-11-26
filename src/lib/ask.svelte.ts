// The app's own yes or no question, drawn in its look instead of a system box. `AskBox`
// in App shows it over any open dialog; Esc and Cancel answer no.
export interface Asking { message: string; ok: string; danger: boolean; done: (yes: boolean) => void }

export const asking = $state<{ q: Asking | null }>({ q: null })

// While a question is open, keys belong to it alone: no handler under it (Esc in the Viewer,
// Enter in Review, arrows in Photos) sees them. Added as the module loads, so it runs before every
// other window handler; the keys' own work in the box (Esc, Enter on a button, Tab) still happens.
addEventListener('keydown', e => { if (asking.q) e.stopImmediatePropagation() }, true)

/** Asks before anything goes on; a question asked while one is open answers that one no. */
export function ask(message: string, ok: string, danger = false): Promise<boolean> {
  return new Promise(done => {
    asking.q?.done(false)
    asking.q = { message, ok, danger, done }
  })
}
