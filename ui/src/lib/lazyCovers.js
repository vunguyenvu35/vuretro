// A Svelte action for the cards of the library: it tells the store when a card is in view,
// so pictures are read and made only for what the player can see (plus a margin).
import { setOnScreen } from './store.svelte.js';

const paths = new Map(); // card element -> game path
let io;

function observer() {
  io ??= new IntersectionObserver(
    (entries) => {
      for (const e of entries) setOnScreen(paths.get(e.target), e.isIntersecting);
    },
    { rootMargin: '400px 0px' },
  );
  return io;
}

/** Usage: `<button use:lazyCover={game.path}>`. */
export function lazyCover(node, path) {
  paths.set(node, path);
  if (typeof IntersectionObserver === 'undefined') {
    setOnScreen(path, true); // no way to tell: treat every card as in view
    return { destroy: () => setOnScreen(path, false) };
  }
  observer().observe(node);
  return {
    update(next) {
      const old = paths.get(node);
      if (old === next) return;
      setOnScreen(old, false);
      paths.set(node, next);
      // the observer only reports changes; look again at this card
      observer().unobserve(node);
      observer().observe(node);
    },
    destroy() {
      observer().unobserve(node);
      setOnScreen(paths.get(node), false);
      paths.delete(node);
    },
  };
}
