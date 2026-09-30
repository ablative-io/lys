/** A stopped agent, started in place: every machine in use is offered, the one it last ran on first, and the service's own refusal is shown by name. */
import { useState } from 'react';
import { Refused, operationId, request, useLoad } from '../../api';
import type { NetworkView } from '../network/contract';
import type { RuntimeSession } from '../runtime/RuntimeSessions';
import type { StartAnswer } from '../runtime/StartAgent';
import { readRoles } from '../roles/AssignedRoles';

export function Start({ agent, name, started }: { agent: string; name: string; started: () => void }) {
  const load = useLoad(async () => ({
    network: await request<NetworkView>('/network'),
    history: await request<{ sessions: RuntimeSession[] }>('/agents/' + encodeURIComponent(agent) + '/runtime/sessions'),
    roles: await readRoles(),
  }), 'team-start:' + agent);
  const [picked, setPicked] = useState('');
  const [said, setSaid] = useState('');
  const [busy, setBusy] = useState(false);
  const [command, setCommand] = useState('');
  if (load.status === 'loading') return null;
  if (load.status === 'refused') return <p className="why-not">Could not read the machines: {load.refused.refusal.reason}</p>;
  const held = load.data.roles.roles.filter((role) => role.holders.some((holder) => holder.holder === agent && holder.state === 'holding')).map((role) => role.id);
  const machines = load.data.network.machines.filter((entry) => entry.state === 'in_use');
  const permits = (entry: (typeof machines)[number]) => entry.may_run.some((holder) => holder.id === agent) || (entry.may_run_roles ?? []).some((role) => held.includes(role));
  const last = [...load.data.history.sessions].sort((a, b) => b.last_report_at - a.last_report_at)[0]?.machine;
  const ordered = [...machines].sort((a, b) => Number(b.id === last) - Number(a.id === last));
  const machine = picked || last || (ordered.length === 1 ? ordered[0].id : '');
  const start = async () => {
    setBusy(true);
    setSaid('');
    try {
      const answer = await request<StartAnswer>('/agents/' + encodeURIComponent(agent) + '/start-command', { machine, operation: operationId() });
      if (answer.runner?.state === 'running') started();
      else if (answer.runner) setSaid('Its runner started it and saw it end straight away.');
      else { setCommand(answer.command); setSaid('This machine has no runner, so Lys cannot start it for you. Run this command there.'); }
    } catch (error) {
      setSaid(error instanceof Refused ? error.refusal.refusal + ': ' + error.refusal.reason : String(error));
    }
    setBusy(false);
  };
  const facts = (id: string, runtime: string | null, permitted: boolean) => [
    runtime ? 'runs ' + runtime : 'no runner named, so Lys can only hand you the command', id === last ? 'ran here last' : null, permitted ? null : 'not yet permitted to run ' + name,
  ].filter(Boolean).join(' · ');
  return <div className="team-start">
    <p className="team-start-line">{name} is not running.</p>
    {ordered.length
      ? <div className="team-machines" role="radiogroup" aria-label="Where to start it">{ordered.map((entry) => <label key={entry.id} className="team-machine">
        <input type="radio" name="machine" value={entry.id} checked={machine === entry.id} onChange={() => setPicked(entry.id)} />
        <span>{entry.name}</span>
        <span className="dim">{facts(entry.id, entry.runtime, permits(entry))}</span>
      </label>)}</div>
      : <p className="dim">No machine is named on the network yet. <a href="#/network">Name one</a>.</p>}
    <div className="team-start-acts">
      <button type="button" className="btn primary" disabled={!machine || busy} onClick={() => void start()}>Start</button>
      {said ? <span role="status" className="why-not">{said}</span> : null}
    </div>
    {command ? <textarea className="team-command" readOnly rows={3} value={command} aria-label="Command to run on that machine" onFocus={(event) => event.currentTarget.select()} /> : null}
  </div>;
}
