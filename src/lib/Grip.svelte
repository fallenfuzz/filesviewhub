<script lang="ts">
  // The handle on a column's right edge: drag, or arrow keys, to resize; double-click resets.
  import { saveUi, ui } from './ui.svelte'
  import { S } from './strings'

  let { level, def }: { level: string; def: number } = $props()
  const MIN = 160, MAX = 640
  const width = $derived(ui.colw[level] ?? def)
  let x0 = 0, w0 = 0, dragging = false

  function set(px: number, save: boolean) {
    px = Math.round(Math.min(MAX, Math.max(MIN, px)))
    if (px === def) delete ui.colw[level]
    else ui.colw[level] = px
    if (save) saveUi()
  }
  function down(e: PointerEvent) {
    dragging = true; x0 = e.clientX; w0 = width
    ;(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId)
  }
  function key(e: KeyboardEvent) {
    const d = e.key === 'ArrowLeft' ? -16 : e.key === 'ArrowRight' ? 16 : 0
    if (d) { e.preventDefault(); set(width + d, true) }
  }
</script>

<!-- A focusable separator with a value is a control in ARIA; the linter does not know it. -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
<div class="grip" role="separator" tabindex="0" aria-orientation="vertical" aria-valuemin={MIN} aria-valuemax={MAX}
  aria-valuenow={width} aria-label={S.colWidth} title={S.colWidthTip}
  onpointerdown={down} onpointermove={e => dragging && set(w0 + e.clientX - x0, false)}
  onlostpointercapture={() => { dragging = false; saveUi() }} ondblclick={() => set(def, true)} onkeydown={key}></div>

<style>
  .grip { position: absolute; top: 0; bottom: 0; right: -5px; width: 9px; z-index: 3; cursor: col-resize; touch-action: none; }
  .grip:hover, .grip:focus-visible { background: linear-gradient(90deg, transparent 3px, var(--focus) 3px 6px, transparent 6px); outline: none; }
</style>
