<script lang="ts">
  // The toast from `showToast`, as Windows 11 draws an in-app notification: raised and rounded at
  // the foot of the window, with Undo or another action and a close button. It goes after 10 s, but not while the
  // pointer or focus is on it; an error from `showError` stays until it is closed, and Esc closes it too.
  // Ctrl+Z takes it back too, outside a text box. In the top layer, so
  // it shows over an open dialog, and it moves into the top open dialog, shown or opened, since all
  // outside an open dialog takes no clicks. There it keeps clear of the dialog's box: below it, or
  // above it, and only when the dialog fills the window, just above its bottom row of buttons.
  import { hideToast, showError, showToast, toast } from './toast.svelte'
  import { S } from './strings'
  import Icon from './Icon.svelte'

  let box: HTMLDivElement
  let held = $state(false)
  let busy = $state(false)
  /** Where focus was before it came into the toast: it goes back there after Undo. */
  let back: HTMLElement | null = null
  $effect(() => {
    const t = toast.now
    // A new toast starts its own time, even when the one it replaces was held.
    held = box.matches(':hover') || box.contains(document.activeElement)
    if (!t) { if (box.matches(':popover-open')) box.hidePopover(); return }
    origin = [...document.querySelectorAll('dialog:modal')].at(-1) ?? null
    show()
  })
  let home: HTMLElement | null = null
  /** The dialog on top when the toast came, or none: Ctrl+Z undoes only there. */
  let origin: Element | null = null
  /** Shown again, inside the top open dialog and above it. */
  function show() {
    home ??= box.parentElement
    // A dialog taken away without closing took the toast with it.
    if (!box.isConnected && home) home.append(box)
    const host = [...document.querySelectorAll<HTMLElement>('dialog:modal')].at(-1) ?? home
    if (box.matches(':popover-open')) box.hidePopover()
    if (host && box.parentElement !== host) host.append(box)
    // Review and the viewer are drawn over the window with their buttons at the foot, as a dialog.
    layer = host instanceof HTMLDialogElement ? host : [...document.querySelectorAll<HTMLElement>('[role=dialog][aria-modal=true]')].at(-1) ?? null
    box.showPopover()
    place()
    watch.disconnect()
    watch.observe(box)
    if (layer) watch.observe(layer)
  }
  /** The dialog or layer the toast keeps clear of. */
  let layer: HTMLElement | null = null
  /** Placed again as the dialog, the toast or the window changes size: an error stays until closed. */
  const watch = new ResizeObserver(() => place())
  function place() {
    if (!box.matches(':popover-open')) return
    box.style.bottom = ''
    if (layer?.isConnected) box.style.bottom = clearOf(layer)
  }
  /** Where the toast goes so it never covers the dialog's heading: at the foot of the window when
   *  the dialog ends above it; over a dialog that reaches the foot (Review, the viewer), just above
   *  its bottom row of buttons; else in the larger space, above the dialog or at the foot. */
  function clearOf(d: HTMLElement) {
    const r = d.getBoundingClientRect(), t = box.getBoundingClientRect()
    if (t.top >= r.bottom + 8) return ''
    if (r.bottom > innerHeight - 72) return aboveButtons(d)
    return r.top >= t.height + 16 || r.top > innerHeight - r.bottom ? `${Math.max(0, innerHeight - r.top + 8)}px` : ''
  }
  /** In a dialog, the toast sits just above its bottom row of buttons, which stay clickable. */
  function aboveButtons(d: HTMLElement) {
    const end = d.getBoundingClientRect().bottom
    const tops = [...d.querySelectorAll('button')].filter(b => !box.contains(b) && b.offsetParent)
      .map(b => b.getBoundingClientRect()).filter(r => r.bottom > end - 72).map(r => r.top)
    return tops.length ? `${Math.max(44, innerHeight - Math.min(...tops) + 8)}px` : ''
  }
  // A dialog that closes with the toast in it hands it back before it goes.
  function closing(e: Event) {
    if (!(e.target instanceof Node) || e.target === box || !e.target.contains(box) || !home) return
    if (box.matches(':popover-open')) box.hidePopover()
    home.append(box)
    if (toast.now) queueMicrotask(show)
  }
  // A dialog opened while the toast shows takes it in.
  function opening(e: Event) {
    if (toast.now && e.target instanceof HTMLDialogElement && (e as ToggleEvent).newState === 'open') queueMicrotask(show)
  }
  $effect(() => {
    addEventListener('close', closing, true)
    addEventListener('toggle', opening, true)
    addEventListener('resize', place)
    return () => { removeEventListener('close', closing, true); removeEventListener('toggle', opening, true); removeEventListener('resize', place); watch.disconnect() }
  })
  $effect(() => {
    if (held || !toast.now || toast.now.error) return
    const id = toast.now.id
    const timer = setTimeout(() => toast.now?.id === id && hideToast(), 10000)
    return () => clearTimeout(timer)
  })
  async function undo() {
    const t = toast.now
    if (!t?.undo || busy) return
    busy = true
    const focused = box.contains(document.activeElement)
    try { await t.undo(); showToast(S.undone) } catch (e) { showError(e) } finally { busy = false }
    if (focused) (back?.isConnected ? back : document.querySelector<HTMLElement>('#works'))?.focus()
  }
  /** The toast's other action: it goes, then the action runs. */
  function act() {
    const a = toast.now?.action
    hideToast()
    a?.run()
  }
  function keys(e: KeyboardEvent) {
    // Esc closes an error, unless a text box, a dialog, Review, the viewer or a menu takes it.
    if (e.key === 'Escape' && toast.now?.error && !e.defaultPrevented && !(e.target as Element).closest?.('input, textarea, [contenteditable]')
      && !document.querySelector('dialog:modal, [role=dialog][aria-modal=true], [popover]:popover-open:not(.toast)')) { hideToast(); return }
    if (!toast.now?.undo || !(e.ctrlKey || e.metaKey) || e.shiftKey || e.altKey || e.key.toLowerCase() !== 'z') return
    // Not in a text box, and only where the change was made: not in a dialog opened since.
    if ((e.target as Element).closest?.('input, textarea, [contenteditable], [data-no-undo]')) return
    if (([...document.querySelectorAll('dialog:modal')].at(-1) ?? null) !== origin) return
    e.preventDefault()
    undo()
  }
</script>

<svelte:window onkeydown={keys} />

<div bind:this={box} popover="manual" class="toast" role="status" aria-live="polite"
  onpointerenter={() => (held = true)} onpointerleave={() => (held = false)} onfocusin={e => { held = true; if (e.relatedTarget instanceof HTMLElement && !box.contains(e.relatedTarget)) back = e.relatedTarget }} onfocusout={() => (held = false)}>
  {#if toast.now}
    {#key toast.now.id}
      <div class="in">
        <!-- An error is read out at once: its text is a new alert each time. -->
        <span class="text" dir="auto" role={toast.now.error ? 'alert' : undefined}>{toast.now.text}</span>
        {#if toast.now.undo}<button class="btn undo" disabled={busy} title={S.undoKey} onclick={undo}>{S.undo}</button>{/if}
        {#if toast.now.action}<button class="btn undo" onclick={act}>{toast.now.action.label}</button>{/if}
        <button class="x" aria-label={S.close} onclick={hideToast}><Icon name="clear" size={14} /></button>
      </div>
    {/key}
  {/if}
</div>

<style>
  /* Centred and kept clear of the rail on both sides, so the rail's Folders button stays reachable in a narrow window. */
  .toast { inset: auto 0 44px 0; margin: 0 auto; width: max-content; max-width: min(640px, calc(100vw - 176px)); padding: 0; border: 1px solid var(--line);
    border-radius: 10px; background: var(--bg); color: var(--ink); box-shadow: 0 12px 36px rgba(0, 0, 0, .24); overflow: visible; }
  .in { display: flex; align-items: center; gap: 12px; padding: 8px 8px 8px 16px; animation: up .18s cubic-bezier(.1, .9, .2, 1); }
  @keyframes up { from { opacity: 0; transform: translateY(10px) } }
  .text { font-size: 13.5px; overflow-wrap: anywhere; min-width: 0; max-height: 30vh; overflow-y: auto; }
  .undo { padding: 4px 14px; font-weight: 600; flex: none; }
  .x { flex: none; width: 28px; height: 28px; border: 0; background: none; border-radius: 6px; display: grid; place-items: center; color: var(--soft); }
  .x:hover { background: var(--hover); color: var(--ink); }
</style>
