/** Ending a handle retains its operation until confirmed; local withdrawal and provider confirmation stay separate. */
import { useState } from 'react';
import { operationId } from '../../api';
import { useRoleChange } from '../roles/useRoleChange';
import { ChangeStatus } from '../roles/ChangeStatus';
interface DropAnswer { handle: string; operation: string; outcome: 'ended' | 'repeated' | 'already_ended'; ended: string[]; stopped_here: boolean; upstream: 'not_asked' | 'unconfirmed' | 'confirmed'; upstream_reason: string | null }
export function DropHandle({ handle, person }: { handle: string; person: string }) {
  const [confirm, setConfirm] = useState(false);
  const [receipt, setReceipt] = useState<DropAnswer | null>(null);
  const change = useRoleChange<DropAnswer>('lys.pending.handle-drop.' + person + '.' + handle, '/secrets/drop', (answer, body) => {
    const matches = answer.handle === handle && answer.operation === body.operation && answer.stopped_here === true && ['ended', 'repeated', 'already_ended'].includes(answer.outcome) && ['not_asked', 'unconfirmed', 'confirmed'].includes(answer.upstream) && Array.isArray(answer.ended);
    if (matches) setReceipt(answer);
    return matches;
  }, () => setConfirm(false));
  return <>
    {!change.done && !confirm ? <button className="btn danger" disabled={change.blocked} onClick={() => setConfirm(true)}>End handle</button> : null}
    {confirm ? <section aria-label="Confirm handle withdrawal"><p>End this handle and every handle derived from it? Access through this broker stops when confirmed. The provider’s response is a separate fact.</p><button className="btn danger" disabled={change.blocked} onClick={() => change.submit({ handle, operation: operationId() })}>Confirm end handle</button><button className="btn" disabled={change.busy} onClick={() => setConfirm(false)}>Cancel</button></section> : null}
    <ChangeStatus change={change} />
    {receipt ? <section role="status"><p>Use stopped at this broker.</p><p>Provider withdrawal: {receipt.upstream === 'confirmed' ? 'confirmed by the provider' : receipt.upstream === 'not_asked' ? 'not asked' : 'asked, not yet confirmed'}.</p>{receipt.upstream_reason ? <p>{receipt.upstream_reason}</p> : null}<details><summary>Withdrawal receipt</summary><p>Operation: {receipt.operation}</p><p>Outcome: {receipt.outcome}</p><p>Handles recorded as ended: {receipt.ended.length}</p></details></section> : null}
  </>;
}
