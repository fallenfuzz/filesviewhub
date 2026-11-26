<script lang="ts">
  // The question from `ask`: the answer that changes nothing has focus when the change cannot be undone.
  import { tick } from 'svelte'
  import { asking } from './ask.svelte'
  import { S } from './strings'

  let dialog: HTMLDialogElement
  let yes = $state<HTMLButtonElement>()
  let no = $state<HTMLButtonElement>()
  $effect(() => {
    const q = asking.q
    if (!q) return
    if (!dialog.open) dialog.showModal()
    tick().then(() => (q.danger ? no : yes)?.focus())
  })
  function answer(v: boolean) {
    const q = asking.q
    asking.q = null
    if (dialog.open) dialog.close()
    q?.done(v)
  }
</script>

<dialog bind:this={dialog} role="alertdialog" aria-labelledby="ask-text" oncancel={e => { e.preventDefault(); answer(false) }} onclose={() => asking.q && answer(false)}>
  {#if asking.q}
    <p id="ask-text" dir="auto">{asking.q.message}</p>
    <div class="bar">
      <button class="btn {asking.q.danger ? 'red' : 'primary'}" bind:this={yes} onclick={() => answer(true)}>{asking.q.ok}</button>
      <button class="btn" bind:this={no} onclick={() => answer(false)}>{S.cancel}</button>
    </div>
  {/if}
</dialog>

<style>
  dialog { border: 0; border-radius: 12px; padding: 22px 24px 18px; width: min(460px, 92vw); background: var(--bg); color: var(--ink); box-shadow: 0 20px 60px rgba(0,0,0,.35); }
  dialog[open] { animation: rise .14s ease-out; }
  dialog::backdrop { background: rgba(10, 14, 20, .45); }
  @keyframes rise { from { opacity: 0; transform: translateY(6px) scale(.98) } }
  p { margin: 0 0 18px; line-height: 1.5; font-size: 14.5px; white-space: pre-line; overflow-wrap: anywhere; }
  .bar { display: flex; justify-content: flex-end; gap: 8px; }
  .btn { min-width: 90px; padding: 6px 14px; }
  .red { background: var(--danger); color: var(--danger-ink); border-color: var(--danger); font-weight: 600; }
  .red:hover { background: color-mix(in srgb, var(--danger) 88%, black); }
</style>
