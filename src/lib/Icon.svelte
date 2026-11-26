<script lang="ts" module>
  export type IconName = 'folder' | 'open' | 'globe' | 'launch' | 'refresh' | 'image' | 'gear' | 'trash' | 'star' | 'check' | 'done' | 'clear' | 'more' | 'down' | 'filter' | 'view' | 'home' | 'book' | 'play' | 'columns' | 'chev' | 'back' | 'review' | 'expand' | 'collapse' | 'section' | 'shortcut'
</script>

<script lang="ts">
  // Line icons drawn like the title bar's: a folder, an open folder for "show in File Explorer", a
  // globe for a book's cover from the web, the menus' icons (Menu.svelte), and the rail's, the
  // address bar's and the subfolder rows' (Library.svelte).
  let { name, size = 16 }: { name: IconName; size?: number } = $props()
</script>

<svg class="icon" width={size} height={size} viewBox="0 0 16 16" aria-hidden="true">
  {#if name === 'folder'}
    <path d="M1.5 4.5v8a1 1 0 0 0 1 1h11a1 1 0 0 0 1-1V6a1 1 0 0 0-1-1H8L6.5 3.5h-4a1 1 0 0 0-1 1z" />
  {:else if name === 'globe'}
    <circle cx="8" cy="8" r="6.5" /><path d="M1.5 8h13M8 1.5c-2.2 2.4-2.2 10.6 0 13M8 1.5c2.2 2.4 2.2 10.6 0 13" />
  {:else if name === 'section'}
    <rect x="1.5" y="2.5" width="13" height="11" rx="1.5" /><path d="M5.5 2.5v11M3 5.5h1M3 7.5h1" />
  {:else if name === 'shortcut'}
    <path d="M4.5 12.5V9a3 3 0 0 1 3-3h5M10 3.5 12.5 6 10 8.5" />
  {:else if name === 'launch'}
    <path d="M7 2.5H3.5a1 1 0 0 0-1 1v9a1 1 0 0 0 1 1h9a1 1 0 0 0 1-1V9M13.5 2.5l-6 6M9.5 2.5h4v4" />
  {:else if name === 'refresh'}
    <path d="M13.5 8a5.5 5.5 0 1 1-1.6-3.9M13.5 2.5v3h-3" />
  {:else if name === 'image'}
    <rect x="1.5" y="2.5" width="13" height="11" rx="2" /><circle cx="5.5" cy="6.5" r="1.3" /><path d="m2 12 4-4 3 3 2-2 3.5 3.5" />
  {:else if name === 'gear'}
    <circle cx="8" cy="8" r="2.2" /><path d="M8 1.5v2M8 12.5v2M1.5 8h2M12.5 8h2M3.4 3.4l1.4 1.4M11.2 11.2l1.4 1.4M3.4 12.6l1.4-1.4M11.2 4.8l1.4-1.4" />
  {:else if name === 'trash'}
    <path d="M2.5 4h11M6 4V2.5h4V4M4 4l.7 9.5h6.6L12 4" />
  {:else if name === 'star'}
    <path d="M8 1.8l1.9 3.9 4.3.6-3.1 3 .7 4.3L8 11.6l-3.8 2 .7-4.3-3.1-3 4.3-.6z" />
  {:else if name === 'check'}
    <path d="M2.5 8.5l3.5 3.5 7.5-8" />
  {:else if name === 'done'}
    <circle cx="8" cy="8" r="6.5" /><path d="M5 8.2l2 2 4-4.4" />
  {:else if name === 'clear'}
    <path d="M3.5 3.5l9 9M12.5 3.5l-9 9" />
  {:else if name === 'more'}
    <circle cx="3" cy="8" r="1.1" class="f" /><circle cx="8" cy="8" r="1.1" class="f" /><circle cx="13" cy="8" r="1.1" class="f" />
  {:else if name === 'down'}
    <path d="M3.5 6 8 10.5 12.5 6" />
  {:else if name === 'filter'}
    <path d="M2 3h12l-4.5 5.5V13l-3-1.5v-3z" />
  {:else if name === 'view'}
    <rect x="1.5" y="2.5" width="5" height="5" rx="1" /><rect x="9.5" y="2.5" width="5" height="5" rx="1" /><rect x="1.5" y="9.5" width="5" height="4" rx="1" /><rect x="9.5" y="9.5" width="5" height="4" rx="1" />
  {:else if name === 'home'}
    <path d="M2 7.2 8 2.2l6 5V13a.8.8 0 0 1-.8.8H10V10H6v3.8H2.8A.8.8 0 0 1 2 13z" />
  {:else if name === 'book'}
    <path d="M8 3.8C6.6 2.7 4.3 2.4 2 2.7v10c2.3-.3 4.6 0 6 1.1 1.4-1.1 3.7-1.4 6-1.1v-10c-2.3-.3-4.6 0-6 1.1zM8 3.8v10" />
  {:else if name === 'play'}
    <rect x="1.5" y="2.5" width="13" height="11" rx="2" /><path d="M6.5 5.6v4.8L10.6 8z" />
  {:else if name === 'columns'}
    <rect x="1.5" y="2.5" width="13" height="11" rx="2" /><path d="M5.8 2.5v11M10.2 2.5v11" />
  {:else if name === 'expand'}
    <path d="M4.5 6 8 2.5 11.5 6M4.5 10 8 13.5 11.5 10" />
  {:else if name === 'collapse'}
    <path d="M4.5 2.5 8 6 11.5 2.5M4.5 13.5 8 10 11.5 13.5" />
  {:else if name === 'chev'}
    <path d="M6 3.5 10.5 8 6 12.5" />
  {:else if name === 'back'}
    <path d="M10 3.5 5.5 8 10 12.5" />
  {:else if name === 'review'}
    <rect x="1.5" y="4" width="10" height="9.5" rx="1.5" /><path d="M4.5 4V3a1 1 0 0 1 1-1h8a1 1 0 0 1 1 1v7.5a1 1 0 0 1-1 1h-2M4 8.8l1.8 1.8 3.2-3.4" />
  {:else}
    <path d="M1.5 12.5v-8a1 1 0 0 1 1-1h4L8 5h5a1 1 0 0 1 1 1v1" /><path d="M1.5 12.5 3.6 7.5h11l-2.1 5z" />
  {/if}
</svg>

<style>
  .icon { fill: none; stroke: currentColor; stroke-width: 1.3; stroke-linecap: round; stroke-linejoin: round; vertical-align: -2px; flex: none; }
  .icon .f { fill: currentColor; stroke: none; }
</style>
