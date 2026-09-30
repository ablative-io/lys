/** Start a stopped agent in place: it runs where it last ran, or on the one machine permitted to run it; a choice is asked for only when neither settles it. */
import { useState } from 'react';
import { Refused, operationId, request, useLoad } from '../../api';
import type { NetworkView } from '../network/contract';
import type { RuntimeSession } from '../runtime/RuntimeSessions';
import type { StartAnswer } from '../runtime/StartAgent';

export function Start({ agent, started }: { agent: string; started: () => void }) {
  const load = useLoad(async () => ({
    network: await request<NetworkView>('/network'),
    history: await request<{ sessions: RuntimeSession[] }>('/agents/' + encodeURIComponent(agent) + '/runtime/sessions'),
  }), 'team-start:' + agent);
  const [picked, setPicked] = useState('');
  const [said, setSaid] = useState('');
  const [busy, setBusy] = useState(false);
  if (load.status === 'loading') return null;
  if (load.status === 'refused') return <p className="why-not">Could not read where it may run: {load.refused.refusal.reason}</p>;
  const machines = load.data.network.machines.filter((entry) => entry.state === 'in_use' && entry.runtime !== null && entry.may_run.some((holder) => holder.id === agent));
  const last = [...load.data.history.sessions].sort((a, b) => b.last_report_at - a.last_report_at).find((entry) => machines.some((machine) => machine.id === entry.machine))?.machine;
  const machine = picked || last || (machines.length === 1 ? machines[0].id : '');
  const start = async () => {
    setBusy(true); setSaid('');
    try {
      const answer = await request<StartAnswer>('/agents/' + encodeURIComponent(agent) + '/start-command', { machine, operation: operationId() });
      if (answer.runner?.state === 'running') started(); else setSaid('Start recorded; its runner has not confirmed it running.');
    } catch (error) { setSaid(error instanceof Refused ? error.refusal.reason : String(error)); }
    setBusy(false);
  };
  return <div className="team-start">
    {!machine ? (machines.length ? <label className="field">Where it runs is not in its settings yet; choose once<select value={picked} onChange={(event) => setPicked(event.target.value)}><option value="">Machine</option>{machines.map((entry) => <option key={entry.id} value={entry.id}>{entry.name}</option>)}</select></label>
      : <p className="why-not">No machine is permitted to run this agent.</p>) : null}
    <button type="button" className="btn primary" disabled={!machine || busy} onClick={() => void start()}>Start{machine ? ' on ' + (machines.find((entry) => entry.id === machine)?.name ?? machine) : ''}</button>
    {said ? <p role="status">{said}</p> : null}
  </div>;
}
