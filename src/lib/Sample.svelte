<script lang="ts">
  // A small made-up picture of one view, for its tile in Section settings: never the
  // person's files, with one long name, one in Cyrillic and one with an emoji (S.sample). Sized in
  // container units, so one drawing serves a small tile and a larger one.
  import type { Kind } from './api'
  import { S } from './strings'

  let { kind, view, color = '#2F6FDB' }: { kind: Kind; view: string; color?: string } = $props()
  const T = S.sample
  const names = $derived(T.names[kind])
  const tall = $derived(kind === 'Books')
  /** A made-up picture: two colours from a number, the same each time. */
  const pic = (n: number, sat = 52) => {
    const h = (n * 67 + 18) % 360
    return `background:linear-gradient(160deg,hsl(${h} ${sat}% 68%),hsl(${(h + 40) % 360} ${sat - 8}% 44%))`
  }
  /** A made-up share of photos, 0 to 1, from a number. */
  const lv = (n: number) => [0.15, 0.8, 0.45, 0, 0.95, 0.3, 0.6, 0.1, 0.7, 0.25, 0.5, 0.85, 0.05, 0.4][n % 14]
</script>

<div class="s" style="--c:{color}" aria-hidden="true"><div class="b">
  {#if view === 'cards'}
    <div class={tall ? 'g5' : 'g4'} class:tall>
      {#each [...names, ...names].slice(0, tall ? 10 : 8) as n, i (i)}<div class="card"><i style={pic(i)}></i><span class="t2">{n}</span></div>{/each}
    </div>
  {:else if view === 'rows'}
    <div class="rows">
      <div class="row fold"><b class="chev">⌄</b><i class="fd"></i><span class="t1 grow">{T.folders[0]}</span><span class="mut">6</span></div>
      <div class="g3 in" class:tall>{#each names.slice(0, 3) as n, i (i)}<div class="card"><i style={pic(i)}></i><span class="t1">{n}</span></div>{/each}</div>
      <div class="row fold"><b class="chev">›</b><i class="fd"></i><span class="t1 grow">{T.folders[1]}</span><span class="mut">4</span></div>
      <div class="row fold"><b class="chev">›</b><i class="fd"></i><span class="t1 grow">{T.folders[2]}</span><span class="mut">2</span></div>
    </div>
  {:else if view === 'tiles'}
    <div class="g4 ft">
      {#each T.folders as f, i (i)}<div class="card"><div class="mos" class:tall>{#each [0, 1, 2, 3] as k (k)}<i style={pic(i * 4 + k)}></i>{/each}<b class="cnt">{T.counts[i]}</b></div><span class="t2"><span class="fdi"></span>{f}</span></div>{/each}
    </div>
    <div class="g4" class:tall>{#each names.slice(3, 6) as n, i (i)}<div class="card"><i style={pic(i + 3)}></i><span class="t1">{n}</span></div>{/each}</div>
  {:else if view === 'flat'}
    <div class="g3" class:tall>
      {#each names as n, i (i)}<div class="card"><i style={pic(i)}></i><span class="t1 path">{T.folders[i % 3]}</span><span class="t1">{n}</span></div>{/each}
    </div>
  {:else if view === 'lessons' || view === 'files'}
    <div class="whead"><i class="sq big" style={pic(1)}></i><div class="grow"><span class="t2 h">{names[0]}</span><span class="mut">{view === 'lessons' ? T.lessons : T.files}</span></div></div>
    <div class="rows">
      {#each [...T.parts, ...T.parts.slice(0, 2)] as p, i (i)}
        <div class="row">{#if view === 'lessons'}<span class="tick" class:on={i < 2}></span><span class="mut num">{String(i + 1).padStart(2, '0')}</span>{:else}<i class="file"></i>{/if}<span class="t1 grow">{view === 'files' ? T.fileName(p) : p}</span><span class="mut">{view === 'lessons' ? T.lengths[i] : T.sizes[i]}</span></div>
      {/each}
    </div>
  {:else if view === 'formats' || view === 'details'}
    <div class="split2">
      <i class="cover" class:wide={view === 'details'} style={pic(3)}></i>
      <div class="grow col">
        <span class="t2 h">{names[0]}</span>
        <span class="mut">{view === 'formats' ? T.author : T.video}</span>
        <div class="chips">{#each view === 'formats' ? T.formats : T.videoChips as c (c)}<span class="chip">{c}</span>{/each}</div>
        <div class="lines">{#each [80, 92, 70, 85] as w (w)}<i style="width:{w}%"></i>{/each}</div>
      </div>
    </div>
  {:else if view === 'heat' || view === 'months'}
    {#each view === 'heat' ? T.years : T.years.slice(0, 1) as y, r (y)}
      <div class="hrow"><b class="yl">{y}</b>{#each T.monthShort as m, k (k)}<span class="hm" style="--lv:{lv(k + r * 5)}"><small>{m}</small></span>{/each}</div>
    {/each}
    {#if view === 'months'}
      {#each T.monthsShown as mo, r (mo)}
        <span class="t1 h sub">{mo}</span>
        <div class="g4">{#each names.slice(r * 2, r * 2 + 4) as n, i (i)}<div class="card"><i style={pic(i + r * 3)}></i><span class="t1">{n}</span></div>{/each}</div>
      {/each}
    {:else}<span class="mut sub">{S.onThisDay}</span><div class="strip">{#each Array(6) as _, i (i)}<i style={pic(i + 7)}></i>{/each}</div>{/if}
  {:else if view === 'calendar'}
    <div class="cal">
      {#each T.dayLetters as d, i (i)}<small class="dow">{d}</small>{/each}
      {#each Array(28) as _, i (i)}<div class="cd">{#if [2, 5, 6, 12, 13, 19, 23].includes(i)}<i style={pic(i)}></i>{/if}<small>{i + 1}</small>{#if i === 5}<b class="bar">{names[0]}</b>{/if}{#if i === 19}<b class="bar b2">{names[1]}</b>{/if}</div>{/each}
    </div>
  {:else if view === 'grid'}
    {#each names.slice(0, 2) as n, r (r)}
      <div class="whead slim"><span class="t1 h grow">{n}</span><span class="mut">{T.times[r]}</span></div>
      <div class="g6">{#each Array(r ? 6 : 12) as _, i (i)}<i style={pic(i + r * 5)}></i>{/each}</div>
    {/each}
  {/if}
</div></div>

<style>
  .s { container-type: inline-size; aspect-ratio: 16 / 10; overflow: hidden; background: var(--pane); border-radius: 6px; display: block; width: 100%; }
  .b { padding: 4cqw; display: flex; flex-direction: column; gap: 2.4cqw; font-size: clamp(6px, 3.3cqw, 13.5px); line-height: 1.25; color: var(--ink); text-align: left; font-weight: 400; }
  i { display: block; border-radius: 1.2cqw; font-style: normal; }
  .grow { flex: 1; min-width: 0; }
  .col { display: flex; flex-direction: column; gap: 1.2cqw; }
  .mut { color: var(--muted); font-size: .88em; white-space: nowrap; flex: none; }
  .t1 { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; min-width: 0; display: block; }
  .t2 { display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; overflow-wrap: anywhere; }
  .h { font-weight: 600; }
  .sub { margin-top: 1cqw; }
  .g3, .g4, .g5, .g6 { display: grid; gap: 2.4cqw; }
  .g3 { grid-template-columns: repeat(3, minmax(0, 1fr)); }
  .g4 { grid-template-columns: repeat(4, minmax(0, 1fr)); }
  .g5 { grid-template-columns: repeat(5, minmax(0, 1fr)); }
  .g6 { grid-template-columns: repeat(6, minmax(0, 1fr)); gap: 1cqw; }
  .g6 i { aspect-ratio: 1; border-radius: .6cqw; }
  .card { display: flex; flex-direction: column; gap: .8cqw; min-width: 0; }
  .card > i { aspect-ratio: 4 / 3; position: relative; }
  .tall .card > i { aspect-ratio: 3 / 4; }
  .rows { display: flex; flex-direction: column; gap: 1.3cqw; }
  .row { display: flex; align-items: center; gap: 2cqw; min-width: 0; }
  .sq { width: 7cqw; aspect-ratio: 1; flex: none; }
  .sq.big { width: 14cqw; }
  .fold { padding: 1.2cqw 1.6cqw; background: var(--field); border: 1px solid var(--line); border-radius: 1.2cqw; }
  .chev { width: 2.4cqw; color: var(--soft); text-align: center; flex: none; }
  .fd { width: 4.6cqw; height: 3.6cqw; background: #E9B949; border-radius: .6cqw; flex: none; }
  .in { padding-left: 4cqw; }
  .path { color: var(--muted); font-size: .8em; }
  .mos { position: relative; display: grid; grid-template-columns: 1fr 1fr; gap: .3cqw; aspect-ratio: 16 / 9; border-radius: 1.2cqw; overflow: hidden; }
  .mos.tall { aspect-ratio: 3 / 4; }
  .mos i { border-radius: 0; }
  .cnt { position: absolute; right: 1cqw; bottom: 1cqw; background: rgba(0,0,0,.6); color: #fff; font-size: .75em; font-weight: 600; border-radius: .8cqw; padding: 0 1cqw; }
  .fdi { display: inline-block; width: 2.4cqw; height: 1.9cqw; border: 1px solid var(--soft); border-radius: .4cqw; margin-right: 1cqw; vertical-align: -.1em; }
  .ft .t2 { font-weight: 600; }
  .whead { display: flex; gap: 2.4cqw; align-items: center; min-width: 0; }
  .whead div.grow { display: flex; flex-direction: column; gap: .6cqw; }
  .whead.slim { gap: 2cqw; }
  .tick { width: 2.8cqw; height: 2.8cqw; border: 1px solid var(--off); border-radius: .6cqw; flex: none; }
  .tick.on { background: var(--done); border-color: var(--done); }
  .num { width: 3.6cqw; }
  .file { width: 2.8cqw; height: 3.6cqw; background: var(--well); border: 1px solid var(--line); border-radius: .4cqw; flex: none; }
  .split2 { display: flex; gap: 3.4cqw; min-height: 0; }
  .cover { width: 30cqw; aspect-ratio: 3 / 4; flex: none; }
  .cover.wide { width: 44cqw; aspect-ratio: 16 / 10; }
  .chips { display: flex; gap: 1.2cqw; flex-wrap: wrap; }
  .chip { border: 1px solid var(--line); border-radius: 99px; padding: 0 1.6cqw; background: var(--field); font-size: .85em; }
  .lines { display: flex; flex-direction: column; gap: 1.3cqw; margin-top: 1cqw; }
  .lines i { height: 1.1cqw; background: var(--line); border-radius: 1cqw; }
  .hrow { display: grid; grid-template-columns: 2.6em repeat(12, minmax(0, 1fr)); gap: .8cqw; align-items: stretch; }
  .yl { font-weight: 700; align-self: center; white-space: nowrap; }
  .hm { aspect-ratio: 1; border-radius: .8cqw; background: color-mix(in srgb, var(--c) calc(var(--lv) * 90% + 6%), var(--field)); position: relative; }
  .hm small { position: absolute; left: .6cqw; top: .3cqw; font-size: .62em; color: var(--ink); opacity: .75; }
  .strip { display: flex; gap: 1cqw; min-width: 0; }
  .strip i { flex: 1; aspect-ratio: 1; min-width: 0; }
  .cal { display: grid; grid-template-columns: repeat(7, minmax(0, 1fr)); gap: .6cqw; }
  .dow { color: var(--muted); text-align: center; font-size: .8em; }
  .cd { aspect-ratio: 5 / 4; background: var(--field); border: 1px solid var(--line); border-radius: .8cqw; position: relative; overflow: visible; }
  .cd > i { position: absolute; inset: 0; border-radius: .7cqw; opacity: .9; }
  .cd small { position: absolute; left: .6cqw; top: .2cqw; font-size: .7em; color: var(--ink); }
  .cd:has(i) small { color: #fff; }
  .bar { position: absolute; left: 0; bottom: .4cqw; width: 190%; z-index: 1; background: var(--c); color: #fff; font-size: .7em; font-weight: 500; border-radius: .6cqw; padding: 0 1cqw;
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .bar.b2 { background: #7D4F96; }
</style>
