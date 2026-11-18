// Sidebar width rules, kept free of DOM and Tauri so `npm test` covers them.

export const SIDEBAR_DEFAULT = 224;
export const SIDEBAR_MIN = 180;
export const SIDEBAR_MAX = 440;
/** Never let the sidebar take more than this share of the window. */
const MAX_SHARE = 0.5;

/** A usable width: within the limits, and never more than half of the window. */
export function clampSidebar(width, windowWidth = Infinity) {
  const w = Number.isFinite(width) ? Math.round(width) : SIDEBAR_DEFAULT;
  const max = Math.max(SIDEBAR_MIN, Math.min(SIDEBAR_MAX, Math.floor(windowWidth * MAX_SHARE)));
  return Math.min(max, Math.max(SIDEBAR_MIN, w));
}

/** Read the saved width back (`null`, junk or a missing value give the default). */
export function parseSidebar(saved) {
  const n = Number(saved);
  return saved != null && saved !== '' && Number.isFinite(n) ? clampSidebar(n) : SIDEBAR_DEFAULT;
}
