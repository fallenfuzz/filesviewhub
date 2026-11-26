<script lang="ts">
  // The drive and folder tree with checkboxes (`decisions/2026/10/05/checked-folders-on-any-drive.md`). A tick on a folder covers everything in it;
  // a folder unticked inside a ticked one is left out. A drive takes a tick as a folder does, except
  // the system drive and Computer, where only folders can be ticked.
  import { onMount } from 'svelte'
  import { api, size, type Drive, type Folder } from './api'
  import { S } from './strings'
  import TreeNode from './TreeNode.svelte'
  import { setCaseKept } from './folders.svelte'

  let { picks = $bindable() }: { picks: Map<string, 'on' | 'off'> } = $props()

  let drives = $state<Drive[]>([])
  let open = $state<Record<string, Folder[] | null>>({})

  onMount(async () => { drives = await api.drives(); setCaseKept(drives) })

  // Windows paths use a backslash, Linux and macOS ones /; a Windows drive root's key is `d:`.
  const sep = (p: string) => (p.includes('\\') ? '\\' : '/')
  // A path compares as the drive holding it compares names: as written where they keep their case
  // (Linux), else with case aside.
  const keepsCase = (p: string) => drives.map(d => ({ d, root: d.path.replace(/[\\/]+$/, '') }))
    .filter(({ root }) => p === root || p.startsWith(root + sep(p)))
    .reduce<{ d: Drive; root: string } | undefined>((a, x) => (!a || x.root.length > a.root.length ? x : a), undefined)?.d.keeps_case ?? false
  const key = (p: string) => (keepsCase(p) ? p : p.toLowerCase()).replace(/[\\/]+$/, '')
  function parent(p: string) { const i = key(p).lastIndexOf(sep(p)); return i > (sep(p) === '\\' ? 1 : 0) ? key(p).slice(0, i) : null }
  // A drive mounted inside another drive's folders (a network share in Home on Linux) is a drive
  // of its own: a tick never passes into it or out of it (drives.rs `mount_of`). Linux's
  // Home (serial 1) is a folder of Computer, no mount.
  const mounts = $derived(drives.filter(d => !(d.path.startsWith('/') && d.serial === 1)).map(d => key(d.path)))
  const isMount = (p: string) => mounts.includes(key(p))
  const mountOf = (p: string) => mounts.filter(m => key(p) === m || key(p).startsWith(m + sep(p))).reduce((a, m) => (m.length > a.length ? m : a), '')
  const sameMount = (q: string, p: string) => mountOf(q) === mountOf(p)
  function effective(p: string, m = picks): boolean {
    for (let q: string | null = p; q; q = isMount(q) ? null : parent(q)) {
      for (const [k, v] of m) if (key(k) === q || key(k) === key(q)) return v === 'on'
    }
    return false
  }
  function below(p: string, on: 'on' | 'off') { const k = key(p) + sep(p); for (const [q, v] of picks) if (v === on && key(q).startsWith(k) && sameMount(q, p)) return true; return false }
  function tickOf(p: string): 'on' | 'off' | 'some' {
    const on = effective(p)
    if (on) return below(p, 'off') ? 'some' : 'on'
    return below(p, 'on') ? 'some' : 'off'
  }
  function toggle(p: string) {
    const was = effective(p)
    const k = key(p)
    const next = new Map([...picks].filter(([q]) => key(q) !== k && !(key(q).startsWith(k + sep(p)) && sameMount(q, p))))
    const par = isMount(p) ? null : parent(p)
    const parentOn = par ? effective(par, next) : false
    if (was && parentOn) next.set(p, 'off')
    if (!was && !parentOn) next.set(p, 'on')
    picks = next
  }

  async function expand(d: Drive) {
    if (d.path in open) { delete open[d.path]; return }
    open[d.path] = null
    open[d.path] = await api.folders(d.path).catch(() => [])
  }
  const ticked = $derived([...picks.values()].filter(v => v === 'on').length)
</script>

<div class="tree">
  {#each drives as d (d.path)}
    {@const st = tickOf(d.path)}
    <div class="drive">
      <button class="tw" tabindex="-1" aria-hidden="true" onclick={() => expand(d)}>{d.path in open ? '▾' : '▸'}</button>
      {#if d.system}
        <span class="nobox" title={S.noSystemDrive}></span>
      {:else}
        <input type="checkbox" aria-label={S.tickDrive(d.path.startsWith('/') ? d.label || d.path : `${d.path.replace(/\\$/, '')} ${d.label}`.trim())} checked={st === 'on'} indeterminate={st === 'some'} onchange={() => toggle(d.path)} />
      {/if}
      <button class="dn" aria-expanded={d.path in open} onclick={() => expand(d)} title={d.system ? S.noSystemDrive : d.label ? `${d.label}\n${d.path}` : undefined}>
        {#if d.path.startsWith('/')}
          <!-- A Linux drive: its label, else its mount point, then where it is mounted -->
          <b class="nm" dir="auto">{d.label || d.path}</b>{#if d.label}<i class="at muted" dir="auto">{d.path}</i>{/if}
        {:else}
          <b>{d.path.replace(/\\$/, '')}</b><span class="nm" dir="auto">{d.label || (d.kind === 'removable' ? S.removable : d.kind === 'network' ? S.networkDrive : S.localDisk)}</span>
        {/if}
        {#if d.system}<span class="sys muted">{S.foldersOnly}</span>{/if}
        <span class="free muted">{d.total ? `${size(d.free)} free of ${size(d.total)}` : ''}</span>
      </button>
    </div>
    {#if d.path in open}
      {#if open[d.path]}
        <ul>{#each open[d.path] ?? [] as f (f.path)}<TreeNode folder={f} depth={1} {tickOf} {toggle} />{/each}</ul>
      {:else}<p class="muted">…</p>{/if}
    {/if}
  {/each}
</div>
<p class="count muted">{S.ticked(ticked)}</p>

<style>
  .tree { overflow: auto; border: 1px solid var(--line); border-radius: 8px; background: var(--field); padding: 6px; min-height: 0; flex: 1; }
  ul { list-style: none; margin: 0; padding: 0; }
  .drive { display: flex; align-items: center; height: 32px; }
  .tw { width: 24px; height: 24px; border: 0; background: none; color: var(--muted); font-size: 12px; flex-shrink: 0; }
  .drive input, .nobox { width: 15px; height: 15px; margin: 0 4px 0 2px; flex-shrink: 0; }
  .drive input { accent-color: var(--focus); cursor: pointer; }
  .dn { border: 0; background: none; text-align: left; flex: 1; min-width: 0; padding: 4px; border-radius: 5px; display: flex; gap: 8px; align-items: baseline; }
  .dn:hover { background: var(--hover); }
  .dn .free { margin-left: auto; padding-left: 8px; font-size: 12.5px; white-space: nowrap; }
  .sys { font-size: 12.5px; white-space: nowrap; min-width: 0; overflow: hidden; text-overflow: ellipsis; flex: 0 1000 auto; }
  .nm, .at { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .nm { flex: 0 1 auto; }
  .at { flex: 0 1000 auto; font-style: normal; font-size: 12.5px; }
  p { margin: 2px 0 2px 50px; }
  .count { margin: 8px 0 0; font-size: 13px; }
</style>
