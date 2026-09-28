/** A recorded launch request retains its operation; showing a command never reports a running process. */
import { useState } from 'react';
import { api, operationId, request, useLoad } from '../../api';
import { readRoles } from '../roles/AssignedRoles';
import type { Role } from '../roles/contract';
import type { NetworkView } from '../network/contract';
import { useRoleChange } from '../roles/useRoleChange';
import { ChangeStatus } from '../roles/ChangeStatus';
import { Gate } from '../signin/Gate';
export interface StartAnswer {
  agent: string; machine: string; runtime: string; session: string; provisioning_version: number; harness: string;
  handles: { env: string; id: string; secret: string }[]; template: string; template_sha256: string; command: string; left_out: string[]; executed: false;
  /** The machine's runner's word, when the machine names a runner and it ran the start. */
  runner?: { session: string; state: 'running' | 'ended'; pid: number | null; started_at: number };
}
export function StartAgent({ agent }: { agent: string }) {
  const load = useLoad(async () => ({ me: await api.me(), network: await request<NetworkView>('/network'), roles: await readRoles() }), 'launch-options:' + agent);
  return <section className="card"><h2>Prepare an agent start</h2><p>This records a start request and gives you its command. You run it on the selected machine. Running is confirmed only by a runtime report.</p>
    <Gate load={load} title="Start options" ok={({ me, network, roles }) => <StartForm agent={agent} person={me.person.id} network={network} roles={roles.roles} />} />
  </section>;
}
function StartForm({ agent, person, network, roles }: { agent: string; person: string; network: NetworkView; roles: Role[] }) {
  const [machine, setMachine] = useState('');
  const [answer, setAnswer] = useState<StartAnswer | null>(null);
  const [copyNotice, setCopyNotice] = useState('');
  const held = roles.filter((role) => role.holders.some((holder) => holder.holder === agent && holder.state === 'holding')).map((role) => role.id);
  const machines = network.machines.filter((entry) => entry.state === 'in_use' && entry.runtime !== null && (entry.may_run.some((entry) => entry.id === agent) || entry.may_run_roles?.some((role) => held.includes(role))));
  const change = useRoleChange<StartAnswer>('lys.pending.start.' + person + '.' + agent, '/agents/' + encodeURIComponent(agent) + '/start-command', (receipt, body) => {
    const matches = receipt.agent === agent && receipt.machine === body.machine && receipt.session === body.operation && receipt.executed === false && typeof receipt.command === 'string' && receipt.command.length > 0 && Array.isArray(receipt.left_out);
    if (matches) setAnswer(receipt);
    return matches;
  }, () => {});
  const copy = async () => {
    if (!answer) return;
    try { await navigator.clipboard.writeText(answer.command); setCopyNotice('Command copied. This has not started the agent.'); }
    catch (error) { setCopyNotice('Could not copy: ' + String(error)); }
  };
  return <>
    {!answer ? <form onSubmit={(event) => { event.preventDefault(); if (machine) change.submit({ machine, operation: operationId() }); }}>
      <label className="field">Machine<select required value={machine} disabled={change.blocked} onChange={(event) => setMachine(event.target.value)}><option value="">Choose where to run</option>{machines.map((entry) => <option key={entry.id} value={entry.id}>{entry.name} · {entry.runtime}</option>)}</select></label>
      {!machines.length ? <p>No permitted machine with a runtime was returned. <a href="#/network">Manage machines</a>.</p> : null}
      <button type="submit" className="btn primary" disabled={change.blocked || !machine}>Record start request</button>
    </form> : <section role="status">{answer.runner ? <><h3>{answer.runner.state === 'running' ? 'Started — its runner has it running' : 'Started — its runner saw it end'}</h3><p><a href={'#/runtime/' + encodeURIComponent(answer.session)}>Open its terminal</a></p></> : <h3>Start recorded — running is unconfirmed</h3>}<p>Session: <span className="mono">{answer.session}</span></p><p>Profile version {answer.provisioning_version} · {answer.runtime}</p>
      {answer.left_out.length ? <div className="why-not"><h3>Not carried into this command</h3><ul>{answer.left_out.map((entry, index) => <li key={index}>{entry}</li>)}</ul></div> : null}
      <label className="field">Command<textarea readOnly rows={4} value={answer.command} /></label><button className="btn" onClick={() => void copy()}>Copy command</button>{copyNotice ? <p role="status">{copyNotice}</p> : null}
      <details><summary>Launch template and evidence</summary><p>Template hash: {answer.template_sha256}</p><pre>{answer.template}</pre></details>
      <p><a href={'#/file/' + agent + '/sessions'}>Read runtime reports</a>. Do not start another copy while this start’s outcome is unknown.</p>
    </section>}
    <ChangeStatus change={change} />
  </>;
}
