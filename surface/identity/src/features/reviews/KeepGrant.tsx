/** Keeping access records the reviewer's exact decision; it never renews or widens the grant. */
import { useRef, useState } from 'react';
import { operationId } from '../../api';
import { ChangeResult } from '../signin/words';
import { useRoleChange } from '../roles/useRoleChange';
export interface Kept { grant: string; kept_by: string; note: string; operation: string; at: number; revision: number }
export function KeepGrant({ grant, person, changed }: { grant: string; person: string; changed: (answer: Kept) => void }) {
  const [open, setOpen] = useState(false);
  const [note, setNote] = useState('');
  const confirmed = useRef<Kept | null>(null);
  const change = useRoleChange<Kept>('lys.pending.keep.' + person + '.' + grant, '/reviews/' + encodeURIComponent(grant) + '/keep',
    (answer, body) => {
      const matches = answer.grant === grant && answer.kept_by === person && answer.operation === body.operation && answer.note === body.note && Number.isSafeInteger(answer.at) && answer.at >= 0;
      if (matches) confirmed.current = answer;
      return matches;
    }, () => { if (confirmed.current) changed(confirmed.current); });
  return <><button className="btn" disabled={change.blocked} onClick={() => setOpen(true)}>Keep access</button>
    {open ? <form aria-label="Keep this access" onSubmit={(event) => { event.preventDefault(); change.submit({ operation: operationId(), note: note.trim() }); }}>
      <p>Record that this access is still needed. Its permissions and expiry stay the same.</p>
      <label className="field">Review note (optional)<textarea maxLength={500} value={note} disabled={change.blocked} onChange={(event) => setNote(event.target.value)} /></label>
      <button className="btn primary" type="submit" disabled={change.blocked}>Confirm keep</button><button className="btn" type="button" disabled={change.busy} onClick={() => setOpen(false)}>Cancel</button>
    </form> : null}<ChangeResult change={change} /></>;
}
