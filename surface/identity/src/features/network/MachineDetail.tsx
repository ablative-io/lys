/** One computer, in the half of the page beside the list: its team, what runs there, who may start there, how Lys reaches it, and retiring it. */
import { useEffect, useRef, useState } from 'react';
import { Refused, api, operationId, request, useLoad } from '../../api';
import type { OrgTeam } from '../../shell/org';
import { useRoleChange } from '../roles/useRoleChange';
import { ChangeStatus } from '../roles/ChangeStatus';
import { ago } from '../file/time';
import { counted } from '../../shell/count';
import type { Computer } from './Network';
import type { Machine } from './contract';
import { ConnectComputer } from './JoinCode';
import type { Connecting } from './JoinCode';
import { Act } from '../../shell/Act';
import { MachineHolders } from './MachineIdentities';
import type { MachineIdentity } from './contract';
import { ErrorWords, problemWords } from '../people/Words';
import { roleWords } from './allowed';
import { retryWords, useConnectionClock } from './reconnect-clock';
import type { Retry } from './reconnect-clock';
import { subscribeChanges } from '../../live';

export type RunnerRecord = { kind: 'lys' } | { kind: 'socket'; path: string } | { kind: 'dialled'; key: string; runner?: string };

/** Whether the computer is up, by its runner's own answer when Lys asked it just now; never guessed from a report's age. */
export function status({ machine, asked, runner, answers, joining, running }: Computer): { words: string; state: 'up' | 'down' | 'unknown' | 'off' } {
  if (machine.state === 'retired') return { words: 'Retired', state: 'off' };
  if (machine.runtime === null) return { words: 'Does not run agents', state: 'off' };
  if (running?.length) return { words: 'Up, ' + counted(running.length, 'agents') + ' running', state: 'up' };
  // Until its runner is asked, nothing is said of it: it counts neither as up nor as not answering.
  if (asked === 'asking') return { words: 'Asking its runner…', state: 'unknown' };
  if (asked === 'unread') return { words: 'Lys could not ask its runner', state: 'unknown' };
  if (answers === true) return { words: 'Up', state: 'up' };
  if (waiting({ runner, joining })) return { words: 'Waiting for its runner to connect', state: 'unknown' };
  if (!runner) return { words: 'No runner connected', state: 'down' };
  return { words: 'Runner not answering' + (machine.last_report_at === null ? '' : ', last heard ' + ago(machine.last_report_at)), state: 'down' };
}

/** Whether a computer's runner has yet to connect: its installer or terminal code waits, or it joined and has never connected since. */
export const waiting = ({ runner, joining }: { runner: RunnerRecord | null; joining: boolean }): boolean =>
  joining || (runner?.kind === 'dialled' && !runner.runner);

/** How Lys reaches it. */
function reached(machine: Machine, runner: RunnerRecord | null, joining: boolean): string {
  if (machine.runtime === null) return 'Lys does not start agents here.';
  if (runner?.kind === 'lys') return 'This is the computer Lys runs on; Lys starts agents through its own runner.';
  if (joining) return 'Open its installer on that computer to connect it. If you chose the terminal steps, run its command instead.';
  if (runner?.kind === 'dialled') return 'Its runner connects to Lys from that computer.';
  if (runner?.kind === 'socket') return 'Lys starts agents through the runner at ' + runner.path + '.';
  return 'No runner is connected, so Lys cannot start agents here.';
}

/** Another computer's runner can be connected from the panel: one that runs agents and is not the computer Lys runs on, nor a runner at a socket here. */
const connectable = ({ machine, asked, runner }: Computer): boolean =>
  machine.state === 'in_use' && machine.runtime !== null && asked === 'answered' && (runner === null || runner.kind === 'dialled');

export function MachineDetail({ computer, admin, me, teams, names, changed, machines = null, connecting, connect, done, roleNames = new Map() }: { computer: Computer; admin: boolean; me: string; teams: OrgTeam[]; names: Map<string, string>; changed: (message: string) => void; roleNames?: Map<string, string>;
  machines?: MachineIdentity[] | null; connecting: Connecting | null; connect: (next: Connecting) => void; done: () => void }) {
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
      {[...new Map(machine.may_run.map(agent => [agent.id, agent])).values()].map((agent) => <tr key={agent.id}><td><a title={agent.id} href={'#/file/' + agent.id}>{agent.display_name}</a></td><td>Named on this computer</td></tr>)}
      {(machine.may_run_roles ?? []).map((role) => <tr key={role}><td>Anyone holding {roleWords(role, roleNames)}</td><td>Their role</td></tr>)}
      {!machine.may_run.length && !(machine.may_run_roles ?? []).length ? <tr><td colSpan={2} className="dim">No agent may start here yet.</td></tr> : null}
    </tbody></table>}
    {machine.named_by === me && machine.runtime && machine.state !== 'retired' ? <OwnAgents machine={machine} me={me} changed={changed} /> : null}
    <div className="section-h">How Lys reaches it</div>
    {(admin || machine.named_by === me) && machine.runtime && runner && machine.state !== 'retired' ? <RunnerConnection machine={machine.id} name={machine.name} /> : null}
    {computer.asked === 'answered' || machine.runtime === null ? <p>{reached(machine, runner, computer.joining)}</p> : computer.asked === 'asking' ? <p className="dim">Asking its runner…</p> : null}
    {computer.answers === false && computer.reason && !waiting(computer) ? <p className="why-not" title={(computer.refusal ? computer.refusal + ': ' : '') + computer.reason}>Its runner did not answer. {computer.refusal ? problemWords(new Refused(0, { refusal: computer.refusal, reason: computer.reason })) : computer.reason}</p> : null}
    {computer.asked === 'unread' ? <><p className="why-not">Lys could not ask its runner whether this computer is up.</p><ErrorWords problem={computer.problem ?? computer.reason} /></> : null}
    {(admin || machine.named_by === me) && connectable(computer) ? <ConnectComputer machine={machine.id} name={machine.name} connecting={connecting?.machine === machine.id && connecting.at === 'panel' ? connecting : null} connect={connect} done={done} /> : null}
    {machines ? <><div className="section-h">As a machine</div><MachineHolders machine={machine.id} name={machine.name} identities={machines} me={me} changed={changed} /></> : null}
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
    <span><Act symbol="save" name="Save team" word="Save" type="submit" disabled={change.blocked || team === (machine.team ?? '')} /></span>
    <ChangeStatus change={change} />
  </form>;
}

function Retire({ machine, changed }: { machine: Machine; changed: (message: string) => void }) {
  const [confirm, setConfirm] = useState(false);
  const [busy, setBusy] = useState(false);
  const [failure, setFailure] = useState<unknown>(null);
  const retire = async () => {
    if (busy) return; setBusy(true); setFailure(null);
    try {
      const answer = await request<Machine>('/network/machines/' + encodeURIComponent(machine.id) + '/retire', {});
      if (answer.id !== machine.id || answer.state !== 'retired') throw new Error('Retirement is not confirmed. Reload this page to see whether it was retired.');
      changed(machine.name + ' was retired.');
    } catch (error) { setFailure(error); }
    finally { setBusy(false); }
  };
  return <div className="machine-retire" style={{ marginTop: 20 }}>{confirm ? <><p>Retire {machine.name}? No agent can be started on it afterwards. Agents already running on it keep running.</p><Act symbol="approve" name="Confirm retirement" word="Confirm" tone="danger" disabled={busy} onClick={() => void retire()} />{' '}<Act symbol="close" name="Cancel" word="Cancel" disabled={busy} onClick={() => setConfirm(false)} /></>
    : <Act symbol="retire" name="Retire this computer" word="Retire" onClick={() => setConfirm(true)} />}{failure ? <ErrorWords problem={failure} /> : null}</div>;
}

/** The owner allows their own active agents; role allowances remain an administrator's act. */
function OwnAgents({ machine, me, changed }: { machine: Machine; me: string; changed: (message: string) => void }) {
  const load = useLoad(async () => {
    const person = (await api.people()).people.find((person) => person.id === me);
    if (!person) throw new Refused(403, { refusal: 'NoPerson', reason: 'Your person was not returned with your agents.' });
    return person.agents;
  }, 'computer-own-agents:' + me);
  const [chosen, setChosen] = useState('');
  const agents = load.status === 'ok' ? load.data.filter((agent) => agent.state === 'active') : [];
  const agent = agents.find((agent) => agent.id === chosen) ?? agents[0];
  const allowed = machine.may_run.some((each) => each.id === agent?.id);
  const change = useRoleChange<{ machine: Machine; recorded: { operation: string; agent: string; allow: boolean } }>(
    'lys.pending.computer-agent.' + me + '.' + machine.id, '/network/machines/' + encodeURIComponent(machine.id) + '/agents',
    (answer, body) => answer.machine?.id === machine.id && answer.recorded?.operation === body.operation
      && answer.recorded?.agent === body.agent && answer.recorded?.allow === body.allow
      && answer.machine.may_run.some((each) => each.id === body.agent) === body.allow,
    () => changed(machine.name + ' agent allowance was saved.'));
  if (load.status === 'loading') return <p role="status">Reading your agents…</p>;
  if (load.status === 'refused') return <><p className="why-not">Your agents could not be read, so they cannot be allowed here now.</p><ErrorWords problem={load.refused} /></>;
  if (!agent) return <p className="dim">You have no active agents to allow here.</p>;
  return <form className="usage-add-row own-agents" aria-label="Your agents on this computer" style={{ gridTemplateColumns: 'minmax(0, 1fr) auto' }} onSubmit={(event) => {
    event.preventDefault();
    if (!change.blocked) change.submit({ operation: operationId(), agent: agent.id, allow: !allowed });
  }}>
    <label className="field own-agents-pick">Your agent<select aria-label="Agent to allow on this computer" value={agent.id} disabled={change.blocked} onChange={(event) => setChosen(event.target.value)}>
      {agents.map((each) => <option key={each.id} value={each.id}>{each.display_name}{machine.may_run.some((held) => held.id === each.id) ? ' (may start here)' : ''}</option>)}
    </select></label>
    <span className="own-agents-act"><Act symbol={allowed ? 'remove' : 'add'} name={(allowed ? 'Remove ' : 'Allow ') + agent.display_name} word={allowed ? 'Stop allowing here' : 'Allow here'} type="submit" disabled={change.blocked} /></span>
    <ChangeStatus change={change} />
  </form>;
}


/** The explicit reconnect act interrupts a failed connection's wait. */
function RunnerConnection({ machine, name }: { machine: string; name: string }) {
  const [words, setWords] = useState('Reading runner connection…');
  const [retry, setRetry] = useState<Retry | null>(null);
  const host = useRef<HTMLDivElement>(null);
  const now = useConnectionClock(retry !== null, host);
  const [failure, setFailure] = useState<unknown>(null);
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    const leave = new AbortController();
    const load = () => {
      void request<{ state: string; retry?: Retry | null }>('/network/machines/' + encodeURIComponent(machine) + '/connection', undefined, 'POST', leave.signal)
        .then((answer) => { if (!leave.signal.aborted) { setRetry(answer.retry ?? null); setWords('Runner connection: ' + answer.state); } },
          (error: unknown) => { if (!leave.signal.aborted) setFailure(error); });
    };
    load();
    const unsubscribe = subscribeChanges(load, (error) => { if (!leave.signal.aborted) setFailure(error); });
    return () => { unsubscribe(); leave.abort(); };
  }, [machine]);
  const reconnect = async () => {
    setBusy(true); setFailure(null);
    try {
      const answer = await request<{ reconnecting: boolean }>('/network/machines/' + encodeURIComponent(machine) + '/reconnect', {});
      if (answer.reconnecting !== true) throw new Error('runner_reconnect_unconfirmed: Lys did not confirm the reconnect signal.');
      setRetry(null); setWords('Reconnecting…');
    } catch (error) { setFailure(error); }
    finally { setBusy(false); }
  };
  return <div ref={host}><p role="status">{retry ? retryWords(retry, now) : words}</p>{retry?.reason ? <p>{retry.reason}</p> : null}<Act symbol="retry" name={'Reconnect ' + name} word="Reconnect" disabled={busy} onClick={() => void reconnect()} />
    {failure ? <ErrorWords problem={failure} /> : null}</div>;
}
