<script lang="ts">
  import { api, type Folder } from './api'
  import { S } from './strings'
  import Self from './TreeNode.svelte'

  let { folder, depth, tickOf, toggle }: {
    folder: Folder; depth: number
    tickOf: (p: string) => 'on' | 'off' | 'some'
    toggle: (p: string) => void
  } = $props()

  let open = $state(false)
  let kids = $state<Folder[] | null>(null)
  let error = $state('')

  async function expand() {
    open = !open
    if (open && kids === null) {
      try { kids = await api.folders(folder.path) } catch (e) { error = String(e); kids = [] }
    }
  }
  const st = $derived(tickOf(folder.path))
</script>

<li>
  <div class="row" style="padding-left:{depth * 18 + 4}px">
    <button class="tw" aria-label={open ? S.collapse(folder.name) : S.expand(folder.name)} aria-expanded={open} onclick={expand}>{open ? '▾' : '▸'}</button>
    <label>
      <input type="checkbox" checked={st === 'on'} indeterminate={st === 'some'} onchange={() => toggle(folder.path)} />
      <span title={folder.name}>{folder.name}</span>
    </label>
  </div>
  {#if open}
    {#if error}<p class="err" style="padding-left:{depth * 18 + 50}px">{error}</p>{/if}
    {#if kids}
      <ul>{#each kids as k (k.path)}<Self folder={k} depth={depth + 1} {tickOf} {toggle} />{/each}</ul>
    {:else}
      <p class="muted" style="padding-left:{depth * 18 + 50}px">…</p>
    {/if}
  {/if}
</li>

<style>
  ul { list-style: none; margin: 0; padding: 0; }
  .row { display: flex; align-items: center; gap: 2px; height: 28px; border-radius: 5px; }
  .row:hover { background: var(--hover); }
  .tw { width: 24px; height: 24px; border: 0; background: none; color: var(--muted); font-size: 12px; }
  label { display: flex; align-items: center; gap: 8px; flex: 1; min-width: 0; cursor: pointer; }
  label span { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  input { flex: none; width: 15px; height: 15px; accent-color: var(--focus); margin: 0; }
  p { margin: 2px 0; font-size: 12.5px; }
  .err { color: var(--danger); }
</style>
