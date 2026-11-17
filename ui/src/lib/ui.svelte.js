// Presentation infrastructure shared by the components: the confirmation box, the
// right-click menu and toast notifications. No app rules live here.

/**
 * Our own confirmation box. The browser's confirm() cannot be used here: inside the app it
 * answers at once instead of waiting (the dialog plugin makes it asynchronous), so a
 * "Remove?" question would run its action before anyone had a chance to say no.
 */
export const dlg = $state({ open: false, title: '', message: '', note: '', confirmLabel: 'OK', cancelLabel: 'Cancel', danger: false });
let dlgResolve = null;

/** Ask a yes/no question; resolves to true only when the user confirms. */
export function confirmDialog(opts) {
  dlgResolve?.(false);
  Object.assign(dlg, { note: '', confirmLabel: 'OK', cancelLabel: 'Cancel', danger: false, ...opts, open: true });
  return new Promise((resolve) => (dlgResolve = resolve));
}
export function answerDialog(yes) {
  dlg.open = false;
  dlgResolve?.(yes);
  dlgResolve = null;
}

/** The right-click menu: where it is, what it says, what each entry does. */
export const ctx = $state({ open: false, x: 0, y: 0, title: '', items: [] });

/** Open the menu at the pointer. `items` are { label, hint?, danger?, disabled?, action } or { separator: true }. */
export function openMenu(e, items, title = '') {
  e.preventDefault();
  e.stopPropagation();
  Object.assign(ctx, { open: true, x: e.clientX, y: e.clientY, title, items });
}
export function closeMenu() {
  ctx.open = false;
}

/** Notifications in the bottom-right corner; the newest is at the bottom. */
export const toasts = $state([]);
let toastId = 0;

export function toast(text, kind = 'info') {
  toasts.push({ id: ++toastId, text, kind, ms: kind === 'error' ? 8000 : 4200 });
  if (toasts.length > 4) toasts.splice(0, toasts.length - 4);
}
export function dismissToast(id) {
  const i = toasts.findIndex((t) => t.id === id);
  if (i >= 0) toasts.splice(i, 1);
}
