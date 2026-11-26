<script lang="ts">
  // A subfolder as a tile: up to four covers or frames from inside it and how many Works it holds (`decisions/2026/09/29/folder-shows-own-works-and-one-box-shape.md`).
  import type { Marks, Thumb, Work } from './api'
  import { thumbSrc } from './api'
  import { S } from './strings'
  import Icon, { type IconName } from './Icon.svelte'

  let { name, works, subs, color, marks, thumbs, cover, compact = false, ev = false, sub = 'folder', path, link, off = false, onopen }: {
    name: string; works: Work[]; subs: number; color: string; marks: Marks; thumbs: Record<string, Thumb>
    /** The cover chosen for this folder (`decisions/2026/10/22/covers-are-chosen-never-fetched-by-themselves.md`): shown alone, in place of its Works'. */
    cover?: Thumb
    /** In Photos: the Works are events, and a year's subfolders are months. */
    ev?: boolean; sub?: string
    /** A row, as the compact cards are. */
    compact?: boolean
    /** Its full path on disk, shown on hover. */
    path?: string
    /** A folder split out as a Section of its own (`decisions/2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`): that Section's icon; the tile opens it. */
    link?: IconName
    /** Its Section's drive is not connected: greyed, still opening what was read last time. */
    off?: boolean
    onopen: () => void
  } = $props()

  // Spread the picks over the folder rather than its first four.
  const pics = $derived.by(() => {
    if (cover) return [cover]
    const have = works.filter(w => thumbs[w.id])
    const k = Math.min(4, have.length)
    return Array.from({ length: k }, (_, i) => thumbs[have[Math.floor((i * have.length) / k)].id])
  })
  const next = $derived(works.filter(w => marks.marks[w.id]?.m === 'next').length)
  const done = $derived(works.filter(w => marks.marks[w.id]?.m === 'done').length)
</script>

<button class="card" class:compact onclick={onopen} title={path} style="--c:{color}" class:link={!!link} class:off
  aria-label={link ? S.linkTile(name, S.works(works.length, ev)) : S.folderTile(name, works.length, subs, ev, sub)}>
  <div class="tile mosaic n{pics.length}" class:pic={pics.length > 0}>
    {#each pics as p (p.key)}<img src={thumbSrc(p.key)} alt="" loading="lazy" decoding="async" />{/each}
    {#if !pics.length}<span class="t">{name}</span>{/if}
    <span class="badge">{#if link}<Icon name={link} size={14} /><Icon name="shortcut" size={11} />{:else}<Icon name="folder" size={14} />{/if}</span>
    <span class="meta">{S.works(works.length, ev)}</span>
    {#if next || done}
      <span class="flags">
        {#if next}<span class="next">{S.nNext(next)}</span>{/if}
        {#if done}<span class="done">✓ {S.nDone(done)}</span>{/if}
      </span>
    {/if}
  </div>
  <span class="name"><b><Icon name={link ?? 'folder'} /><span>{name}</span></b>
  {#if compact}<span class="sub">{S.works(works.length, ev)}{subs ? `, ${S.subfolders(subs, sub)}` : ''}</span>{:else if subs}<span class="sub">{S.subfolders(subs, sub)}</span>{/if}</span>
</button>

<style>
  /* Beside cards, which take three rows of the grid each (Card.svelte), a tile takes three too. */
  .card:not(.compact) { grid-row: span 3; align-self: start; }
  .card { border: 0; background: none; padding: 0; text-align: left; min-width: 0; display: block; width: 100%; }
  .tile { position: relative; aspect-ratio: var(--ratio, 16/9); border-radius: 8px; overflow: hidden; color: #fff;
    background: linear-gradient(155deg, color-mix(in srgb, var(--c) 88%, #fff) 0%, var(--c) 55%, color-mix(in srgb, var(--c) 70%) 100%); }
  .tile.pic { background: var(--well); display: grid; gap: 2px; grid-template-rows: 1fr; }
  .tile.n2, .tile.n3, .tile.n4 { grid-template-columns: 1fr 1fr; }
  .tile.n3, .tile.n4 { grid-template-rows: 1fr 1fr; }
  .tile.n3 img:first-child { grid-row: span 2; }
  img { width: 100%; height: 100%; min-height: 0; object-fit: cover; display: block; }
  .t { position: absolute; inset: 0; display: flex; align-items: flex-end; padding: 12px 14px 32px; font-weight: 700; font-size: 15px; line-height: 1.2;
    background: linear-gradient(to top, rgba(0, 0, 0, .5), rgba(0, 0, 0, 0) 75%); text-shadow: 0 1px 2px rgba(0, 0, 0, .35); overflow-wrap: anywhere; }
  /* A folder, not a Work: a folder badge on the tile and a sheet showing behind it. */
  .badge { position: absolute; top: 8px; right: 8px; display: grid; place-items: center; width: 24px; height: 22px; border-radius: 5px;
    background: rgba(22, 32, 42, .75); color: #fff; }
  .card:not(.compact) .tile { box-shadow: 4px -4px 0 -1px var(--pane), 4px -4px 0 0 color-mix(in srgb, var(--c) 55%, var(--line)); margin: 4px 4px 0 0; }
  .meta { position: absolute; right: 8px; bottom: 8px; font-size: 11.5px; font-weight: 600; padding: 1px 7px; border-radius: 5px; background: rgba(22, 32, 42, .75); }
  .flags { position: absolute; top: 8px; left: 8px; display: flex; flex-wrap: wrap; gap: 4px; }
  .flags span { border: 1px solid rgba(255, 255, 255, .55); border-radius: 999px; padding: 1px 8px; font-size: 11.5px; font-weight: 700; box-shadow: 0 1px 3px rgba(0, 0, 0, .3); }
  .flags .next { background: var(--next); color: var(--next-ink); }
  .flags .done { background: var(--done); color: var(--done-ink); }
  .card:hover .tile, .card:focus-visible .tile { outline: 2px solid var(--focus); outline-offset: 2px; }
  .card:focus-visible { outline: none; }
  /* The icon sits in the first line's indent, so a long word with no spaces starts beside it, not under it.
     The indent is on a block inside the clamped box: WebKit (Linux, macOS) ignores it on the box itself. */
  b :global(.icon) { color: var(--muted); position: absolute; left: 0; top: .15em; }
  b span { display: block; text-indent: 20px; }
  b { position: relative; display: -webkit-box; -webkit-box-orient: vertical; -webkit-line-clamp: 2; line-clamp: 2; overflow: hidden; margin-top: 8px; font-weight: 600; font-size: 14px; line-height: 1.3; overflow-wrap: anywhere; }
  .sub { color: var(--muted); font-size: 12.5px; }
  .compact { display: grid; grid-template-columns: 96px minmax(0, 1fr); gap: 14px; align-items: center; padding: 7px 6px; border-bottom: 1px solid var(--line); border-radius: 0; }
  .compact:hover { background: var(--hover); }
  .compact .tile .t, .compact .tile .meta, .compact .flags, .compact .badge { display: none; }
  .compact b { margin-top: 0; }
  /* A Split-out Section: a Section, not a folder, so no sheet behind it; its badge carries a shortcut arrow. */
  .card.link:not(.compact) .tile { box-shadow: none; margin: 4px 0 0; }
  .link .badge { display: flex; align-items: center; width: auto; gap: 1px; padding: 0 5px; }
  .link b :global(.icon) { color: var(--c); }
  /* Greyed as a button that is off (.btn:disabled), its pictures and colour grey; what is inside
     the tile, so the focus ring on it keeps its strength. */
  .card.off .tile { --c: var(--soft); }
  .card.off .tile > *, .card.off .name { opacity: .45; }
  .card.off img { filter: grayscale(1); }
</style>
