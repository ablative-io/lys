import { readTogether } from '../../reads';
/** The team's owner or administrator manages named membership; no inherited permissions are implied. */
import { useState } from 'react';
import { api, operationId, request, useLoad } from '../../api';
import type { Login } from '../../generated';
import { DirectoryGate as Gate } from '../people/Words';
import { useRoleChange } from '../roles/useRoleChange';
import { DirectoryChangeStatus as ChangeStatus } from '../roles/ChangeStatus';
import { entries } from '../people/directory';
import { TeamActions } from './TeamActions';
import { sameLogin } from './contract';
import type { Team, TeamChanged } from './contract';
import type { MeView, PeopleView } from '../../generated';
import { subtree, treeOrder } from '../../shell/org';
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
  const needle = query.trim().toLowerCase();
  const order = [...treeOrder(teams), ...teams.filter((team) => team.state === 'retired').map((team) => ({ team, depth: 0 }))];
  const shown = order.filter(({ team }) => !needle || team.name.toLowerCase().includes(needle));
  const team = teams.find((each) => each.id === (picked ?? shown[0]?.team.id)) ?? null;
  return <>
    <div className="tools">
      <input className="search" type="search" aria-label="Search teams" placeholder={'Search ' + teams.length.toLocaleString('en-AU') + ' teams'} value={query} onChange={(event) => setQuery(event.target.value)} />
      <button className="btn primary" onClick={() => { setCreating(true); setNotice(''); }}>+ Create a team</button>
      {notice ? <span role="status" className="note">{notice}</span> : null}
    </div>
    <div className="body work">
      <div className="pane">
        <table className="team-tree">
          <thead><tr><th>Team</th><th>Lead</th><th>Holds</th></tr></thead>
          <tbody>{shown.map(({ team: each, depth }) => <tr key={each.id} data-team={each.id} data-depth={depth} className={each.id === team?.id && !creating ? 'cursor' : ''} tabIndex={0}
            onClick={() => { setPicked(each.id); setCreating(false); }} onKeyDown={(event) => { if (event.key === 'Enter') { setPicked(each.id); setCreating(false); } }}>
            <td style={{ paddingLeft: 10 + depth * 18 }}>{each.name}{each.state === 'retired' ? <span className="note"> retired</span> : null}</td>
            <td className="sec">{each.lead ? name(each.lead) : <span className="dim">none named</span>}</td>
            <td className="sec">{size(teams, each.id, people)}</td>
          </tr>)}</tbody>
        </table>
        {teams.length ? null : <p className="dim">No teams yet. A team is how Lys groups people and their agents: create the first one.</p>}
      </div>
      <div className="detail">
        {creating ? <Create key={revision} person={me.person.id} login={me.signed_in} changed={changed} /> : team ? <section className="card" key={team.id}>
          <h2>{team.name}</h2>
          {team.description ? <p className="sec">{team.description}</p> : null}
          <p>{team.parent ? <>Part of {teams.find((each) => each.id === team.parent)?.name ?? 'a team outside your view'}. </> : null}{team.lead ? <>Led by <a href={'#/file/' + team.lead}>{name(team.lead)}</a>. </> : null}Managed by <a href={'#/file/' + team.owner}>{name(team.owner)}</a>.</p>
          <div className="section-h">Members <span>{team.members.length.toLocaleString('en-AU')}</span></div>
          {team.members.length ? <ul className="plain">{team.members.map((id) => <li key={id}><a href={'#/file/' + id}>{name(id)}</a>{team.held?.find((held) => held.member === id) ? <p className="why-not">Awaiting administrator confirmation. Team reminders and budget actions do not include this member. {team.held.find((held) => held.member === id)?.reason}</p> : null}</li>)}</ul> : <p className="dim">No members yet.</p>}
          {team.owner === me.person.id || people.scope === 'directory' ? <TeamActions key={team.id + ':' + revision} team={team} person={me.person.id} login={me.signed_in} members={members} administrator={people.scope === 'directory'} changed={changed} /> : null}
        </section> : null}
      </div>
    </div>
  </>;
}
function Create({ person, login, changed }: { person: string; login: Login; changed: (answer: TeamChanged, message: string) => void }) {
  const [name, setName] = useState(''); const [description, setDescription] = useState('');
  const change = useRoleChange<TeamChanged>('lys.pending.team-create.' + person, '/teams', (answer, body) => answer.id === body.operation && answer.owner === person && answer.name === body.name && answer.description === body.description && answer.recorded?.operation === body.operation && answer.recorded.act === 'created' && sameLogin(answer.recorded.by, login), (answer) => changed(answer, 'Your team was recorded.'));
  return <form className="card" aria-label="Create team" onSubmit={(event) => { event.preventDefault(); if (name.trim()) change.submit({ operation: operationId(), name: name.trim(), description: description.trim() }); }}><h3>Create a team</h3>
    <label className="field">Team name<input value={name} required maxLength={100} disabled={change.blocked} onChange={(event) => setName(event.target.value)} /></label>
    <label className="field">What the team does<textarea value={description} maxLength={500} disabled={change.blocked} onChange={(event) => setDescription(event.target.value)} /></label>
    <button className="btn primary" type="submit" disabled={change.blocked || !name.trim()}>Create team</button><ChangeStatus change={change} />
  </form>;
}
