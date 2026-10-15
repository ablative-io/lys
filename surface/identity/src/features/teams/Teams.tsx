import { readTogether } from '../../reads';
/** The team's owner or administrator manages named membership; no inherited permissions are implied. */
import { useState } from 'react';
import { api, operationId, request, useLoad } from '../../api';
import type { Login } from '../../generated';
import { DirectoryGate as Gate } from '../people/Words';
import { useRoleChange } from '../roles/useRoleChange';
import { ChangeStatus } from '../roles/ChangeStatus';
import { entries } from '../people/directory';
import { TeamMembers } from './TeamActions';
import { TeamUsage } from '../usage/Usage';
import { sameLogin } from './contract';
import type { Team, TeamChanged } from './contract';
import type { MeView, PeopleView } from '../../generated';
import { subtree, treeOrder } from '../../shell/org';
import { Act } from '../../shell/Act';
import { TableScroll } from '../../shell/Listing';
import './teams.css';
/** The organisation's teams, as /teams serves them, parent and lead included. */
export async function readTeams(): Promise<Team[]> {
  return (await request<{ teams: Team[] }>('/teams')).teams;
}

export function Teams() {
  const load = useLoad(() => readTogether({ teams: readTeams(), me: api.me(), people: api.people() }), 'teams');
  return <Gate load={load} title="Teams" ok={({ teams, me, people }) => <TeamList initial={teams} me={me} people={people} />} />;
}

/** People and agents in a team and every team under it, each once; a person's agents come with them. */
function size(teams: Team[], id: string, people: PeopleView): string {
  const below = subtree(teams, id);
  const members = new Set(teams.filter((team) => below.has(team.id)).flatMap((team) => team.members));
  const persons = people.people.filter((person) => members.has(person.id));
  const agents = new Set([...people.people.flatMap((person) => person.agents).filter((agent) => members.has(agent.id)).map((agent) => agent.id), ...persons.flatMap((person) => person.agents.map((agent) => agent.id))]);
  const n = (count: number, one: string, many: string) => count.toLocaleString('en-AU') + ' ' + (count === 1 ? one : many);
  return n(persons.length, 'person', 'people') + ', ' + n(agents.size, 'agent', 'agents');
}

function TeamList({ initial, me, people }: { initial: Team[]; me: MeView; people: PeopleView }) {
  const [teams, setTeams] = useState(initial);
  const [revision, setRevision] = useState(0);
  const [notice, setNotice] = useState('');
  const [query, setQuery] = useState('');
  const [creating, setCreating] = useState(initial.length === 0);
  const [picked, setPicked] = useState<string | null>(null);
  const changed = (answer: TeamChanged, message: string) => {
    setTeams((values) => values.some((team) => team.id === answer.id) ? values.map((team) => team.id === answer.id ? answer : team) : [...values, answer]);
    setNotice(message); setRevision((value) => value + 1); setCreating(false); setPicked(answer.id);
  };
  const members = entries(people);
  const name = (id: string) => members.find((entry) => entry.id === id)?.display_name ?? 'someone outside your view';
  const manages = (each: Team) => each.owner === me.person.id || people.scope === 'directory';
  const needle = query.trim().toLowerCase();
  // Retired teams are left out until asked for, with the same control the All view has.
  const [retired, setRetired] = useState(false);
  const retiredCount = teams.filter((each) => each.state === 'retired').length;
  const order = [...treeOrder(teams), ...(retired ? teams.filter((each) => each.state === 'retired').map((each) => ({ team: each, depth: 0 })) : [])];
  const shown = order.filter(({ team }) => !needle || team.name.toLowerCase().includes(needle));
  const team = teams.find((each) => each.id === (picked ?? shown[0]?.team.id)) ?? null;
  const listed = teams.length - (retired ? 0 : retiredCount);
  return <>
    <div className="tools">
      <input className="search" type="search" aria-label="Search teams" placeholder={'Search ' + listed.toLocaleString('en-AU') + (listed === 1 ? ' team' : ' teams')} value={query} onChange={(event) => setQuery(event.target.value)} />
      {retiredCount ? <button type="button" className="btn" aria-pressed={retired} onClick={() => setRetired(!retired)}>{retired ? 'Hide retired' : 'Show retired (' + retiredCount + ')'}</button> : null}
      <Act symbol="add" name="Create a team" word="New team" tone="primary" onClick={() => { setCreating(true); setNotice(''); }} />
      {notice ? <span role="status" className="note">{notice}</span> : null}
    </div>
    <div className="body halves teams-body">
      <div className="pane">
        {/* The list only says where each team sits; its place is changed beside it, on the chosen team, where there is room for the choices. */}
        <TableScroll label="Teams"><table className="teams-table usage-table">
          <thead><tr><th>Team</th><th>Part of</th><th>Lead</th><th>Members, with teams under it</th></tr></thead>
          <tbody>{shown.map(({ team: each, depth }) => <tr key={each.id} data-team={each.id} data-depth={depth} className={each.id === team?.id && !creating ? 'cursor' : ''} tabIndex={0} aria-selected={each.id === team?.id && !creating}
            onClick={() => { setPicked(each.id); setCreating(false); }} onKeyDown={(event) => { if (event.key === 'Enter' && event.target === event.currentTarget) { setPicked(each.id); setCreating(false); } }}>
            <td style={{ paddingLeft: 10 + depth * 18 }}>{each.name}{each.state === 'retired' ? <span className="note"> (retired)</span> : null}</td>
            <td className="sec">{each.parent ? teams.find((other) => other.id === each.parent)?.name ?? 'A team outside your view' : <span className="dim">None</span>}</td>
            <td className="sec">{each.lead ? name(each.lead) : <span className="dim">None named</span>}</td>
            <td className="sec">{size(teams, each.id, people)}</td>
          </tr>)}
            {teams.length && !shown.length ? <tr><td colSpan={4} className="dim">{needle ? 'No team matches that search.' : 'Every team is retired.'}</td></tr> : null}</tbody>
        </table></TableScroll>
        {teams.length ? null : <p className="dim">No teams yet. A team is how Lys groups people and their agents: create the first one.</p>}
      </div>
      <div className="pane">
        {creating ? <Create key={revision} person={me.person.id} login={me.signed_in} changed={changed} cancel={teams.length ? () => setCreating(false) : undefined} /> : team ? <div key={team.id} className="team-detail">
          <section className="card" aria-label={'About ' + team.name}><h2>{team.name}{team.state === 'retired' ? <span className="note"> (retired)</span> : null}</h2>
            {team.description ? <p className="sec">{team.description}</p> : null}
            <p>Managed by <a href={'#/file/' + team.owner}>{name(team.owner)}</a>.</p>
            <TeamPlace key={team.id + ':' + revision} team={team} teams={teams} manages={manages(team)} person={me.person.id} login={me.signed_in} name={name} changed={changed} />
          </section>
          <TeamMembers key={team.id + ':' + revision} team={team} person={me.person.id} login={me.signed_in} members={members} name={name} administrator={people.scope === 'directory'} manages={manages(team)} changed={changed} />
          <TeamUsage team={team.id} name={team.name} />
        </div> : null}
      </div>
    </div>
  </>;
}

/** Where a team sits: the team it is part of and its lead. Whoever manages it changes both here, kept by one change. */
function TeamPlace({ team, teams, manages, person, login, name, changed }: { team: Team; teams: Team[]; manages: boolean; person: string; login: Login; name: (id: string) => string; changed: (answer: TeamChanged, message: string) => void }) {
  const [parent, setParent] = useState(team.parent ?? '');
  const [lead, setLead] = useState(team.lead ?? '');
  const change = useRoleChange<TeamChanged>('lys.pending.team-nest.' + person + '.' + team.id, '/teams/' + encodeURIComponent(team.id) + '/nesting',
    (answer, body) => answer.id === team.id && (answer.parent ?? null) === body.parent && (answer.lead ?? null) === body.lead && answer.recorded?.operation === body.operation && answer.recorded.act === 'nested' && sameLogin(answer.recorded.by, login),
    (answer) => changed(answer, 'The team’s place was recorded.'));
  const below = subtree(teams, team.id);
  const parents = teams.filter((each) => each.state === 'active' && !below.has(each.id) && each.id !== team.id);
  const leads = [...new Set([...team.members.filter((id) => !team.held?.some((each) => each.member === id)), ...(team.lead ? [team.lead] : [])])];
  const dirty = parent !== (team.parent ?? '') || lead !== (team.lead ?? '');
  if (!manages || team.state !== 'active') return null;
  return <form className="form-grid team-place" aria-label={'Where ' + team.name + ' sits'} onSubmit={(event) => { event.preventDefault(); if (dirty && !change.blocked) change.submit({ operation: operationId(), parent: parent || null, lead: lead || null }); }}>
    <label className="field">Part of<select aria-label={'Team that ' + team.name + ' is part of'} value={parent} disabled={change.blocked} onChange={(event) => setParent(event.target.value)}><option value="">No team above it</option>{parents.map((each) => <option key={each.id} value={each.id}>{each.name}</option>)}</select></label>
    <label className="field">Lead<select aria-label={'Lead of ' + team.name} value={lead} disabled={change.blocked || !leads.length} onChange={(event) => setLead(event.target.value)}><option value="">None named</option>{leads.map((id) => <option key={id} value={id}>{name(id)}</option>)}</select>
      {leads.length ? null : <span className="hint">Add a member first; the lead is one of its members.</span>}</label>
    {dirty || change.pending ? <p className="wide"><Act symbol="save" name="Save place" word="Save" tone="primary" type="submit" disabled={change.blocked || !dirty} />{' '}
      <Act symbol="close" name="Undo these changes" word="Undo" disabled={change.blocked} onClick={() => { setParent(team.parent ?? ''); setLead(team.lead ?? ''); }} /></p> : null}
    <div className="wide"><ChangeStatus change={change} /></div>
  </form>;
}

function Create({ person, login, changed, cancel }: { person: string; login: Login; changed: (answer: TeamChanged, message: string) => void; /** Back to the teams, when there are any to go back to. */ cancel?: () => void }) {
  const [name, setName] = useState(''); const [description, setDescription] = useState('');
  const change = useRoleChange<TeamChanged>('lys.pending.team-create.' + person, '/teams', (answer, body) => answer.id === body.operation && answer.owner === person && answer.name === body.name && answer.description === body.description && answer.recorded?.operation === body.operation && answer.recorded.act === 'created' && sameLogin(answer.recorded.by, login), (answer) => changed(answer, 'Your team was recorded.'));
  return <form className="card" aria-label="Create team" onSubmit={(event) => { event.preventDefault(); if (name.trim()) change.submit({ operation: operationId(), name: name.trim(), description: description.trim() }); }}><h3>Create a team</h3>
    <label className="field">Team name<input name="team-name" value={name} required autoFocus autoComplete="off" disabled={change.blocked} onChange={(event) => setName(event.target.value)} /></label>
    <label className="field">What the team does <span className="hint">Optional.</span><textarea name="team-description" rows={3} value={description} disabled={change.blocked} onChange={(event) => setDescription(event.target.value)} /></label>
    <p className="team-create-acts"><Act symbol="add" name="Create team" word="Create" tone="primary" type="submit" disabled={change.blocked || !name.trim()} aria-describedby={name.trim() ? undefined : 'team-create-why'} />
      {cancel ? <><Act symbol="close" name="Cancel" word="Cancel" disabled={change.busy} onClick={cancel} /></> : null}</p>
    {name.trim() || change.blocked ? null : <p className="why-not" id="team-create-why">Give the team a name.</p>}
    <ChangeStatus change={change} />
  </form>;
}
