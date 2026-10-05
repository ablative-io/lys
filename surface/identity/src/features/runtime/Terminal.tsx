/** One session's terminal, read live through its runner: each read follows on from the last cursor and is answered when output arrives or the session ends, and leaving the screen closes the read. The screen takes typing itself, so there is no separate line to type into and no key buttons. Stop goes through Lys under the caller's grant; a refusal is shown by name. */
import { useState } from 'react';
import { Refused, api, request, useLoad } from '../../api';
import { clock } from '../file/time';
import { GpuTerminal } from './GpuTerminal';
import { Act } from '../../shell/Act';

export interface SessionEnd { how: 'exited' | 'ended_by_runner_restart' | 'accounts_exhausted'; at: number; status: number | null; signal: string | null }
interface EndAnswer { answer?: { kind?: string; ended?: SessionEnd } }

const asRefused = (error: unknown): Refused =>
  error instanceof Refused ? error : new Refused(0, { refusal: 'TerminalUnavailable', reason: String(error) });

function endWords(ended: SessionEnd): string {
  const how = ended.how === 'exited' ? 'The process exited' : ended.how === 'accounts_exhausted' ? 'Stopped at a usage limit on its last account' : 'Ended when its runner restarted';
  const status = ended.status !== null ? `, status ${ended.status}` : ended.signal ? `, ${ended.signal}` : ', no exit status';
  return `${how}${status}, ${clock(Math.floor(ended.at / 1000))}.`;
}

/** bare: the screen alone, which takes keystrokes itself; the caller shows the name, the state and Stop. */
export function Terminal({ session, agent, machine, bare = false }: { session: string; agent: string | null; /** The computer's name, said in the head. */ machine?: string; bare?: boolean }) {
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

  const stop = async () => {
    const answer = await send('/end', {});
    setConfirming(false);
    if (answer?.answer?.ended) setEnded(answer.answer.ended);
  };

  return <section className="terminal" aria-label={'Terminal of ' + name}>
    {bare ? null : <div className="terminal-head">
      <div><h2 title={session}>{name}</h2>{machine ? <p className="sec">on {machine}</p> : null}</div>
      <p className="terminal-state" role="status" data-ended={ended ? 'true' : 'false'} title={ended ? endWords(ended) : undefined}>{ended ? endWords(ended) : 'Running'}</p>
      {!ended ? <Act symbol="stop" name="Stop" hint={'Stop ' + name} word="Stop" className="small" data-act="stop" disabled={busy} onClick={() => setConfirming(true)} /> : null}
    </div>}
    {confirming ? <div className="terminal-stop" role="alertdialog" aria-label={'Stop ' + name}>
      <p>Stop {name}? This ends its session on the machine it runs on. What it has not saved is lost.</p>
      <div className="actions">
        <Act symbol="stop" name={'Stop ' + name} word={'Stop ' + name} tone="danger" className="small" data-act="confirm-stop" disabled={busy} onClick={() => void stop()} />
        <Act symbol="close" name="Keep it running" word="Keep it running" className="small" data-act="keep-running" onClick={() => setConfirming(false)} />
      </div>
    </div> : null}
    <GpuTerminal session={session} onEnd={setEnded} onFailure={(error) => setRefused(asRefused(error))} />
    {refused ? <p className="why-not" role="alert">{refused.refusal.refusal}: {refused.refusal.reason}</p> : null}
  </section>;
}
