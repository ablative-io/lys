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
export function Teams() {
  const load = useLoad(async () => ({ teams: await request<{ teams: Team[] }>('/teams'), me: await api.me(), people: await api.people() }), 'teams');
  return <Gate load={load} title="Teams" ok={({ teams, me, people }) => <TeamList initial={teams.teams} me={me} people={people} />} />;
}
function TeamList({ initial, me, people }: { initial: Team[]; me: MeView; people: PeopleView }) {
  const [teams, setTeams] = useState(initial);
  const [revision, setRevision] = useState(0);
  const [notice, setNotice] = useState('');
  const changed = (answer: TeamChanged, message: string) => {
    setTeams((values) => values.some((team) => team.id === answer.id) ? values.map((team) => team.id === answer.id ? answer : team) : [...values, answer]);
    setNotice(message); setRevision((value) => value + 1);
  };
  const members = entries(people);
  const name = (id: string) => members.find((entry) => entry.id === id)?.display_name ?? 'Name unavailable';
  return <section><p>Make a team, add or remove its members, or retire it. Membership does not grant access.</p>{notice ? <p role="status">{notice}</p> : null}
      {teams.length ? teams.map((team) => <section className="card" key={team.id}><h3>{team.name} <span className="note">{team.state === 'active' ? 'Active team' : 'Retired team'}</span></h3><p>{team.description}</p><p>Managed by: <a href={'#/file/' + team.owner}>{name(team.owner)}</a></p>
        {team.members.length ? <ul>{team.members.map((id) => <li key={id}><a href={'#/file/' + id}>{name(id)}</a>{team.held?.find((held) => held.member === id) ? <p className="why-not">Awaiting administrator confirmation. Team reminders and budget actions do not include this member. {team.held.find((held) => held.member === id)?.reason}</p> : null}</li>)}</ul> : <p>No members yet.</p>}
        <details><summary>Team details</summary><p>Team identifier: {team.id}</p><p>Owner identifier: {team.owner}</p><p>Member identifiers: {team.members.join(', ') || 'None'}</p></details>
        {team.owner === me.person.id || people.scope === 'directory' ? <TeamActions key={team.id + ':' + revision} team={team} person={me.person.id} login={me.signed_in} members={members} administrator={people.scope === 'directory'} changed={changed} /> : null}
      </section>) : <p>No teams have been recorded.</p>}
      <Create key={revision} person={me.person.id} login={me.signed_in} changed={changed} />
  </section>;
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
