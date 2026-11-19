// Helpers shared by the screens that deal with macros.

/** The key a game's macros are stored under: its file name without extension, lower case. */
export function gameKey(path) {
  const name = path.split(/[\\/]/).pop();
  const i = name.lastIndexOf('.');
  return (i > 0 ? name.slice(0, i) : name).toLowerCase();
}

/** The profiles of a game and the one in use (its id, or '' when only the shared macros play). */
export function profilesOf(cfg, key) {
  return { profiles: cfg?.profiles?.[key] ?? [], active: cfg?.active?.[key] ?? '' };
}
