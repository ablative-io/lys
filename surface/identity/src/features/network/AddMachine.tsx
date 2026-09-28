/** Register a machine with explicit placement and host choices, retaining unknown submissions. */
import { useRef, useState } from 'react';
import type { FormEvent } from 'react';
import { operationId, Refused, request, useLoad } from '../../api';
import type { AgentSummary } from '../../generated';
import { readRoles } from '../roles/AssignedRoles';
import { Gate } from '../signin/Gate';
import { field } from '../people/RecordedForm';
import { matchesMachine, savedMachine } from './contract';
import type { Machine, NameMachine } from './contract';

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
  const [done, setDone] = useState('');
  const [runtime, setRuntime] = useState(false);
  const working = useRef(false);
  const send = async (body: NameMachine, retry: boolean) => {
    if (working.current || restored.error) return;
    working.current = true; setBusy(true); setFailure('');
    try {
      sessionStorage.setItem(key, JSON.stringify(body)); setPending(body);
      const result = await request<Machine>('/network/machines', body);
      if (!matchesMachine(result, body, person)) throw new Error('The answer did not confirm this machine registration. The original request is retained.');
      sessionStorage.removeItem(key); setPending(null); setDone('Machine recorded.'); changed('Machine recorded. This does not confirm that its runtime is connected.');
    } catch (error) {
      if (!retry && error instanceof Refused && error.status >= 400 && error.status < 500) {
        sessionStorage.removeItem(key); setPending(null);
      }
      setFailure(error instanceof Refused ? error.refusal.refusal + ': ' + error.message : String(error));
    } finally { working.current = false; setBusy(false); }
  };
  const submit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault(); if (pending || working.current || restored.error) return;
    const data = new FormData(event.currentTarget);
    const name = field(data, 'name'); const kind = field(data, 'kind');
    const slots = runtime ? Number(field(data, 'slots')) : 0;
    const selectedRuntime = runtime ? field(data, 'runtime') : null;
    const mayRun = runtime ? data.getAll('agent').map(String) : [];
    const mayReach = [...new Set(field(data, 'hosts').split(/\s+/).filter(Boolean).map((host) => host.toLowerCase()))];
    if (!name || !kind || (runtime && !selectedRuntime) || !Number.isInteger(slots) || slots < 0) { setFailure('Name the machine, its kind and its runtime capacity.'); return; }
    void send({ operation: operationId(), name, kind, runtime: selectedRuntime, slots, may_run: mayRun, ...(runtime ? { may_run_roles: data.getAll('role').map(String) } : {}), may_reach: mayReach }, false);
  };
  return <form className="card recorded-form" aria-label="Register machine" onSubmit={submit}>
    <h2>Register a machine</h2><p>Name the machine and record which agents and hosts it may use. This does not install a runtime or start an agent.</p>
    <fieldset disabled={busy || pending !== null || Boolean(restored.error) || Boolean(done)} style={{ border: 0, padding: 0 }}>
      <label className="field">Machine name<input name="name" required maxLength={100} /></label>
      <label className="field">Kind<input name="kind" required maxLength={100} placeholder="For example, laptop or server" /></label>
      <label><input type="checkbox" checked={runtime} onChange={(event) => setRuntime(event.target.checked)} /> A runtime is installed on this machine</label>
      {runtime ? <><label className="field">Runtime name<input name="runtime" required maxLength={100} /></label>
        <label className="field">Concurrent agent slots<input name="slots" type="number" min={0} step={1} required /></label>
        <fieldset><legend>Agents permitted on this machine</legend>{agents.map((agent) => <label key={agent.id} className="row"><span>{agent.display_name} · {agent.state}</span><input type="checkbox" name="agent" value={agent.id} /></label>)}
          {!agents.length ? <p>No registered agents are available.</p> : null}</fieldset><fieldset><legend>Roles permitted on this machine</legend><Gate load={roles} title="Machine roles" ok={(view) => <>{view.roles.map((role) => <label key={role.id} className="row"><span>{role.name}</span><input type="checkbox" name="role" value={role.id} /></label>)}{!view.roles.length ? <p>No roles have been recorded.</p> : null}</>} /></fieldset></> : <p className="note">Without a runtime, no agent slots or placements are recorded.</p>}
      <label className="field">Hosts agents may reach<textarea name="hosts" placeholder="One hostname per line" /></label>
      <p className="note">Use hostnames without a scheme, port or path. Leave empty to record no permitted hosts.</p>
      <button className="btn primary" type="submit">Register machine</button>
    </fieldset>
    {pending ? <div role="status"><p>Registration of {pending.name} is not confirmed. Its original details are retained.</p><button className="btn" type="button" disabled={busy} onClick={() => void send(pending, true)}>Check original registration</button></div> : null}
    {failure ? <p className="why-not" role="alert">{failure}</p> : null}{done ? <p role="status">{done}</p> : null}
  </form>;
}
