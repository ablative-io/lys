/** One session's terminal, read live through its runner: each read follows on from the last cursor and is answered when output arrives or the session ends, and leaving the screen closes the read. Typed lines, keys and Stop each go through Lys under the caller's grant; a refusal is shown by name. */
import { useState } from 'react';
import type { FormEvent } from 'react';
import { Refused, api, request, useLoad } from '../../api';
import { clock } from '../file/time';
import { GpuTerminal } from './GpuTerminal';

export interface SessionEnd { how: 'exited' | 'ended_by_runner_restart' | 'accounts_exhausted'; at: number; status: number | null; signal: string | null }
interface EndAnswer { answer?: { kind?: string; ended?: SessionEnd } }

/** The keys offered in one press each, by the name the runner takes. */
export const KEYS: readonly (readonly [string, string])[] = [
  ['enter', 'Enter'], ['tab', 'Tab'], ['escape', 'Esc'], ['up', '↑'], ['down', '↓'], ['ctrl_c', 'Ctrl-C'], ['ctrl_d', 'Ctrl-D'],
];

const asRefused = (error: unknown): Refused =>
  error instanceof Refused ? error : new Refused(0, { refusal: 'TerminalUnavailable', reason: String(error) });

function endWords(ended: SessionEnd): string {
  const how = ended.how === 'exited' ? 'The process exited' : ended.how === 'accounts_exhausted' ? 'Stopped at a usage limit on its last account' : 'Ended when its runner restarted';
  const status = ended.status !== null ? `, status ${ended.status}` : ended.signal ? `, ${ended.signal}` : ', no exit status';
  return `${how}${status}, ${clock(Math.floor(ended.at / 1000))}.`;
}

/** bare: the screen, the line and the keys only; the caller shows the name, the state and Stop itself. */
export function Terminal({ session, agent, bare = false }: { session: string; agent: string | null; bare?: boolean }) {
  const base = '/runtime/sessions/' + encodeURIComponent(session);
  const [ended, setEnded] = useState<SessionEnd | null>(null);
  const [refused, setRefused] = useState<Refused | null>(null);
  const [confirming, setConfirming] = useState(false);
  const [busy, setBusy] = useState(false);
  const named = useLoad(async () => (agent ? (await api.agent(agent)).display_name : session), 'terminal-agent:' + (agent ?? session));
  const name = named.status === 'ok' ? named.data : agent ?? session;

  const send = async (path: string, body: unknown) => {
    setBusy(true);
    setRefused(null);
    try {
      return await request<EndAnswer>(base + path, body);
    } catch (error) {
      setRefused(asRefused(error));
      return null;
    } finally {
      setBusy(false);
    }
  };

  const typed = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const form = event.currentTarget;
    const line = String(new FormData(form).get('line') ?? '');
    if (await send('/input', { text: line, enter: true })) form.reset();
  };

  const stop = async () => {
    const answer = await send('/end', {});
    setConfirming(false);
    if (answer?.answer?.ended) setEnded(answer.answer.ended);
  };

  return <section className="terminal" aria-label={'Terminal of ' + name}>
    {bare ? null : <div className="terminal-head">
      <div><h2>{name}</h2><p className="mono">{session}</p></div>
      <p className="terminal-state" role="status" data-ended={ended ? 'true' : 'false'}>{ended ? endWords(ended) : 'Runner session'}</p>
      {!ended ? <button type="button" className="btn" data-act="stop" disabled={busy} onClick={() => setConfirming(true)}>Stop</button> : null}
    </div>}
    {confirming ? <div className="terminal-stop" role="alertdialog" aria-label={'Stop ' + name}>
      <p>Stop {name}? This ends its session on the machine it runs on. What it has not saved is lost.</p>
      <div className="actions">
        <button type="button" className="btn danger" data-act="confirm-stop" disabled={busy} onClick={() => void stop()}>Stop {name}</button>
        <button type="button" className="btn" data-act="keep-running" onClick={() => setConfirming(false)}>Keep it running</button>
      </div>
    </div> : null}
    <GpuTerminal session={session} onEnd={setEnded} onFailure={(error) => setRefused(asRefused(error))} />
    {!ended ? <>
      <form className="terminal-line" aria-label="Type to the session" onSubmit={(event) => void typed(event)}>
        <input name="line" aria-label="Line to type" autoComplete="off" spellCheck={false} disabled={busy} />
        <button type="submit" className="btn primary" disabled={busy}>Send</button>
      </form>
      <div className="terminal-keys" aria-label="Keys">
        {KEYS.map(([key, label]) => <button type="button" className="btn" key={key} data-key={key} disabled={busy} onClick={() => void send('/keys', { keys: [key] })}>{label}</button>)}
      </div>
    </> : null}
    {refused ? <p className="why-not" role="alert">{refused.refusal.refusal}: {refused.refusal.reason}</p> : null}
  </section>;
}
