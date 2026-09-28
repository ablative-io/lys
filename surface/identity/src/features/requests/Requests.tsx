import { HeldApproval } from './HeldApproval';
/** Requests use the service's visibility and approval decisions, never an inferred permission. */
import { useState } from 'react';
import { Link } from 'react-router';
import { api, request, useLoad } from '../../api';
import { clock } from '../file/time';
import { Gate } from '../signin/Gate';
import { AskForm } from './AskForm';
import { DecisionForm } from './DecisionForm';
import type { AccessRequest } from './contract';

export function Requests() {
  const [revision, setRevision] = useState(0);
  const refresh = () => setRevision((value) => value + 1);
  const load = useLoad(() => request<{ requests: AccessRequest[] }>('/requests'), 'requests:' + revision);
  const choices = useLoad(async () => {
    const [me, grants, model] = await Promise.all([api.me(), api.grants(), api.model()]);
    const resources = [...new Map(grants.grants.map((grant) => [JSON.stringify(grant.resource), grant.resource])).values()];
    return { person: me.person.id, resources, model };
  }, 'request-choices');
  return <div className="page">
    <div className="head"><div><div className="eyebrow">Access</div><h1>Requests</h1><p className="sub">Ask for access and follow the decision.</p></div><button className="btn" disabled={load.status === 'loading'} onClick={refresh}>Refresh requests</button></div>
    <Gate load={choices} title="Request choices" ok={(data) => <AskForm {...data} changed={refresh} />} />
    <Gate load={load} title="Requests" ok={(list) => <section aria-label="Visible requests">
      <h2>Requests you can see</h2>
      {!list.requests.length ? <p className="note">No requests yet.</p> : list.requests.map((entry) => <article className="card" key={entry.id}>
        <h3>{entry.asked_by_name ?? entry.asked_by} · {entry.relation} on {entry.resource.id}</h3>
        <p><strong>{entry.state === 'waiting' ? 'Awaiting a decision' : entry.state === 'approved' ? 'Approved' : 'Declined'}</strong> · Asked {clock(entry.asked_at)}</p>
        <p>{entry.why}</p><p className="note">Allows {entry.actions.join(', ')}. {entry.ends_at === null ? 'No expiry requested.' : 'Until ' + clock(entry.ends_at) + '.'}</p>
        {entry.decision ? <p>{entry.decision.note}{entry.decision.grant ? <> · <Link to={'/file/' + encodeURIComponent(entry.asked_by) + '/access'}>View granted access</Link></> : null}</p>
          : <p className="note">Can be reviewed by {entry.approvers.map((person) => person.display_name).join(', ') || 'no currently eligible person'}.</p>}
        {entry.state === 'waiting' && entry.held_by ? <HeldApproval entry={entry} changed={refresh} /> : null}
        {entry.state === 'waiting' && !entry.held_by && choices.status === 'ok' && (entry.can_decide ?? entry.approvers.some((person) => person.id === choices.data.person))
          ? <DecisionForm entry={entry} person={choices.data.person} canIssueRoot={entry.can_issue_root === true} changed={refresh} /> : null}
      </article>)}
    </section>} />
  </div>;
}
