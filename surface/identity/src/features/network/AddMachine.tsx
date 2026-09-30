/** Add a computer: this one or another, who may start there, and the rarely needed rest under More; its runner is recorded in the same step. */
import { useRef, useState } from 'react';
import type { FormEvent } from 'react';
import { operationId, Refused, request, useLoad } from '../../api';
import type { AgentSummary } from '../../generated';
import { readRoles } from '../roles/AssignedRoles';
import { Gate } from '../signin/Gate';
import { matchesMachine, savedMachine } from './contract';
import type { Machine, NameMachine } from './contract';

type Starts = 'own' | 'dialled';
const RUNNER = 'lys-runner';

export function AddMachine({ agents, person, changed, cancel }: { agents: AgentSummary[]; person: string; changed: (message: string) => void; cancel: () => void }) {
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
    const name = text('name'); const kind = 'Computer';
    if (!name) { setFailure('Give the computer a name.'); return; }
    const runs = true;
    const runnerKey = text('runner-key').toLowerCase();
    if (starts === 'dialled' && !/^[0-9a-f]{64}$/.test(runnerKey)) { setFailure('The runner key is 64 letters and digits (0 to 9, a to f), as the runner printed it when it was set up.'); return; }
    const runner: Record<string, string> = starts === 'own' ? { kind: 'lys' } : { kind: 'dialled', key: runnerKey };
    const mayReach = [...new Set(text('hosts').split(/\s+/).filter(Boolean).map((host) => host.toLowerCase()))];
    void send({ operation: operationId(), name, kind, runtime: runs ? RUNNER : null, slots: 0,
      may_run: runs ? data.getAll('agent').map(String) : [], ...(runs ? { may_run_roles: data.getAll('role').map(String) } : {}), may_reach: mayReach }, runner, false);
  };
  return <form className="card recorded-form" aria-label="Add a computer" onSubmit={submit}>
    <h2>Add a computer</h2>
    <fieldset disabled={busy || pending !== null || Boolean(restored.error)} style={{ border: 0, padding: 0, margin: 0 }}>
      <div className="field">Which computer
        <div className="seg" role="group" aria-label="Which computer">
          <button type="button" className={starts === 'own' ? 'on' : ''} aria-pressed={starts === 'own'} onClick={() => setStarts('own')}>The one Lys runs on</button>
          <button type="button" className={starts === 'dialled' ? 'on' : ''} aria-pressed={starts === 'dialled'} onClick={() => setStarts('dialled')}>Another computer</button>
        </div>
      </div>
      <label className="field">Name<input name="name" required maxLength={100} placeholder={starts === 'own' ? 'For example Office Mac' : 'For example Build server 3'} /></label>
      {starts === 'dialled' ? <label className="field">Its runner's key<span className="hint">Set up the Lys runner on that computer and paste the key it prints.</span><input name="runner-key" required autoComplete="off" /></label> : null}
      <div className="field">Who may start agents here
        <Gate load={roles} title="Roles" ok={(view) => view.roles.length ? <>{view.roles.map((role) => <label key={role.id} className="sec"><input type="checkbox" name="role" value={role.id} /> Anyone holding {role.name}</label>)}</> : null} />
        <AgentPicker agents={agents} />
      </div>
      <details><summary className="sec">Websites its agents' services may connect to</summary>
        <label className="field"><span className="hint">One per line, for example mcp.example.com.</span><textarea name="hosts" rows={3} /></label>
      </details>
      <div className="chain" style={{ marginTop: 12 }}><button className="btn primary" type="submit">Add this computer</button><button className="btn" type="button" onClick={cancel}>Cancel</button></div>
    </fieldset>
    {pending ? <div role="status"><p>Adding {pending.name} is not confirmed. What you entered is kept.</p><button className="btn" type="button" disabled={busy} onClick={() => void send(pending, null, true)}>Check whether it was added</button></div> : null}
    {failure ? <p className="why-not" role="alert">{failure}</p> : null}
  </form>;
}

/** Agents chosen by name, however many there are: type, pick from the few that match, remove with ×. */
function AgentPicker({ agents }: { agents: AgentSummary[] }) {
  const [query, setQuery] = useState('');
  const [chosen, setChosen] = useState<AgentSummary[]>([]);
  const needle = query.trim().toLowerCase();
  const matches = needle ? agents.filter((agent) => agent.display_name.toLowerCase().includes(needle) && !chosen.some((each) => each.id === agent.id)).slice(0, 8) : [];
  if (!agents.length) return <span className="hint">There are no agents yet.</span>;
  return <div>
    {chosen.map((agent) => <span key={agent.id} className="pill">{agent.display_name}<input type="hidden" name="agent" value={agent.id} />{' '}<button type="button" aria-label={'Remove ' + agent.display_name} onClick={() => setChosen((list) => list.filter((each) => each.id !== agent.id))}>×</button></span>)}
    <input className="search" type="search" aria-label="Add an agent" placeholder={'Add an agent by name (' + agents.length.toLocaleString('en-AU') + ')'} value={query} onChange={(event) => setQuery(event.target.value)} onKeyDown={(event) => { if (event.key === 'Enter') { event.preventDefault(); if (matches[0]) { setChosen((list) => [...list, matches[0]]); setQuery(''); } } }} />
    {matches.length ? <ul className="plain">{matches.map((agent) => <li key={agent.id}><button type="button" className="btn" onClick={() => { setChosen((list) => [...list, agent]); setQuery(''); }}>{agent.display_name}</button></li>)}</ul> : null}
  </div>;
}
