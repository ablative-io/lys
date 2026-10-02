import { actionWords } from '../grants/action-words';
import type { GrantModel } from '../../generated/grants';
import { readTogether } from '../../reads';
/** Requests use the service's visibility and approval decisions, never an inferred permission. A queue, oldest first, grouped by the asker's team; one request opens beside it. */
import { useState } from 'react';
import { Link } from 'react-router';
import { api, request, useLoad } from '../../api';
import { Listing } from '../../shell/Listing';
import type { Column } from '../../shell/Listing';
import { groupByTeam, inWhose } from '../../shell/org';
import type { Held } from '../../shell/org';
import { useWhose, WhoseSelect } from '../../shell/Whose';
import { clock } from '../file/time';
import { problemWords } from '../people/Words';
import { Gate } from '../signin/Gate';
import { ReadFailure } from '../signin/words';
import { readTeams } from '../teams/Teams';
import { AskForm } from './AskForm';
import { DecisionForm } from './DecisionForm';
import { HeldApproval } from './HeldApproval';
import type { AccessRequest } from './contract';

type Show = 'mine' | 'waiting' | 'decided';

/** How long a request has waited, as a person says it. */
function waited(asked: number): string {
  const gone = Math.max(0, Math.floor(Date.now() / 1000) - asked);
  if (gone < 3600) return Math.max(1, Math.floor(gone / 60)) + ' min';
  if (gone < 86400) return Math.floor(gone / 3600) + ' h';
  return Math.floor(gone / 86400) + ' d';
}

export function Requests() {
  const [confirmed, setConfirmed] = useState<Record<string, AccessRequest>>({});
  const changed = (answer: AccessRequest) => setConfirmed((held) => ({ ...held, [answer.id]: answer }));
  const load = useLoad(() => readTogether({
    list: request<{ requests: AccessRequest[] }>('/requests'), people: api.people(),
    teams: readTeams().then((list) => ({ list, refused: '' }), (problem: unknown) => ({ list: [], refused: problemWords(problem) })),
  }), 'requests');
  const choices = useLoad(async () => {
    const [me, grants, model] = await Promise.all([api.me(), api.grants(), api.model()]);
    const resources = [...new Map(grants.grants.map((grant) => [JSON.stringify(grant.resource), grant.resource])).values()];
    return { person: me.person.id, resources, model };
  }, 'request-choices');
  return <div className="page fill">
    <Gate load={load} title="your access requests" renderError={(error) => <ReadFailure error={error} subject="your access requests" />} ok={(data) => {
      const entries = [...data.list.requests.filter((entry) => !confirmed[entry.id]), ...Object.values(confirmed)];
      return <Queue entries={entries} data={data} choices={choices} changed={changed} />;
    }} />
  </div>;
}

type Loaded = { people: Awaited<ReturnType<typeof api.people>>; teams: { list: Awaited<ReturnType<typeof readTeams>>; refused: string } };
type Choices = ReturnType<typeof useLoad<{ person: string; resources: { kind: string; id: string }[]; model: Awaited<ReturnType<typeof api.model>> }>>;

function Queue({ entries, data, choices, changed }: { entries: AccessRequest[]; data: Loaded; choices: Choices; changed: (answer: AccessRequest) => void }) {
  const admin = data.people.scope === 'directory';
  const person = choices.status === 'ok' ? choices.data.person : '';
  const mayDecide = (entry: AccessRequest) => entry.state === 'waiting' && (entry.can_decide ?? entry.approvers.some((each) => each.id === person));
  const [whose, setWhose] = useWhose(admin);
  const [show, setShow] = useState<Show>(() => entries.some(mayDecide) ? 'mine' : 'waiting');
  const retained = person !== '' && sessionStorage.getItem('lys.pending.request.' + person) !== null;
  const [asking, setAsking] = useState(false);
  const [picked, setPicked] = useState<string | null>(null);
  const agents = new Map(data.people.people.flatMap((each) => each.agents.map((agent) => [agent.id, each.id] as const)));
  const names = new Map(data.people.people.flatMap((each) => [[each.id, each.display_name] as const, ...each.agents.map((agent) => [agent.id, agent.display_name] as const)]));
  const held = (entry: AccessRequest): Held => ({ id: entry.asked_by, person: agents.get(entry.asked_by) ?? null });
  const shown = entries
    .filter((entry) => show === 'mine' ? mayDecide(entry) : show === 'waiting' ? entry.state === 'waiting' : entry.state !== 'waiting')
    .filter((entry) => whose.kind !== 'mine' ? inWhose(whose, data.teams.list, person, held(entry)) : entry.asked_by === person || held(entry).person === person || mayDecide(entry))
    .sort((left, right) => left.asked_at - right.asked_at);
  const groups = groupByTeam(shown, held, data.teams.list, whose, (id) => names.get(id) ?? 'someone outside your view');
  const open = entries.find((entry) => entry.id === picked) ?? shown[0] ?? null;
  const ask = asking || retained || entries.length === 0;
  const columns: Column<AccessRequest>[] = [
    { head: 'Who asked', cell: (entry) => entry.asked_by_name ?? 'Name unavailable' },
    { head: 'For', cell: (entry) => <span className="sec">{entry.relation} on {entry.resource.id}</span> },
    { head: 'Waiting', cell: (entry) => entry.state === 'waiting' ? waited(entry.asked_at) : <span className="dim">{entry.state}</span> },
    { head: 'Can decide', cell: (entry) => <span className="sec">{mayDecide(entry) ? 'you' : entry.approvers.map((each) => each.display_name).join(', ') || 'no one eligible'}</span> },
  ];
  const count = (items: AccessRequest[]) => { const waiting = items.filter((entry) => entry.state === 'waiting').length; return waiting ? waiting + ' waiting' : items.length + ' decided'; };
  return <>
    <div className="head">
      <div><div className="eyebrow">Access</div><h1>Requests</h1><p className="sub">Ask for permission to use something, and decide what others have asked for, oldest first.</p></div>
      {!ask ? <button className="btn primary" onClick={() => setAsking(true)}>+ Ask for access</button> : null}
    </div>
    {data.teams.refused ? <p className="why-not">Teams cannot be read, so requests are listed without their team. {data.teams.refused}</p> : null}
    <div className="body">
      <Listing<AccessRequest> groups={groups} columns={columns} id={(entry) => entry.id} href={(entry) => '#/requests?request=' + entry.id}
        words={(entry) => (entry.asked_by_name ?? '') + ' ' + entry.relation + ' ' + entry.resource.id + ' ' + entry.why} noun="requests" holds={count}
        selected={open?.id ?? null} select={() => undefined} open={(entry) => { setPicked(entry.id); setAsking(false); }}
        tools={<>
          <WhoseSelect whose={whose} set={setWhose} teams={data.teams.list} admin={admin} />
          <div className="seg">{([['mine', 'Waiting on me'], ['waiting', 'All waiting'], ['decided', 'Decided']] as [Show, string][]).map(([key, label]) => <button key={key} className={show === key ? 'on' : ''} onClick={() => setShow(key)}>{label}</button>)}</div>
        </>} />
      <div className="detail">
        {ask ? <Gate load={choices} title="the access you can request" renderError={(error) => <ReadFailure error={error} subject="the access you can request" />} ok={(each) => <AskForm {...each} changed={(answer) => { changed(answer); setAsking(true); setPicked(answer.id); }} />} />
          : open ? <Detail model={choices.status === 'ok' ? choices.data.model : null} entry={open} person={person} mayDecide={mayDecide(open)} changed={(answer) => { setPicked(answer.id); changed(answer); }} /> : <p className="dim">Nothing is waiting.</p>}
      </div>
    </div>
  </>;
}

function Detail({ model, entry, person, mayDecide, changed }: { model: GrantModel | null; entry: AccessRequest; person: string; mayDecide: boolean; changed: (answer: AccessRequest) => void }) {
  return <article className="card" aria-label="Request">
    <h2>{entry.asked_by_name ?? 'Name unavailable'} · {entry.relation} on {entry.resource.id}</h2>
    <p><strong>{entry.state === 'waiting' ? 'Awaiting a decision' : entry.state === 'approved' ? 'Approved' : 'Declined'}</strong> · Asked {clock(entry.asked_at)}</p>
    <p>{entry.why}</p>
    <p className="note">Allows {model ? actionWords(model, entry.resource, entry.actions) : 'permission descriptions are unavailable'}. {entry.ends_at === null ? 'No expiry requested.' : 'Until ' + clock(entry.ends_at) + '.'}</p>
    {entry.decision ? <p>{entry.decision.note}{entry.decision.grant ? <> · <Link to={'/file/' + encodeURIComponent(entry.asked_by) + '/access'}>View granted access</Link></> : null}</p>
      : <p className="note">Can be reviewed by {entry.approvers.map((each) => each.display_name).join(', ') || 'no currently eligible person'}.</p>}
    {entry.state === 'waiting' && entry.held_by ? <HeldApproval entry={entry} changed={changed} /> : null}
    {entry.state === 'waiting' && !entry.held_by && mayDecide ? <DecisionForm entry={entry} person={person} canIssueRoot={entry.can_issue_root === true} changed={changed} /> : null}
  </article>;
}
