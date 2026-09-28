/** The team's owner or administrator manages named membership; no inherited permissions are implied. */
import { useState } from 'react';
import { api, operationId, request, useLoad } from '../../api';
import type { Login } from '../../generated';
import { Gate } from '../signin/Gate';
import { useRoleChange } from '../roles/useRoleChange';
import { ChangeStatus } from '../roles/ChangeStatus';
import { entries } from '../people/directory';
import { TeamActions } from './TeamActions';
import { sameLogin } from './contract';
import type { Team, TeamChanged } from './contract';
export function Teams() {
  const [revision, setRevision] = useState(0); const [notice, setNotice] = useState('');
  const changed = (message: string) => { setNotice(message); setRevision((value) => value + 1); };
  const load = useLoad(async () => ({ teams: await request<{ teams: Team[] }>('/teams'), me: await api.me(), people: await api.people() }), 'teams:' + revision);
  return <section><div className="head"><div><h2>Teams</h2><p>Group people and agents who work together. Membership does not grant access.</p></div><button className="btn" onClick={() => setRevision((value) => value + 1)}>Refresh teams</button></div>{notice ? <p role="status">{notice}</p> : null}
    <Gate load={load} title="Teams" ok={({ teams, me, people }) => <>
      {teams.teams.length ? teams.teams.map((team) => <section className="card" key={team.id}><h3>{team.name} <span className="note">{team.state}</span></h3><p>{team.description}</p><p>Owner: <a href={'#/file/' + team.owner}>{entries(people).find((entry) => entry.id === team.owner)?.display_name ?? team.owner}</a></p>
        {team.members.length ? <ul>{team.members.map((id) => <li key={id}><a href={'#/file/' + id}>{entries(people).find((entry) => entry.id === id)?.display_name ?? id}</a></li>)}</ul> : <p>No members yet.</p>}
        {team.owner === me.person.id || people.scope === 'directory' ? <TeamActions key={team.id + ':' + revision} team={team} person={me.person.id} login={me.signed_in} members={entries(people)} changed={changed} /> : null}
      </section>) : <p>No teams have been recorded.</p>}
      <Create key={revision} person={me.person.id} login={me.signed_in} changed={changed} />
    </>} />
  </section>;
}
function Create({ person, login, changed }: { person: string; login: Login; changed: (message: string) => void }) {
  const [name, setName] = useState(''); const [description, setDescription] = useState('');
  const change = useRoleChange<TeamChanged>('lys.pending.team-create.' + person, '/teams', (answer, body) => answer.id === body.operation && answer.owner === person && answer.name === body.name && answer.description === body.description && answer.recorded?.operation === body.operation && answer.recorded.act === 'created' && sameLogin(answer.recorded.by, login), () => changed('Your team was recorded.'));
  return <form className="card" aria-label="Create team" onSubmit={(event) => { event.preventDefault(); if (name.trim()) change.submit({ operation: operationId(), name: name.trim(), description: description.trim() }); }}><h3>Create a team</h3>
    <label className="field">Team name<input value={name} required maxLength={100} disabled={change.blocked} onChange={(event) => setName(event.target.value)} /></label>
    <label className="field">What the team does<textarea value={description} maxLength={500} disabled={change.blocked} onChange={(event) => setDescription(event.target.value)} /></label>
    <button className="btn primary" type="submit" disabled={change.blocked || !name.trim()}>Create team</button><ChangeStatus change={change} />
  </form>;
}
