/** The server identifies grants for review; this view never invents a recorded keep decision. */
import { useState } from 'react';
import { Link } from 'react-router';
import { api, request, useLoad } from '../../api';
import type { AgentSummary, PersonSummary } from '../../generated';
import type { Grant } from '../../generated/grants';
import { clock } from '../file/time';
import { Gate } from '../signin/Gate';
import { KeepGrant } from './KeepGrant';
import type { Kept } from './KeepGrant';
import { IdentityName, ReadFailure } from '../signin/words';

/** crates/lys-identity-server/src/reviews_api.rs: ReviewView. */
interface ReviewsView {
  scope: 'personal' | 'directory';
  due: { grant: Grant; agent: AgentSummary; reviewer: PersonSummary; last_kept?: { by: string; at: number; note: string } | null }[];
  unanswered: { agent: AgentSummary; person: PersonSummary }[];
  revision: number;
  judged_at: number;
  decisions_recorded?: boolean;
}
export function Reviews() {
  const [confirmed, setConfirmed] = useState<Record<string, Kept>>({});
  const [notice, setNotice] = useState('');
  const load = useLoad(async () => ({ view: await request<ReviewsView>('/reviews'), me: await api.me(), people: await api.people() }), 'reviews');
  return <div className="page">
    <div className="head"><div>
      <p className="sub">Check what each agent can do, and withdraw access it no longer needs.</p></div></div>
    {notice ? <p role="status">{notice}</p> : null}
    <Gate load={load} title="agent access to review" renderError={(error) => <ReadFailure error={error} subject="agent access to review" />} ok={({ view, me, people }) => <>
      <p className="note">{view.scope === 'directory' ? 'All agents in the directory.' : 'Agents you are responsible for.'} Checked {clock(view.judged_at)}.</p>
      {view.unanswered.length ? <section className="card" aria-label="Agents needing an active owner"><h2>These agents need an active owner</h2>
        {view.unanswered.map(({ agent, person }) => <p key={agent.id}><Link to={'/file/' + encodeURIComponent(agent.id)}>{agent.display_name}</Link> answers to {person.display_name}, whose account is {person.state}.</p>)}
      </section> : null}
      {view.due.length ? <table><thead><tr><th>Agent</th><th>Access</th><th>Responsible person</th><th>Action</th></tr></thead><tbody>
        {view.due.map(({ grant, agent, reviewer, last_kept: previous }) => {
          const answer = confirmed[grant.id];
          const last_kept = answer ? { by: answer.kept_by, at: answer.at, note: answer.note } : previous;
          return <tr key={grant.id}>
          <td>{agent.display_name}</td><td>{grant.relation} on {grant.resource.kind} {grant.resource.id}<div className="note">{grant.actions.join(', ')}</div></td>
          <td>{reviewer.display_name}</td><td><Link to={'/file/' + encodeURIComponent(agent.id) + '/access'}>Review access</Link>{last_kept ? <div>Last kept by <IdentityName id={last_kept.by} people={people} /> on {clock(last_kept.at)}. {last_kept.note}</div> : null}{view.decisions_recorded ? <KeepGrant grant={grant.id} person={me.person.id} changed={(answer) => { setConfirmed((held) => ({ ...held, [answer.grant]: answer })); setNotice('Your decision to keep this access was recorded. It does not extend the grant or change its permissions.'); }} /> : <p>Keep decisions are unavailable here.</p>}</td>
        </tr>; })}
      </tbody></table> : <p className="note">No current agent grants need your review.</p>}
    </>} />
  </div>;
}
