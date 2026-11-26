<script lang="ts">
  // The context menu, as Windows 11 draws one: rounded, raised, a light acrylic tint, icons in a
  // column and hints on the right. It opens on its ticked choice, if any. Up and Down, Home and
  // End move, Enter picks; Esc, Tab, a press outside, the wheel or leaving the window closes it.
  // In the top layer, so nothing clips it.
  import { tick } from 'svelte'
  import { closeMenu, menu, type MenuItem } from './menus.svelte'
  import Icon from './Icon.svelte'

  let box: HTMLDivElement
  let x = $state(0), y = $state(0)
  /** Where the menu lives when no modal dialog is open. */
  let home: Node | null = null
  $effect(() => {
    const m = menu.open
    home ??= box.parentNode
    // A modal dialog makes the rest of the page inert, so a menu opened in one goes inside it to
    // take the pointer and keys; it closes with the dialog and goes back after.
    const modal = m ? [...document.querySelectorAll('dialog')].findLast(d => d.matches(':modal')) : undefined
    const into = modal ?? home
    if (into && box.parentNode !== into) {
      if (box.matches(':popover-open')) box.hidePopover()
      into.appendChild(box)
      modal?.addEventListener('close', () => closeMenu(), { once: true })
    }
    if (!m) { if (box.matches(':popover-open')) box.hidePopover(); return }
    x = m.x; y = m.y
    if (!box.matches(':popover-open')) box.showPopover()
    tick().then(() => {
      if (menu.open !== m) return
      // It stays inside the window: beside the pointer or under its button where there is room,
      // else turned to the other side.
      const w = box.offsetWidth, h = box.offsetHeight
      const r = m.under
      if (x + w > innerWidth - 8) x = Math.max(8, (r ? r.right : x) - w)
      if (y + h > innerHeight - 8) y = Math.max(8, (r ? r.top - 4 : y) - h)
      box.scrollTop = 0
      // A menu with a ticked choice opens on it, as Windows 11's dropdowns do; with a ring only
      // when opened from the keyboard. Any other opens on its first item from the keyboard.
      const ticked = items().find(e => e.getAttribute('role') === 'menuitemradio' && e.getAttribute('aria-checked') === 'true')
      if (ticked) ticked.focus({ focusVisible: m.keyboard } as FocusOptions)
      else if (m.keyboard) (items()[0] ?? box).focus()
      else box.focus()
    })
  })
  const items = () => [...box.querySelectorAll<HTMLElement>('[role^="menuitem"]:not(:disabled)')]
  // Focus goes back with a ring only to a menu opened from the keyboard: Esc after a right-click
  // or a click leaves none.
  function close() {
    const from = menu.open?.from, keyboard = !!menu.open?.keyboard
    closeMenu()
    if (from?.isConnected) from.focus({ focusVisible: keyboard } as FocusOptions)
  }
  function pick(run: () => void) { close(); run() }
  // Esc and the arrows are kept from Library's and Photos' own keys by preventDefault, which
  // both check: keep it on every key handled here.
  function keys(e: KeyboardEvent) {
    if (!menu.open || !box.contains(document.activeElement)) return
    const all = items(), i = all.indexOf(document.activeElement as HTMLElement)
    const to = (k: number) => { e.preventDefault(); all[(k + all.length) % all.length]?.focus() }
    if (e.key === 'ArrowDown') to(i + 1)
    else if (e.key === 'ArrowUp') to(i < 0 ? -1 : i - 1)
    else if (e.key === 'Home') to(0)
    else if (e.key === 'End') to(-1)
    else if (e.key === 'Escape' || e.key === 'Tab') { e.preventDefault(); e.stopPropagation(); close() }
    else if (e.key === 'ArrowLeft' || e.key === 'ArrowRight') e.preventDefault()
  }
  /** A label cut short shows in full on hover. */
  function named(e: PointerEvent, label: string) {
    const b = e.currentTarget as HTMLElement, l = b.querySelector<HTMLElement>('.l')
    b.title = l && l.scrollWidth > l.clientWidth ? label : ''
  }
  const outside = (e: Event) => menu.open && !box.contains(e.target as Node)
  /** The items between separators, each set under its heading as one group, so a screen reader
   * reads a tick as one choice of its set. */
  const groups = $derived.by(() => {
    const out: { head: string; items: Exclude<MenuItem, { sep: true } | { head: string }>[] }[] = [{ head: '', items: [] }]
    for (const it of menu.open?.items ?? []) {
      if ('sep' in it) out.push({ head: '', items: [] })
      else if ('head' in it) { if (out.at(-1)!.items.length || out.at(-1)!.head) out.push({ head: '', items: [] }); out.at(-1)!.head = it.head }
      else out.at(-1)!.items.push(it)
    }
    return out.filter(g => g.items.length)
  })
</script>

<!-- The wheel closes it, not any scroll: the page also scrolls itself, as Back restores a place. -->
<svelte:window onkeydowncapture={keys} onpointerdowncapture={e => outside(e) && closeMenu(true)} onblur={() => menu.open && closeMenu()}
  onresize={() => menu.open && closeMenu()} onwheelcapture={e => outside(e) && closeMenu()} ontouchmovecapture={e => outside(e) && closeMenu()} />

<div class="menu" popover="manual" role="menu" tabindex="-1" aria-label={menu.open?.label} bind:this={box} style="left:{x}px;top:{y}px" oncontextmenu={e => e.preventDefault()}
  onfocusout={e => menu.open && e.relatedTarget && !box.contains(e.relatedTarget as Node) && closeMenu()}>
  {#each groups as g, gi (gi)}
    {#if gi}<div class="sep" role="separator"></div>{/if}
    <div role="group" aria-labelledby={g.head ? `menu-head-${gi}` : undefined}>
    {#if g.head}<div class="head" id="menu-head-{gi}">{g.head}</div>{/if}
    {#each g.items as it, i (`${i}|${it.label}`)}
      <button role={it.checked === undefined ? 'menuitem' : it.toggle ? 'menuitemcheckbox' : 'menuitemradio'} aria-checked={it.checked} class:danger={it.danger}
        disabled={it.disabled} tabindex="-1" onclick={() => pick(it.run)} onpointermove={e => (e.currentTarget as HTMLElement).focus()} onpointerenter={e => named(e, it.label)}>
        <span class="ic" class:sec={!!it.section} style={it.section ? `--c:${it.section}` : undefined}>{#if it.checked}<Icon name="check" />{:else if it.icon}<Icon name={it.icon} />{/if}{#if it.section}<span class="sc"><Icon name="shortcut" size={9} /></span>{/if}</span>
        <span class="l">{it.label}</span>
        {#if it.hint}<span class="k">{it.hint}</span>{/if}
      </button>
    {/each}
    </div>
  {/each}
</div>

<style>
  .menu { position: fixed; inset: auto; margin: 0; width: max-content; min-width: 220px; max-width: min(440px, calc(100vw - 16px)); max-height: calc(100vh - 16px); overflow: auto;
    padding: 4px; border-radius: 8px; color: var(--ink); outline: none;
    background: color-mix(in srgb, var(--field) 86%, transparent); backdrop-filter: blur(24px) saturate(1.5);
    border: 1px solid color-mix(in srgb, var(--line) 85%, transparent); box-shadow: 0 8px 28px rgba(0,0,0,.2), 0 2px 6px rgba(0,0,0,.08); }
  .menu:popover-open { animation: pop .14s cubic-bezier(.1, .9, .2, 1); }
  @keyframes pop { from { opacity: 0; transform: translateY(-4px) } }
  button { display: flex; align-items: center; gap: 10px; width: 100%; border: 0; background: none; padding: 6px 10px; border-radius: 5px; text-align: left; font-size: 13.5px; white-space: nowrap; }
  button:focus, button:hover:not(:disabled) { background: var(--hover); outline: none; }
  button:focus-visible { box-shadow: inset 0 0 0 2px var(--focus); }
  button:disabled { color: var(--muted); cursor: default; opacity: .7; }
  .ic { width: 16px; display: inline-grid; place-items: center; color: var(--soft); flex: none; }
  [aria-checked='true'] .ic { color: var(--link); }
  .ic.sec { position: relative; color: var(--c); }
  .ic .sc { position: absolute; right: -5px; bottom: -4px; display: grid; place-items: center; width: 12px; height: 12px; border-radius: 3px; background: var(--field); color: var(--c);
    box-shadow: 0 0 0 1px color-mix(in srgb, var(--c) 45%, var(--line)); }
  .l { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; }
  .k { flex: none; color: var(--muted); font-size: 12px; padding-left: 16px; max-width: 220px; overflow: hidden; text-overflow: ellipsis; }
  .danger, .danger .ic { color: var(--danger); }
  .sep { height: 1px; background: var(--line); margin: 4px 6px; }
  .head { padding: 7px 10px 3px; font-size: 12px; font-weight: 600; color: var(--muted); }
</style>
