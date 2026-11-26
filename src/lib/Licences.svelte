<script lang="ts">
  // Settings, Licences: the app's own licence, then every part the package carries, from the
  // tools/notices.json scripts/notices.mjs made for it, with its licence texts and its source.
  import { onMount } from 'svelte'
  import { api, type NoticeItem, type NoticeList } from './api'
  import { S } from './strings'
  import Icon from './Icon.svelte'

  let { onerror }: { onerror: (e: string) => void } = $props()

  let licence = $state('')
  let list = $state<NoticeList | null>(null)
  let file = $state('')
  let loaded = $state(false)
  let own = $state(false)
  let query = $state('')
  /** Open groups and open parts, by key. */
  let open = $state(new Set<string>())
  const toggle = (k: string) => { const s = new Set(open); if (!s.delete(k)) s.add(k); open = s }

  onMount(async () => {
    try {
      const n = await api.notices()
      licence = n.licence; file = n.file
      list = n.notices ? JSON.parse(n.notices) : null
    } catch (e) { onerror(String(e)) }
    loaded = true
  })

  const q = $derived(query.trim().toLowerCase())
  const shown = (i: NoticeItem) => !q || i.name.toLowerCase().includes(q)
  const groups = $derived((list?.groups ?? []).map(g => ({ ...g, items: g.items.filter(shown) })).filter(g => g.items.length || !q))
  const folder = $derived(file.replace(/[\\/][^\\/]*$/, ''))
  const local = (url: string) => `${folder}${folder.includes('\\') ? '\\' : '/'}${url.replaceAll('/', folder.includes('\\') ? '\\' : '/')}`
  /** What failed, shown by the control that failed, where the person is looking. */
  let failed = $state<{ at: string; text: string } | null>(null)
  const run = (at: string, f: () => Promise<unknown>) => { failed = null; f().catch(e => (failed = { at, text: String(e) })) }
  // A group of a few parts shows them; a long one folds them until asked, or until a search.
  const folds = (n: number) => n > 6
</script>

{#snippet part(g: string, i: NoticeItem)}
  {@const k = `${g}/${i.name}/${i.version}`}
  <div class="part">
    <div class="head">
      <div class="name"><b dir="auto">{i.name}</b>{#if i.version}<span class="ver muted">{i.version}</span>{/if}{' '}
        {#if i.licence}<span class="lic">{i.licence}</span>{/if}</div>
      {#if i.texts.length}
        <button class="more" aria-expanded={open.has(k)} onclick={() => toggle(k)}>{S.licenceTexts(i.texts.length)}<Icon name="down" size={12} /></button>
      {/if}
    </div>
    <div class="src">
      {#each i.sources as s (s.url)}
        {#if s.url.startsWith('https://')}
          <span>{s.what}: <a href={s.url} onclick={e => { e.preventDefault(); run(k, () => api.openWeb(s.url)) }}>{s.url}</a></span>
        {:else}
          <span>{s.what}: <button class="link" onclick={() => run(k, () => api.reveal(local(s.url)))}>{s.url}</button></span>
        {/if}
      {/each}
      {#if !i.texts.length}<span class="muted">{S.noLicenceText}</span>{/if}
      {#if failed?.at === k}<span class="err" role="alert">{failed.text}</span>{/if}
    </div>
    {#if open.has(k)}
      {#each i.texts as t (t.file + t.text)}
        <p class="file">{t.file}</p>
        <!-- svelte-ignore a11y_no_noninteractive_tabindex (a long text that scrolls must take the focus, to scroll by keys) -->
        <pre tabindex="0">{list?.texts[t.text] ?? ''}</pre>
      {/each}
    {/if}
  </div>
{/snippet}

<h4>{S.ownLicenceTitle}</h4>
<p>{S.ownLicence}</p>
<div class="foot left"><button class="btn" aria-expanded={own} onclick={() => (own = !own)}>{own ? S.hideLicence : S.showLicence}</button></div>
{#if own}
  <!-- svelte-ignore a11y_no_noninteractive_tabindex (as above) -->
  <pre class="own" tabindex="0">{licence}</pre>
{/if}

{#if loaded && !list}
  <p class="muted gap">{S.partsMissing}</p>
{:else if list}
  <p class="muted gap">{S.partsWhat} {S.webLinks}</p>
  <div class="tools">
    <input class="find" type="search" placeholder={S.findPart} aria-label={S.findPartIn} bind:value={query} />
    <button class="btn" onclick={() => run('file', () => api.reveal(file))}>{S.noticesFile}</button>
  </div>
  {#if failed?.at === 'file'}<p class="err" role="alert">{failed.text}</p>{/if}
  {#each groups as g (g.id)}
    <section class="group" aria-labelledby="lic-{g.id}">
      <div class="ghead">
        <h4 id="lic-{g.id}">{S.groupTitles[g.id] ?? g.id} <span class="count">{S.nParts(g.items.length)}</span></h4>
        {#if folds(g.items.length) && !q}
          <button class="more" aria-expanded={open.has(g.id)} onclick={() => toggle(g.id)}>{open.has(g.id) ? S.hideParts : S.showParts(g.items.length)}<Icon name="down" size={12} /></button>
        {/if}
      </div>
      <p class="muted what">{S.groupWhat[g.id] ?? ''}
        {#if g.source}<a href="https://github.com/BtbN/FFmpeg-Builds/tree/{g.source}" onclick={e => { e.preventDefault(); run(g.id, () => api.openWeb(`https://github.com/BtbN/FFmpeg-Builds/tree/${g.source}`)) }}>{S.buildScripts}</a>{/if}</p>
      {#if failed?.at === g.id}<p class="err" role="alert">{failed.text}</p>{/if}
      {#if !folds(g.items.length) || q || open.has(g.id)}
        {#each g.items as i (i.name + i.version)}{@render part(g.id, i)}{/each}
      {/if}
    </section>
  {:else}
    <p>{S.noParts(query.trim())}</p>
  {/each}
{/if}

<style>
  h4 { margin: 18px 0 4px; font-size: 14px; }
  p { margin: 0 0 10px; max-width: 70ch; }
  .gap { margin-top: 18px; }
  .foot { display: flex; gap: 8px; margin-top: 4px; }
  .err { color: var(--danger); }
  pre { flex: none; white-space: pre-wrap; overflow-wrap: anywhere; font-family: ui-monospace, Consolas, monospace; font-size: 12px; line-height: 1.45;
    background: var(--side); border-radius: 8px; padding: 10px 12px; margin: 4px 0 10px; max-height: 320px; overflow: auto; }
  pre:focus-visible { outline: 2px solid var(--focus); outline-offset: 1px; }
  pre.own { margin-top: 10px; }
  .tools { display: flex; gap: 10px; align-items: center; flex-wrap: wrap; margin: 4px 0 6px; }
  .find { flex: 1; min-width: 180px; max-width: 360px; border: 1px solid var(--line); border-radius: 6px; padding: 5px 9px; background: var(--field); }
  .group { margin-top: 10px; }
  .ghead { display: flex; align-items: baseline; gap: 12px; flex-wrap: wrap; }
  .ghead h4 { flex: 1; min-width: 0; }
  .count { font-weight: 400; color: var(--muted); font-size: 12.5px; margin-left: 4px; }
  .what { font-size: 12.5px; }
  .part { padding: 8px 0; border-bottom: 1px solid var(--line); min-width: 0; }
  .head { display: flex; gap: 12px; align-items: flex-start; }
  .name { flex: 1; min-width: 0; overflow-wrap: anywhere; }
  .ver { margin-left: 6px; overflow-wrap: anywhere; }
  .lic { display: inline-block; margin: 0 8px 0 0; font-size: 11.5px; background: var(--side); border-radius: 9px; padding: 0 8px; overflow-wrap: anywhere; }
  .src { display: flex; flex-direction: column; font-size: 12.5px; color: var(--muted); min-width: 0; }
  .src span { overflow-wrap: anywhere; }
  .src a, .what a, .link { color: var(--link); }
  .link { border: 0; background: none; padding: 0; font: inherit; text-decoration: underline; cursor: pointer; overflow-wrap: anywhere; text-align: left; }
  .more { flex: none; display: inline-flex; align-items: center; gap: 6px; border: 1px solid var(--line); background: var(--field); border-radius: 6px;
    padding: 2px 8px 2px 10px; font-size: 12.5px; transition: background .15s; }
  .more:hover { background: var(--hover); }
  .more :global(svg) { transition: transform .15s; }
  .more[aria-expanded='true'] :global(svg) { transform: rotate(180deg); }
  .file { margin: 8px 0 0; font-size: 12px; font-weight: 600; overflow-wrap: anywhere; }
</style>
