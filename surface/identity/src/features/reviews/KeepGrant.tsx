/** Keeping access records the reviewer's exact decision; it never renews or widens the grant. */
import { useRef, useState } from 'react';
import { operationId } from '../../api';
import { ChangeStatus } from '../roles/ChangeStatus';
import { useRoleChange } from '../roles/useRoleChange';
import { Act } from '../../shell/Act';
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
  return <><Act symbol="approve" name="Keep access" word="Keep" disabled={change.blocked} onClick={() => setOpen(true)} />
    {open ? <form aria-label="Keep this access" onSubmit={(event) => { event.preventDefault(); change.submit({ operation: operationId(), note: note.trim() }); }}>
      <p>Record that this access is still needed. Its permissions and expiry stay the same.</p>
      <label className="field">Review note (optional)<textarea value={note} disabled={change.blocked} onChange={(event) => setNote(event.target.value)} /></label>
      <Act symbol="approve" name="Confirm keep" word="Confirm" tone="primary" type="submit" disabled={change.blocked} /><Act symbol="close" name="Cancel" word="Cancel" disabled={change.busy} onClick={() => setOpen(false)} />
    </form> : null}<ChangeStatus change={change} /></>;
}
