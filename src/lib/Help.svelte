<script lang="ts">
  // Settings, Help: README.md, made into HTML at build time, with a chip for each chapter.
  import html, { chapters } from 'virtual:readme'
  import { onMount } from 'svelte'
  import { S } from './strings'

  let body = $state<HTMLElement>()
  let bar = $state<HTMLElement>()
  let at = $state(chapters[0]?.id ?? '')
  /** The pane that scrolls: Settings' section. */
  const pane = () => body?.closest('section') ?? null

  function go(id: string) {
    const h = body?.querySelector<HTMLElement>(`#${CSS.escape(id)}`), p = pane()
    if (!h || !p) return
    p.scrollTo({ top: h.offsetTop - (bar?.offsetHeight ?? 0) - 8, behavior: 'smooth' })
    at = id
    picked = id
    h.setAttribute('tabindex', '-1')
    h.focus({ preventScroll: true })
  }
  // The chapter shown is the last one whose heading is above the chips' bottom edge; at the end of
  // the page, where the last chapters cannot reach that edge, the last one whose heading shows.
  function track() {
    const p = pane()
    if (!p || !body) return
    const box = p.getBoundingClientRect()
    const end = p.scrollTop + p.clientHeight >= p.scrollHeight - 2
    const line = end ? box.bottom - 24 : box.top + (bar?.offsetHeight ?? 0) + 16
    let id = chapters[0]?.id ?? ''
    for (const c of chapters) {
      const h = body.querySelector(`#${CSS.escape(c.id)}`)
      if (h && h.getBoundingClientRect().top <= line) id = c.id
    }
    // A chapter picked from the chips stays picked when the page cannot scroll it any higher.
    if (end && picked && chapters.findIndex(c => c.id === picked) > chapters.findIndex(c => c.id === id)) id = picked
    at = id
  }
  let picked = ''
  onMount(() => {
    const p = pane()
    p?.addEventListener('scroll', track, { passive: true })
    // The README's own links to its chapters scroll the pane; it has no other links.
    const click = (e: MouseEvent) => {
      const a = (e.target as HTMLElement).closest('a')
      if (!a) return
      e.preventDefault()
      const href = a.getAttribute('href') ?? ''
      if (href.startsWith('#')) go(href.slice(1))
    }
    body?.addEventListener('click', click)
    return () => { p?.removeEventListener('scroll', track); body?.removeEventListener('click', click) }
  })
</script>

<nav class="chips" aria-label={S.helpChapters} bind:this={bar}>
  {#each chapters as c (c.id)}
    <button class:on={at === c.id} aria-current={at === c.id ? 'location' : undefined} onclick={() => go(c.id)}>{c.text}</button>
  {/each}
</nav>
<div class="readme" bind:this={body}>{@html html}</div>

<style>
  /* Clear of the dialog's close button, which stays above it. */
  .chips { position: sticky; top: -22px; z-index: 1; display: flex; flex-wrap: wrap; gap: 6px; margin: 0 -28px 4px; padding: 10px 56px 10px 28px;
    background: var(--pane); border-bottom: 1px solid var(--line); }
  .chips button { border: 1px solid var(--line); background: var(--field); border-radius: 999px; padding: 3px 12px; font-size: 13px;
    transition: background .15s, border-color .15s, color .15s; }
  .chips button:hover { background: var(--hover); }
  .chips button.on { background: var(--focus); border-color: var(--focus); color: var(--on-focus); }
  .readme { max-width: 72ch; line-height: 1.55; overflow-wrap: anywhere; }
  .readme :global(h1) { display: none; }
  .readme :global(h2) { font-size: 17px; margin: 26px 0 8px; scroll-margin-top: 60px; }
  .readme :global(h2:first-child) { margin-top: 12px; }
  .readme :global(h3) { font-size: 14.5px; margin: 18px 0 6px; }
  .readme :global(h2:focus), .readme :global(h3:focus) { outline: none; }
  .readme :global(p) { margin: 0 0 10px; }
  .readme :global(ul), .readme :global(ol) { margin: 0 0 12px; padding-left: 22px; }
  .readme :global(li) { margin: 3px 0; }
  .readme :global(li > p) { margin: 0; }
  .readme :global(code) { font-family: ui-monospace, Consolas, monospace; font-size: .9em; background: var(--side); border-radius: 4px; padding: 1px 5px; }
  .readme :global(img) { display: block; max-width: 100%; height: auto; margin: 6px 0 14px; border-radius: 8px; border: 1px solid var(--line);
    box-shadow: 0 4px 16px rgba(0, 0, 0, .12); }
  .readme :global(a) { color: var(--link); }
  .readme :global(a:focus-visible) { outline: 2px solid var(--focus); outline-offset: 2px; border-radius: 3px; }
  .readme :global(table) { overflow-wrap: break-word; border-collapse: collapse; margin: 0 0 12px; font-size: 13.5px; }
  .readme :global(th), .readme :global(td) { text-align: left; vertical-align: top; padding: 5px 12px 5px 0; border-bottom: 1px solid var(--line); }
</style>
