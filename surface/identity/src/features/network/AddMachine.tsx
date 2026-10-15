/**
 * Naming a computer keeps its request until it is confirmed: this computer, with Lys's own runner, until both the
 * computer and its runner are; another computer until it is named, after which it is given a connection code that is
 * shown once and never kept.
 */
import { useRef, useState } from 'react';
import type { FormEvent, RefObject } from 'react';
import { api, operationId, Refused, request, useLoad } from '../../api';
import { confirmRunner, matchesMachine, savedMachine } from './contract';
import type { Machine, NetworkView, PendingMachine } from './contract';
import { ErrorWords } from '../people/Words';
import { answeredNo, keepRecord, releaseRecord } from '../../kept';
import { askJoinCode, joinDetail, joinFailure } from './JoinCode';
import type { Connecting, JoinCode } from './JoinCode';
import { Act } from '../../shell/Act';
import './network.css';

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

/**
 * The agents a person's computer is added for: their own, every one not retired. A computer lists the agents that may
 * run on it by id (the server keeps no rule for "the owner's agents"), so the person's own agents are named when it is
 * added, and each agent added later is allowed on the computer it is added to run on.
 */
export async function ownAgentIds(person: string): Promise<string[]> {
  const people = await api.people();
  return (people.people.find((entry) => entry.id === person)?.agents ?? []).filter((agent) => agent.state !== 'retired').map((agent) => agent.id);
}

/** The name the computer Lys runs on gives itself, offered as this computer's name; empty until read or when it names none. */
export function useHostName(): string {
  const load = useLoad(() => request<NetworkView>('/network'), 'host-name');
  return load.status === 'ok' ? load.data.host_name ?? '' : '';
}

type Adding = { person: string; agent?: string; agents?: string[]; changed: (message: string, machine: Machine) => void };
type Remote = { remote?: boolean; connect?: (next: Connecting) => void };

/** The addition itself, whichever form shows it: what is kept until it is confirmed, what failed, and the name to send. */
function useAddition({ person, agent, agents = [], changed, remote = false, connect }: Adding & Remote) {
  const key = 'lys.pending.machine.' + person;
  const [restored] = useState(() => {
    try { return { pending: savedMachine(key), error: '' }; }
    catch (error) { return { pending: null, error: error instanceof Refused ? error : new Refused(0, { refusal: 'PendingMachineUnreadable', reason: 'The saved computer addition cannot be read. ' + String(error) }) }; }
  });
  const [pending, setPending] = useState(restored.pending);
  // What failed: the service's refusal, shown in its own words with the code folded away; or what to type, said plainly.
  const [failure, setFailure] = useState<unknown>(restored.error);
  const [invalid, setInvalid] = useState('');
  const [busy, setBusy] = useState(false);
  const working = useRef(false);
  const keep = (next: PendingMachine) => { keepRecord(key, { version: 1, ...next }); setPending(next); };
  const send = async (initial: PendingMachine, retry = false) => {
    if (working.current || restored.error) return;
    working.current = true; setBusy(true); setFailure(null); setInvalid('');
    let current = initial;
    try {
      const machine = await recordComputer(initial, person, (next) => { current = next; keep(next); });
      releaseRecord(key); setPending(null);
      if (current.remote) {
        let given: JoinCode | null = null;
        let failure = '';
        let detail = '';
        try { given = await askJoinCode(machine.id); } catch (error) { failure = joinFailure(error); detail = joinDetail(error); }
        connect?.({ machine: machine.id, name: current.body.name, given, failure, at: 'row', ...(detail ? { detail } : {}) });
        changed(current.body.name + (given ? ' was added. Run the command below on that computer to connect it.' : ' was added. It is not connected yet.'), machine);
      } else changed(current.body.name + ' was added. Its runner is recorded.', machine);
    } catch (error) {
      if (!retry && current.phase === 'machine' && answeredNo(error) && error instanceof Refused && error.refusal.refusal !== 'Unanswered') {
        releaseRecord(key); setPending(null);
      }
      setFailure(error);
    }
    finally { working.current = false; setBusy(false); }
  };
  const submit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault(); if (pending || working.current || restored.error) return;
    const name = String(new FormData(event.currentTarget).get('name') ?? '').trim();
    if (!validComputerName(name)) { setInvalid('Give this computer a name, without control characters.'); return; }
    if (agent !== undefined && !/^agent-[0-9a-f]{32}$/.test(agent)) { setFailure(new Refused(0, { refusal: 'AgentIdentifierMalformed', reason: 'The agent for this addition could not be read.' })); return; }
    void send({ body: { operation: operationId(), name, kind: 'Computer', runtime: 'lys-runner', slots: 0, may_run: agent ? [agent] : agents, may_run_roles: [], may_reach: [] }, phase: 'machine', machine: null, ...(remote ? { remote: true as const } : {}) });
  };
  const blocked = busy || pending !== null || Boolean(restored.error);
  /** How the addition stands: kept and unconfirmed, with its check, or why it failed. */
  const standing = <>
    {pending ? <div role="status"><p>Adding {pending.body.name} is not confirmed. Its original request is kept.</p><Act symbol="again" name="Check whether it was added" word="Check" disabled={busy} onClick={() => void send(pending, true)} /></div> : null}
    {invalid ? <p className="why-not" role="alert">{invalid}</p> : null}
    {failure ? <ErrorWords problem={failure} /> : null}
  </>;
  return { submit, blocked, standing, pendingRemote: pending?.remote === true };
}

/** Adding the computer an agent is to run on, where no computer is in use yet: its name, and the button. */
export function AddMachine({ person, agent, changed, cancel }: Adding & { cancel: () => void }) {
  const { submit, blocked, standing } = useAddition({ person, agent, changed });
  const host = useHostName();
  return <form className="recorded-form" aria-label="Add a computer" onSubmit={submit}>
    <h2>Add this computer</h2>
    <fieldset disabled={blocked} style={{ border: 0, padding: 0, margin: 0 }}>
      <label className="field">Name<input key={host} name="name" defaultValue={host} /></label>
      <div className="chain"><Act symbol="add" name="Add this computer" word="Add" tone="primary" type="submit" /><Act symbol="close" name="Cancel" word="Cancel" onClick={cancel} /></div>
    </fieldset>
    {standing}
  </form>;
}

/**
 * The computers table's own last row, which adds one. It spans the whole table, so its name box is wide enough for a
 * computer's name and its controls are never cut off by the columns above: the name, which computer it is, and Add,
 * with how the addition stands under them.
 */
export function AddComputerRow({ person, agents, changed, name, connect, selfService = false, width = 4 }: Omit<Adding, 'agent'> & { name: RefObject<HTMLInputElement | null>; connect: (next: Connecting) => void; selfService?: boolean; width?: number }) {
  const [chosen, setChosen] = useState<'this' | 'another'>(selfService ? 'another' : 'this');
  const { submit, blocked, standing, pendingRemote } = useAddition({ person, agents, changed, remote: chosen === 'another', connect });
  // A kept addition is finished as it was begun, whatever is chosen now.
  const where = pendingRemote ? 'another' : chosen;
  const id = 'add-computer-' + person.replace(/[^A-Za-z0-9_-]/g, '_');
  const host = useHostName();
  // This computer starts with the name it gives itself; another computer's name is the person's to type.
  const offered = where === 'this' ? host : '';
  return <tr className="listing-add" data-add="computer">
    <td colSpan={width}><div className="add-computer">
      <form id={id} className="add-computer-name" aria-label="Add a computer" onSubmit={submit}>
        <label className="field">Add a computer<input key={where + ':' + offered} ref={name} name="name" aria-label="Name of the computer to add" placeholder="For example, Studio Mac mini" defaultValue={offered} disabled={blocked} /></label>
      </form>
      <div className="add-computer-which"><div className="seg" role="group" aria-label="Which computer">
        {([['this', 'This computer'], ['another', 'Another computer']] as const).map(([key, label]) =>
          <button key={key} type="button" className={where === key ? 'on' : ''} aria-pressed={where === key} disabled={blocked || (selfService && key === 'this')}
            title={selfService && key === 'this' ? 'Only an administrator adds the computer Lys runs on.' : undefined} onClick={() => setChosen(key)}>{label}</button>)}
      </div></div>
      <Act symbol="add" name={where === 'another' ? 'Add and get its code' : 'Add this computer'} word="Add" tone="primary" type="submit" form={id} disabled={blocked} />
      <p className="hint add-computer-hint">{where === 'another' ? 'You get a one-time code and a command to run on that computer to connect it.' : 'The computer Lys itself runs on, named as it names itself.'}{selfService ? ' Only an administrator adds the computer Lys runs on.' : ''}</p>
    </div>
    <div className="add-computer-standing">{standing}</div></td>
  </tr>;
}
