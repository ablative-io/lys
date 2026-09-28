/** Resolve an existing approval intent without creating another decision or grant operation. */
import { useRef, useState } from 'react';
import { Refused, request } from '../../api';
import type { AccessRequest } from './contract';

export function HeldApproval({ entry, changed }: { entry: AccessRequest; changed: () => void }) {
  const [busy, setBusy] = useState(false);
  const [failure, setFailure] = useState('');
  const working = useRef(false);
  const check = async () => {
    if (working.current) return; working.current = true; setBusy(true); setFailure('');
    try {
      const answer = await request<AccessRequest>('/requests/' + encodeURIComponent(entry.id) + '/reconcile', {});
      if (answer.id !== entry.id) throw new Error('The answer names another request. This approval remains unconfirmed.');
      changed();
    } catch (error) { setFailure(error instanceof Refused ? error.refusal.refusal + ': ' + error.message : String(error)); }
    finally { working.current = false; setBusy(false); }
  };
  const who = entry.approvers.find((person) => person.id === entry.held_by)?.display_name ?? entry.held_by;
  return <div role="status"><p>An approval by {who} is being settled. A new decision cannot replace it.</p>
    <button className="btn" disabled={busy} onClick={() => void check()}>{busy ? 'Checking…' : 'Check pending approval'}</button>
    {failure ? <p role="alert">{failure}</p> : null}
  </div>;
}
