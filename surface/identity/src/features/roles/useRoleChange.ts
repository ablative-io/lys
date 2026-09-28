/** A role change retains its path and exact JSON until a matching answer establishes its outcome. */
import { useRef, useState } from 'react';
import { Refused, request } from '../../api';

interface Saved { path: string; body: Record<string, unknown> }
function read(key: string, allowed: string): Saved | null {
  const raw = sessionStorage.getItem(key);
  if (raw === null) return null;
  const value: unknown = JSON.parse(raw);
  if (!value || typeof value !== 'object' || !('path' in value) || value.path !== allowed
    || !('body' in value) || typeof value.body !== 'object' || value.body === null || Array.isArray(value.body)) {
    throw new Error('The retained role change could not be read. Its outcome must be established before another change.');
  }
  return { path: allowed, body: value.body as Record<string, unknown> };
}
export function useRoleChange<T>(key: string, path: string, accepts: (answer: T, body: Record<string, unknown>) => boolean, changed: () => void) {
  const [initial] = useState(() => { try { return { saved: read(key, path), error: '' }; } catch (error) { return { saved: null, error: String(error) }; } });
  const [saved, setSaved] = useState(initial.saved);
  const [failure, setFailure] = useState(initial.error);
  const [busy, setBusy] = useState(false);
  const [done, setDone] = useState(false);
  const working = useRef(false);
  const send = async (body: Record<string, unknown>, retry: boolean) => {
    if (working.current || initial.error) return;
    working.current = true; setBusy(true); setFailure('');
    try {
      sessionStorage.setItem(key, JSON.stringify({ path, body })); setSaved({ path, body });
      const answer = await request<T>(path, body);
      if (!accepts(answer, body)) throw new Error('The answer did not confirm this role change. Its original request is retained.');
      sessionStorage.removeItem(key); setSaved(null); setDone(true); changed();
    } catch (error) {
      if (!retry && error instanceof Refused && error.status >= 400 && error.status < 500) { sessionStorage.removeItem(key); setSaved(null); }
      setFailure(error instanceof Refused ? error.refusal.refusal + ': ' + error.message : String(error));
    } finally { working.current = false; setBusy(false); }
  };
  return { busy, done, failure, pending: saved !== null, blocked: Boolean(initial.error) || saved !== null || busy || done,
    submit: (body: Record<string, unknown>) => { if (!saved && !done) void send(body, false); },
    retry: () => { if (saved) void send(saved.body, true); },
  };
}
export type RoleChange = ReturnType<typeof useRoleChange<unknown>>;
