/** Ending a handle retains its operation until confirmed; local withdrawal and provider confirmation stay separate. */
import { useState } from 'react';
import { operationId } from '../../api';
import { useRoleChange } from '../roles/useRoleChange';
import { ChangeStatus } from '../roles/ChangeStatus';
import { Act } from '../../shell/Act';
interface DropAnswer { handle: string; operation: string; outcome: 'ended' | 'repeated' | 'already_ended'; ended: string[]; stopped_here: boolean; upstream: 'not_asked' | 'unconfirmed' | 'confirmed'; upstream_reason: string | null }
export function DropHandle({ handle, person, dropped }: { handle: string; person: string; dropped: boolean }) {
  const [confirm, setConfirm] = useState(false);
  const [receipt, setReceipt] = useState<DropAnswer | null>(null);
  const change = useRoleChange<DropAnswer>('lys.pending.handle-drop.' + person + '.' + handle, '/secrets/drop', (answer, body) => {
    const matches = answer.handle === handle && answer.operation === body.operation && answer.stopped_here === true && ['ended', 'repeated', 'already_ended'].includes(answer.outcome) && ['not_asked', 'unconfirmed', 'confirmed'].includes(answer.upstream) && Array.isArray(answer.ended);
    if (matches) setReceipt(answer);
    return matches;
  }, () => setConfirm(false));
  return <>
    {!dropped && !change.done && !confirm ? <Act symbol="stop" name="End handle" word="End" tone="danger" disabled={change.blocked} onClick={() => setConfirm(true)} /> : null}
    {confirm ? <section aria-label="Confirm handle withdrawal"><p>End this handle, and every handle passed on from it? The agent can no longer use the secret through Lys once this is confirmed. Whether the secret’s own provider also withdrew it is said separately.</p><Act symbol="approve" name="Confirm end handle" word="Confirm" tone="danger" disabled={change.blocked} onClick={() => change.submit({ handle, operation: operationId() })} /><Act symbol="close" name="Cancel" word="Cancel" disabled={change.busy} onClick={() => setConfirm(false)} /></section> : null}
    <ChangeStatus change={change} />
    {receipt ? <section role="status"><p>{receipt.outcome === 'already_ended' ? 'This handle had already ended.' : 'The handle has ended: the agent can no longer use the secret through Lys.'}</p><p>The secret’s provider: {receipt.upstream === 'confirmed' ? 'confirmed it withdrew it too' : receipt.upstream === 'not_asked' ? 'was not asked' : 'was asked, and has not confirmed yet'}.</p>{receipt.upstream_reason ? <p>{receipt.upstream_reason}</p> : null}<p className="note">{receipt.ended.length === 1 ? '1 handle ended.' : receipt.ended.length + ' handles ended.'}</p></section> : null}
  </>;
}
