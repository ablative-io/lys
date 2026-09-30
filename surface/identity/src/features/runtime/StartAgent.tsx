/** The agent's one start: what still stands in its way, in order, or the start itself, saying what pressing it does on the chosen computer. */
import { useState } from 'react';
import { api, operationId, request, useLoad } from '../../api';
import { readRoles } from '../roles/AssignedRoles';
import type { Machine, NetworkView } from '../network/contract';
import { entries } from '../people/directory';
import type { ProvisioningProfile } from '../provisioning/Provisioning';
import { useRoleChange } from '../roles/useRoleChange';
import { ChangeStatus } from '../roles/ChangeStatus';
import { Gate } from '../signin/Gate';
export interface StartAnswer {
  agent: string; machine: string; runtime: string; session: string; provisioning_version: number; harness: string;
  handles: { env: string; id: string; secret: string }[]; template: string; template_sha256: string; command: string; left_out: string[]; executed: false;
  /** The machine's runner's word, when the machine names a runner and it ran the start. */
  runner?: { session: string; state: 'running' | 'ended'; pid: number | null; started_at: number };
}

/** Why a computer cannot start this agent, in the words the server's refusal uses; null when it can. */
function excluded(machine: Machine, agent: string, held: string[]): string | null {
  if (machine.state !== 'in_use') return 'it is retired';
  if (machine.runtime === null) return 'it has no Lys runner';
  if (machine.may_run.some((entry) => entry.id === agent) || machine.may_run_roles?.some((role) => held.includes(role))) return null;
  return 'this agent is not allowed on it: it needs to be named on the computer, or hold a role the computer admits' + (machine.may_run_roles?.length ? ' (' + machine.may_run_roles.join(', ') + ')' : '');
}

export function StartAgent({ agent, profile }: { agent: string; profile: ProvisioningProfile | null }) {
  const load = useLoad(async () => ({ me: await api.me(), people: await api.people(), network: await request<NetworkView>('/network'), roles: await readRoles() }), 'launch-options:' + agent + ':' + (profile?.version ?? 0) + ':' + (profile?.reviewed_by ?? ''));
  return <section className="card"><Gate load={load} title="Start" ok={({ me, people, network, roles }) => {
    const state = entries(people).find((entry) => entry.id === agent)?.state;
    const held = roles.roles.filter((role) => role.holders.some((holder) => holder.holder === agent && holder.state === 'holding')).map((role) => role.id);
    const permitted = network.machines.filter((machine) => excluded(machine, agent, held) === null);
    const steps: [string, boolean, string][] = [
      ['Turn this agent on', state === 'active', '#/directory/manage?action=status&identity=' + encodeURIComponent(agent)],
      ['Let it use a computer', permitted.length > 0, '#/roles'],
      ['Choose its program in the settings below', Boolean(profile?.harness), '#/file/' + encodeURIComponent(agent) + '/provisioning'],
      ['Approve its settings below', Boolean(profile?.reviewed_by), '#/file/' + encodeURIComponent(agent) + '/provisioning'],
    ];
    if (steps.some(([, done]) => !done)) return <><p>Before this agent can start:</p><ol>{steps.map(([words, done, href]) => <li key={words}>{done ? words + ' — done' : <a href={href}>{words}</a>}</li>)}</ol>
      {!permitted.length && network.machines.length ? <ul>{network.machines.map((machine) => <li key={machine.id}>{machine.name}: {excluded(machine, agent, held)}</li>)}</ul> : null}</>;
    return <StartForm agent={agent} person={me.person.id} machines={permitted} preferred={profile?.runs_on ?? ''} />;
  }} /></section>;
}

function StartForm({ agent, person, machines, preferred }: { agent: string; person: string; machines: Machine[]; preferred: string }) {
  const initial = machines.some((machine) => machine.id === preferred) ? preferred : machines.length === 1 ? machines[0].id : '';
  const [machine, setMachine] = useState(initial);
  const [answer, setAnswer] = useState<StartAnswer | null>(null);
  const chosen = machines.find((entry) => entry.id === machine);
  const change = useRoleChange<StartAnswer>('lys.pending.start.' + person + '.' + agent, '/agents/' + encodeURIComponent(agent) + '/start-command', (receipt, body) => {
    const matches = receipt.agent === agent && receipt.machine === body.machine && receipt.session === body.operation && receipt.executed === false && typeof receipt.command === 'string' && receipt.command.length > 0 && Array.isArray(receipt.left_out);
    if (matches) setAnswer(receipt);
    return matches;
  }, () => {});
  if (answer) return <section role="status">
    {answer.runner ? <><p>{answer.runner.state === 'running' ? 'Started on ' + (chosen?.name ?? answer.machine) + '. Its Lys runner has it running.' : 'Started, and its Lys runner saw it end.'}</p><a className="btn primary" href={'#/runtime/' + encodeURIComponent(answer.session)}>Open its terminal</a></>
      : <><p>No Lys runner on {chosen?.name ?? answer.machine} can start it, so run this command there yourself. Do not run it twice.</p><textarea readOnly rows={4} value={answer.command} /></>}
    {answer.left_out.length ? <><p>Not carried into this start:</p><ul>{answer.left_out.map((entry, index) => <li key={index}>{entry}</li>)}</ul></> : null}
    {answer.runner ? <details><summary>What was run</summary><pre>{answer.command}</pre></details> : null}
    <ChangeStatus change={change} />
  </section>;
  return <form onSubmit={(event) => { event.preventDefault(); if (machine) change.submit({ machine, operation: operationId() }); }}>
    <label className="field">Computer<select value={machine} disabled={change.blocked} onChange={(event) => setMachine(event.target.value)}>{machines.length > 1 ? <option value="">Choose a computer</option> : null}{machines.map((entry) => <option key={entry.id} value={entry.id}>{entry.name}</option>)}</select></label>
    <p>{chosen ? 'Pressing Start makes Lys ask the runner on ' + chosen.name + ' to start this agent with the approved settings.' : 'Choose a computer first.'}</p>
    <button type="submit" className="btn primary" disabled={change.blocked || !machine}>Start</button>
    <ChangeStatus change={change} />
  </form>;
}
