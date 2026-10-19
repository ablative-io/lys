/**
 * Runtime observations are displayed as reported; absence and copied commands never imply a process state. On an
 * agent's own Sessions tab each session not confirmed stopped has the ordinary Stop beside it (AGENTS-017 R2): it ends
 * that run and keeps the agent's certificate.
 */
import { api, request, useLoad } from '../../api';
import { SessionStop } from '../run/SessionStop';
import { entries } from '../people/directory';
import { Gate } from '../signin/Gate';
import { clock } from '../file/time';
export interface SeatProgramEvidence {
  provisioning_version: number; contract: 'seat-program/v1'; executable: string; seat_file: string;
  reviewed_executable_sha256: string; reviewed_seat_file_sha256: string;
  found_executable_sha256: string; found_seat_file_sha256: string;
}
export interface RuntimeSession {
  session: string; agent: string | null; machine: string; machine_name: string | null; runtime: string | null;
  shown: 'unconfirmed' | 'running' | 'stopped'; last_reported: string; first_report_at: number; last_report_at: number;
  what: string; stopped: null | { at: number; confirmation: string }; reported_by: string; stop_asked_at?: number | null;
  seat_program?: SeatProgramEvidence | null;
}
export function RuntimeSessions({ agent, found = false }: { agent?: string; found?: boolean }) {
  const path = agent ? '/agents/' + encodeURIComponent(agent) + '/runtime/sessions' : found ? '/runtime/found' : '/runtime/sessions';
  const load = useLoad(async () => {
    const answer = await request<{ sessions: RuntimeSession[] }>(path);
    if (!Array.isArray(answer.sessions)) throw new Error('The runtime did not answer a session list.');
    for (const session of answer.sessions) {
      if ((agent && session.agent !== agent) || (found && session.agent !== null)) throw new Error('The runtime answered a session outside this view.');
      if (!['unconfirmed', 'running', 'stopped'].includes(session.shown) || (session.shown === 'stopped' && !session.stopped?.confirmation)) throw new Error('The runtime did not give a valid status and stop confirmation.');
    }
    const names = new Map(entries(await api.people()).map((entry) => [entry.id, entry.display_name]));
    return { ...answer, names };
  }, path);
  return <section className="card" aria-label={found ? 'Sessions with no agent' : undefined}>{found ? <h2>Sessions with no agent</h2> : null}<p>{found ? 'Sessions a runner reported that belong to no agent in the directory. A report does not add an agent or give access.' : 'What each computer\'s Lys runner last reported about these sessions.'}</p>
    <Gate load={load} title="Sessions" ok={({ sessions, names }) => sessions.length ? <table><thead><tr><th>Agent</th><th>Computer</th><th>State</th><th>Last report</th>{agent ? <th>Stop</th> : null}</tr></thead><tbody>{sessions.map((session) => <tr key={session.session}>
      <td>{session.agent ? <a href={'#/file/' + session.agent} title={session.agent}>{names.get(session.agent) ?? 'An agent outside your view'}</a> : <>No agent attached <span className="mono dim">{session.session}</span></>}</td>
      <td>{session.machine_name ?? session.machine}</td>
      <td>{session.shown === 'running' ? 'Running, as its runner reported' : session.shown === 'stopped' ? 'Stopped, as its runner confirmed' : 'Not yet confirmed by its runner'}{session.stopped ? <p>{session.stopped.confirmation}</p> : null}{session.stop_asked_at && !session.stopped ? <p className="why-not">Emergency stop asked {clock(session.stop_asked_at)}; its runner has not confirmed it ended.</p> : null}</td>
      <td>{clock(session.last_report_at)}<p>{session.what}</p><p className="note">Reported by {names.get(session.reported_by) ?? 'a runner outside your view'}. First report {clock(session.first_report_at)}.</p></td>
      {agent ? <td>{session.shown !== 'stopped' ? <SessionStop agent={agent} session={session.session} name={names.get(agent) ?? 'this agent'} where={session.machine_name ?? session.machine} /> : null}</td> : null}
    </tr>)}</tbody></table> : <p>No runner has reported a session here. That does not show whether a process is running.</p>} />
  </section>;
}
