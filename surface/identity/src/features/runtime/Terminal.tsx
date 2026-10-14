/** One session's terminal, read live through its runner: each read follows on from the last cursor and is answered when output arrives or the session ends, and leaving the screen closes the read. The screen takes typing and offers the terminal keys beside it. Stop and Restart ask for confirmation; a refusal is shown by name. */
import { useRef, useState } from 'react';
import { Refused, api, operationId, request, useLoad } from '../../api';
import { clock } from '../file/time';
import { GpuTerminal } from './GpuTerminal';
import { Act } from '../../shell/Act';
import { answeredNo } from '../../kept';
import { refreshLive } from '../../live';
import { ErrorWords } from '../people/Words';

export interface SessionEnd { how: 'exited' | 'ended_by_runner_restart' | 'accounts_exhausted'; at: number; status: number | null; signal: string | null }
interface EndAnswer { answer?: { kind?: string; ended?: SessionEnd } }

const terminalKeys = [['Escape', 'escape'], ['Enter', 'enter'], ['Up', 'up'], ['Down', 'down'], ['Tab', 'tab'], ['Ctrl-C', 'ctrl_c']] as const;

const asRefused = (error: unknown): Refused =>
  error instanceof Refused ? error : new Refused(0, { refusal: 'TerminalUnavailable', reason: String(error) });

function endWords(ended: SessionEnd): string {
  const how = ended.how === 'exited' ? 'The process exited' : ended.how === 'accounts_exhausted' ? 'Stopped at a usage limit on its last account' : 'Ended when its runner restarted';
  const status = ended.status !== null ? `, status ${ended.status}` : ended.signal ? `, ${ended.signal}` : ', no exit status';
  return `${how}${status}, ${clock(Math.floor(ended.at / 1000))}.`;
}

/** bare: the screen alone, which takes keystrokes itself; the caller shows the name and state. */
export function Terminal({ session, agent, machine, bare = false }: { session: string; agent: string | null; /** The computer's name, said in the head. */ machine?: string; bare?: boolean }) {
  const base = '/runtime/sessions/' + encodeURIComponent(session);
  const [ended, setEnded] = useState<SessionEnd | null>(null);
  const [refused, setRefused] = useState<Refused | null>(null);
  const [confirming, setConfirming] = useState<'stop' | 'restart' | null>(null);
  const [busy, setBusy] = useState(false);
  const working = useRef(false);
  const restartOperation = useRef('');
  const named = useLoad(async () => (agent ? (await api.agent(agent)).display_name : session), 'terminal-agent:' + (agent ?? session));
  const name = named.status === 'ok' ? named.data : agent ?? session;

  const send = async (path: string, body: unknown) => {
    if (working.current) return null;
    working.current = true; setBusy(true);
    setRefused(null);
    try {
      return await request<EndAnswer>(path, body);
    } catch (error) {
      setRefused(asRefused(error));
      return null;
    } finally {
      working.current = false; setBusy(false);
    }
  };

  const stop = async () => {
    const answer = await send(base + '/end', {});
    setConfirming(null);
    if (answer?.answer?.ended) setEnded(answer.answer.ended);
  };

  const restart = async () => {
    if (!agent || working.current) return;
    working.current = true; setBusy(true); setRefused(null);
    try {
      restartOperation.current ||= operationId();
      await request('/agents/' + encodeURIComponent(agent) + '/restart', { session, operation: restartOperation.current });
      restartOperation.current = '';
      setConfirming(null);
      refreshLive();
    } catch (error) {
      const failure = asRefused(error);
      if (answeredNo(failure)) restartOperation.current = '';
      setRefused(failure);
    } finally { working.current = false; setBusy(false); }
  };

  return <section className="terminal" aria-label={'Terminal of ' + name}>
    {bare ? null : <div className="terminal-head">
      <div><h2 title={session}>{name}</h2>{machine ? <p className="sec">on {machine}</p> : null}</div>
      <p className="terminal-state" role="status" data-ended={ended ? 'true' : 'false'} title={ended ? endWords(ended) : undefined}>{ended ? endWords(ended) : 'Running'}</p>
      {agent ? <Act symbol="retry" name="Restart" hint={'Restart ' + name} word="Restart" className="small" data-act="restart" disabled={busy} onClick={() => setConfirming('restart')} /> : null}
      {!ended ? <Act symbol="stop" name="Stop" hint={'Stop ' + name} word="Stop" className="small" data-act="stop" disabled={busy} onClick={() => setConfirming('stop')} /> : null}
    </div>}
    {confirming === 'stop' ? <div className="terminal-stop" role="alertdialog" aria-label={'Stop ' + name}>
      <p>Stop {name}? This ends its session on the machine it runs on. What it has not saved is lost.</p>
      <div className="actions">
        <Act symbol="stop" name={'Stop ' + name} word={'Stop ' + name} tone="danger" className="small" data-act="confirm-stop" disabled={busy} onClick={() => void stop()} />
        <Act symbol="close" name="Keep it running" word="Keep it running" className="small" data-act="keep-running" onClick={() => setConfirming(null)} />
      </div>
    </div> : null}
    {confirming === 'restart' ? <div className="terminal-stop" role="alertdialog" aria-label={'Restart ' + name}>
      <p>Restart {name}? This ends its session and starts it again in the same folder{machine ? ', on ' + machine : ''}. What it has not saved is lost.</p>
      <div className="actions">
        <Act symbol="retry" name={'Restart ' + name} word={'Restart ' + name} tone="danger" className="small" data-act="confirm-restart" disabled={busy} onClick={() => void restart()} />
        <Act symbol="close" name="Keep it running" word="Keep it running" className="small" data-act="cancel-restart" disabled={busy} onClick={() => setConfirming(null)} />
      </div>
    </div> : null}
    {bare && agent ? <Act symbol="retry" name="Restart" word="Restart" data-act="restart" disabled={busy} onClick={() => setConfirming('restart')} /> : null}
    <div className="terminal-keys" role="group" aria-label="Terminal keys">
      {terminalKeys.map(([label, key]) => <Act key={key} symbol="enter" name={label} word={label} data-key={key} disabled={busy || Boolean(ended)} onClick={() => void send(base + '/keys', { keys: [key] })} />)}
    </div>
    <GpuTerminal session={session} onEnd={setEnded} onFailure={(error) => setRefused(asRefused(error))} />
    {refused ? <ErrorWords problem={refused} /> : null}
  </section>;
}
