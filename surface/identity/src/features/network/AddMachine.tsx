/** Add a computer: what each answer changes is said beside it; a computer Lys starts agents on is given its Lys runner in the same step. */
import { useRef, useState } from 'react';
import type { FormEvent } from 'react';
import { operationId, Refused, request, useLoad } from '../../api';
import type { AgentSummary } from '../../generated';
import { readRoles } from '../roles/AssignedRoles';
import { Gate } from '../signin/Gate';
import { matchesMachine, savedMachine } from './contract';
import type { Machine, NameMachine } from './contract';

type Starts = 'own' | 'dialled' | 'none';
const RUNNER = 'lys-runner';

export function AddMachine({ agents, person, changed }: { agents: AgentSummary[]; person: string; changed: (message: string) => void }) {
  const roles = useLoad(readRoles, 'machine-roles');
  const key = 'lys.pending.machine.' + person;
  const [restored] = useState(() => {
    try { return { body: savedMachine(key), error: '' }; }
    catch (error) { return { body: null, error: String(error) }; }
  });
  const [pending, setPending] = useState(restored.body);
  const [failure, setFailure] = useState(restored.error);
  const [busy, setBusy] = useState(false);
  const [starts, setStarts] = useState<Starts>('own');
  const working = useRef(false);
  const send = async (body: NameMachine, runner: Record<string, string> | null, retry: boolean) => {
    if (working.current || restored.error) return;
    working.current = true; setBusy(true); setFailure('');
    try {
      sessionStorage.setItem(key, JSON.stringify(body)); setPending(body);
      const result = await request<Machine>('/network/machines', body);
      if (!matchesMachine(result, body, person)) throw new Error('The answer did not confirm this computer was added. What you entered is kept.');
      sessionStorage.removeItem(key); setPending(null);
      if (runner) {
        try { await request('/network/machines/' + encodeURIComponent(result.id) + '/runner', { runner }); }
        catch (error) { changed(body.name + ' was added, but its Lys runner was not recorded: ' + (error instanceof Refused ? error.refusal.refusal + ': ' + error.message : String(error)) + '. Lys cannot start agents on it until it is.'); return; }
      }
      changed(body.name + ' was added.' + (runner ? ' Lys will start agents on it through its Lys runner.' : ' Lys will not start agents on it.'));
    } catch (error) {
      if (!retry && error instanceof Refused && error.status >= 400 && error.status < 500) { sessionStorage.removeItem(key); setPending(null); }
      setFailure(error instanceof Refused ? error.refusal.refusal + ': ' + error.message : String(error));
    } finally { working.current = false; setBusy(false); }
  };
  const submit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault(); if (pending || working.current || restored.error) return;
    const data = new FormData(event.currentTarget);
    const text = (name: string) => String(data.get(name) ?? '').trim();
    const name = text('name'); const kind = text('kind');
    if (!name) { setFailure('Give the computer a name.'); return; }
    const runs = starts !== 'none';
    const runnerKey = text('runner-key').toLowerCase();
    if (starts === 'dialled' && !/^[0-9a-f]{64}$/.test(runnerKey)) { setFailure('The runner key is 64 letters and digits (0 to 9, a to f), as the runner printed it when it was set up.'); return; }
    const runner: Record<string, string> | null = starts === 'own' ? { kind: 'lys' } : starts === 'dialled' ? { kind: 'dialled', key: runnerKey } : null;
    const mayReach = [...new Set(text('hosts').split(/\s+/).filter(Boolean).map((host) => host.toLowerCase()))];
    void send({ operation: operationId(), name, kind, runtime: runs ? RUNNER : null, slots: 0,
      may_run: runs ? data.getAll('agent').map(String) : [], ...(runs ? { may_run_roles: data.getAll('role').map(String) } : {}), may_reach: mayReach }, runner, false);
  };
  return <form className="card recorded-form" aria-label="Add a computer" onSubmit={submit}>
    <fieldset disabled={busy || pending !== null || Boolean(restored.error)} style={{ border: 0, padding: 0 }}>
      <label className="field">Name<span className="hint">What Lys calls this computer everywhere, for example Tom's laptop.</span><input name="name" required maxLength={100} /></label>
      <label className="field">Kind<span className="hint">A description only; it changes nothing.</span>
        <select name="kind" defaultValue="Laptop"><option>Laptop</option><option>Desktop</option><option>Server</option><option>Virtual machine</option></select></label>
      <div className="field">How Lys starts agents on it
        <label><input type="radio" name="starts" checked={starts === 'own'} onChange={() => setStarts('own')} /> This is the computer Lys runs on. Lys starts agents here through its own Lys runner.</label>
        <label><input type="radio" name="starts" checked={starts === 'dialled'} onChange={() => setStarts('dialled')} /> Another computer with a Lys runner installed. Its runner connects to Lys, and Lys asks it to start agents.</label>
        <label><input type="radio" name="starts" checked={starts === 'none'} onChange={() => setStarts('none')} /> Lys does not start agents here. No agent can be started on it.</label></div>
      {starts === 'dialled' ? <label className="field">Its runner's key<span className="hint">The 64 letters and digits the runner printed when it was set up on that computer. Lys accepts a connection only from the runner holding this key.</span><input name="runner-key" required autoComplete="off" /></label> : null}
      {starts !== 'none' ? <>
        <div className="field">Agents that may start here<span className="hint">A start of any other agent on this computer is refused.</span>
          {agents.map((agent) => <label key={agent.id}><input type="checkbox" name="agent" value={agent.id} /> {agent.display_name}</label>)}
          {!agents.length ? <span>There are no agents yet.</span> : null}</div>
        <div className="field">Agents with these roles may also start here<Gate load={roles} title="Roles" ok={(view) => <>{view.roles.map((role) => <label key={role.id}><input type="checkbox" name="role" value={role.id} /> {role.name}</label>)}{!view.roles.length ? <span>There are no roles yet.</span> : null}</>} /></div>
      </> : null}
      <label className="field">Websites its agents' services may connect to<span className="hint">One per line, for example mcp.example.com. A start is refused if one of the agent's connected services is at a website not listed here.</span><textarea name="hosts" rows={3} /></label>
      <button className="btn primary" type="submit">Add this computer</button>
    </fieldset>
    {pending ? <div role="status"><p>Adding {pending.name} is not confirmed. What you entered is kept.</p><button className="btn" type="button" disabled={busy} onClick={() => void send(pending, null, true)}>Check whether it was added</button></div> : null}
    {failure ? <p className="why-not" role="alert">{failure}</p> : null}
  </form>;
}
