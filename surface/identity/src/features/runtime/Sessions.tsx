/** Every running agent session the caller may see, as its runner says. A session its runner saw end is not listed; nothing is inferred from a clock. */
import { useParams } from 'react-router';
import { readTogether } from '../../reads';
import { api, request, useLive } from '../../api';
import { Gate } from '../signin/Gate';
import { clock } from '../file/time';
import type { RuntimeSession } from './RuntimeSessions';

/** A session whose runner did not answer when it was asked, named with the refusal. */
export interface Unanswered { session: string; machine: string; refusal: string; reason: string }

/** Everything the strip reads at once: the sessions, and the names to say them by. */
async function readRunning() {
  const { answer, people } = await readTogether({
    answer: request<{ sessions: RuntimeSession[]; unanswered: Unanswered[] }>('/runtime/live'),
    people: api.people(),
  });
  if (!Array.isArray(answer.sessions)) throw new Error('The service did not answer a session list.');
  if (!Array.isArray(answer.unanswered)) throw new Error('The service did not say which runners did not answer.');
  if (answer.sessions.some((entry) => entry.shown === 'stopped')) throw new Error('The service listed a stopped session as running.');
  return { ...answer, people };
}

/**
 * What is running: each agent with its computer and state. A name goes to that agent's terminal on the canvas, which is
 * the one place a terminal is shown. On the canvas this is behind the Agents button, never laid over the surface; `named`
 * keeps only the agents whose name holds those letters, and `go` is told the session a pressed name belongs to.
 */
export function RunningList({ named = '', go }: { named?: string; go?: (session: string) => void }) {
  const load = useLive(readRunning, 'runtime-live');
  return <Gate load={load} title="Running sessions" ok={(data) => <Running {...data} named={named} go={go} />} />;
}

function Running({ sessions, unanswered, people, named, go }: Awaited<ReturnType<typeof readRunning>> & { named: string; go?: (session: string) => void }) {
  const { agent: shown } = useParams();
  const names = new Map(people.people.flatMap((person) => person.agents.map((agent) => [agent.id, agent.display_name] as const)));
  const silent = (entry: RuntimeSession) => unanswered.some((each) => each.session === entry.session);
  const name = (entry: RuntimeSession) => entry.agent ? names.get(entry.agent) ?? 'An agent outside your view' : 'Unattached session';
  const at = (entry: RuntimeSession) => '#/canvas' + (entry.agent ? '/' + encodeURIComponent(entry.agent) : '');
  return <nav className="running-list" aria-label="Running now">
    {sessions.filter((entry) => name(entry).toLowerCase().includes(named.trim().toLowerCase())).map((entry) => <span className="running-chip" key={entry.session} data-session={entry.session} title={'Since ' + clock(entry.first_report_at)}>
      <a href={at(entry)} aria-current={entry.agent !== null && entry.agent === shown ? 'page' : undefined} onClick={() => go?.(entry.session)}>{name(entry)}</a>
      <span className="sec">{entry.machine_name ?? entry.machine}</span>
      <span className="running-state">{silent(entry) ? <span className="why-not">Its runner did not answer; last reported {entry.last_reported}</span> : entry.shown === 'running' ? <><span className="dot s-active" />Running</> : 'Starting, not yet confirmed'}</span>
    </span>)}
    {sessions.length ? null : <span className="empty dim">Nothing is running.</span>}
  </nav>;
}
