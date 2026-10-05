/** An agent's run, on the agent's own page. Running: where, with Restart and Stop, and a link to its terminal on the canvas. Stopped: Start, in place, with one press. */
import { useRef, useState } from 'react';
import { useNavigate } from 'react-router';
import { Refused, api, operationId, request, useLoad } from '../../api';
import type { Entry } from '../people/directory';
import type { RuntimeSession } from '../runtime/RuntimeSessions';
import { Start } from './Start';
import './team.css';
import { answeredNo } from '../../kept';
import { Act } from '../../shell/Act';

const asRefused = (error: unknown): Refused =>
  error instanceof Refused ? error : new Refused(0, { refusal: 'Unexpected', reason: String(error) });

/** Stop ends the run on its computer and leaves the agent ready to start again. It is not the emergency stop, which pauses the agent. */
export function Stop({ entry, session, changed, done }: { entry: Entry; session: RuntimeSession; changed: () => void; done: () => void }) {
  const [busy, setBusy] = useState(false);
  const [refused, setRefused] = useState<Refused | null>(null);
  const stop = async () => {
    setBusy(true); setRefused(null);
    try {
      await request('/runtime/sessions/' + encodeURIComponent(session.session) + '/end', {});
      done();
      changed();
    } catch (error) { setRefused(asRefused(error)); }
    setBusy(false);
  };
  return <div className="team-stop" role="alertdialog" aria-label={'Stop ' + entry.display_name}>
    <p>Stop {entry.display_name}? What it has not saved is lost. You can start it again afterwards.</p>
    {refused ? <p role="alert" className="why-not">{refused.message} <small className="refusal-name">{refused.refusal.refusal}</small></p> : null}
    <Act symbol="stop" name={'Stop ' + entry.display_name} word="Stop" tone="danger" disabled={busy} onClick={() => { void stop(); }} />
    <Act symbol="close" name="Keep it running" word="Keep running" onClick={done} />
  </div>;
}

/** Restart ends the run and starts it again in the same folder, on the same computer, from its approved settings. */
function Restart({ entry, session, changed, done }: { entry: Entry; session: RuntimeSession; changed: () => void; done: () => void }) {
  const [busy, setBusy] = useState(false);
  const [refused, setRefused] = useState<Refused | null>(null);
  // One request, kept while this is shown, so a second press after no answer sends the same one.
  const operation = useRef('');
  const restart = async () => {
    setBusy(true); setRefused(null);
    try {
      operation.current ||= operationId();
      await request('/agents/' + encodeURIComponent(entry.id) + '/restart', { session: session.session, operation: operation.current });
      operation.current = '';
      done();
      changed();
    } catch (error) {
      const refusal = asRefused(error);
      if (answeredNo(refusal)) operation.current = '';
      setRefused(refusal);
    }
    setBusy(false);
  };
  return <div className="team-stop" role="alertdialog" aria-label={'Restart ' + entry.display_name}>
    <p>Restart {entry.display_name}? This ends the run and starts it again in the same folder, on {session.machine_name ?? session.machine}. What it has not saved is lost.</p>
    {refused ? <p role="alert" className="why-not">{refused.message} <small className="refusal-name">{refused.refusal.refusal}</small></p> : null}
    <Act symbol="retry" name={'Restart ' + entry.display_name} word="Restart" tone="danger" disabled={busy} onClick={() => { void restart(); }} />
    <Act symbol="close" name="Keep it running" word="Keep running" onClick={done} />
  </div>;
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
  if (load.status === 'refused') return <p className="why-not agent-run">Lys could not read whether {entry.display_name} is running: {load.refused.message} <small className="refusal-name">{load.refused.refusal.refusal}</small></p>;
  const { session, silent, me, admin } = load.data;
  if (!session) return <section className="agent-run" aria-label="Run" data-live="false"><Start key={entry.id} entry={entry} me={me} admin={admin} changed={changed} settings={() => navigate(settings)} /></section>;
  const where = session.machine_name ?? session.machine;
  return <section className="agent-run" aria-label="Run" data-live="true">
    <div className="agent-run-line">
      <span className="agent-run-words">{session.shown === 'running' && !silent ? entry.display_name + ' is running on ' + where + ', as its runner last reported.'
        : 'Whether ' + entry.display_name + ' is still running on ' + where + ' is not confirmed: its runner has not answered since its last report.'}</span>
      <a className="btn" data-act="watch" href={'#/canvas/' + encodeURIComponent(entry.id)}>Open its terminal</a>
      <Act symbol="retry" name="Restart" word="Restart" data-act="restart" onClick={() => setAsking('restart')} />
      <Act symbol="stop" name="Stop" word="Stop" tone="danger" data-act="stop" onClick={() => setAsking('stop')} />
    </div>
    {session.stop_asked_at && !session.stopped ? <p className="why-not">A stop was requested. The runner has not confirmed it ended.</p> : null}
    {asking === 'stop' ? <Stop entry={entry} session={session} changed={changed} done={() => setAsking(null)} /> : null}
    {asking === 'restart' ? <Restart entry={entry} session={session} changed={changed} done={() => setAsking(null)} /> : null}
  </section>;
}
