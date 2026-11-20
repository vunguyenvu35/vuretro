// Updates of the app itself. `status` is one of: idle, checking, current (this is the latest),
// available, installing, error. The check is also made once at start (unless switched off),
// quietly: it only ever shows a pill in the sidebar, never a dialog.
import { api } from './api.js';

const KEY = 'vuretro.checkUpdates';
const read = () => {
  try {
    return localStorage.getItem(KEY) !== 'off';
  } catch {
    return true;
  }
};

export const update = $state({ status: 'idle', version: '', notes: '', error: '', percent: null, auto: read() });

export function setAutoCheck(on) {
  update.auto = on;
  try {
    localStorage.setItem(KEY, on ? 'on' : 'off');
  } catch {
    /* a private window: it applies until the app is closed */
  }
}

/** Ask whether a newer version is out. `quiet` keeps a failure (offline) to oneself. */
export async function checkForUpdate({ quiet = false } = {}) {
  if (update.status === 'checking' || update.status === 'installing') return;
  update.status = 'checking';
  update.error = '';
  try {
    const found = await api.checkUpdate();
    if (found) {
      Object.assign(update, { status: 'available', version: found.version, notes: found.notes ?? '' });
    } else {
      update.status = 'current';
    }
  } catch (e) {
    update.status = quiet ? 'idle' : 'error';
    update.error = String(e);
  }
}

export async function installUpdate() {
  if (update.status !== 'available') return;
  update.status = 'installing';
  update.percent = null;
  const off = await api.onUpdateProgress(({ done, total }) => {
    update.percent = total ? Math.min(100, Math.round((done / total) * 100)) : null;
  });
  try {
    await api.installUpdate(); // the app restarts into the new version when this succeeds
  } catch (e) {
    update.status = 'error';
    update.error = String(e);
  } finally {
    off();
  }
}

/** At start: look once, a few seconds in so the library gets the first go. */
export function checkAtStart() {
  if (update.auto) setTimeout(() => checkForUpdate({ quiet: true }), 4000);
}
