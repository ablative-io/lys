/** Resolve an existing approval intent without creating another decision or grant operation. */
import { useRef, useState } from 'react';
import { request } from '../../api';
import { failureWords } from '../signin/words';
import type { AccessRequest } from './contract';
import { Act } from '../../shell/Act';

export function HeldApproval({ entry, changed }: { entry: AccessRequest; changed: (answer: AccessRequest) => void }) {
  const [busy, setBusy] = useState(false);
  const [failure, setFailure] = useState('');
  const working = useRef(false);
  const check = async () => {
    if (working.current) return; working.current = true; setBusy(true); setFailure('');
    try {
      const answer = await request<AccessRequest>('/requests/' + encodeURIComponent(entry.id) + '/reconcile', {});
      if (answer.id !== entry.id) throw new Error('The answer names another request. This approval remains unconfirmed.');
      changed(answer);
    } catch (error) { setFailure(failureWords(error, 'Choose Check pending approval to check this same decision; do not make another decision.')); }
    finally { working.current = false; setBusy(false); }
  };
  const who = entry.approvers.find((person) => person.id === entry.held_by)?.display_name ?? 'a person whose name is unavailable';
  return <div role="status"><p>An approval by {who} is being settled. A new decision cannot replace it.</p>
    <Act symbol="again" name={busy ? 'Checking…' : 'Check pending approval'} word={busy ? 'Checking…' : 'Check'} disabled={busy} onClick={() => void check()} />
    {failure ? <p role="alert">{failure}</p> : null}
  </div>;
}
