<script lang="ts">
  import logo from '../../src-tauri/icons/icon.svg'
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import { S } from './strings'
  import { saveUi, ui } from './ui.svelte'

  // The search box shows only once there is a library to search; Settings is always there.
  let { query = $bindable(''), search, onhome, onsettings }: { query: string; search: boolean; onhome: () => void; onsettings: () => void } = $props()
  // A narrow window always hides the folder columns, so the switch has nothing to do there.
  let narrow = $state(false)
  $effect(() => {
    const mq = window.matchMedia('(max-width: 999px)')
    narrow = mq.matches
    const f = (e: MediaQueryListEvent) => (narrow = e.matches)
    mq.addEventListener('change', f)
    return () => mq.removeEventListener('change', f)
  })
  function autohide() { ui.autohide = !ui.autohide; saveUi() }
  const win = getCurrentWindow()
  let input = $state<HTMLInputElement>()

  function keys(e: KeyboardEvent) {
    const typing = e.target instanceof HTMLElement && e.target.closest('input, textarea, select, [contenteditable]')
    // Ctrl+K, or / as in the Hub, jumps to the search box.
    if (((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') || (e.key === '/' && !typing && !e.ctrlKey && !e.altKey)) {
      e.preventDefault(); input?.focus(); input?.select()
    }
  }
</script>

<svelte:window onkeydown={keys} />

<header data-tauri-drag-region>
  <div class="name" data-tauri-drag-region>
    <button class="home" onclick={onhome} title={S.nextUp}><img class="logo" src={logo} alt="" />{S.app}</button>
    {#if search && !narrow}
      <button class="side" aria-pressed={ui.autohide} onclick={autohide} aria-label={S.autohide} title={ui.autohide ? S.autohideOff : S.autohideTip}>
        <svg viewBox="0 0 16 16"><rect x="1.5" y="2.5" width="13" height="11" rx="2" /><path d="M6 2.5v11" />{#if !ui.autohide}<path class="fill" d="M2 3h4v10H2z" />{/if}</svg>
      </button>
    {/if}
    <button class="side" onclick={onsettings} aria-label={S.settings} title={S.settings}>
      <svg class="gear" viewBox="0 0 24 24"><path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z" /><circle cx="12" cy="12" r="3" /></svg>
    </button>
  </div>
  {#if search}
    <input bind:this={input} bind:value={query} type="search" placeholder="{S.search}  {S.searchKey}" aria-label={S.searchLabel} />
  {:else}
    <div data-tauri-drag-region></div>
  {/if}
  <div class="win">
    <button aria-label={S.minimize} onclick={() => win.minimize()}><svg viewBox="0 0 10 10"><path d="M1 5h8" /></svg></button>
    <button aria-label={S.maximize} onclick={() => win.toggleMaximize()}><svg viewBox="0 0 10 10"><rect x="1.5" y="1.5" width="7" height="7" /></svg></button>
    <button class="close" aria-label={S.close} onclick={() => win.close()}><svg viewBox="0 0 10 10"><path d="M1.5 1.5l7 7M8.5 1.5l-7 7" /></svg></button>
  </div>
</header>

<style>
  header { height: 44px; display: grid; grid-template-columns: 1fr minmax(320px, 520px) 1fr; align-items: center; padding-left: 14px; background: var(--bar); user-select: none; }
  .name { display: flex; gap: 8px; align-items: center; font-weight: 600; font-size: 13px; }
  .logo { width: 20px; height: 20px; }
  .side { width: 28px; height: 26px; border: 0; border-radius: 6px; background: none; display: grid; place-items: center; color: var(--soft); }
  .side:hover { background: var(--hover); color: var(--ink); }
  .side[aria-pressed='true'] { color: var(--focus); }
  .side svg { width: 16px; height: 16px; stroke-width: 1.3; }
  .side .gear { stroke-width: 1.8; }
  .side .fill { fill: currentColor; stroke: none; opacity: .35; }
  .home { display: flex; gap: 8px; align-items: center; border: 0; background: none; padding: 2px 6px 2px 0; font-weight: 600; font-size: 13px; }
  input { height: 30px; border: 1px solid var(--line); background: var(--field); border-radius: 6px; padding: 0 12px; width: 100%; }
  .win { justify-self: end; display: flex; height: 100%; }
  .win button { width: 46px; height: 100%; border: 0; background: none; display: grid; place-items: center; }
  .win button:hover { background: var(--hover); }
  .win .close:hover { background: #C42B1C; color: #fff; }
  svg { width: 10px; height: 10px; fill: none; stroke: currentColor; stroke-width: 1; }
</style>
