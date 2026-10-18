/**
 * Approve and Decline in a waiting draft's own row, on the drafts page and on the dashboard alike: nothing is decided
 * on another page. Decline asks for its reason where the row is. Each is a kept change: sent once, confirmed by the
 * service's own answer, and never created twice.
 */
import { useState } from 'react';
import { operationId } from '../../api';
import { Act } from '../../shell/Act';
import { ChangeStatus } from '../roles/ChangeStatus';
import { useRoleChange } from '../roles/useRoleChange';
import { APPROVE, DECLINE } from '../requests/decision-words';
import { confirms } from './contract';
import type { Draft, DraftAnswer } from './contract';

const agentOf = (draft: Draft): string => draft.agent?.display_name ?? 'An agent outside your view';

/** Approve and Decline in the draft's own row. Decline asks for the reason where it stands. Each is a kept change. */
export function DraftDecide({ draft, person, changed }: { draft: Draft; person: string; changed: () => void }) {
  const path = '/drafts/' + encodeURIComponent(draft.id);
  const approve = useRoleChange<DraftAnswer>('lys.pending.draft-approve.' + person + '.' + draft.id, path + '/approve', confirms(draft.id), changed);
  const refuse = useRoleChange<DraftAnswer>('lys.pending.draft-refuse.' + person + '.' + draft.id, path + '/refuse', confirms(draft.id), changed);
  const [refusing, setRefusing] = useState(false);
  const [reason, setReason] = useState('');
  const blocked = approve.blocked || refuse.blocked;
  return <div className="draft-decide">
    {refusing && !refuse.pending && !refuse.done
      ? <form aria-label={'Decline the draft of ' + agentOf(draft)} onSubmit={(event) => { event.preventDefault(); if (reason.trim()) refuse.submit({ operation: operationId(), creation_hash: draft.creation_hash, reason: reason.trim() }); }}>
        <label className="field draft-reason-field">Why you decline it<input aria-label="Why you decline it" required value={reason} disabled={blocked} onChange={(event) => setReason(event.target.value)} /><span className="hint">The agent is told this reason.</span></label>
        <Act symbol="decline" name={DECLINE} word={DECLINE} tone="danger" type="submit" data-act="refuse-confirm" disabled={blocked || !reason.trim()} />
        <Act symbol="close" name="Cancel" word="Cancel" data-act="refuse-cancel" disabled={refuse.busy} onClick={() => setRefusing(false)} />
      </form>
      : approve.done || refuse.done || approve.pending || refuse.pending ? null
      : <>
        <Act symbol="approve" name={APPROVE} word={APPROVE} tone="primary" data-act="approve" disabled={blocked} onClick={() => approve.submit({ operation: operationId(), creation_hash: draft.creation_hash, application: operationId() })} />
        <Act symbol="decline" name={DECLINE} word={DECLINE} data-act="refuse" disabled={blocked} onClick={() => setRefusing(true)} />
      </>}
    <ChangeStatus change={approve} />
    <ChangeStatus change={refuse} />
  </div>;
}

