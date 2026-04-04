<!-- Inline SVG icon system — uses currentColor, inherits size from font-size
     or explicit size prop. All icons from 24×24 viewBox.
     Usage: <Icon name="arrow-left" /> or <Icon name="delete" size={18} /> -->
<script lang="ts">
  export let name: string;
  export let size: number | string = '1em';
  export let strokeWidth: number = 2;
</script>

<svg
  viewBox="0 0 24 24"
  width={size}
  height={size}
  fill="none"
  stroke="currentColor"
  stroke-width={strokeWidth}
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
  style="flex-shrink:0;vertical-align:middle"
>
  {#if name === 'arrow-left'}
    <line x1="20" y1="12" x2="4" y2="12" />
    <polyline points="10 6 4 12 10 18" />
  {:else if name === 'arrow-right'}
    <line x1="4" y1="12" x2="20" y2="12" />
    <polyline points="14 6 20 12 14 18" />
  {:else if name === 'authors'}
    <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" />
    <circle cx="12" cy="7" r="4" />
  {:else if name === 'close'}
    <line x1="18" y1="6" x2="6" y2="18" />
    <line x1="6" y1="6" x2="18" y2="18" />
  {:else if name === 'database'}
    <ellipse cx="12" cy="5" rx="9" ry="3" />
    <path d="M3 5v7c0 1.66 4 3 9 3s9-1.34 9-3V5" />
    <path d="M3 12v7c0 1.66 4 3 9 3s9-1.34 9-3v-7" />
  {:else if name === 'delete'}
    <polyline points="3 6 5 6 21 6" />
    <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
    <line x1="10" y1="11" x2="10" y2="17" />
    <line x1="14" y1="11" x2="14" y2="17" />
  {:else if name === 'edit'}
    <path d="M17 3a2.828 2.828 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5L17 3z" />
    <path d="M15 5l4 4" />
  {:else if name === 'edit-mode'}
    <line x1="9" y1="5" x2="15" y2="5" />
    <line x1="9" y1="19" x2="15" y2="19" />
    <line x1="12" y1="5" x2="12" y2="19" />
  {:else if name === 'events'}
    <rect x="3" y="4" width="18" height="18" rx="2" ry="2" />
    <line x1="16" y1="2" x2="16" y2="6" />
    <line x1="8" y1="2" x2="8" y2="6" />
    <line x1="3" y1="10" x2="21" y2="10" />
    <line x1="8" y1="16" x2="16" y2="16" />
  {:else if name === 'import'}
    <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
    <polyline points="7 10 12 15 17 10" />
    <line x1="12" y1="15" x2="12" y2="3" />
  {:else if name === 'logo'}
    <rect x="2" y="2" width="20" height="20" rx="4.5" stroke-width="2" />
    <g stroke-width="1.5">
      <ellipse cx="12" cy="12" rx="9" ry="3.5" />
      <ellipse cx="12" cy="12" rx="3.5" ry="9" />
    </g>
    <circle cx="12" cy="12" r="2" fill="currentColor" stroke="none" />
  {:else if name === 'plus'}
    <line x1="12" y1="5" x2="12" y2="19" />
    <line x1="5" y1="12" x2="19" y2="12" />
  {:else if name === 'read-mode'}
    <path d="M2 12s4-7 10-7 10 7 10 7-4 7-10 7-10-7-10-7z" />
    <circle cx="12" cy="12" r="3" />
  {:else if name === 'settings'}
    <circle cx="12" cy="12" r="3" />
    <path
      d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"
    />
  {:else if name === 'theme'}
    <circle cx="12" cy="12" r="9" />
    <path d="M12 3v18" />
    <line x1="12" y1="7" x2="19" y2="7" />
    <line x1="12" y1="12" x2="21" y2="12" />
    <line x1="12" y1="17" x2="19" y2="17" />
  {:else if name === 'types'}
    <path d="M20.59 13.41l-7.17 7.17a2 2 0 0 1-2.83 0L2 12V2h10l8.59 8.59a2 2 0 0 1 0 2.82z" />
    <circle cx="7" cy="7" r="1.5" />
  {:else if name === 'words'}
    <path d="M12 6.5A6.5 6.5 0 0 0 4 8v11a6.5 6.5 0 0 1 8-1.5" />
    <path d="M12 6.5A6.5 6.5 0 0 1 20 8v11a6.5 6.5 0 0 0-8-1.5" />
    <line x1="12" y1="6.5" x2="12" y2="17.5" />
  {:else if name === 'update'}
    <path d="M21 12a9 9 0 1 1-6.219-8.56" />
    <polyline points="21 3 21 9 15 9" />
  {:else if name === 'bug'}
    <path d="M12 20v-9" />
    <path d="M14 7a4 4 0 0 1 4 4v3a6 6 0 0 1-12 0v-3a4 4 0 0 1 4-4z" />
    <path d="M14.12 3.88 16 2" />
    <path d="M21 21a4 4 0 0 0-3.81-4" />
    <path d="M21 5a4 4 0 0 1-3.55 3.97" />
    <path d="M22 13h-4" />
    <path d="M3 21a4 4 0 0 1 3.81-4" />
    <path d="M3 5a4 4 0 0 0 3.55 3.97" />
    <path d="M6 13H2" />
    <path d="m8 2 1.88 1.88" />
    <path d="M9 7.13V6a3 3 0 1 1 6 0v1.13" />
  {/if}
</svg>
