/** One computer, opened beside the list: what runs there, who may start there, how Lys reaches it, and retiring it. */
import { useState } from 'react';
import { Refused, request } from '../../api';
import type { Computer } from './Network';
import type { Machine } from './contract';

export type RunnerRecord = { kind: 'lys' } | { kind: 'socket'; path: string } | { kind: 'dialled'; key: string; runner?: string };

/** How long ago, as a person says it. */
function ago(seconds: number): string {
  const gone = Math.max(0, Math.floor(Date.now() / 1000) - seconds);
  if (gone < 60) return 'just now';
  if (gone < 3600) return Math.floor(gone / 60) + ' min ago';
  if (gone < 86400) return Math.floor(gone / 3600) + ' h ago';
  return Math.floor(gone / 86400) + ' d ago';
}

/** Whether the computer is up, from what its runner last reported. */
export function status({ machine, runner, reports }: Computer): { words: string; state: 'up' | 'down' | 'unknown' | 'off' } {
  if (machine.state === 'retired') return { words: 'Retired', state: 'off' };
  if (machine.runtime === null) return { words: 'Does not run agents', state: 'off' };
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

export function MachineDetail({ computer, admin, names, changed }: { computer: Computer; admin: boolean; names: Map<string, string>; changed: (message: string) => void }) {
  const { machine, runner, running } = computer;
  const now = status(computer);
  return <section className="card" aria-label={machine.name}>
    <div className="row" style={{ padding: '0 0 8px' }}><h2>{machine.name}</h2><span className="sec">{now.words}</span></div>
    <div className="section-h">Running now</div>
    {running === null ? <p className="dim">Lys cannot read what is running.</p> : running.length
      ? <ul className="plain">{running.map((session) => <li key={session.session}>{session.agent ? <a href={'#/file/' + session.agent}>{names.get(session.agent) ?? 'an agent outside your view'}</a> : 'a session with no agent'}</li>)}</ul>
      : <p className="dim">Nothing.</p>}
    <div className="section-h">May start here</div>
    {machine.runtime === null ? <p>Lys does not start agents here.</p> : <>
      {machine.may_run.length ? <ul className="plain">{machine.may_run.map((agent) => <li key={agent.id}><a href={'#/file/' + agent.id}>{agent.display_name}</a></li>)}</ul> : null}
      {(machine.may_run_roles ?? []).map((role) => <p key={role}>Anyone holding {role}</p>)}
      {!machine.may_run.length && !(machine.may_run_roles ?? []).length ? <p className="dim">No agent may start here yet.</p> : null}
    </>}
    <div className="section-h">How Lys reaches it</div>
    <p>{reached(machine, runner)}</p>
    <p className="sec">Websites its agents' services may connect to: {machine.may_reach.join(', ') || 'none'}.</p>
    {admin && machine.state !== 'retired' ? <Retire machine={machine} changed={changed} /> : null}
  </section>;
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
