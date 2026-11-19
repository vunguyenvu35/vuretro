<script>
  /** A PlayStation face button drawn as on the pad: ✕ blue, ○ red, □ pink, △ green. */
  let { id, size = 20 } = $props();

  // RetroPad button -> the symbol printed on a PlayStation pad
  const SYMBOLS = {
    b: { name: 'Cross', color: '#6aa9ff', d: 'M5 5 L15 15 M15 5 L5 15' },
    a: { name: 'Circle', color: '#ff6b73', d: 'M10 3.6 A6.4 6.4 0 1 1 9.99 3.6 Z' },
    y: { name: 'Square', color: '#ff8fd0', d: 'M4.5 4.5 H15.5 V15.5 H4.5 Z' },
    x: { name: 'Triangle', color: '#5fd38d', d: 'M10 3.4 L16.6 15.6 H3.4 Z' },
  };
  const s = $derived(SYMBOLS[id]);
</script>

{#if s}
  <svg viewBox="0 0 20 20" width={size} height={size} role="img" aria-label={s.name} class="ps">
    <path d={s.d} fill="none" stroke={s.color} stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" />
  </svg>
{/if}

<style>
  .ps { display: inline-block; vertical-align: -4px; }
</style>
