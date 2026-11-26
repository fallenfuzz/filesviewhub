// The one context menu of the app (Menu.svelte, mounted once in App): a right-click on a thing
// and the ⋯ button for it open the same items. `openMenu` takes the event that asked for it.
import type { IconName } from './Icon.svelte'

export type MenuItem =
  | { sep: true }
  | { head: string }
  | {
      label: string; run: () => void
      /** A name from Icon.svelte, shown in the icon column. */
      icon?: IconName
      /** A Split-out Section (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`): its icon in its colour, with a shortcut arrow. */
      section?: string
      /** Right-aligned and muted: a key, or what the choice is set to now. */
      hint?: string
      disabled?: boolean; danger?: boolean
      /** One of a set (a tick when on); `toggle` makes it an on/off switch instead. */
      checked?: boolean; toggle?: boolean
    }

type Open = {
  items: MenuItem[]; label: string
  /** Where it opens: at the pointer, or under the button that opened it. */
  x: number; y: number; under?: DOMRect
  /** Which button opened it, for its aria-expanded; empty for a right-click. */
  key: string
  /** Opened from the keyboard: the first item takes focus at once. */
  keyboard: boolean
  /** Focus goes back here when the menu closes without a pick. */
  from: HTMLElement | null
}

export const menu = $state<{ open: Open | null }>({ open: null })

/** Opens `items` for the event: under a clicked button, at the pointer for a right-click, or
 * under the focused thing for the menu key or Shift+F10. `key` names the button for aria-expanded. */
export function openMenu(e: MouseEvent | KeyboardEvent, items: MenuItem[], label: string, key = '') {
  e.preventDefault(); e.stopPropagation()
  const el = e.currentTarget as HTMLElement
  // A click from the keyboard (Enter, Space) has no pointer; a contextmenu with no right press
  // just before it came from the menu key or Shift+F10.
  const keyboard = e instanceof KeyboardEvent || (e.type === 'contextmenu' ? performance.now() - rightAt > 1000 : e.detail === 0)
  // Focus goes back where it was when the menu closes without a pick, or after one.
  const now = document.activeElement as HTMLElement | null
  const from = now && now !== document.body ? now : el
  // A press on the button of the open menu closes it (Menu.svelte); its click must not reopen it.
  if (key && (menu.open?.key === key || (closed.key === key && performance.now() - closed.at < 400))) { menu.open = null; closed.key = ''; return }
  if (e.type !== 'contextmenu' || keyboard) {
    const t = e.type === 'contextmenu' ? (e.target as HTMLElement) : el
    const r = t.getBoundingClientRect()
    menu.open = { items, label, x: r.left, y: r.bottom + 4, under: r, key, keyboard, from }
  } else {
    const m = e as MouseEvent
    menu.open = { items, label, x: m.clientX, y: m.clientY, key, keyboard, from }
  }
}

const closed = { key: '', at: 0 }
/** When the right button last went down: a contextmenu right after it is a right-click. */
let rightAt = -Infinity
addEventListener('pointerdown', e => { if (e.button === 2) rightAt = performance.now() }, true)
/** Closes the menu; `pressed` says a press outside it closed it, maybe on its own button. */
export function closeMenu(pressed = false) {
  if (pressed) { closed.key = menu.open?.key ?? ''; closed.at = performance.now() }
  menu.open = null
}
export const menuOpen = (key: string) => menu.open?.key === key
