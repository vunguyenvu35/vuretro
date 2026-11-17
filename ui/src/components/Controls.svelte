<script>
  import { modKey } from '../lib/systems.js';

  const buttons = [
    ['↑ ↓ ← →', 'D-pad', 'Joystick'],
    ['Z', 'B', 'A'],
    ['X', 'A', 'B'],
    ['A', 'Y', 'C'],
    ['S', 'X', 'D'],
    ['Enter / 1', 'Start', 'Start'],
    ['Right Shift / 5', 'Select', 'Coin'],
    ['Q', 'L', ''],
    ['E', 'R', ''],
    ['W', 'L2', ''],
    ['R', 'R2', ''],
  ];
  const hotkeys = [
    ['Esc', 'Quit game'],
    ['F5', 'Save state'],
    ['F7', 'Load state'],
    ['F6', 'Next save slot (1–4)'],
    ['F2', 'Reset'],
    ['Tab (hold)', 'Fast-forward ×4'],
  ];
</script>

<section class="page">
  <h1>Controls</h1>
  <div class="cols">
    <div class="panel">
      <h3>Keyboard · Player 1</h3>
      <table>
        <thead><tr><th>Key</th><th>RetroPad</th><th>Neo Geo</th></tr></thead>
        <tbody>
          {#each buttons as [k, pad, neo] (k)}
            <tr><td><kbd>{k}</kbd></td><td>{pad}</td><td>{neo || '—'}</td></tr>
          {/each}
        </tbody>
      </table>
    </div>
    <div class="stack">
      <div class="panel">
        <h3>Hotkeys in a game</h3>
        <table>
          <tbody>
            {#each hotkeys as [k, what] (k)}
              <tr><td><kbd>{k}</kbd></td><td>{what}</td></tr>
            {/each}
          </tbody>
        </table>
      </div>
      <div class="panel">
        <h3>Gamepads</h3>
        <p>Xbox, DualSense and Switch Pro pads are detected automatically. The first pad is player 1, the second is player 2. The left stick works as the D-pad.</p>
        <p class="note">Search the library with <kbd>{modKey}</kbd> <kbd>K</kbd>. Rebinding keys in the app arrives with the in-game menu (see docs/PLAN.md, Phase 3).</p>
      </div>
    </div>
  </div>
</section>

<style>
  .page { flex: 1; overflow: auto; padding: 22px 28px 40px; }
  h1 { font-size: 24px; margin-bottom: 18px; }
  .cols { display: grid; grid-template-columns: minmax(300px, 1fr) minmax(300px, 1fr); gap: 16px; align-items: start; }
  .stack { display: grid; gap: 16px; }
  .panel { background: var(--surface); border: 1px solid var(--border); border-radius: 10px; padding: 16px; }
  h3 { font-size: 15px; margin-bottom: 10px; }
  table { width: 100%; border-collapse: collapse; }
  th { text-align: left; color: var(--muted); font-weight: 500; font-size: 11px; text-transform: uppercase; letter-spacing: .06em; }
  td, th { padding: 6px 4px; border-bottom: 1px solid var(--border); }
  tr:last-child td { border-bottom: 0; }
  kbd {
    font: 12px var(--font-mono); background: var(--surface-2); border: 1px solid var(--border);
    border-bottom-width: 2px; border-radius: 5px; padding: 2px 7px;
  }
  p { color: var(--muted); margin: 6px 0; }
  .note { color: var(--faint); }
  @media (max-width: 900px) { .cols { grid-template-columns: 1fr; } }
</style>
