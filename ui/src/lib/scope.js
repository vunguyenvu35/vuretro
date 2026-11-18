// What the Settings and Controls screens are editing: every game, one system or one game.
// The scope is kept as one string: '' (all), a system id, or 'game:' + a game's path.

const GAME = 'game:';

export const gameScope = (path) => GAME + path;

/** `{ kind: 'all' | 'system' | 'game', id }`: `id` is the system id or the game's path. */
export function parseScope(scope) {
  if (!scope) return { kind: 'all', id: '' };
  if (scope.startsWith(GAME)) return { kind: 'game', id: scope.slice(GAME.length) };
  return { kind: 'system', id: scope };
}
