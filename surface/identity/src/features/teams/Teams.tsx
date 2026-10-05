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
  return <>
    <div className="tools">
      <input className="search" type="search" aria-label="Search teams" placeholder={'Search ' + (teams.length - (retired ? 0 : retiredCount)).toLocaleString('en-AU') + (teams.length - (retired ? 0 : retiredCount) === 1 ? ' team' : ' teams')} value={query} onChange={(event) => setQuery(event.target.value)} />
      {retiredCount ? <button type="button" className="btn" aria-pressed={retired} onClick={() => setRetired(!retired)}>{retired ? 'Hide retired' : 'Show retired (' + retiredCount + ')'}</button> : null}
      <Act symbol="add" name="Create a team" word="Team" tone="primary" onClick={() => { setCreating(true); setNotice(''); }} />
      {notice ? <span role="status" className="note">{notice}</span> : null}
    </div>
    <div className="body halves">
      <div className="pane">
        <table className="teams-table usage-table">
          <thead><tr><th>Team</th><th>Part of</th><th>Lead</th><th>Holds</th><th>Change</th></tr></thead>
          <tbody>{shown.map(({ team: each, depth }) => <TeamRow key={each.id + ':' + revision} team={each} depth={depth} teams={teams} current={each.id === team?.id && !creating} manages={manages(each)} person={me.person.id} login={me.signed_in}
            name={name} holds={size(teams, each.id, people)} pick={() => { setPicked(each.id); setCreating(false); }} changed={changed} />)}
            {teams.length && !shown.length ? <tr><td colSpan={5} className="dim">{needle ? 'No team matches that search.' : 'Every team is retired.'}</td></tr> : null}</tbody>
        </table>
        {teams.length ? null : <p className="dim">No teams yet. A team is how Lys groups people and their agents: create the first one.</p>}
      </div>
      <div className="pane">
        {creating ? <Create key={revision} person={me.person.id} login={me.signed_in} changed={changed} /> : team ? <div key={team.id}>
          <section className="card"><h2>{team.name}</h2>
            {team.description ? <p className="sec">{team.description}</p> : null}
            <p>Managed by <a href={'#/file/' + team.owner}>{name(team.owner)}</a>.</p>
          </section>
          <TeamMembers key={team.id + ':' + revision} team={team} person={me.person.id} login={me.signed_in} members={members} name={name} administrator={people.scope === 'directory'} manages={manages(team)} changed={changed} />
          <TeamUsage team={team.id} name={team.name} />
        </div> : null}
      </div>
    </div>
  </>;
}

/** One team as a row. Whoever manages it sets the team it is part of and its lead here; both are kept by one change. */
function TeamRow({ team, depth, teams, current, manages, person, login, name, holds, pick, changed }: { team: Team; depth: number; teams: Team[]; current: boolean; manages: boolean; person: string; login: Login; name: (id: string) => string; holds: string; pick: () => void; changed: (answer: TeamChanged, message: string) => void }) {
  const [parent, setParent] = useState(team.parent ?? '');
  const [lead, setLead] = useState(team.lead ?? '');
  const change = useRoleChange<TeamChanged>('lys.pending.team-nest.' + person + '.' + team.id, '/teams/' + encodeURIComponent(team.id) + '/nesting',
    (answer, body) => answer.id === team.id && (answer.parent ?? null) === body.parent && (answer.lead ?? null) === body.lead && answer.recorded?.operation === body.operation && answer.recorded.act === 'nested' && sameLogin(answer.recorded.by, login),
    (answer) => changed(answer, 'The team’s place was recorded.'));
  const below = subtree(teams, team.id);
  const parents = teams.filter((each) => each.state === 'active' && !below.has(each.id) && each.id !== team.id);
  const leads = [...new Set([...team.members.filter((id) => !team.held?.some((each) => each.member === id)), ...(team.lead ? [team.lead] : [])])];
  const edits = manages && team.state === 'active';
  const dirty = parent !== (team.parent ?? '') || lead !== (team.lead ?? '');
  return <tr data-team={team.id} data-depth={depth} className={current ? 'cursor' : ''} tabIndex={0} onClick={pick} onKeyDown={(event) => { if (event.key === 'Enter' && event.target === event.currentTarget) pick(); }}>
    <td style={{ paddingLeft: 10 + depth * 18 }}>{team.name}{team.state === 'retired' ? <span className="note"> retired</span> : null}</td>
    <td className="sec">{edits ? <select aria-label={'Team that ' + team.name + ' is part of'} value={parent} disabled={change.blocked} onChange={(event) => setParent(event.target.value)}><option value="">No team above it</option>{parents.map((each) => <option key={each.id} value={each.id}>{each.name}</option>)}</select>
      : team.parent ? teams.find((each) => each.id === team.parent)?.name ?? 'a team outside your view' : <span className="dim">none</span>}</td>
    <td className="sec">{edits ? <select aria-label={'Lead of ' + team.name} value={lead} disabled={change.blocked} onChange={(event) => setLead(event.target.value)}><option value="">None named</option>{leads.map((id) => <option key={id} value={id}>{name(id)}</option>)}</select>
      : team.lead ? name(team.lead) : <span className="dim">none named</span>}</td>
    <td className="sec">{holds}</td>
    <td>{edits && (dirty || change.pending) ? <Act symbol="save" name="Save place" word="Save" disabled={change.blocked || !dirty} onClick={(event) => { event.stopPropagation(); change.submit({ operation: operationId(), parent: parent || null, lead: lead || null }); }} /> : null}<ChangeStatus change={change} /></td>
  </tr>;
}
function Create({ person, login, changed }: { person: string; login: Login; changed: (answer: TeamChanged, message: string) => void }) {
  const [name, setName] = useState(''); const [description, setDescription] = useState('');
  const change = useRoleChange<TeamChanged>('lys.pending.team-create.' + person, '/teams', (answer, body) => answer.id === body.operation && answer.owner === person && answer.name === body.name && answer.description === body.description && answer.recorded?.operation === body.operation && answer.recorded.act === 'created' && sameLogin(answer.recorded.by, login), (answer) => changed(answer, 'Your team was recorded.'));
  return <form className="card" aria-label="Create team" onSubmit={(event) => { event.preventDefault(); if (name.trim()) change.submit({ operation: operationId(), name: name.trim(), description: description.trim() }); }}><h3>Create a team</h3>
    <label className="field">Team name<input value={name} required disabled={change.blocked} onChange={(event) => setName(event.target.value)} /></label>
    <label className="field">What the team does<textarea value={description} disabled={change.blocked} onChange={(event) => setDescription(event.target.value)} /></label>
    <Act symbol="add" name="Create team" word="Create" tone="primary" type="submit" disabled={change.blocked || !name.trim()} /><ChangeStatus change={change} />
  </form>;
}
