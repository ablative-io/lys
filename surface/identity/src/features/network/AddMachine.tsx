/**
 * Naming a computer keeps its request until it is confirmed: this computer, with Lys's own runner, until both the
 * computer and its runner are; another computer until it is named, after which it is given a connection code that is
 * shown once and never kept.
 */
import { useRef, useState } from 'react';
import type { FormEvent, RefObject } from 'react';
import { operationId, Refused, request } from '../../api';
import { confirmRunner, matchesMachine, savedMachine } from './contract';
import type { Machine, PendingMachine } from './contract';
import { answeredNo, keepRecord, releaseRecord } from '../../kept';
import { askJoinCode, joinFailure } from './JoinCode';
import type { Connecting, JoinCode } from './JoinCode';
import { Act } from '../../shell/Act';

export async function recordComputer(initial: PendingMachine, person: string, keep: (next: PendingMachine) => void): Promise<Machine> {
  let current = initial;
  keep(current);
  if (current.phase === 'machine') {
    const result = await request<unknown>('/network/machines', current.body);
    if (!matchesMachine(result, current.body, person)) throw new Refused(200, { refusal: 'MachineReceiptMismatch', reason: 'The answer did not confirm this computer was added. What you entered is kept.' });
    // Another computer's runner is not recorded here: it joins with its connection code.
    if (current.remote) return result;
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
type Remote = { remote?: boolean; connect?: (next: Connecting) => void };

/** The addition itself, whichever form shows it: what is kept until it is confirmed, what failed, and the name to send. */
function useAddition({ person, agent, changed, remote = false, connect }: Adding & Remote) {
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
      if (current.remote) {
        let given: JoinCode | null = null;
        let failure = '';
        try { given = await askJoinCode(machine.id); } catch (error) { failure = joinFailure(error); }
        connect?.({ machine: machine.id, name: current.body.name, given, failure, at: 'row' });
        changed(current.body.name + (given ? ' was added. Run the command below on that computer to connect it.' : ' was added. It is not connected yet.'), machine);
      } else changed(current.body.name + ' was added. Its runner is recorded.', machine);
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
    void send({ body: { operation: operationId(), name, kind: 'Computer', runtime: 'lys-runner', slots: 0, may_run: agent ? [agent] : [], may_run_roles: [], may_reach: [] }, phase: 'machine', machine: null, ...(remote ? { remote: true as const } : {}) });
  };
  const blocked = busy || pending !== null || Boolean(restored.error);
  /** How the addition stands: kept and unconfirmed, with its check, or why it failed. */
  const standing = <>
    {pending ? <div role="status"><p>Adding {pending.body.name} is not confirmed. Its original request is kept.</p><Act symbol="again" name="Check whether it was added" word="Check" disabled={busy} onClick={() => void send(pending, true)} /></div> : null}
    {failure ? <><p className="why-not" role="alert">Lys could not confirm this computer addition.</p><p><small className="refusal-name">{failure}</small></p></> : null}
  </>;
  return { submit, blocked, standing, pendingRemote: pending?.remote === true };
}

/** Adding the computer an agent is to run on, where no computer is in use yet: its name, and the button. */
export function AddMachine({ person, agent, changed, cancel }: Adding & { cancel: () => void }) {
  const { submit, blocked, standing } = useAddition({ person, agent, changed });
  return <form className="recorded-form" aria-label="Add a computer" onSubmit={submit}>
    <h2>Add this computer</h2>
    <fieldset disabled={blocked} style={{ border: 0, padding: 0, margin: 0 }}>
      <label className="field">Name<input name="name" /></label>
      <div className="chain"><Act symbol="add" name="Add this computer" word="Add" tone="primary" type="submit" /><Act symbol="close" name="Cancel" word="Cancel" onClick={cancel} /></div>
    </fieldset>
    {standing}
  </form>;
}

/**
 * The computers table's own last row, which adds one: the name under Computer, how the addition stands under Status,
 * which computer it is in a cell of its own, the button in the last column.
 */
export function AddComputerRow({ person, changed, name, connect }: Omit<Adding, 'agent'> & { name: RefObject<HTMLInputElement | null>; connect: (next: Connecting) => void }) {
  const [chosen, setChosen] = useState<'this' | 'another'>('this');
  const { submit, blocked, standing, pendingRemote } = useAddition({ person, changed, remote: chosen === 'another', connect });
  // A kept addition is finished as it was begun, whatever is chosen now.
  const where = pendingRemote ? 'another' : chosen;
  const id = 'add-computer-' + person.replace(/[^A-Za-z0-9_-]/g, '_');
  return <tr className="listing-add" data-add="computer">
    <td><form id={id} aria-label="Add a computer" onSubmit={submit}><input ref={name} name="name" aria-label="Name of the computer to add" placeholder="Name" disabled={blocked} /></form></td>
    <td>{standing}</td>
    <td><div className="seg" role="group" aria-label="Which computer">
      {([['this', 'This computer'], ['another', 'Another computer']] as const).map(([key, label]) =>
        <button key={key} type="button" className={where === key ? 'on' : ''} aria-pressed={where === key} disabled={blocked} onClick={() => setChosen(key)}>{label}</button>)}
    </div></td>
    <td><Act symbol="add" name={where === 'another' ? 'Add and get its code' : 'Add this computer'} word="Add" tone="primary" type="submit" form={id} disabled={blocked} /></td>
  </tr>;
}
