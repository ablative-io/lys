/** The server identifies grants for review; this view never invents a recorded keep decision. */
import { useState } from 'react';
import { Link } from 'react-router';
import { request, useLoad } from '../../api';
import type { AgentSummary, PersonSummary } from '../../generated';
import type { Grant } from '../../generated/grants';
import { clock } from '../file/time';
import { Gate } from '../signin/Gate';

/** crates/lys-identity-server/src/reviews_api.rs: ReviewView. */
interface ReviewsView {
  scope: 'personal' | 'directory';
  due: { grant: Grant; agent: AgentSummary; reviewer: PersonSummary }[];
  unanswered: { agent: AgentSummary; person: PersonSummary }[];
  revision: number;
  judged_at: number;
}
export function Reviews() {
  const [revision, setRevision] = useState(0);
  const load = useLoad(() => request<ReviewsView>('/reviews'), 'reviews:' + revision);
  return <div className="page">
    <div className="head"><div><div className="eyebrow">Access</div><h1>Review agent access</h1>
      <p className="sub">Check what each agent can do, and withdraw access it no longer needs.</p></div>
      <button className="btn" disabled={load.status === 'loading'} onClick={() => setRevision((value) => value + 1)}>Refresh</button></div>
    <Gate load={load} title="Reviews" ok={(view) => <>
      <p className="note">{view.scope === 'directory' ? 'All agents in the directory.' : 'Agents you are responsible for.'} Checked {clock(view.judged_at)}.</p>
      {view.unanswered.length ? <section className="card" aria-label="Agents needing an active owner"><h2>These agents need an active owner</h2>
        {view.unanswered.map(({ agent, person }) => <p key={agent.id}><Link to={'/file/' + encodeURIComponent(agent.id)}>{agent.display_name}</Link> answers to {person.display_name}, whose account is {person.state}.</p>)}
      </section> : null}
      {view.due.length ? <table><thead><tr><th>Agent</th><th>Access</th><th>Responsible person</th><th>Action</th></tr></thead><tbody>
        {view.due.map(({ grant, agent, reviewer }) => <tr key={grant.id}>
          <td>{agent.display_name}</td><td>{grant.relation} on {grant.resource.kind} {grant.resource.id}<div className="note">{grant.actions.join(', ')}</div></td>
          <td>{reviewer.display_name}</td><td><Link to={'/file/' + encodeURIComponent(agent.id) + '/access'}>Review access</Link></td>
        </tr>)}
      </tbody></table> : <p className="note">No current agent grants need your review.</p>}
    </>} />
  </div>;
}
