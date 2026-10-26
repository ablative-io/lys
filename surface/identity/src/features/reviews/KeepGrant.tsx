/** Approving access that is due a review records the reviewer's exact decision that it stays; it never renews or widens the grant. */
import { useRef, useState } from 'react';
import { operationId } from '../../api';
import { ChangeStatus } from '../roles/ChangeStatus';
import { useRoleChange } from '../roles/useRoleChange';
import { Act } from '../../shell/Act';
import { APPROVE } from '../requests/decision-words';
export interface Kept { grant: string; kept_by: string; note: string; operation: string; at: number; revision: number }
/** `reviews_state.rs` `KeptBatch`: one recorded act, with a decision for every id shown. */
export interface KeptBatch { agent: string; operation: string; kept: Kept[] }

export function KeepAll({ agent, grants, person, changed }: { agent: string; grants: string[]; person: string; changed: () => void }) {
  const change = useRoleChange<KeptBatch>('lys.pending.keep-all.' + person + '.' + agent, '/reviews/agents/' + encodeURIComponent(agent) + '/keep',
    (answer, body) => {
      const sent = body.grants;
      return answer.agent === agent && answer.operation === body.operation && Array.isArray(sent)
        && Array.isArray(answer.kept) && answer.kept.length === sent.length
        && answer.kept.every((kept, index) => kept.grant === sent[index] && kept.kept_by === person
          && kept.operation === body.operation && kept.note === '' && Number.isSafeInteger(kept.at) && kept.at >= 0);
    }, changed);
  return <><Act symbol="approve" name="Keep all" word="Keep all" tone="primary" disabled={change.blocked}
    onClick={() => change.submit({ operation: operationId(), grants })} /><ChangeStatus change={change} /></>;
}
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
  return <>{open ? null : <Act symbol="approve" name="Approve: this access stays" word={APPROVE} disabled={change.blocked} onClick={() => setOpen(true)} />}
    {open ? <form aria-label="Approve this access" onSubmit={(event) => { event.preventDefault(); change.submit({ operation: operationId(), note: note.trim() }); }}>
      <p>Approve that this access is still needed. What it allows and when it ends stay the same.</p>
      <label className="field">Review note (optional)<textarea value={note} disabled={change.blocked} onChange={(event) => setNote(event.target.value)} /></label>
      <Act symbol="approve" name="Confirm approval" word="Confirm" tone="primary" type="submit" disabled={change.blocked} /><Act symbol="close" name="Cancel" word="Cancel" disabled={change.busy} onClick={() => setOpen(false)} />
    </form> : null}<ChangeStatus change={change} /></>;
}
