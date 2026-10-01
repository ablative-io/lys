import { useRef, useState } from 'react';
import { operationId, Refused, request, useLoad } from '../../api';
import type { Machine } from './contract';
import type { Role } from '../roles/contract';
import { AddMachine } from './AddMachine';
import { confirmAdmission, machineAdmissionServed, savedAdmissions } from './machine-admission';
import type { MachineAdmission, PendingAdmissions } from './machine-admission';

export function ComputerAdmission({ agent, name, person, admin, machines, roles, changed }: {
  agent: string; name: string; person: string; admin: boolean; machines: Machine[]; roles: Role[]; changed: (machine: Machine) => void;
}) {
  const capability = useLoad(machineAdmissionServed, 'computer-admission:' + agent);
  const key = 'lys.pending.machine-agents.' + person + '.' + agent;
  const [initial] = useState(() => {
    try { return { pending: savedAdmissions(key, agent), error: '' }; }
    catch (error) { return { pending: {}, error: error instanceof Refused ? error.refusal.refusal + ': ' + error.message : 'PendingMachineAdmissionUnreadable: ' + String(error) }; }
  });
  const [pending, setPending] = useState<PendingAdmissions>(initial.pending);
  const retained = useRef<PendingAdmissions>(initial.pending);
  const [failure, setFailure] = useState(initial.error);
  const [busy, setBusy] = useState<string[]>([]);
  const working = useRef(new Set<string>());
  const [adding, setAdding] = useState(false);
  const [notice, setNotice] = useState('');
  const computers = machines.filter((machine) => machine.state === 'in_use');
  const heldRoles = new Map(roles.filter((role) => role.holders.some((holder) => holder.holder === agent && holder.state === 'holding')).map((role) => [role.id, role.name]));
  const served = capability.status === 'ok' && capability.data;
  const keep = (next: PendingAdmissions) => {
    if (Object.keys(next).length) sessionStorage.setItem(key, JSON.stringify(next)); else sessionStorage.removeItem(key);
    retained.current = next; setPending(next);
  };
  const send = async (machine: string, body: MachineAdmission) => {
    if (!admin || !served || initial.error || working.current.has(machine)) return;
    working.current.add(machine); setBusy([...working.current]); setFailure('');
    try {
      keep({ ...retained.current, [machine]: body });
      const answer = await request<unknown>('/network/machines/' + encodeURIComponent(machine) + '/agents', body);
      const current = confirmAdmission(answer, machine, body, person);
      const next = { ...retained.current }; delete next[machine]; keep(next); changed(current);
    } catch (error) { setFailure(error instanceof Refused ? error.refusal.refusal + ': ' + error.message : String(error)); }
    finally { working.current.delete(machine); setBusy([...working.current]); }
  };
  const retry = (machine: string) => <button className="btn" type="button" disabled={!admin || !served || Boolean(initial.error) || busy.includes(machine)} onClick={() => void send(machine, pending[machine])}>Check computer permission</button>;
  return <section className="computer-admission" aria-label={'Where can ' + name + ' run?'}>
    <h2>Where can {name} run?</h2>
    {computers.map((machine) => {
      const via = (machine.may_run_roles ?? []).flatMap((role) => { const title = heldRoles.get(role); return title === undefined ? [] : [title]; });
      const allowed = machine.may_run.some((entry) => entry.id === agent) || via.length > 0;
      return <div key={machine.id} className="computer-permission">
        <label><input type="checkbox" value={machine.id} checked={allowed} disabled={!admin || !served || Boolean(initial.error) || busy.includes(machine.id) || Boolean(pending[machine.id]) || machine.runtime === null || via.length > 0}
          onChange={(event) => { if (!retained.current[machine.id]) void send(machine.id, { operation: operationId(), agent, allow: event.target.checked }); }} /> {machine.name}</label>
        {via.length ? <span className="hint"> allowed through role {via.join(', ')}</span> : null}
        {machine.runtime === null ? <span className="hint"> This computer has no runtime.</span> : null}
        {pending[machine.id] ? <div><p>This computer permission has no confirmed answer. Its original request is kept.</p>{retry(machine.id)}</div> : null}
      </div>;
    })}
    {!served ? <p className="hint">Changing existing computer permissions is coming.</p> : null}
    {!admin ? <p className="hint">An administrator changes computer permissions.</p> : null}
    {capability.status === 'refused' ? <><p role="alert">Lys could not read the computer permission route.</p><details><summary>Details</summary><p>{capability.refused.refusal.refusal}: {capability.refused.message}</p></details></> : null}
    {failure ? <><p role="alert">{initial.error ? 'Lys could not read the saved computer permission.' : 'Lys could not confirm this computer permission.'}</p><details><summary>Details</summary><p>{failure}</p></details></> : null}
    {Object.keys(pending).filter((machine) => !computers.some((entry) => entry.id === machine)).map((machine) => <div key={machine}><p>A retained permission names a computer outside this list: {machine}.</p>{retry(machine)}</div>)}
    {notice ? <p role="status">{notice}</p> : null}
    {!computers.length && admin ? adding ? <AddMachine person={person} agent={agent} cancel={() => setAdding(false)} changed={(message, machine) => { setAdding(false); changed(machine); setNotice(message); setFailure(''); }} /> : <button className="btn" type="button" onClick={() => setAdding(true)}>Add this computer</button> : null}
  </section>;
}
