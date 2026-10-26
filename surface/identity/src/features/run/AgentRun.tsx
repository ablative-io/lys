/** An agent's run, on the agent's own page. Running: where, with Restart and Stop, and a link to its terminal on the canvas. Stopped: Start, in place, with one press. */
import { useState } from 'react';
import { useNavigate } from 'react-router';
import { api, request, useLoad } from '../../api';
import type { Entry } from '../people/directory';
import type { RuntimeSession } from '../runtime/RuntimeSessions';
import { Start } from './Start';
import './run.css';
import { Act } from '../../shell/Act';
import { ErrorWords } from '../people/Words';
import { SeatProgramChange } from '../provisioning/SeatProgramChange';
import { RUN_WORDS, RunConfirm, useRunActs } from './RunConfirm';
import type { RunDoing } from './RunConfirm';

/** Stop or Restart, asked first, on the agent's own page and on the dashboard: the one question and the one pair of requests. */
function RunAsk({ doing, entry, session, changed, done }: { doing: RunDoing; entry: Entry; session: RuntimeSession; changed: () => void; done: () => void }) {
  const acts = useRunActs(entry.id, session.session, () => { done(); changed(); });
  return <RunConfirm className="run-stop" doing={doing} name={entry.display_name} where={session.machine_name ?? session.machine} busy={acts.busy} refused={acts.refused}
    waitingSince={acts.waitingSince} endingNow={acts.endingNow} endNow={() => { void acts.endNow(); }}
    confirm={() => { void acts.act(doing); }} cancel={done} />;
}

/** Stop ends the run on its computer and leaves the agent ready to start again. It is not the emergency stop, which pauses the agent. */
export function Stop(props: { entry: Entry; session: RuntimeSession; changed: () => void; done: () => void }) {
  return <RunAsk doing="stop" {...props} />;
}

export function AgentRun({ entry }: { entry: Entry }) {
  const [revision, setRevision] = useState(0);
  const [asking, setAsking] = useState<'stop' | 'restart' | null>(null);
  const navigate = useNavigate();
  const settings = '/file/' + encodeURIComponent(entry.id) + '/provisioning';
  const load = useLoad(async () => {
    const [live, me, people] = await Promise.all([request<{ sessions: RuntimeSession[]; unanswered?: { session: string }[] }>('/runtime/live'), api.me(), api.people()]);
    const session = live.sessions.find((one) => one.agent === entry.id && one.shown !== 'stopped');
    return { session, silent: !!session && (live.unanswered ?? []).some((each) => each.session === session.session), me: me.person.id, admin: people.scope === 'directory' };
  }, 'agent-run:' + entry.id + ':' + revision);
  const changed = () => setRevision((value) => value + 1);
  if (load.status === 'loading') return null;
  if (load.status === 'refused') return <section className="agent-run" aria-label="Run"><p className="why-not">Lys could not read whether {entry.display_name} is running.</p><ErrorWords problem={load.refused} />
    <Act symbol="retry" name="Read it again" word="Try again" onClick={changed} /></section>;
  const { session, silent, me, admin } = load.data;
  if (!session) return <section className="agent-run" aria-label="Run" data-live="false"><Start key={entry.id} entry={entry} me={me} admin={admin} changed={changed} settings={() => navigate(settings)} /></section>;
  const where = session.machine_name ?? session.machine;
  return <section className="agent-run" aria-label="Run" data-live="true">
    <div className="agent-run-line">
      <span className="agent-run-words">{session.shown === 'running' && !silent ? entry.display_name + ' is running on ' + where + ', as its runner last reported.'
        : 'Whether ' + entry.display_name + ' is still running on ' + where + ' is not confirmed: its runner has not answered since its last report.'}</span>
      <a className="btn" data-act="watch" href={'#/canvas/' + encodeURIComponent(entry.id)}>Open its terminal</a>
      <Act symbol="retry" name={RUN_WORDS.restart} word={RUN_WORDS.restart} data-act="restart" onClick={() => setAsking('restart')} />
      <Act symbol="stop" name={RUN_WORDS.stop} word={RUN_WORDS.stop} tone="danger" data-act="stop" onClick={() => setAsking('stop')} />
    </div>
    {session.stop_asked_at && !session.stopped ? <p className="why-not">A stop was requested. The runner has not confirmed it ended.</p> : null}
    {session.seat_program ? <SeatProgramChange key={session.session + ':' + session.seat_program.provisioning_version} id={entry.id} evidence={session.seat_program} canReview={admin || entry.person?.id === me} changed={changed} /> : null}
    {asking === 'stop' ? <Stop entry={entry} session={session} changed={changed} done={() => setAsking(null)} /> : null}
    {asking === 'restart' ? <RunAsk doing="restart" entry={entry} session={session} changed={changed} done={() => setAsking(null)} /> : null}
  </section>;
}
