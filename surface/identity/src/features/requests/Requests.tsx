import { HeldApproval } from './HeldApproval';
/** Requests use the service's visibility and approval decisions, never an inferred permission. */
import { useState } from 'react';
import { Link } from 'react-router';
import { api, request, useLoad } from '../../api';
import { clock } from '../file/time';
import { Gate } from '../signin/Gate';
import { ReadFailure } from '../signin/words';
import { AskForm } from './AskForm';
import { DecisionForm } from './DecisionForm';
import type { AccessRequest } from './contract';

export function Requests() {
  const [confirmed, setConfirmed] = useState<Record<string, AccessRequest>>({});
  const changed = (answer: AccessRequest) => setConfirmed((held) => ({ ...held, [answer.id]: answer }));
  const load = useLoad(() => request<{ requests: AccessRequest[] }>('/requests'), 'requests');
  const choices = useLoad(async () => {
    const [me, grants, model] = await Promise.all([api.me(), api.grants(), api.model()]);
    const resources = [...new Map(grants.grants.map((grant) => [JSON.stringify(grant.resource), grant.resource])).values()];
    return { person: me.person.id, resources, model };
  }, 'request-choices');
  return <div className="page">
    <p className="sub">Ask for permission to use something, then follow the decision.</p>
    <Gate load={choices} title="the access you can request" renderError={(error) => <ReadFailure error={error} subject="the access you can request" />} ok={(data) => <AskForm {...data} changed={changed} />} />
    <Gate load={load} title="your access requests" renderError={(error) => <ReadFailure error={error} subject="your access requests" />} ok={(list) => <section aria-label="Visible requests">
      <h2>Requests you can see</h2>
      {!list.requests.length && !Object.keys(confirmed).length ? <p className="note">No requests yet.</p> : [...list.requests.filter((entry) => !confirmed[entry.id]), ...Object.values(confirmed)].map((entry) => <article className="card" key={entry.id}>
        <h3>{entry.asked_by_name ?? 'Name unavailable'} · {entry.relation} on {entry.resource.id}</h3>
        <p><strong>{entry.state === 'waiting' ? 'Awaiting a decision' : entry.state === 'approved' ? 'Approved' : 'Declined'}</strong> · Asked {clock(entry.asked_at)}</p>
        <details><summary>Request details</summary><p>Request identifier: {entry.id}</p><p>Account identifier: {entry.asked_by}</p></details><p>{entry.why}</p><p className="note">Allows {entry.actions.join(', ')}. {entry.ends_at === null ? 'No expiry requested.' : 'Until ' + clock(entry.ends_at) + '.'}</p>
        {entry.decision ? <p>{entry.decision.note}{entry.decision.grant ? <> · <Link to={'/file/' + encodeURIComponent(entry.asked_by) + '/access'}>View granted access</Link></> : null}</p>
          : <p className="note">Can be reviewed by {entry.approvers.map((person) => person.display_name).join(', ') || 'no currently eligible person'}.</p>}
        {entry.state === 'waiting' && entry.held_by ? <HeldApproval entry={entry} changed={changed} /> : null}
        {entry.state === 'waiting' && !entry.held_by && choices.status === 'ok' && (entry.can_decide ?? entry.approvers.some((person) => person.id === choices.data.person))
          ? <DecisionForm entry={entry} person={choices.data.person} canIssueRoot={entry.can_issue_root === true} changed={changed} /> : null}
      </article>)}
    </section>} />
  </div>;
}
