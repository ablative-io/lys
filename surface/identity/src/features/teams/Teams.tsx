/** Teams are named groups of people and agents. Being in one confers no access and answers for nothing. */
import { useState } from 'react';
import type { ReactNode } from 'react';
import { api, operationId, request, useLoad } from '../../api';
import { Gate } from '../signin/Gate';
import { ChangeStatus } from '../roles/ChangeStatus';
import { useRoleChange } from '../roles/useRoleChange';
import { clock } from '../file/time';

/** One team as `GET /teams` answers it. */
export interface TeamView {
  id: string;
  owner: string;
  name: string;
  description: string;
  members: string[];
  state: 'active' | 'retired';
  created_at: number;
  retired_at: number | null;
}
interface TeamsView { teams: TeamView[] }

/** The teams screen; `head` is shown above it when it opens as the directory's Teams filter. */
export function Teams({ head }: { head?: ReactNode }) {
  const [revision, setRevision] = useState(0);
  const refresh = () => setRevision((value) => value + 1);
  const load = useLoad(async () => ({ me: await api.me(), teams: await request<TeamsView>('/teams') }), 'teams:' + revision);
  return <div className="page">{head}<div className="head"><div><div className="eyebrow">Directory</div><h1>Teams</h1><p>A team is a named group of people and agents. Being in a team gives no access and does not make anyone answerable for an agent.</p></div><button className="btn" onClick={refresh}>Refresh teams</button></div>
    <Gate load={load} title="Teams" ok={({ me, teams }) => <>
      {teams.teams.length ? teams.teams.map((team) => <Team key={team.id + ':' + revision} team={team} person={me.person.id} changed={refresh} />) : <p>No teams yet.</p>}
      <Create key={me.person.id + ':' + revision} person={me.person.id} changed={refresh} />
    </>} />
  </div>;
}

function Team({ team, person, changed }: { team: TeamView; person: string; changed: () => void }) {
  const active = team.state === 'active';
  return <section className="card" aria-label={'Team ' + team.name}><h2>{team.name}</h2>{team.description ? <p>{team.description}</p> : null}
    <dl className="facts"><dt>State</dt><dd>{active ? 'Active' : 'Retired'}</dd><dt>Owner</dt><dd><a href={'#/file/' + team.owner}>{team.owner}</a></dd><dt>Created</dt><dd>{clock(team.created_at)}</dd>{team.retired_at !== null ? <><dt>Retired</dt><dd>{clock(team.retired_at)}</dd></> : null}</dl>
    <h3>Members</h3>
    {team.members.length ? <ul>{team.members.map((member) => <li key={member}><a href={'#/file/' + member}>{member}</a>{active ? <Remove team={team} member={member} person={person} changed={changed} /> : null}</li>)}</ul> : <p>No members.</p>}
    {active ? <><Add team={team} person={person} changed={changed} /><Retire team={team} person={person} changed={changed} /></> : null}
  </section>;
}

function Create({ person, changed }: { person: string; changed: () => void }) {
  const [name, setName] = useState(''); const [description, setDescription] = useState('');
  const change = useRoleChange<TeamView>('lys.pending.team-create.' + person, '/teams', (answer, body) => answer.id === body.operation && answer.owner === person && answer.name === body.name, changed);
  return <form className="card" aria-label="Create team" onSubmit={(event) => { event.preventDefault(); if (name.trim()) change.submit({ operation: operationId(), name: name.trim(), description: description.trim() }); }}><h2>Create a team you own</h2>
    <label className="field">Team name<input required maxLength={100} value={name} disabled={change.blocked} onChange={(event) => setName(event.target.value)} placeholder="For example, identity screens" /></label>
    <label className="field">What it is for<input maxLength={500} value={description} disabled={change.blocked} onChange={(event) => setDescription(event.target.value)} /></label>
    <button className="btn primary" disabled={change.blocked || !name.trim()} type="submit">Create team</button><ChangeStatus change={change} />
  </form>;
}

function Add({ team, person, changed }: { team: TeamView; person: string; changed: () => void }) {
  const [member, setMember] = useState('');
  const change = useRoleChange<TeamView>('lys.pending.team-add.' + person + '.' + team.id, '/teams/' + encodeURIComponent(team.id) + '/members', (answer, body) => answer.id === team.id && answer.members.includes(String(body.member)), changed);
  return <form aria-label={'Add a member to ' + team.name} onSubmit={(event) => { event.preventDefault(); if (member.trim()) change.submit({ operation: operationId(), member: member.trim() }); }}>
    <label className="field">Person or agent id<input value={member} disabled={change.blocked} onChange={(event) => setMember(event.target.value)} placeholder="person-… or agent-…" /></label>
    <button className="btn" disabled={change.blocked || !member.trim()} type="submit">Add member</button><ChangeStatus change={change} />
  </form>;
}

function Remove({ team, member, person, changed }: { team: TeamView; member: string; person: string; changed: () => void }) {
  const change = useRoleChange<TeamView>('lys.pending.team-remove.' + person + '.' + team.id + '.' + member, '/teams/' + encodeURIComponent(team.id) + '/members/' + encodeURIComponent(member) + '/remove', (answer) => answer.id === team.id && !answer.members.includes(member), changed);
  return <> <button className="btn" disabled={change.blocked} onClick={() => change.submit({ operation: operationId() })}>Remove</button><ChangeStatus change={change} /></>;
}

function Retire({ team, person, changed }: { team: TeamView; person: string; changed: () => void }) {
  const [confirm, setConfirm] = useState(false);
  const change = useRoleChange<TeamView>('lys.pending.team-retire.' + person + '.' + team.id, '/teams/' + encodeURIComponent(team.id) + '/retire', (answer) => answer.id === team.id && answer.state === 'retired', changed);
  return <>{!confirm ? <button className="btn danger" disabled={change.blocked} onClick={() => setConfirm(true)}>Retire team</button> : null}
    {confirm ? <section aria-label="Confirm team retirement"><p>Retire {team.name}? Its members stay as they are in the directory; the team takes no further changes.</p><button className="btn danger" disabled={change.blocked} onClick={() => change.submit({ operation: operationId() })}>Confirm retirement</button><button className="btn" disabled={change.busy} onClick={() => setConfirm(false)}>Cancel</button></section> : null}<ChangeStatus change={change} />
  </>;
}
