import { Refused, request } from './api';

type Listener = { changed: () => void; refused: (error: unknown) => void };
const listeners = new Set<Listener>();
let controller: AbortController | null = null;
let ready = false;
let retry: ReturnType<typeof setTimeout> | null = null;
let failures = 0;

function notify(): void { for (const listener of listeners) listener.changed(); }
function stop(): void {
  controller?.abort(); controller = null; ready = false;
  if (retry !== null) clearTimeout(retry);
  retry = null; failures = 0;
}

function start(): void {
  if (controller || !listeners.size || document.visibilityState === 'hidden') return;
  if (retry !== null) clearTimeout(retry);
  retry = null;
  const current = new AbortController();
  controller = current;
  void (async () => {
    let generation: string | undefined;
    let reconnect = false;
    try {
      while (!current.signal.aborted) {
        const answer = await request<{ generation: string }>('/changes' + (generation ? '?after=' + encodeURIComponent(generation) : ''), undefined, 'POST', current.signal);
        if (current.signal.aborted) return;
        if (!/^op-[0-9a-f]{32}$/.test(answer.generation) || answer.generation === generation) {
          throw new Refused(0, { refusal: 'ChangeSignalUnreadable', reason: 'The service did not provide a new change signal. Refresh to reconnect.' });
        }
        generation = answer.generation;
        ready = true;
        failures = 0;
        notify();
      }
    } catch (error) {
      if (!current.signal.aborted) {
        ready = false;
        for (const listener of listeners) listener.refused(error);
        reconnect = !(error instanceof Refused) || error.status === 0 || error.status >= 500;
      }
    } finally {
      if (controller === current) controller = null;
      if (reconnect && !current.signal.aborted && listeners.size && document.visibilityState !== 'hidden') {
        // A transport failure has no service change signal to wake us; backoff bounds reconnect traffic.
        const delay = Math.min(250 * 2 ** Math.min(failures++, 5), 5000);
        retry = setTimeout(() => { retry = null; start(); }, delay);
      }
    }
  })();
}

/** Explicit action, focus and restored connectivity also retry a refused feed. */
export function refreshLive(): void {
  if (document.visibilityState === 'hidden') return;
  if (ready) notify();
  start();
}
function visibility(): void { if (document.visibilityState === 'hidden') stop(); else refreshLive(); }

/** All mounted views share one service wait; only a failed transport schedules a reconnect. */
export function subscribeChanges(changed: () => void, refused: (error: unknown) => void): () => void {
  const listener = { changed, refused };
  listeners.add(listener);
  if (listeners.size === 1) {
    window.addEventListener('focus', refreshLive);
    window.addEventListener('online', refreshLive);
    document.addEventListener('visibilitychange', visibility);
  }
  if (ready) changed();
  start();
  return () => {
    listeners.delete(listener);
    if (!listeners.size) {
      stop();
      window.removeEventListener('focus', refreshLive);
      window.removeEventListener('online', refreshLive);
      document.removeEventListener('visibilitychange', visibility);
    }
  };
}
