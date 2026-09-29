/** The Sessions screen: every running agent session the caller may see, as its runner says, each opened to its live terminal with a line to type into, common keys and Stop. A session its runner saw end is not listed; nothing is inferred from a clock. */
import { useState } from 'react';
import { useParams } from 'react-router';
import { api, request, useLoad } from '../../api';
import { Gate } from '../signin/Gate';
import { clock } from '../file/time';
import type { RuntimeSession } from './RuntimeSessions';
import { Terminal } from './Terminal';
import './terminal.css';

/** A session whose runner did not answer when it was asked, named with the refusal. */
export interface Unanswered { session: string; machine: string; refusal: string; reason: string }

function SessionName({ entry }: { entry: RuntimeSession }) {
  const name = useLoad(async () => entry.agent ? (await api.agent(entry.agent)).display_name : 'Unattached session', 'session-name:' + entry.session);
  return <strong>{name.status === 'ok' ? name.data : entry.agent ?? entry.session}</strong>;
}

export function RunningSessions() {
  const { session } = useParams();
  const [revision, setRevision] = useState(0);
  const load = useLoad(async () => {
    const answer = await request<{ sessions: RuntimeSession[]; unanswered: Unanswered[] }>('/runtime/live');
    if (!Array.isArray(answer.sessions)) throw new Error('The service did not answer a session list.');
    if (!Array.isArray(answer.unanswered)) throw new Error('The service did not say which runners did not answer.');
    if (answer.sessions.some((entry) => entry.shown === 'stopped')) throw new Error('The service listed a stopped session as running.');
    return answer;
  }, 'runtime-live:' + revision);
  const open = load.status === 'ok' ? load.data.sessions.find((entry) => entry.session === session) : undefined;
  return <div className="page runner-sessions">
    <h1>Running sessions</h1>
    <p className="sub">Choose an agent to watch or type in its terminal. Switching leaves the other sessions running.</p>
    <a className="btn" href="#/runtime/canvas">Open agent canvas</a>
    <button type="button" className="btn" onClick={() => setRevision((value) => value + 1)}>Ask the runners again</button>
    <Gate load={load} title="Running sessions" ok={({ sessions, unanswered }) => <>
      {unanswered.length ? <div className="why-not" role="alert"><h3>Runners that did not answer</h3><ul>{unanswered.map((entry) => <li key={entry.session}><span className="mono">{entry.session}</span> on {entry.machine}: {entry.refusal}: {entry.reason}</li>)}</ul></div> : null}
      {sessions.length ? <div className="session-workspace">
        <nav className="session-switcher" aria-label="Running agents">{sessions.map((entry) =>
          <a className="session-choice" key={entry.session} aria-current={entry.session === session ? 'page' : undefined} href={'#/runtime/' + encodeURIComponent(entry.session)}>
            <SessionName entry={entry} />
            <span>{entry.machine_name ?? entry.machine}</span>
            <span className="note">{unanswered.some((silent) => silent.session === entry.session) ? 'Its runner did not answer; last reported ' + entry.last_reported : entry.shown === 'running' ? 'Running' : 'Starting, not yet confirmed'}</span>
            <span className="note">Since {clock(entry.first_report_at)}</span>
          </a>)}</nav>
        <div className="session-stage">{open ? <Terminal key={open.session} session={open.session} agent={open.agent} />
          : <div className="session-empty"><h2>{session ? 'Session not returned' : 'Choose a running agent'}</h2><p>{session ? 'This session is not in the current list. Ask the runners again to refresh it.' : 'Its terminal will open here. Your input and controls use your existing permissions.'}</p></div>}</div>
      </div> : <p>No running session was returned.</p>}
    </>} />
  </div>;
}
