/**
 * Connecting another computer: a connection code is asked for, shown once with the command to run there, and never
 * kept in this tab's storage. A code whose answer is lost cannot be read again, so a new one is asked for instead,
 * and it replaces the old one.
 */
import { useState } from 'react';
import { Refused, operationId, request } from '../../api';
import { isRecord } from './contract';

/** A connection code as the service answers it, once. */
export interface JoinCode { machine: string; server: string; command: string; code: string }

/** Another computer being connected: its code when one was given just now, or why none was. */
export interface Connecting { machine: string; name: string; given: JoinCode | null; failure: string; at: 'row' | 'panel' }

export function readJoinCode(value: unknown, machine: string): JoinCode {
  if (!isRecord(value) || value.machine !== machine || typeof value.server !== 'string' || typeof value.command !== 'string'
    || typeof value.code !== 'string' || !value.code || !value.command.includes(value.server)) {
    throw new Refused(200, { refusal: 'JoinCodeUnreadable', reason: 'The answer did not carry a connection code for this computer.' });
  }
  return { machine, server: value.server, command: value.command, code: value.code };
}

/** Ask for a new connection code for `machine`, under an operation of its own: the answer is the one time it is shown. */
export async function askJoinCode(machine: string): Promise<JoinCode> {
  const answer = await request<unknown>('/network/machines/' + encodeURIComponent(machine) + '/join-code', { operation: operationId() });
  return readJoinCode(answer, machine);
}

/** Why a code was not given, in the words the add row and the panel both show. */
export const joinFailure = (error: unknown): string => error instanceof Refused ? error.refusal.refusal + ': ' + error.message : String(error);

function Copy({ text, what }: { text: string; what: string }) {
  const [said, setSaid] = useState('');
  const copy = async () => {
    try {
      if (!navigator.clipboard) throw new Error('this browser does not let the page copy');
      await navigator.clipboard.writeText(text);
      setSaid(what + ' is copied.');
    } catch (error) { setSaid(what + ' was not copied (' + (error instanceof Error ? error.message : String(error)) + '). Select it and copy it yourself.'); }
  };
  return <><button type="button" className="btn" aria-label={'Copy ' + what.toLowerCase()} onClick={() => void copy()}>Copy</button>{said ? <span role="status" className="sec"> {said}</span> : null}</>;
}

/** The command and the code, each with its Copy button, shown until the person is done with them. */
export function JoinShown({ name, given, done }: { name: string; given: JoinCode; done: () => void }) {
  return <div className="join-code" role="group" aria-label={'Connect ' + name}>
    <p>Run this on that computer. The code works once and is not shown again.</p>
    <p className="sec">The command asks for the code: paste it there.</p>
    <p><code data-join="command">{given.command}</code> <Copy text={given.command} what="The command" /></p>
    <p><code data-join="code">{given.code}</code> <Copy text={given.code} what="The code" /></p>
    <button type="button" className="btn" onClick={done}>Done</button>
  </div>;
}

/** How connecting stands: the code shown once, or why none was given. Either way a new code can be asked for. */
export function ConnectingNow({ connecting, renew, busy, done }: { connecting: Connecting; renew: () => void; busy: boolean; done: () => void }) {
  return <>
    {connecting.given ? <JoinShown name={connecting.name} given={connecting.given} done={done} /> : null}
    {connecting.failure ? <><p className="why-not" role="alert">Lys could not give a connection code for {connecting.name}. If one was given, it is not shown again: a new code replaces it.</p><p><small className="refusal-name">{connecting.failure}</small></p></> : null}
    {connecting.given ? null : <button type="button" className="btn" disabled={busy} onClick={renew}>Get a new connection code</button>}
  </>;
}

/** Ask for a new code for `machine`, telling `connect` how it went. */
export function useNewCode(machine: string, name: string, at: Connecting['at'], connect: (next: Connecting) => void) {
  const [busy, setBusy] = useState(false);
  const renew = async () => {
    if (busy) return;
    setBusy(true);
    try { connect({ machine, name, given: await askJoinCode(machine), failure: '', at }); }
    catch (error) { connect({ machine, name, given: null, failure: joinFailure(error), at }); }
    finally { setBusy(false); }
  };
  return { busy, renew: () => void renew() };
}

/** The row opened under the computers just added: its command and code, or why none was given and a way to get one. */
export function ConnectRow({ connecting, connect, done, width }: { connecting: Connecting; connect: (next: Connecting) => void; done: () => void; width: number }) {
  const { busy, renew } = useNewCode(connecting.machine, connecting.name, 'row', connect);
  return <tr className="listing-add" data-add="connect"><td colSpan={width}><ConnectingNow connecting={connecting} renew={renew} busy={busy} done={done} /></td></tr>;
}

/** The panel's own way to connect a computer again: a new code, which replaces the old one, as it says. */
export function NewCode({ machine, name, connecting, connect, done }: { machine: string; name: string; connecting: Connecting | null; connect: (next: Connecting) => void; done: () => void }) {
  const { busy, renew } = useNewCode(machine, name, 'panel', connect);
  return <div className="join-new">
    {connecting ? <ConnectingNow connecting={connecting} renew={renew} busy={busy} done={done} /> : null}
    {connecting?.failure ? null : <>
      <p className="sec">To connect it from that computer, get a connection code. A new code replaces any code given before, and the earlier one stops working.</p>
      <button type="button" className="btn" disabled={busy} onClick={renew}>Get a new connection code</button>
    </>}
  </div>;
}
