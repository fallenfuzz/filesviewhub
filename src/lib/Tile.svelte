<script lang="ts">
  // A Work's picture box, as the Hub draws it: the thumbnail on a plain background with the
  // Section colour on top, or a generated tile tinted by its folder. A video's strip of frames
  // steps along as the pointer crosses it.
  import { glued, thumbSrc, type Thumb } from './api'
  import { S } from './strings'

  let { thumb, color, title, parent = '', meta = '', progress = 0, tall = false, cloud = false, band = false }: {
    thumb?: Thumb; color: string; title: string; parent?: string; meta?: string; progress?: number; tall?: boolean; cloud?: boolean
    /** The Section's colour along the top, where Sections mix. */
    band?: boolean
  } = $props()

  let frame = $state(0)
  const n = $derived(thumb?.n ?? 1)
  // A picture near the box's shape fills it; any other shows whole.
  const fills = $derived(!!thumb && Math.abs(Math.log(thumb.w / thumb.h / (tall ? 3 / 4 : 16 / 9))) < Math.log(1.3))

  /** Tints vary by folder so a shelf of generated tiles is not one flat colour. */
  function tint(s: string) {
    let h = 0
    for (const ch of s) h = (h * 31 + ch.charCodeAt(0)) | 0
    const k = Math.abs(h) % 5
    return `color-mix(in oklab, ${color} ${[100, 86, 74, 92, 80][k]}%)`
  }
  function move(e: PointerEvent) {
    if (n < 2) return
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect()
    frame = Math.min(n - 1, Math.max(0, Math.floor(((e.clientX - r.left) / r.width) * n)))
  }
</script>

<div class="tile" class:band class:pic={!!thumb} class:frames={n > 1} style="--c:{color}" role="presentation" onpointermove={move} onpointerleave={() => (frame = 0)}>
  {#if thumb && n > 1}
    <div class="strip" style="background-image:url('{thumbSrc(thumb.key)}'); --n:{n}; background-position:{(frame / (n - 1)) * 100}% 50%" aria-hidden="true"></div>
    <div class="scrub" aria-hidden="true">{#each Array(n) as _, i (i)}<i class:on={i === frame}></i>{/each}</div>
  {:else if thumb}
    <img src={thumbSrc(thumb.key)} alt="" class:fills loading="lazy" decoding="async" />
  {:else}
    <div class="gen" style="--g:{tint(parent || title)}">
      {#if parent && !title.startsWith(`${parent} - `)}<small dir="auto">{parent}</small>{/if}
      <b dir="auto">{glued(title)}</b>
    </div>
  {/if}
  {#if meta}<span class="meta">{meta}</span>{/if}
  {#if progress}<span class="progress"><i style="width:{progress * 100}%"></i></span>{/if}
  {#if cloud}<span class="cloud" title={S.cloudMark}>☁</span>{/if}
</div>

<style>
  .tile { position: relative; aspect-ratio: var(--ratio, 16/9); border-radius: 8px; overflow: hidden; background: var(--well); }
  .band::after { content: ""; position: absolute; left: 0; right: 0; top: 0; height: 3px; background: var(--c); z-index: 1; }
  img { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: contain; }
  img.fills { object-fit: cover; }
  .strip { position: absolute; inset: 0; background-repeat: no-repeat; background-size: calc(var(--n) * 100%) auto; }
  .scrub { position: absolute; left: 0; right: 0; bottom: 0; z-index: 1; display: flex; gap: 3px; padding: 0 3px 4px; opacity: 0; transition: opacity .15s; }
  .tile:hover .scrub { opacity: 1; }
  .scrub i { flex: 1; height: 6px; border-radius: 3px; background: rgba(255, 255, 255, .6); box-shadow: 0 0 0 1px rgba(0, 0, 0, .45); }
  .scrub i.on { background: #FFB020; }
  .gen { position: absolute; inset: 0; display: flex; flex-direction: column; justify-content: flex-end; gap: 6px; padding: 10% 10% 30px;
    /* A dark wash under the words keeps white text readable on every Section colour. */
    background: linear-gradient(to top, rgba(0, 0, 0, .5), rgba(0, 0, 0, .45) 65%, rgba(0, 0, 0, .1)), var(--g); color: #fff; overflow-wrap: anywhere;
    text-shadow: 0 1px 2px rgba(0, 0, 0, .35); }
  .gen small { font-size: 12px; font-weight: 600; }
  .gen b { font-size: var(--gen, 15px); line-height: 1.2; font-weight: 700; display: -webkit-box; -webkit-box-orient: vertical; -webkit-line-clamp: var(--lines, 3); line-clamp: var(--lines, 3); overflow: hidden; }
  .meta { position: absolute; right: 5px; bottom: 5px; font-size: 12px; background: rgba(22, 32, 42, .85); color: #fff; padding: 2px 6px; border-radius: 4px; }
  .frames .meta { bottom: 16px; }
  .progress { position: absolute; left: 0; right: 0; bottom: 0; height: 5px; background: rgba(22, 32, 42, .35); }
  .progress i { display: block; height: 100%; background: var(--done); }
  .cloud { position: absolute; bottom: 6px; left: 9px; font-size: 14px; color: #fff; text-shadow: 0 1px 3px rgba(0,0,0,.6); }
</style>
