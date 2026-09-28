/** The command that starts an agent on a machine. The service checks and records the start; it never runs the command. */
import { useState } from 'react';
import { operationId, Refused, request, useLoad } from '../../api';
import type { NetworkView } from '../network/contract';

/** The answer of `POST /agents/{id}/start-command`. */
export interface StartCommand {
  agent: string; machine: string; runtime: string; session: string; provisioning_version: number; harness: string;
  handles: { id: string; secret: string; env: string }[]; template: string; template_sha256: string; command: string; left_out: string[]; executed: boolean;
}
interface Asked { machine: string; operation: string }

export function StartAgent({ id }: { id: string }) {
  const key = 'lys.pending.start.' + id;
  const network = useLoad(() => request<NetworkView>('/network'), 'start-machines:' + id);
  const [asked, setAsked] = useState<Asked | null>(() => { const raw = sessionStorage.getItem(key); return raw === null ? null : JSON.parse(raw) as Asked; });
  const [machine, setMachine] = useState(asked?.machine ?? '');
  const [answer, setAnswer] = useState<StartCommand | null>(null);
  const [failure, setFailure] = useState('');
  const [busy, setBusy] = useState(false);
  const send = async (body: Asked) => {
    if (busy) return; setBusy(true); setFailure('');
    sessionStorage.setItem(key, JSON.stringify(body)); setAsked(body);
    try {
      const started = await request<StartCommand>('/agents/' + encodeURIComponent(id) + '/start-command', body);
      if (started.agent !== id || started.session !== body.operation || started.machine !== body.machine || started.executed) throw new Error('The answer did not confirm this start. Its original request is retained.');
      sessionStorage.removeItem(key); setAsked(null); setAnswer(started);
    } catch (error) {
      if (error instanceof Refused && error.status >= 400 && error.status < 500) { sessionStorage.removeItem(key); setAsked(null); }
      setFailure(error instanceof Refused ? error.refusal.refusal + ': ' + error.message : String(error));
    } finally { setBusy(false); }
  };
  const machines = network.status === 'ok' ? network.data.machines.filter((entry) => entry.state === 'in_use' && entry.runtime !== null) : [];
  return <section className="card" aria-label="Start this agent"><h3>Start this agent</h3>
    <p>Choose a machine. The service checks the agent is active, its profile version is reviewed, the machine may run it and can reach its MCP servers, then records the start and answers the command to run there. It does not run the command.</p>
    {network.status === 'refused' ? <p className="why-not">{network.refused.refusal.refusal}: {network.refused.message}</p> : null}
    {network.status === 'ok' && !machines.length ? <p>No registered machine has a runtime installed.</p> : null}
    {machines.length ? <form onSubmit={(event) => { event.preventDefault(); if (machine && !asked) void send({ machine, operation: operationId() }); }}>
      <label className="field">Machine<select value={machine} disabled={busy || asked !== null} onChange={(event) => { setMachine(event.target.value); setAnswer(null); }}><option value="">Choose a machine</option>{machines.map((entry) => <option key={entry.id} value={entry.id}>{entry.name} · {entry.runtime}</option>)}</select></label>
      <button className="btn primary" type="submit" disabled={busy || !machine || asked !== null}>Get start command</button>
    </form> : null}
    {asked ? <div role="status"><p>The start on this machine is not confirmed. Its original request is retained.</p><button className="btn" disabled={busy} onClick={() => void send(asked)}>Check original start</button></div> : null}
    {failure ? <p className="why-not" role="alert">{failure}</p> : null}
    {answer ? <div role="status"><p>Start recorded as session {answer.session}, from profile version {answer.provisioning_version}, for {answer.harness} on {answer.runtime}. Run this on the machine:</p>
      <pre style={{ whiteSpace: 'pre-wrap' }}>{answer.command}</pre>
      {answer.handles.length ? <p>Credential handles it names: {answer.handles.map((handle) => handle.secret + ' as ' + handle.env).join(', ')}.</p> : null}
      {answer.left_out.length ? <p className="note">Not carried by the command: {answer.left_out.join('; ')}.</p> : null}
      <details><summary>Launch template</summary><p>SHA-256 {answer.template_sha256}</p><pre style={{ whiteSpace: 'pre-wrap' }}>{answer.template}</pre></details>
    </div> : null}
  </section>;
}
