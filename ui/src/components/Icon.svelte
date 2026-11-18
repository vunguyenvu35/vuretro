<script>
  /**
   * The app's icon set: 20x20, 1.6 px round strokes, drawn to match each other.
   * Colour comes from `currentColor`, so an icon takes the colour of the text around it.
   */
  let { name, size = 18 } = $props();
</script>

<svg viewBox="0 0 20 20" width={size} height={size} fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false">
  {#if name === 'all'}
    <rect x="3" y="3" width="6" height="6" rx="1.7" /><rect x="11" y="3" width="6" height="6" rx="1.7" />
    <rect x="3" y="11" width="6" height="6" rx="1.7" /><rect x="11" y="11" width="6" height="6" rx="1.7" />
  {:else if name === 'recent'}
    <circle cx="10" cy="10" r="7" /><path d="M10 5.8V10l2.8 1.8" />
  {:else if name === 'favorites'}
    <path d="M10 2.9l2.1 4.4 4.8.6-3.5 3.3.9 4.8L10 13.7l-4.3 2.3.9-4.8L3.1 7.9l4.8-.6L10 2.9z" />
  {:else if name === 'neogeo'}
    <!-- arcade stick -->
    <circle cx="10" cy="5" r="2.6" /><path d="M10 7.6V11" /><rect x="3" y="11" width="14" height="6.2" rx="2.2" />
    <circle cx="7.2" cy="14.1" r=".9" fill="currentColor" stroke="none" /><circle cx="12.8" cy="14.1" r=".9" fill="currentColor" stroke="none" />
  {:else if name === 'psx'}
    <!-- triangle, circle, cross, square -->
    <path d="M10 2.6l2.3 4H7.7l2.3-4z" /><circle cx="15.3" cy="10.2" r="2.2" />
    <path d="M7.9 13.3l4.2 4.2M12.1 13.3l-4.2 4.2" /><rect x="2.8" y="8" width="4.4" height="4.4" rx=".9" />
  {:else if ['snes', 'nes', 'genesis', 'sms', 'pce', 'atari2600', 'atari7800', 'sega32x', 'arcade', 'saturn'].includes(name)}
    <rect x="1.6" y="5.4" width="16.8" height="9.2" rx="4.6" /><path d="M5.7 7.6v4.8M3.3 10h4.8" />
    <circle cx="13.2" cy="11.4" r=".95" fill="currentColor" stroke="none" /><circle cx="15.7" cy="8.8" r=".95" fill="currentColor" stroke="none" />
  {:else if ['gba', 'gb', 'gbc', 'gg', 'lynx', 'ngp', 'wonderswan', 'virtualboy', 'nds'].includes(name)}
    <rect x="1.8" y="4.2" width="16.4" height="11.6" rx="2.8" /><rect x="7" y="6.6" width="6" height="5.2" rx=".9" />
    <path d="M4.4 8.8v2.8M3 10.2h2.8" /><circle cx="15.2" cy="9.1" r=".8" fill="currentColor" stroke="none" /><circle cx="16.4" cy="11.4" r=".8" fill="currentColor" stroke="none" />
  {:else if name === 'cores'}
    <rect x="5.2" y="5.2" width="9.6" height="9.6" rx="1.9" /><rect x="8.3" y="8.3" width="3.4" height="3.4" rx=".6" />
    <path d="M8 2.8v2.4M12 2.8v2.4M8 14.8v2.4M12 14.8v2.4M2.8 8h2.4M2.8 12h2.4M14.8 8h2.4M14.8 12h2.4" />
  {:else if name === 'controls'}
    <path d="M5.8 6h8.4c2 0 3.2 1.6 3.6 4l.6 3.3c.2 1.5-.8 2.7-2.3 2.5-.9-.1-1.6-.7-2.1-1.5l-.8-1.2H7.1l-.8 1.2c-.5.8-1.2 1.4-2.1 1.5-1.5.2-2.5-1-2.3-2.5l.6-3.3C2.6 7.6 3.8 6 5.8 6z" />
    <path d="M6.3 8.4v3M4.8 9.9h3" /><circle cx="13.3" cy="9.2" r=".8" fill="currentColor" stroke="none" /><circle cx="15.1" cy="10.9" r=".8" fill="currentColor" stroke="none" />
  {:else if name === 'macros'}
    <path d="M11.3 2.6L4.7 11h4.5l-.9 6.4L15.3 9h-4.6l.6-6.4z" />
  {:else if name === 'settings'}
    <circle cx="7" cy="5.5" r="1.8" /><path d="M3 5.5h2.2M8.8 5.5H17" />
    <circle cx="13" cy="10" r="1.8" /><path d="M3 10h8.2M14.8 10H17" />
    <circle cx="8.5" cy="14.5" r="1.8" /><path d="M3 14.5h3.7M10.3 14.5H17" />
  {:else}
    <circle cx="10" cy="10" r="3" fill="currentColor" stroke="none" />
  {/if}
</svg>
