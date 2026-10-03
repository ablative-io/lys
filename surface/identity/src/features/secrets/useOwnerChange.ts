/** Retain each owner's exact change and operation id until its own receipt confirms it. */
import { useRef, useState } from 'react';
import { Refused } from '../../api';
import type { ChangeOutcome } from './SecretsDetail';
import { definitelyRefused, pendingAt } from './pendingChange';
import type { PendingStore } from './pendingChange';
import type { Recipients, ScopeKind } from './secretsApi';

export type OwnerChange =
  | { type: 'scope'; secret: string; kind: ScopeKind; name: string }
  | { type: 'recipients'; secret: string; recipients: Recipients };
interface Saved { asked: string; operation: string; change: OwnerChange }
function savedAt(store: PendingStore, key: string): Saved | null {
  const raw = store.getItem(key);
  if (raw === null) return null;
  const value: unknown = JSON.parse(raw);
  if (!value || typeof value !== 'object' || !('asked' in value) || typeof value.asked !== 'string'
    || !('operation' in value) || typeof value.operation !== 'string' || !/^[A-Za-z0-9_-]{16,}$/.test(value.operation)
    || !('change' in value) || !value.change || typeof value.change !== 'object') return null;
  const change = value.change;
  if (!('secret' in change) || typeof change.secret !== 'string' || !change.secret || !('type' in change)) return null;
  if (key !== `lys.pending.secrets.${String(change.type)}.${change.secret}`) return null;
  if (change.type === 'scope' && 'kind' in change && 'name' in change && typeof change.name === 'string'
    && (change.kind === 'personal' || change.kind === 'team' || change.kind === 'organisation')) {
    return { asked: value.asked, operation: value.operation, change: { type: 'scope', secret: change.secret, kind: change.kind, name: change.name } };
  }
  if (change.type === 'recipients' && 'recipients' in change && (change.recipients === 'anyone' || change.recipients === 'people_only')) {
    return { asked: value.asked, operation: value.operation, change: { type: 'recipients', secret: change.secret, recipients: change.recipients } };
  }
  return null;
}

export function useOwnerChange<T>(key: string, store: PendingStore, send: (change: OwnerChange, operation: string) => Promise<T>) {
  const [outcome, setOutcome] = useState<ChangeOutcome<T>>({ at: 'editing' });
  const sending = useRef(false);
  let pending: string | null = null;
  let saved: Saved | null = null;
  try { pending = pendingAt(store, key); saved = savedAt(store, key); }
  catch (error) { pending = `the pending record could not be read: ${String(error)}`; }
  const shown: ChangeOutcome<T> = pending !== null && (outcome.at === 'editing' || outcome.at === 'refused' || outcome.at === 'done')
    ? { at: 'held', asked: pending } : outcome;
  const perform = async (record: Saved, retry: boolean) => {
    if (sending.current) return;
    sending.current = true;
    try { if (!retry) store.setItem(key, JSON.stringify(record)); }
    catch (error) {
      sending.current = false;
      setOutcome({ at: 'refused', refused: new Refused(0, { refusal: 'PendingNotRecorded', reason: `The change was not sent: ${String(error)}` }) });
      return;
    }
    setOutcome({ at: 'sending' });
    try {
      const answer = await send(record.change, record.operation);
      store.removeItem(key);
      setOutcome({ at: 'done', answer });
    } catch (error) {
      const refused = error instanceof Refused ? error : new Refused(0, { refusal: 'Unanswered', reason: String(error) });
      if (!retry && definitelyRefused(refused)) {
        store.removeItem(key); setOutcome({ at: 'refused', refused });
      } else setOutcome({ at: 'unknown', refused });
    } finally { sending.current = false; }
  };
  return { outcome: shown,
    run: (asked: string, change: OwnerChange) => {
      if (pending !== null || (shown.at !== 'editing' && shown.at !== 'refused')) return;
      void perform({ asked, change, operation: crypto.randomUUID() }, false);
    },
    retry: saved && shown.at !== 'sending' ? () => { if (saved) void perform(saved, true); } : null,
    edited: () => { if (outcome.at === 'done' || outcome.at === 'refused') setOutcome({ at: 'editing' }); },
  };
}
