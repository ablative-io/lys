import { AccessTabs } from '../access/AccessTabs';
import { actionWords } from '../grants/action-words';
import type { GrantModel } from '../../generated/grants';
import { readTogether } from '../../reads';
/** The server identifies grants for review; this view never invents a recorded keep decision. */
import { useState } from 'react';
import { Link } from 'react-router';
import { api, request, useLoad } from '../../api';
import type { AgentSummary, MeView, PeopleView, PersonSummary } from '../../generated';
import { Listing } from '../../shell/Listing';
import type { Column } from '../../shell/Listing';
import { groupByTeam, inWhose } from '../../shell/org';
import type { Held } from '../../shell/org';
import { useWhose, WhoseSelect } from '../../shell/Whose';
import { problemWords } from '../people/Words';
import { readTeams } from '../teams/Teams';
import type { Grant } from '../../generated/grants';
import { clock } from '../file/time';
import { Gate } from '../signin/Gate';
import { ActForm, ActPanel } from '../grants/GrantTable';
import { Chain } from '../grants/Chain';
import { chainOf, lastUsedText, mayText, nameOf, readGrantWorld, resourceName, resourceTitle, voidOf, whoTitle, windowText } from '../grants/model';
import type { GrantWorld } from '../grants/model';
import '../grants/grants.css';
import { KeepGrant } from './KeepGrant';
import type { Kept } from './KeepGrant';
import { IdentityName, ReadFailure } from '../signin/words';
import { Act } from '../../shell/Act';

/** crates/lys-identity-server/src/reviews_api.rs: ReviewView. */
interface ReviewsView {
  scope: 'personal' | 'directory';
  due: { grant: Grant; agent: AgentSummary; reviewer: PersonSummary; last_kept?: { by: string; at: number; note: string } | null }[];
  unanswered: { agent: AgentSummary; person: PersonSummary }[];
  revision: number;
  judged_at: number;
  decisions_recorded?: boolean;
}
type Due = ReviewsView['due'][number];

export function Reviews() {
  const [confirmed, setConfirmed] = useState<Record<string, Kept>>({});
  const [notice, setNotice] = useState('');
  const [revision, setRevision] = useState(0);
  const load = useLoad(() => readTogether({
    model: api.model(), view: request<ReviewsView>('/reviews'), me: api.me(), people: api.people(),
    teams: readTeams().then((list) => ({ list, refused: '' }), (problem: unknown) => ({ list: [], refused: problemWords(problem) })),
  }), 'reviews:' + revision);
  const kept = (answer: Kept) => { setConfirmed((held) => ({ ...held, [answer.grant]: answer })); setNotice('Your decision to keep this access was recorded. It does not extend the grant or change its permissions.'); };
  return <div className="page fill">
    <div className="head"><div><h1>Access</h1>
      <p className="sub">Check what each agent can do, and withdraw access it no longer needs.</p></div></div>
    <AccessTabs on="reviews" />
    {notice ? <p role="status">{notice}</p> : null}
    <Gate load={load} title="agent access to review" renderError={(error) => <ReadFailure error={error} subject="agent access to review" />} ok={(data) => <Due {...data} confirmed={confirmed} kept={kept} revision={revision} reload={() => setRevision((value) => value + 1)} />} />
  </div>;
}

function Due({ model, view, me, people, teams, confirmed, kept, reload, revision }: { reload: () => void; revision: number; model: GrantModel; view: ReviewsView; me: MeView; people: PeopleView; teams: { list: Awaited<ReturnType<typeof readTeams>>; refused: string }; confirmed: Record<string, Kept>; kept: (answer: Kept) => void }) {
  const world = useLoad(() => readGrantWorld(me), 'reviews-grants:' + revision);
  const admin = people.scope === 'directory';
  const [whose, setWhose] = useWhose(admin);
  const [picked, setPicked] = useState<string | null>(null);
  const [revoking, setRevoking] = useState<HTMLElement | null>(null);
  const owners = new Map(people.people.flatMap((person) => person.agents.map((agent) => [agent.id, person.id] as const)));
  const names = new Map(people.people.flatMap((person) => [[person.id, person.display_name] as const, ...person.agents.map((agent) => [agent.id, agent.display_name] as const)]));
  const held = (due: Due): Held => ({ id: due.agent.id, person: owners.get(due.agent.id) ?? due.reviewer.id });
  const scoped = view.due.filter((due) => whose.kind === 'mine' ? due.reviewer.id === me.person.id || held(due).person === me.person.id : inWhose(whose, teams.list, me.person.id, held(due)));
  const groups = groupByTeam(scoped, held, teams.list, whose, (id) => names.get(id) ?? 'someone outside your view');
  const open = view.due.find((due) => due.grant.id === picked) ?? scoped[0] ?? null;
  const drawn = open && world.status === 'ok' ? world.data.byId.get(open.grant.id) ?? null : null;
  const last = (due: Due) => { const answer = confirmed[due.grant.id]; return answer ? { by: answer.kept_by, at: answer.at, note: answer.note } : due.last_kept ?? null; };
  const columns: Column<Due>[] = [
    { head: 'Agent', cell: (due) => due.agent.display_name },
    { head: 'Access', cell: (due) => <span className="sec">{due.grant.relation} on {due.grant.resource.kind} {names.get(due.grant.resource.id) ?? due.grant.resource.id}</span> },
    { head: 'Responsible person', cell: (due) => <span className="sec">{due.reviewer.display_name}</span> },
    { head: 'Last kept', cell: (due) => { const at = last(due); return at ? <span className="sec">{clock(at.at)}</span> : <span className="dim">never</span>; } },
  ];
  return <>
    <p className="note">{view.scope === 'directory' ? 'All agents in the directory.' : 'Agents you are responsible for.'} Checked {clock(view.judged_at)}.</p>
    {view.unanswered.length ? <section className="card" aria-label="Agents needing an active owner"><h2>These agents need an active owner</h2>
      {view.unanswered.map(({ agent, person }) => <p key={agent.id}><Link to={'/file/' + encodeURIComponent(agent.id)}>{agent.display_name}</Link> answers to {person.display_name}, whose account is {person.state}.</p>)}
    </section> : null}
    {teams.refused ? <p className="why-not">Teams cannot be read, so access is listed without its team. {teams.refused}</p> : null}
    {view.due.length ? <div className="body work">
      <Listing<Due> groups={groups} columns={columns} id={(due) => due.grant.id} href={(due) => '#/reviews?grant=' + due.grant.id}
        words={(due) => due.agent.display_name + ' ' + due.grant.resource.id + ' ' + due.reviewer.display_name} noun="grants to review"
        holds={(items) => items.length + (items.length === 1 ? ' grant' : ' grants')}
        selected={open?.grant.id ?? null} select={() => undefined} open={(due) => { setPicked(due.grant.id); setRevoking(null); }}
        tools={<WhoseSelect whose={whose} set={setWhose} teams={teams.list} admin={admin} />} />
      <div className="detail">{open ? <section className="card" aria-label="Grant to review">
        {/* The chosen grant as facts, one line each, so it reads whole beside the list; the list row already says who is responsible and when it was last kept, so the panel says who kept it and why. */}
        {drawn && world.status === 'ok' ? <GrantFacts w={world.data} g={drawn} agent={open.agent} />
          : <dl className="facts grant-facts">
            <dt>Holder</dt><dd><Link to={'/file/' + encodeURIComponent(open.agent.id) + '/access'}>{open.agent.display_name}</Link></dd>
            <dt>Allows</dt><dd>{open.grant.actions.length ? actionWords(model, open.grant.resource, open.grant.actions) : 'No action'}</dd>
            <dt>On</dt><dd>{open.grant.resource.kind} {names.get(open.grant.resource.id) ?? open.grant.resource.id}</dd>
            <dt>Relation</dt><dd className="mono">{open.grant.relation}</dd>
          </dl>}
        {last(open) ? <p className="note">Last kept by <IdentityName id={last(open)?.by ?? ''} people={people} />{last(open)?.note ? ': ' + last(open)?.note : '.'}</p> : null}
        <div className="grant-acts">
          {view.decisions_recorded ? <KeepGrant key={open.grant.id} grant={open.grant.id} person={me.person.id} changed={kept} /> : <p>This service does not record keep decisions.</p>}
          {drawn && world.status === 'ok' && !drawn.revoked ? <Act symbol="revoke" name="Revoke" word="Revoke" tone="danger" data-act="revoke" data-g={drawn.id} onClick={(event) => setRevoking(event.currentTarget)} /> : null}
        </div>
        {revoking && drawn && world.status === 'ok' ? <ActPanel key={drawn.id} label="Revoke" opener={revoking} close={() => setRevoking(null)}>
          <ActForm w={world.data} g={drawn} act="revoke" done={reload} close={() => setRevoking(null)} />
        </ActPanel> : null}
      </section> : null}</div>
    </div> : <p className="note">No current agent grants need your review.</p>}
  </>;
}

/** One grant as facts: who holds it, what it allows, on what, the path to a person, its window, its last use and whether it stands. */
function GrantFacts({ w, g, agent }: { w: GrantWorld; g: Grant; agent: AgentSummary }) {
  const v = voidOf(w, g);
  return <dl className="facts grant-facts">
    <dt>Holder</dt><dd><Link to={'/file/' + encodeURIComponent(agent.id) + '/access'} title={whoTitle(w, g.holder)}>{nameOf(w, g.holder)}</Link></dd>
    <dt>Allows</dt><dd>{mayText(w, g)}</dd>
    <dt>On</dt><dd title={resourceTitle(w, g.resource)}>{resourceName(w, g.resource)}</dd>
    <dt>Relation</dt><dd className="mono">{g.relation}</dd>
    <dt>Path to a person</dt><dd><Chain w={w} chain={chainOf(w, g)} /></dd>
    <dt>Window</dt><dd>{windowText(g)}</dd>
    <dt>Last used</dt><dd className={g.last_use.seen ? undefined : 'dim'}>{lastUsedText(g)}</dd>
    <dt>Stands</dt><dd>{v === null ? <><span className="dot s-active" />yes</> : <span className="danger" title={v.title}>no: {v.why}</span>}</dd>
  </dl>;
}
