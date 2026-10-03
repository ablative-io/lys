/** A recorded change retains its path and exact JSON until a matching answer establishes its outcome. */
import { useRef, useState } from 'react';
import { Refused, request } from '../../api';
import { answeredNo, sendKept } from '../../kept';

interface Saved { path: string; body: Record<string, unknown> }
function read(key: string, allowed: string): Saved | null {
  const raw = sessionStorage.getItem(key);
  if (raw === null) return null;
  const value: unknown = JSON.parse(raw);
  if (!value || typeof value !== 'object' || !('path' in value) || value.path !== allowed
    || !('body' in value) || typeof value.body !== 'object' || value.body === null || Array.isArray(value.body)) {
    throw new Error('The retained recorded change could not be read. Its outcome must be established before another change.');
  }
  return { path: allowed, body: value.body as Record<string, unknown> };
}
export function useRoleChange<T>(key: string, path: string, accepts: (answer: T, body: Record<string, unknown>) => boolean, changed: (answer: T) => void) {
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
      const answer = await sendKept(key, { path, body }, async () => {
        setSaved({ path, body });
        const given = await request<T>(path, body);
        if (!accepts(given, body)) throw new Error('The answer did not confirm this recorded change. Its original request is retained.');
        return given;
      }, !retry);
      setSaved(null); setDone(true); changed(answer);
    } catch (error) {
      if (!retry && answeredNo(error)) setSaved(null);
      setFailure(error instanceof Refused ? error.refusal.refusal + ': ' + error.message : String(error));
    } finally { working.current = false; setBusy(false); }
  };
  return { busy, done, failure, pending: saved !== null, blocked: Boolean(initial.error) || saved !== null || busy || done,
    submit: (body: Record<string, unknown>) => { if (!saved && !done) void send(body, false); },
    retry: () => { if (saved) void send(saved.body, true); },
  };
}
export type RoleChange = ReturnType<typeof useRoleChange<unknown>>;
