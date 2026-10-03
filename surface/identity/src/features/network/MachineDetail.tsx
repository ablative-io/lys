/** One computer, in the half of the page beside the list: its team, what runs there, who may start there, how Lys reaches it, and retiring it. */
import { useState } from 'react';
import { Refused, operationId, request } from '../../api';
import type { OrgTeam } from '../../shell/org';
import { useRoleChange } from '../roles/useRoleChange';
import { ChangeStatus } from '../roles/ChangeStatus';
import { ago } from '../file/time';
import type { Computer } from './Network';
import type { Machine } from './contract';

export type RunnerRecord = { kind: 'lys' } | { kind: 'socket'; path: string } | { kind: 'dialled'; key: string; runner?: string };

/** Whether the computer is up, from what its runner last reported. */
export function status({ machine, runner, reports, running }: Computer): { words: string; state: 'up' | 'down' | 'unknown' | 'off' } {
  if (machine.state === 'retired') return { words: 'Retired', state: 'off' };
  if (machine.runtime === null) return { words: 'Does not run agents', state: 'off' };
  // An agent running on it now is the plainest sign it is up, whatever the last report's age.
  if (running?.length) return { words: 'Up, ' + running.length + (running.length === 1 ? ' agent running' : ' agents running'), state: 'up' };
  if (!runner) return { words: 'No runner connected', state: 'down' };
  if (!reports) return { words: 'Lys does not collect runner reports', state: 'unknown' };
  if (machine.last_report_at === null) return { words: 'Never heard from', state: 'down' };
  const fresh = Math.floor(Date.now() / 1000) - machine.last_report_at < 300;
  return { words: (fresh ? 'Up, heard ' : 'Last heard ') + ago(machine.last_report_at), state: fresh ? 'up' : 'down' };
}

/** How Lys reaches it. */
function reached(machine: Machine, runner: RunnerRecord | null): string {
  if (machine.runtime === null) return 'Lys does not start agents here.';
  if (runner?.kind === 'lys') return 'This is the computer Lys runs on; Lys starts agents through its own runner.';
  if (runner?.kind === 'dialled') return 'Its runner connects to Lys with key ' + runner.key.slice(0, 8) + '….';
  if (runner?.kind === 'socket') return 'Lys starts agents through the runner at ' + runner.path + '.';
  return 'No runner is connected, so Lys cannot start agents here.';
}

export function MachineDetail({ computer, admin, me, teams, names, changed }: { computer: Computer; admin: boolean; me: string; teams: OrgTeam[]; names: Map<string, string>; changed: (message: string) => void }) {
  const { machine, runner, running } = computer;
  const now = status(computer);
  return <section className="card" aria-label={machine.name}>
    <div className="row" style={{ padding: '0 0 8px' }}><h2>{machine.name}</h2><span className="sec">{now.words}</span></div>
    <div className="section-h">Team</div>
    <OwningTeam machine={machine} admin={admin} me={me} teams={teams} changed={changed} />
    <div className="section-h">Running now</div>
    {running === null ? <p className="dim">Lys cannot read what is running.</p> : <table className="usage-table" aria-label="Running now"><thead><tr><th>Agent</th><th>Watch</th></tr></thead><tbody>
      {running.map((session) => <tr key={session.session}><td>{session.agent ? <a href={'#/file/' + session.agent}>{names.get(session.agent) ?? 'an agent outside your view'}</a> : 'a session with no agent'}</td><td><a href={'#/canvas/' + encodeURIComponent(session.session)}>Watch</a></td></tr>)}
      {running.length ? null : <tr><td colSpan={2} className="dim">Nothing.</td></tr>}
    </tbody></table>}
    <div className="section-h">May start here</div>
    {machine.runtime === null ? <p>Lys does not start agents here.</p> : <table className="usage-table" aria-label="May start here"><thead><tr><th>Who</th><th>Through</th></tr></thead><tbody>
      {machine.may_run.map((agent) => <tr key={agent.id}><td><a href={'#/file/' + agent.id}>{agent.display_name}</a></td><td>Named on this computer</td></tr>)}
      {(machine.may_run_roles ?? []).map((role) => <tr key={role}><td>Anyone holding {role}</td><td>Role</td></tr>)}
      {!machine.may_run.length && !(machine.may_run_roles ?? []).length ? <tr><td colSpan={2} className="dim">No agent may start here yet.</td></tr> : null}
    </tbody></table>}
    <div className="section-h">How Lys reaches it</div>
    <p>{reached(machine, runner)}</p>
    <p className="sec">Websites its agents' services may connect to: {machine.may_reach.join(', ') || 'none'}.</p>
    {admin && machine.state !== 'retired' ? <Retire machine={machine} changed={changed} /> : null}
  </section>;
}

type TeamChanged = { machine: Machine; recorded: { operation: string } };

/** The team that owns a computer. The administrator sets or clears it; a team's owner may claim an unowned computer for their own team. */
function OwningTeam({ machine, admin, me, teams, changed }: { machine: Machine; admin: boolean; me: string; teams: OrgTeam[]; changed: (message: string) => void }) {
  const [team, setTeam] = useState(machine.team ?? '');
  const active = teams.filter((each) => each.state === 'active' || each.id === machine.team);
  const offered = admin ? active : machine.team ? [] : active.filter((each) => each.owner === me);
  const change = useRoleChange<TeamChanged>('lys.pending.computer-team.' + me + '.' + machine.id, '/network/machines/' + encodeURIComponent(machine.id) + '/team',
    (answer, body) => answer.machine?.id === machine.id && (answer.machine.team ?? null) === body.team && answer.recorded?.operation === body.operation,
    (answer) => changed(answer.machine.team ? machine.name + ' now belongs to ' + (teams.find((each) => each.id === answer.machine.team)?.name ?? 'a team outside your view') + '.' : machine.name + ' now belongs to no team.'));
  const current = machine.team ? teams.find((each) => each.id === machine.team)?.name ?? 'a team outside your view' : 'No team';
  if (machine.state === 'retired' || (!offered.length && !change.pending)) return <p>{current}</p>;
  return <form className="usage-add-row" aria-label={'Team of ' + machine.name} style={{ gridTemplateColumns: 'minmax(0, 1fr) auto' }} onSubmit={(event) => { event.preventDefault(); if (!change.blocked && team !== (machine.team ?? '')) change.submit({ operation: operationId(), team: team || null }); }}>
    <select name="team" aria-label="Team that owns this computer" value={team} disabled={change.blocked} onChange={(event) => setTeam(event.target.value)}>
      {admin || !machine.team ? <option value="">No team</option> : null}
      {offered.map((each) => <option key={each.id} value={each.id}>{each.name}</option>)}
    </select>
    <span><button className="btn" type="submit" disabled={change.blocked || team === (machine.team ?? '')}>Save team</button></span>
    <ChangeStatus change={change} />
  </form>;
}

function Retire({ machine, changed }: { machine: Machine; changed: (message: string) => void }) {
  const [confirm, setConfirm] = useState(false);
  const [busy, setBusy] = useState(false);
  const [failure, setFailure] = useState('');
  const retire = async () => {
    if (busy) return; setBusy(true); setFailure('');
    try {
      const answer = await request<Machine>('/network/machines/' + encodeURIComponent(machine.id) + '/retire', {});
      if (answer.id !== machine.id || answer.state !== 'retired') throw new Error('Retirement is not confirmed. Reload this page to see whether it was retired.');
      changed(machine.name + ' was retired.');
    } catch (error) { setFailure(error instanceof Refused ? error.refusal.refusal + ': ' + error.message : String(error)); }
    finally { setBusy(false); }
  };
  return <div className="section-h" style={{ display: 'block' }}>{confirm ? <><p>Retire {machine.name}? No agent can be started on it afterwards. Agents already running on it keep running.</p><button className="btn danger" disabled={busy} onClick={() => void retire()}>Confirm retirement</button>{' '}<button className="btn" disabled={busy} onClick={() => setConfirm(false)}>Cancel</button></>
    : <button className="btn" onClick={() => setConfirm(true)}>Retire this computer</button>}{failure ? <p role="alert">{failure}</p> : null}</div>;
}
