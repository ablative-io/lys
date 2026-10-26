/**
 * The one Stop and Restart of an agent's run: one set of words and one pair of requests, used by the agent's own page
 * and by its terminal. Stop waits for the session's exit (`/runtime/sessions/{session}/stop`) and leaves the agent ready to start
 * again; it is not the emergency stop, which pauses the agent. Restart (`/agents/{agent}/restart`) ends the session and
 * starts it again in the same folder, on the same computer, from its approved settings; its operation is kept while
 * the outcome is unknown, so a second press sends the same request, never a second restart.
 */
import { useEffect, useRef, useState } from 'react';
import { Refused, operationId, request } from '../../api';
import { answeredNo } from '../../kept';
import { Act } from '../../shell/Act';
import { ErrorWords } from '../people/Words';
import type { SessionEnd } from '../runtime/Terminal';
import './run.css';
import { StopWaiting } from './StopWaiting';

export type RunDoing = 'stop' | 'restart';

/** The word for each act on a run, the same wherever an agent is started, stopped or restarted. */
export const RUN_WORDS = { start: 'Start', stop: 'Stop', restart: 'Restart' } as const;

const asRefused = (error: unknown): Refused =>
  error instanceof Refused ? error : new Refused(0, { refusal: 'Unexpected', reason: String(error) });

/** What the person is asked before the act is sent: what ends, where, and what is lost. */
export function runQuestion(doing: RunDoing, name: string, where: string | null): string {
  const on = where ? ' on ' + where : '';
  return doing === 'stop'
    ? 'Stop ' + name + '? This ends its run' + on + '. What it has not saved is lost. You can start it again afterwards.'
    : 'Restart ' + name + '? This ends its run and starts it again in the same folder' + (where ? ', on ' + where : '') + '. What it has not saved is lost.';
}

/** What a glance at the question must not miss: Lys saves nothing first, and the work is kept only by not stopping. */
export const runLoss = (name: string): string =>
  'Lys does not save ' + name + '’s work first. To keep it, choose Keep it running and let ' + name + ' finish.';

/** Stop and Restart for one session of `agent`. `done` hears each confirmed act, with the session's end when the runner said it. */
export function useRunActs(agent: string | null, session: string, done: (doing: RunDoing, ended: SessionEnd | null) => void) {
  const [busy, setBusy] = useState(false);
  const [refused, setRefused] = useState<Refused | null>(null);
  const [waitingSince, setWaitingSince] = useState<number | null>(null);
  const [endingNow, setEndingNow] = useState(false);
  const callers = useRef(new Set<AbortController>());
  const alive = useRef(true);
  useEffect(() => {
    alive.current = true;
    return () => { alive.current = false; for (const caller of callers.current) caller.abort(); };
  }, []);
  const working = useRef(false);
  const restartOperation = useRef('');
  const act = async (doing: RunDoing) => {
    if (working.current || (doing === 'restart' && !agent)) return;
    working.current = true; setBusy(true); setRefused(null);
    const caller = new AbortController(); callers.current.add(caller);
    try {
      if (doing === 'stop') {
        setWaitingSince(Date.now());
        const answer = await request<{ answer?: { ended?: SessionEnd } }>('/runtime/sessions/' + encodeURIComponent(session) + '/stop', {}, 'POST', caller.signal);
        if (alive.current) done('stop', answer?.answer?.ended ?? null);
      } else {
        restartOperation.current ||= operationId();
        await request('/agents/' + encodeURIComponent(agent ?? '') + '/restart', { session, operation: restartOperation.current }, 'POST', caller.signal);
        restartOperation.current = '';
        if (alive.current) done('restart', null);
      }
    } catch (error) {
      const refusal = asRefused(error);
      if (doing === 'restart' && answeredNo(refusal)) restartOperation.current = '';
      if (alive.current && !caller.signal.aborted) setRefused(refusal);
    } finally { callers.current.delete(caller); working.current = false; if (alive.current) { setBusy(false); setWaitingSince(null); } }
  };
  const endNow = async () => {
    if (endingNow) return;
    setEndingNow(true);
    const caller = new AbortController(); callers.current.add(caller);
    try {
      const answer = await request<{ answer?: { ended?: SessionEnd } }>('/runtime/sessions/' + encodeURIComponent(session) + '/end-now', {}, 'POST', caller.signal);
      if (alive.current) done('stop', answer?.answer?.ended ?? null);
    } catch (error) { if (alive.current && !caller.signal.aborted) setRefused(asRefused(error)); }
    finally { callers.current.delete(caller); if (alive.current) setEndingNow(false); }
  };
  return { busy, refused, act, waitingSince, endingNow, endNow, clear: () => setRefused(null) };
}

/** The question before Stop or Restart, with the act and "Keep it running". `className` places it where it is shown. */
export function RunConfirm({ doing, name, where, busy, refused, confirm, cancel, className, small = false, waitingSince, endingNow = false, endNow }: {
  doing: RunDoing; name: string; where: string | null; busy: boolean; refused: Refused | null;
  confirm: () => void; cancel: () => void; className: string; small?: boolean;
  waitingSince?: number | null; endingNow?: boolean; endNow?: () => void;
}) {
  const word = RUN_WORDS[doing];
  return <div className={className} role="alertdialog" aria-label={word + ' ' + name}>
    <p>{runQuestion(doing, name, where)} <strong className="run-loss">{runLoss(name)}</strong></p>
    {refused ? <ErrorWords problem={refused} /> : null}
    {waitingSince != null && endNow ? <StopWaiting since={waitingSince} ending={endingNow} endNow={endNow} /> : null}
    <div className="actions">
      <Act symbol={doing === 'stop' ? 'stop' : 'retry'} name={word + ' ' + name} word={word} tone="danger" className={small ? 'small' : undefined} data-act={'confirm-' + doing} disabled={busy} onClick={confirm} />
      <Act symbol="close" name="Keep it running" word="Keep it running" className={small ? 'small' : undefined} data-act="keep-running" disabled={busy} onClick={cancel} />
    </div>
  </div>;
}
