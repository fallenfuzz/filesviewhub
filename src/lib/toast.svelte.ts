// The app's toast (ToastBox.svelte, mounted once in App): a short line at the foot of the window that
// says what was done, with Undo when it can be taken back, or another action. One at a time; a new one replaces it.
// An error is a toast too, in full however long: it stays until it is closed or replaced.
export interface Action { label: string; run: () => void }
export interface Toast { text: string; undo?: () => Promise<void> | void; action?: Action; error?: boolean; id: number }

export const toast = $state<{ now: Toast | null }>({ now: null })

let n = 0
export function showToast(text: string, undo?: () => Promise<void> | void, action?: Action) {
  toast.now = { text, undo, action, id: ++n }
}
/** Shows what failed; the error is the thrown one's message. */
export function showError(e: unknown) {
  toast.now = { text: e instanceof Error ? e.message : String(e), error: true, id: ++n }
}
export const hideToast = () => { toast.now = null }
/** Takes an error away as the next action starts, as the status bar did; any other toast stays. */
export const hideError = () => { if (toast.now?.error) toast.now = null }
