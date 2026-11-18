// Rules for the library screen that need no DOM and no Tauri: which games to show, in what
// order, and how results of bulk operations read. Kept pure so `npm test` can cover them.

const SORTS = {
  recent: (a, b) => b.last_played - a.last_played || a.title.localeCompare(b.title),
  az: (a, b) => a.title.localeCompare(b.title),
  size: (a, b) => b.size - a.size,
};

/**
 * Games to show for a view. `view` is { system, filter, query, sort } as kept in the store;
 * games of a system the user switched off are hidden.
 */
export function visibleGames(overview, { system = null, filter = 'all', collection = null, query = '', sort = 'recent' } = {}) {
  if (!overview) return [];
  let list = overview.games.filter((g) => overview.systems.find((s) => s.id === g.system)?.enabled !== false);
  if (system) list = list.filter((g) => g.system === system);
  if (collection) list = list.filter((g) => g.tags?.includes(collection));
  if (filter === 'recent') list = list.filter((g) => g.last_played);
  if (filter === 'favorites') list = list.filter((g) => g.favorite);
  const q = query.trim().toLowerCase();
  if (q) list = list.filter((g) => g.title.toLowerCase().includes(q));
  return [...list].sort(SORTS[sort] ?? SORTS.recent);
}

/** The game played most recently, if any. */
export function lastPlayed(games) {
  return [...games].filter((g) => g.last_played).sort((a, b) => b.last_played - a.last_played)[0];
}

export function headingFor(overview, { system, filter, collection }) {
  if (collection) return `Collection: ${collection}`;
  if (system) return overview?.systems.find((s) => s.id === system)?.name ?? system;
  return { all: 'All games', recent: 'Recently played', favorites: 'Favorites' }[filter];
}

/** Paths of the imported games between two paths of `visible` (shift-click range). */
export function pickRange(visible, fromPath, toPath) {
  const a = visible.findIndex((x) => x.path === fromPath);
  const b = visible.findIndex((x) => x.path === toPath);
  if (a < 0 || b < 0) return [];
  return visible
    .slice(Math.min(a, b), Math.max(a, b) + 1)
    .filter((x) => x.imported)
    .map((x) => x.path);
}

export function formatSize(n) {
  return n >= 1e9 ? `${(n / 1e9).toFixed(1)} GB` : `${Math.max(1, Math.round(n / 1e6))} MB`;
}

const fileName = (r) => r.source.split(/[\\/]/).pop();
const plural = (n, word) => `${n} ${word}${n === 1 ? '' : 's'}`;

/** Toast text for the outcomes of a ROM import. */
export function summarizeImport(results) {
  const ok = results.filter((r) => r.ok);
  const bad = results.filter((r) => !r.ok);
  const added = ok.filter((r) => r.message.startsWith('added')).length;
  const lines = [];
  if (added) lines.push(`✓ ${plural(added, 'game')} added`);
  const same = ok.length - added;
  if (same) lines.push(`${same} already in the library`);
  lines.push(...bad.slice(0, 4).map((r) => `✕ ${fileName(r)}: ${r.message}`));
  if (bad.length > 4) lines.push(`… and ${bad.length - 4} more`);
  return { text: lines.join('\n') || 'Nothing to import', failed: bad.length };
}

/** Toast text for the outcomes of removing games. */
export function summarizeRemoval(results) {
  const ok = results.filter((r) => r.ok).length;
  const bad = results.filter((r) => !r.ok);
  const lines = [`Removed ${plural(ok, 'game')} from the library`];
  lines.push(...bad.slice(0, 3).map((r) => `✕ ${fileName(r)}: ${r.message}`));
  return { text: lines.join('\n'), failed: bad.length };
}

/** Toast text for the outcomes of a BIOS import. */
export function summarizeBios(results) {
  const bad = results.filter((r) => !r.ok);
  const lines = [...results.filter((r) => r.ok).map((r) => `✓ ${r.message}`), ...bad.map((r) => `✕ ${fileName(r)}: ${r.message}`)];
  return { text: lines.join('\n'), failed: bad.length };
}
