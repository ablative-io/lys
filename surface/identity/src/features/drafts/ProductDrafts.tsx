/**
 * Product drafts (ACCESS-001 R4): the acts a product holds for approval because the grant they rest on is held by
 * draft or by two. Each shows its product, what it would do, who holds the grant and who answers for them, who
 * approved and when, and how the product closed it, all as `GET /product-drafts` answers it; nothing is judged here.
 * A waiting one is approved or refused in its own row, each a kept change, never sent twice.
 */
import { useRef, useState } from 'react';
import { operationId, useLoad } from '../../api';
import { clock } from '../file/time';
import { modeWords } from '../grants/mode-words';
import { ChangeStatus } from '../roles/ChangeStatus';
import { useRoleChange } from '../roles/useRoleChange';
import { DirectoryGate as Gate } from '../people/Words';
import { confirmsProduct, readProductDrafts } from './contract';
import type { ProductDraft, ProductDraftAnswer } from './contract';
import { Act } from '../../shell/Act';

const STATES: Record<string, string> = {
  waiting: 'Waiting',
  approved: 'Approved, not yet executed',
  refused: 'Refused',
  executed: 'Executed',
  refused_on_execution: 'Refused on execution',
};

/** Who someone is, by name where the directory names them, else said plainly, never by identifier. */
const nameOf = (names: Map<string, string>, id: string | undefined): string =>
  id === undefined ? 'not answered' : names.get(id) ?? 'someone outside your view';

function Decide({ draft, person, changed }: { draft: ProductDraft; person: string; changed: () => void }) {
  const path = '/product-drafts/' + encodeURIComponent(draft.id);
  const approve = useRoleChange<ProductDraftAnswer>('lys.pending.product-draft-approve.' + person + '.' + draft.id, path + '/approve', confirmsProduct(draft.id), changed);
  const refuse = useRoleChange<ProductDraftAnswer>('lys.pending.product-draft-refuse.' + person + '.' + draft.id, path + '/refuse', confirmsProduct(draft.id), changed);
  const [refusing, setRefusing] = useState(false);
  const [reason, setReason] = useState('');
  const blocked = approve.blocked || refuse.blocked;
  return <div className="draft-decide">
    {refusing && !refuse.pending && !refuse.done
      ? <form aria-label={'Refuse the ' + draft.app + ' draft'} onSubmit={(event) => { event.preventDefault(); if (reason.trim()) refuse.submit({ operation: operationId(), request_digest: draft.request_digest, reason: reason.trim() }); }}>
        <input aria-label="Why you refuse it" required value={reason} disabled={blocked} placeholder="Why you refuse it" onChange={(event) => setReason(event.target.value)} />
        <Act symbol="decline" name="Refuse" word="Refuse" tone="danger" type="submit" data-act="product-refuse-confirm" disabled={blocked || !reason.trim()} />
        <Act symbol="close" name="Cancel" word="Cancel" data-act="product-refuse-cancel" disabled={refuse.busy} onClick={() => setRefusing(false)} />
      </form>
      : approve.done || refuse.done || approve.pending || refuse.pending ? null
      : <>
        <Act symbol="approve" name="Approve" word="Approve" tone="primary" data-act="product-approve" disabled={blocked} onClick={() => approve.submit({ operation: operationId(), request_digest: draft.request_digest })} />
        <Act symbol="decline" name="Refuse" word="Refuse" data-act="product-refuse" disabled={blocked} onClick={() => setRefusing(true)} />
      </>}
    <ChangeStatus change={approve} />
    <ChangeStatus change={refuse} />
  </div>;
}

/** The state as answered, with the product's own words for a refusal on execution. */
function Closed({ draft }: { draft: ProductDraft }) {
  const execution = draft.execution;
  return <span className="sec" data-state={draft.state ?? ''}>{draft.state ? STATES[draft.state] ?? draft.state : 'State not answered'}
    {execution?.state === 'refused_on_execution' ? <span className="draft-reason"><small className="refusal-name">{execution.refusal}</small>: {execution.reason}</span> : null}
  </span>;
}

function Row({ draft, names, person, changed }: { draft: ProductDraft; names: Map<string, string>; person: string; changed: () => void }) {
  const approvals = draft.approvals ?? [];
  return <tr data-product-draft={draft.id}>
    <td>{draft.app}</td>
    <td><span className="mono">{draft.target.action}</span> · {draft.target.kind} {draft.target.id}{draft.mode ? <span className="sec"> · {modeWords(draft.mode)}</span> : null}</td>
    <td>{nameOf(names, draft.holder)}<span className="sec"> · answered for by {nameOf(names, draft.responsible)}</span></td>
    <td className="sec" data-col="approvals">{approvals.length ? approvals.map((approval) => nameOf(names, approval.by) + ', ' + clock(approval.at)).join('; ') : 'No approval yet'}</td>
    <td>{draft.state === 'waiting' ? <Decide draft={draft} person={person} changed={changed} /> : <Closed draft={draft} />}</td>
  </tr>;
}

/** The product drafts the caller may see, oldest first as answered, in their own table under the agents' drafts. */
export function ProductDrafts({ names, person }: { names: Map<string, string>; person: string }) {
  const [version, setVersion] = useState(0);
  const read = useLoad(readProductDrafts, 'product-drafts:' + version);
  // While the table reads itself again after a decision, what it last showed stays, so a decision's answer is not lost.
  const last = useRef<typeof read | null>(null);
  if (read.status === 'ok') last.current = read;
  const load = read.status === 'loading' && last.current ? last.current : read;
  return <section className="pane" aria-label="Held for products">
    <h2>Held for products</h2>
    <p className="note">Acts a product holds because the grant behind them is held by draft or by two approvals. The product does each one only after it is approved, and says here whether it did.</p>
    <Gate load={load} title="product drafts" ok={(drafts) => <table className="product-drafts" aria-label="Product drafts">
      <thead><tr><th>Product</th><th>Wants to</th><th>Holder</th><th>Approved by</th><th>State</th></tr></thead>
      <tbody>
        {drafts.length ? drafts.map((draft) => <Row key={draft.id} draft={draft} names={names} person={person} changed={() => setVersion((v) => v + 1)} />)
          : <tr className="empty"><td colSpan={5} className="dim">No product holds anything for approval.</td></tr>}
      </tbody>
    </table>} />
  </section>;
}
