/** One session's terminal, read live through its runner: each read follows on from the last cursor and is answered when output arrives or the session ends, and leaving the screen closes the read. The screen takes typing and offers the terminal keys beside it. Stop and Restart ask for confirmation; a refusal is shown by name. */
import { useRef, useState } from 'react';
import { Refused, api, request, useLoad } from '../../api';
import { clock } from '../file/time';
import { GpuTerminal } from './GpuTerminal';
import { Act } from '../../shell/Act';
import { refreshLive } from '../../live';
import { ErrorWords } from '../people/Words';
import { RUN_WORDS, RunConfirm, useRunActs } from '../run/RunConfirm';

export interface SessionEnd { how: 'exited' | 'ended_by_runner_restart' | 'accounts_exhausted'; at: number; status: number | null; signal: string | null }
interface EndAnswer { answer?: { kind?: string; ended?: SessionEnd } }

// The keys a phone's keyboard has no way to send, as the runner names them: the line keys are here so a line can be
// edited without a Mac's Command and Option keys.
const terminalKeys = [['Escape', 'escape'], ['Enter', 'enter'], ['Up', 'up'], ['Down', 'down'], ['Left', 'left'], ['Right', 'right'],
  ['Line start', 'home'], ['Line end', 'end'], ['Tab', 'tab'], ['Ctrl-C', 'ctrl_c']] as const;

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

  const acts = useRunActs(agent, session, (doing, end) => {
    setConfirming(null);
    if (doing === 'stop' && end) setEnded(end);
    if (doing === 'restart') refreshLive();
  });
  const held = busy || acts.busy;

  return <section className="terminal" aria-label={'Terminal of ' + name}>
    {bare ? null : <div className="terminal-head">
      <div><h2 title={session}>{name}</h2>{machine ? <p className="sec">on {machine}</p> : null}</div>
      <p className="terminal-state" role="status" data-ended={ended ? 'true' : 'false'} title={ended ? endWords(ended) : undefined}>{ended ? endWords(ended) : 'Running'}</p>
      {agent ? <Act symbol="retry" name={RUN_WORDS.restart} hint={RUN_WORDS.restart + ' ' + name} word={RUN_WORDS.restart} className="small" data-act="restart" disabled={held} onClick={() => setConfirming('restart')} /> : null}
      {!ended ? <Act symbol="stop" name={RUN_WORDS.stop} hint={RUN_WORDS.stop + ' ' + name} word={RUN_WORDS.stop} className="small" data-act="stop" disabled={held} onClick={() => setConfirming('stop')} /> : null}
    </div>}
    {confirming ? <RunConfirm className="terminal-stop" small doing={confirming} name={name} where={machine ?? null} busy={held} refused={acts.refused}
      waitingSince={acts.waitingSince} endingNow={acts.endingNow} endNow={() => { void acts.endNow(); }}
      confirm={() => { void acts.act(confirming); }} cancel={() => { acts.clear(); setConfirming(null); }} /> : null}
    {bare && agent ? <Act symbol="retry" name={RUN_WORDS.restart} word={RUN_WORDS.restart} data-act="restart" disabled={held} onClick={() => setConfirming('restart')} /> : null}
    <div className="terminal-keys" role="group" aria-label="Terminal keys">
      {terminalKeys.map(([label, key]) => <Act key={key} symbol="enter" name={label} word={label} data-key={key} disabled={busy || Boolean(ended)} onClick={() => void send(base + '/keys', { keys: [key] })} />)}
    </div>
    <GpuTerminal session={session} onEnd={setEnded} onFailure={(error) => setRefused(asRefused(error))} />
    {refused ? <ErrorWords problem={refused} /> : null}
  </section>;
}
