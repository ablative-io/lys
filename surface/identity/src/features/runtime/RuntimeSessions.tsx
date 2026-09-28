/** Runtime observations are displayed as reported; absence and copied commands never imply a process state. */
import { useState } from 'react';
import { request, useLoad } from '../../api';
import { Gate } from '../signin/Gate';
import { clock } from '../file/time';
export interface RuntimeSession {
  session: string; agent: string | null; machine: string; machine_name: string | null; runtime: string | null;
  shown: 'unconfirmed' | 'running' | 'stopped'; last_reported: string; first_report_at: number; last_report_at: number;
  what: string; stopped: null | { at: number; confirmation: string }; reported_by: string;
}
export function RuntimeSessions({ agent, found = false }: { agent?: string; found?: boolean }) {
  const [revision, setRevision] = useState(0);
  const path = agent ? '/agents/' + encodeURIComponent(agent) + '/runtime/sessions' : found ? '/runtime/found' : '/runtime/sessions';
  const load = useLoad(async () => {
    const answer = await request<{ sessions: RuntimeSession[] }>(path);
    if (!Array.isArray(answer.sessions)) throw new Error('The runtime did not answer a session list.');
    for (const session of answer.sessions) {
      if ((agent && session.agent !== agent) || (found && session.agent !== null)) throw new Error('The runtime answered a session outside this view.');
      if (!['unconfirmed', 'running', 'stopped'].includes(session.shown) || (session.shown === 'stopped' && !session.stopped?.confirmation)) throw new Error('The runtime did not give a valid status and stop confirmation.');
    }
    return answer;
  }, path + ':' + revision);
  return <section className="card"><div className="head"><div><h2>{found ? 'Found sessions' : 'Agent runtime sessions'}</h2><p>{found ? 'Sessions reported without an identity. A report does not register an agent or grant access.' : 'The latest reports from runtimes. A copied start command is not confirmation that an agent is running.'}</p></div><button className="btn" onClick={() => setRevision((value) => value + 1)}>Refresh runtime reports</button></div>
    <Gate load={load} title="Runtime reports" ok={({ sessions }) => sessions.length ? <table><thead><tr><th>Session</th><th>Machine</th><th>Reported state</th><th>Latest report</th></tr></thead><tbody>{sessions.map((session) => <tr key={session.session}>
      <td><span className="mono">{session.session}</span>{session.agent ? <p><a href={'#/file/' + session.agent}>Agent file</a></p> : <p>No identity attached</p>}</td>
      <td>{session.machine_name ?? session.machine}<p className="note">{session.runtime ?? 'Runtime name not recorded'}</p></td>
      <td>{session.shown === 'running' ? 'Reported running' : session.shown === 'stopped' ? 'Stop confirmed' : 'Unconfirmed'}{session.stopped ? <p>{session.stopped.confirmation}</p> : null}</td>
      <td>{clock(session.last_report_at)}<p>{session.what}</p><details><summary>Report evidence</summary><dl className="facts"><dt>Reported by</dt><dd>{session.reported_by}</dd><dt>First report</dt><dd>{clock(session.first_report_at)}</dd><dt>Last state</dt><dd>{session.last_reported}</dd><dt>Machine identifier</dt><dd>{session.machine}</dd></dl></details></td>
    </tr>)}</tbody></table> : <p>No runtime reports were returned. This does not establish whether a process is running.</p>} />
  </section>;
}
