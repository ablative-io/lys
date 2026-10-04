/** Naming a local computer keeps its request until both the computer and runner are confirmed. */
import { useRef, useState } from 'react';
import type { FormEvent, RefObject } from 'react';
import { operationId, Refused, request } from '../../api';
import { confirmRunner, matchesMachine, savedMachine } from './contract';
import type { Machine, PendingMachine } from './contract';
import { answeredNo, keepRecord, releaseRecord } from '../../kept';

export async function recordComputer(initial: PendingMachine, person: string, keep: (next: PendingMachine) => void): Promise<Machine> {
  let current = initial;
  keep(current);
  if (current.phase === 'machine') {
    const result = await request<unknown>('/network/machines', current.body);
    if (!matchesMachine(result, current.body, person)) throw new Refused(200, { refusal: 'MachineReceiptMismatch', reason: 'The answer did not confirm this computer was added. What you entered is kept.' });
    current = { ...current, machine: result, phase: 'runner' }; keep(current);
  }
  const path = '/network/machines/' + encodeURIComponent(current.body.operation) + '/runner';
  const runner = await request<unknown>(path, { runner: { kind: 'lys' } });
  confirmRunner(runner, current.body.operation, true);
  if (!current.machine) throw new Refused(200, { refusal: 'MachineReceiptMismatch', reason: 'The recorded computer is missing from this pending addition.' });
  return current.machine;
}

export const validComputerName = (name: string): boolean => Boolean(name) && !/[\u0000-\u001f\u007f]/.test(name);

type Adding = { person: string; agent?: string; changed: (message: string, machine: Machine) => void };

/** The addition itself, whichever form shows it: what is kept until it is confirmed, what failed, and the name to send. */
function useAddition({ person, agent, changed }: Adding) {
  const key = 'lys.pending.machine.' + person;
  const [restored] = useState(() => {
    try { return { pending: savedMachine(key), error: '' }; }
    catch (error) { return { pending: null, error: error instanceof Refused ? error.refusal.refusal + ': ' + error.message : 'PendingMachineUnreadable: ' + String(error) }; }
  });
  const [pending, setPending] = useState(restored.pending);
  const [failure, setFailure] = useState(restored.error);
  const [busy, setBusy] = useState(false);
  const working = useRef(false);
  const keep = (next: PendingMachine) => { keepRecord(key, { version: 1, ...next }); setPending(next); };
  const send = async (initial: PendingMachine, retry = false) => {
    if (working.current || restored.error) return;
    working.current = true; setBusy(true); setFailure('');
    let current = initial;
    try {
      const machine = await recordComputer(initial, person, (next) => { current = next; keep(next); });
      releaseRecord(key); setPending(null);
      changed(current.body.name + ' was added. Its runner is recorded.', machine);
    } catch (error) {
      if (!retry && current.phase === 'machine' && answeredNo(error) && error instanceof Refused && error.refusal.refusal !== 'Unanswered') {
        releaseRecord(key); setPending(null);
      }
      setFailure(error instanceof Refused ? error.refusal.refusal + ': ' + error.message : String(error));
    }
    finally { working.current = false; setBusy(false); }
  };
  const submit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault(); if (pending || working.current || restored.error) return;
    const name = String(new FormData(event.currentTarget).get('name') ?? '').trim();
    if (!validComputerName(name)) { setFailure('Give this computer a name, without control characters.'); return; }
    if (agent !== undefined && !/^agent-[0-9a-f]{32}$/.test(agent)) { setFailure('AgentIdentifierMalformed: the agent for this addition could not be read.'); return; }
    void send({ body: { operation: operationId(), name, kind: 'Computer', runtime: 'lys-runner', slots: 0, may_run: agent ? [agent] : [], may_run_roles: [], may_reach: [] }, phase: 'machine', machine: null });
  };
  const blocked = busy || pending !== null || Boolean(restored.error);
  /** How the addition stands: kept and unconfirmed, with its check, or why it failed. */
  const standing = <>
    {pending ? <div role="status"><p>Adding {pending.body.name} is not confirmed. Its original request is kept.</p><button className="btn" type="button" disabled={busy} onClick={() => void send(pending, true)}>Check whether it was added</button></div> : null}
    {failure ? <><p className="why-not" role="alert">Lys could not confirm this computer addition.</p><p><small className="refusal-name">{failure}</small></p></> : null}
  </>;
  return { submit, blocked, standing };
}

/** Adding the computer an agent is to run on, where no computer is in use yet: its name, and the button. */
export function AddMachine({ person, agent, changed, cancel }: Adding & { cancel: () => void }) {
  const { submit, blocked, standing } = useAddition({ person, agent, changed });
  return <form className="recorded-form" aria-label="Add a computer" onSubmit={submit}>
    <h2>Add this computer</h2>
    <fieldset disabled={blocked} style={{ border: 0, padding: 0, margin: 0 }}>
      <label className="field">Name<input name="name" /></label>
      <div className="chain"><button className="btn primary" type="submit">Add this computer</button><button className="btn" type="button" onClick={cancel}>Cancel</button></div>
    </fieldset>
    {standing}
  </form>;
}

/** The computers table's own last row, which adds one: the name under Computer, how the addition stands under Status, the button in the last column. */
export function AddComputerRow({ person, changed, name }: Omit<Adding, 'agent'> & { name: RefObject<HTMLInputElement | null> }) {
  const { submit, blocked, standing } = useAddition({ person, changed });
  const id = 'add-computer-' + person.replace(/[^A-Za-z0-9_-]/g, '_');
  return <tr className="listing-add" data-add="computer">
    <td><form id={id} aria-label="Add a computer" onSubmit={submit}><input ref={name} name="name" aria-label="Name of the computer to add" placeholder="Name" disabled={blocked} /></form></td>
    <td>{standing}</td>
    <td />
    <td><button form={id} className="btn primary" type="submit" disabled={blocked}>Add this computer</button></td>
  </tr>;
}
