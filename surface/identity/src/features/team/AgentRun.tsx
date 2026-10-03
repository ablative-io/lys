/** An agent's run, on the People and agents page beside the list. Running: its terminal, with Stop under it. Stopped: Start, in place, with one press. There is no page of its own for this; the list is the page. */
import { useRef, useState } from 'react';
import { useNavigate } from 'react-router';
import { Refused, api, operationId, request, useLoad } from '../../api';
import type { Entry } from '../people/directory';
import type { RuntimeSession } from '../runtime/RuntimeSessions';
import { Terminal } from '../runtime/Terminal';
import { Start } from './Start';
import '../runtime/terminal.css';
import './team.css';

const asRefused = (error: unknown): Refused =>
  error instanceof Refused ? error : new Refused(0, { refusal: 'Unexpected', reason: String(error) });

/** Stop ends the run on its computer and leaves the agent ready to start again. It is not the emergency stop, which pauses the agent. */
function Stop({ entry, session, changed, done }: { entry: Entry; session: RuntimeSession; changed: () => void; done: () => void }) {
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
    <button type="button" className="btn danger" disabled={busy} onClick={() => { void stop(); }}>Stop {entry.display_name}</button>
    <button type="button" className="btn" onClick={done}>Keep it running</button>
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
      if (refusal.status >= 400 && refusal.status < 500) operation.current = '';
      setRefused(refusal);
    }
    setBusy(false);
  };
  return <div className="team-stop" role="alertdialog" aria-label={'Restart ' + entry.display_name}>
    <p>Restart {entry.display_name}? This ends the run and starts it again in the same folder, on {session.machine_name ?? session.machine}. What it has not saved is lost.</p>
    {refused ? <p role="alert" className="why-not">{refused.message} <small className="refusal-name">{refused.refusal.refusal}</small></p> : null}
    <button type="button" className="btn danger" disabled={busy} onClick={() => { void restart(); }}>Restart {entry.display_name}</button>
    <button type="button" className="btn" onClick={done}>Keep it running</button>
  </div>;
}

export function AgentRun({ entry }: { entry: Entry }) {
  const [revision, setRevision] = useState(0);
  const [asking, setAsking] = useState<'stop' | 'restart' | null>(null);
  const navigate = useNavigate();
  const settings = '/file/' + encodeURIComponent(entry.id) + '/provisioning';
  const load = useLoad(async () => {
    const [live, me, people] = await Promise.all([request<{ sessions: RuntimeSession[] }>('/runtime/live'), api.me(), api.people()]);
    return { session: live.sessions.find((one) => one.agent === entry.id && one.shown !== 'stopped'), me: me.person.id, admin: people.scope === 'directory' };
  }, 'agent-run:' + entry.id + ':' + revision);
  const changed = () => setRevision((value) => value + 1);
  if (load.status === 'loading') return null;
  if (load.status === 'refused') return <p className="why-not agent-run">Lys could not read whether {entry.display_name} is running: {load.refused.message} <small className="refusal-name">{load.refused.refusal.refusal}</small></p>;
  const { session, me, admin } = load.data;
  if (!session) return <div className="agent-run" data-live="false"><div className="team-stopped"><Start key={entry.id} entry={entry} me={me} admin={admin} changed={changed} settings={() => navigate(settings)} /></div></div>;
  return <div className="agent-run" data-live="true"><div className="team-pane">
    <div className="team-pane-body">
      <Terminal key={session.session} session={session.session} agent={entry.id} bare />
      {asking === 'stop' ? <Stop entry={entry} session={session} changed={changed} done={() => setAsking(null)} /> : null}
      {asking === 'restart' ? <Restart entry={entry} session={session} changed={changed} done={() => setAsking(null)} /> : null}
    </div>
    <div className="team-foot">
      <span className="team-foot-name">{entry.display_name}</span>
      <span className="team-foot-machine">on {session.machine_name ?? session.machine}</span>
      <a className="team-foot-act" href={'#' + settings}>Settings</a>
      <button type="button" className="team-foot-act" onClick={() => setAsking('restart')}>Restart</button>
      <button type="button" className="team-foot-act" onClick={() => setAsking('stop')}>Stop</button>
    </div>
  </div></div>;
}
