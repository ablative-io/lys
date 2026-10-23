import { AccessLayout } from '../access/AccessLayout';
import { actionWords } from '../grants/action-words';
import type { GrantModel } from '../../generated/grants';
import { readTogether } from '../../reads';
/** Requests use the service's visibility and approval decisions, never an inferred permission. A queue, oldest first, grouped by the asker's team; one request opens beside it. */
import { useState } from 'react';
import { Link, useParams, useSearchParams } from 'react-router';
import { Refused, api, request, useLoad } from '../../api';
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
import type { Wanted } from './AskForm';
import { DecisionForm } from './DecisionForm';
import { HeldApproval } from './HeldApproval';
import type { AccessRequest } from './contract';
import { Act } from '../../shell/Act';

type Show = 'mine' | 'waiting' | 'decided';

/** Why an extra read is missing, as its cause and never a chore: the page carries on without it, so there is nothing to ask anyone. */
const missing = (problem: unknown): string => problem instanceof Refused && problem.status === 403
  ? 'Lys does not show this account that list.' : 'Lys could not read that list: ' + problemWords(problem);

/** How long a request has waited, as a person says it. */
function waited(asked: number): string {
  const gone = Math.max(0, Math.floor(Date.now() / 1000) - asked);
  if (gone < 3600) return Math.max(1, Math.floor(gone / 60)) + ' min';
  if (gone < 86400) return Math.floor(gone / 3600) + ' h';
  return Math.floor(gone / 86400) + ' d';
}

/** What a refusal's link asks for (`#/requests?kind=…&id=…&action=…`), or nothing when any part is missing. */
function wantedOf(params: URLSearchParams): Wanted | undefined {
  const [kind, id, action] = ['kind', 'id', 'action'].map((name) => params.get(name)?.trim() ?? '');
  return kind && id && action ? { kind, id, action } : undefined;
}

export function Requests() {
  const [params] = useSearchParams();
  // `#/requests/<id>` opens that request: the dashboard's Decide and a shared link land on the request itself.
  const addressed = useParams().request ?? null;
  const wanted = wantedOf(params);
  const [confirmed, setConfirmed] = useState<Record<string, AccessRequest>>({});
  const changed = (answer: AccessRequest) => setConfirmed((held) => ({ ...held, [answer.id]: answer }));
  // Only the queue itself opens the page: names, teams and the asker's own access are extras, each said when missing, so no unrelated read hides the way to ask.
  const load = useLoad(() => readTogether({
    list: request<{ requests: AccessRequest[] }>('/requests'),
    people: api.people().then((read) => ({ read, refused: '' }), (problem: unknown) => ({ read: null, refused: missing(problem) })),
    teams: readTeams().then((list) => ({ list, refused: '' }), (problem: unknown) => ({ list: [], refused: problemWords(problem) })),
  }), 'requests');
  const choices = useLoad(async () => {
    const [me, model, held] = await Promise.all([api.me(), api.model(),
      api.grants().then((read) => ({ grants: read.grants, refused: '' }), (problem: unknown) => ({ grants: [], refused: missing(problem) }))]);
    const resources = [...new Map(held.grants.map((grant) => [JSON.stringify(grant.resource), grant.resource])).values()];
    return { person: me.person.id, resources, model, grants: held.grants, unlisted: held.refused };
  }, 'request-choices');
  // A refusal's link opens the form asking for what it names. Whether the form is open is the page's, so the head's Ask is drawn with it.
  const [asking, setAsking] = useState(wanted !== undefined);
  const person = choices.status === 'ok' ? choices.data.person : '';
  const retained = person !== '' && sessionStorage.getItem('lys.pending.request.' + person) !== null;
  const entriesOf = (list: AccessRequest[]) => [...list.filter((entry) => !confirmed[entry.id]), ...Object.values(confirmed)];
  const ask = asking || retained || (load.status === 'ok' && entriesOf(load.data.list.requests).length === 0);
  return <AccessLayout on="requests" subtitle="Ask for permission to use something, and decide what others have asked for, oldest first."
    actions={load.status === 'ok' && !ask ? <Act symbol="add" name="Ask for access" word="Ask" tone="primary" onClick={() => setAsking(true)} /> : null}>
    <Gate load={load} title="your access requests" renderError={(error) => <ReadFailure error={error} subject="your access requests" />} ok={(data) =>
      <Queue entries={entriesOf(data.list.requests)} data={data} choices={choices} changed={changed} wanted={wanted} ask={ask} retained={retained} setAsking={setAsking} addressed={addressed} />} />
  </AccessLayout>;
}

type Loaded = { people: { read: Awaited<ReturnType<typeof api.people>> | null; refused: string }; teams: { list: Awaited<ReturnType<typeof readTeams>>; refused: string } };
type Choices = ReturnType<typeof useLoad<{ person: string; resources: { kind: string; id: string }[]; model: Awaited<ReturnType<typeof api.model>>; grants: Awaited<ReturnType<typeof api.grants>>['grants']; unlisted: string }>>;

/** A request's state in words. */
const STATE: Record<AccessRequest['state'], string> = { waiting: 'Waiting on a decision', approved: 'Approved', declined: 'Declined' };

function Queue({ entries, data, choices, changed, wanted, ask, retained, setAsking, addressed }: { entries: AccessRequest[]; data: Loaded; choices: Choices; changed: (answer: AccessRequest) => void; wanted: Wanted | undefined;
  ask: boolean; retained: boolean; setAsking: (asking: boolean) => void; addressed: string | null }) {
  const admin = data.people.read?.scope === 'directory';
  const directory = data.people.read?.people ?? [];
  const person = choices.status === 'ok' ? choices.data.person : '';
  const mayDecide = (entry: AccessRequest) => entry.state === 'waiting' && (entry.can_decide ?? entry.approvers.some((each) => each.id === person));
  const [whose, setWhose] = useWhose(admin);
  const [show, setShow] = useState<Show>(() => entries.some(mayDecide) ? 'mine' : 'waiting');
  const [picked, setPicked] = useState<string | null>(addressed);
  const agents = new Map(directory.flatMap((each) => each.agents.map((agent) => [agent.id, each.id] as const)));
  const names = new Map(directory.flatMap((each) => [[each.id, each.display_name] as const, ...each.agents.map((agent) => [agent.id, agent.display_name] as const)]));
  /** What a request is on, by name where it is someone, never a raw id. */
  const onWhat = (entry: AccessRequest): string => names.get(entry.resource.id) ?? entry.resource.id;
  const held = (entry: AccessRequest): Held => ({ id: entry.asked_by, person: agents.get(entry.asked_by) ?? null });
  const shown = entries
    .filter((entry) => show === 'mine' ? mayDecide(entry) : show === 'waiting' ? entry.state === 'waiting' : entry.state !== 'waiting')
    .filter((entry) => whose.kind !== 'mine' ? inWhose(whose, data.teams.list, person, held(entry)) : entry.asked_by === person || held(entry).person === person || mayDecide(entry))
    .sort((left, right) => left.asked_at - right.asked_at);
  const groups = groupByTeam(shown, held, data.teams.list, whose, (id) => names.get(id) ?? 'someone outside your view');
  const open = entries.find((entry) => entry.id === picked) ?? shown[0] ?? null;
  const columns: Column<AccessRequest>[] = [
    { head: 'Who asked', cell: (entry) => entry.asked_by_name ?? 'Name unavailable' },
    { head: 'For', cell: (entry) => <span className="sec">{entry.relation} on {onWhat(entry)}</span> },
    { head: 'Waiting', cell: (entry) => entry.state === 'waiting' ? waited(entry.asked_at) : <span className="dim">{STATE[entry.state] ?? 'Decided'}</span> },
    { head: 'Can decide', cell: (entry) => <span className="sec">{mayDecide(entry) ? 'you' : entry.approvers.map((each) => each.display_name).join(', ') || 'no one eligible'}</span> },
  ];
  const count = (items: AccessRequest[]) => { const waiting = items.filter((entry) => entry.state === 'waiting').length; return waiting ? waiting + ' waiting' : items.length + ' decided'; };
  return <>
    {data.people.refused ? <p className="note">People's names are not shown here, so each request shows who asked by the name they gave. {data.people.refused}</p> : null}
    {data.teams.refused ? <p className="why-not">Teams cannot be read, so requests are listed without their team. {data.teams.refused}</p> : null}
    {addressed && !entries.some((entry) => entry.id === addressed) ? <p className="note" data-request-unknown>Lys shows you no request at this address: it is not one you may see, or the link is wrong. The requests you may see are below.</p> : null}
    <div className="body work">
      <Listing<AccessRequest> groups={groups} columns={columns} id={(entry) => entry.id} href={(entry) => '#/requests/' + entry.id}
        words={(entry) => (entry.asked_by_name ?? '') + ' ' + entry.relation + ' ' + onWhat(entry) + ' ' + entry.why} noun="requests" holds={count}
        empty={show === 'mine' ? 'Nothing is waiting on you.' : show === 'waiting' ? 'Nothing is waiting.' : 'Nothing has been decided yet.'}
        selected={open?.id ?? null} select={() => undefined} open={(entry) => { setPicked(entry.id); setAsking(false); }}
        tools={<>
          <WhoseSelect whose={whose} set={setWhose} teams={data.teams.list} admin={admin} />
          <div className="seg">{([['mine', 'Waiting on me'], ['waiting', 'All waiting'], ['decided', 'Decided']] as [Show, string][]).map(([key, label]) => <button key={key} className={show === key ? 'on' : ''} aria-pressed={show === key} onClick={() => setShow(key)}>{label}</button>)}</div>
        </>} />
      <div className="detail">
        {ask ? <Gate load={choices} title="the access you can request" renderError={(error) => <ReadFailure error={error} subject="the access you can request" />} ok={(each) => <AskForm key={wanted ? wanted.kind + ' ' + wanted.id + ' ' + wanted.action : 'ask'} person={each.person} resources={each.resources} unlisted={each.unlisted} wanted={wanted} model={each.model} changed={(answer) => { changed(answer); setAsking(true); setPicked(answer.id); }}
            cancel={entries.length && !retained ? () => setAsking(false) : undefined} />} />
          : open ? <Detail model={choices.status === 'ok' ? choices.data.model : null} describe={describeSource(choices.status === 'ok' ? choices.data.grants : [])} decider={open.decision ? names.get(open.decision.by) : undefined} entry={open} on={onWhat(open)} person={person} mayDecide={mayDecide(open)} changed={(answer) => { setPicked(answer.id); changed(answer); }} /> : null}
      </div>
    </div>
  </>;
}

/** A source grant the decider holds, said by what it is on; one not in their list is said so, never shown by its identifier. */
const describeSource = (grants: { id: string; relation: string; resource: { kind: string; id: string } }[]) => (source: string, index: number): string => {
  const grant = grants.find((each) => each.id === source);
  return grant ? 'Your ' + grant.relation + ' access to ' + grant.resource.kind + ' ' + grant.resource.id : 'Your access ' + (index + 1) + ' (not in your list of access)';
};

function Detail({ model, describe, decider, entry, on, person, mayDecide, changed }: { model: GrantModel | null; decider: string | undefined; describe: (source: string, index: number) => string; entry: AccessRequest; on: string; person: string; mayDecide: boolean; changed: (answer: AccessRequest) => void }) {
  return <article className="card" aria-label="Request">
    <h2>{entry.asked_by_name ?? 'Name unavailable'} · {entry.relation} on {on}</h2>
    <p><strong>{STATE[entry.state] ?? 'Decided'}</strong> · Asked {clock(entry.asked_at)}</p>
    <p>{entry.why}</p>
    {entry.standing === 'relation_unknown'
      ? <p className="why-not" data-outlived>Cannot be granted as the model stands: {entry.resource.kind} no longer defines {entry.relation}. {entry.ends_at === null ? 'No expiry requested.' : 'Until ' + clock(entry.ends_at) + '.'}</p>
      : <p className="note">Allows {model ? actionWords(model, entry.resource, entry.actions) : 'permission descriptions are unavailable'}. {entry.ends_at === null ? 'No expiry requested.' : 'Until ' + clock(entry.ends_at) + '.'}</p>}
    {entry.decision ? <p>{STATE[entry.state] ?? 'Decided'} by {decider ?? 'someone outside your view'}, {clock(entry.decision.decided_at)}: {entry.decision.note}{entry.decision.grant ? <> · <Link to={'/file/' + encodeURIComponent(entry.asked_by) + '/access'}>View granted access</Link></> : null}</p>
      : <p className="note">Can be reviewed by {entry.approvers.map((each) => each.display_name).join(', ') || 'no currently eligible person'}.</p>}
    {entry.state === 'waiting' && entry.held_by ? <HeldApproval entry={entry} changed={changed} /> : null}
    {entry.state === 'waiting' && !entry.held_by && mayDecide ? <DecisionForm entry={entry} person={person} canIssueRoot={entry.can_issue_root === true} declineOnly={entry.standing === 'relation_unknown'} changed={changed} describe={describe} /> : null}
  </article>;
}
