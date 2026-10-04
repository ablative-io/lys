import { readTogether } from '../../reads';
/** Every running agent session the caller may see, as its runner says. A session its runner saw end is not listed; nothing is inferred from a clock. */
import { useNavigate, useParams } from 'react-router';
import { Listing } from '../../shell/Listing';
import type { Column } from '../../shell/Listing';
import { groupByTeam, inWhose } from '../../shell/org';
import type { Held } from '../../shell/org';
import { useWhose, WhoseSelect } from '../../shell/Whose';
import { problemWords } from '../people/Words';
import { readTeams } from '../teams/Teams';
import { api, request, useLive } from '../../api';
import { Gate } from '../signin/Gate';
import { clock } from '../file/time';
import type { RuntimeSession } from './RuntimeSessions';

/** A session whose runner did not answer when it was asked, named with the refusal. */
export interface Unanswered { session: string; machine: string; refusal: string; reason: string }

/** Everything the screen reads at once: the sessions, and the names and teams to group them by. */
async function readRunning() {
  const { answer, people, me, teams } = await readTogether({
    answer: request<{ sessions: RuntimeSession[]; unanswered: Unanswered[] }>('/runtime/live'),
    people: api.people(), me: api.me(),
    teams: readTeams().then((list) => ({ list, refused: '' }), (problem: unknown) => ({ list: [], refused: problemWords(problem) })),
  });
  if (!Array.isArray(answer.sessions)) throw new Error('The service did not answer a session list.');
  if (!Array.isArray(answer.unanswered)) throw new Error('The service did not say which runners did not answer.');
  if (answer.sessions.some((entry) => entry.shown === 'stopped')) throw new Error('The service listed a stopped session as running.');
  return { ...answer, people, me, teams };
}

/** What is running, down the left of the canvas: each agent with its computer, state and since when. A row opens that agent's terminal on the canvas, which is the one place a terminal is shown. */
export function RunningList() {
  const load = useLive(readRunning, 'runtime-live');
  return <Gate load={load} title="Running sessions" ok={(data) => <Running {...data} />} />;
}

function Running({ sessions, unanswered, people, me, teams }: Awaited<ReturnType<typeof readRunning>>) {
  const admin = people.scope === 'directory';
  const [whose, setWhose] = useWhose(admin);
  const navigate = useNavigate();
  const { agent: shown } = useParams();
  const agents = new Map(people.people.flatMap((person) => person.agents.map((agent) => [agent.id, { name: agent.display_name, person: person.id }] as const)));
  const names = new Map([...people.people.map((person) => [person.id, person.display_name] as const), ...[...agents].map(([id, agent]) => [id, agent.name] as const)]);
  const held = (entry: RuntimeSession): Held => ({ id: entry.agent ?? entry.session, person: entry.agent ? agents.get(entry.agent)?.person ?? null : null });
  const scoped = sessions.filter((entry) => inWhose(whose, teams.list, me.person.id, held(entry)));
  const groups = groupByTeam(scoped, held, teams.list, whose, (id) => names.get(id) ?? 'someone outside your view');
  const silent = (entry: RuntimeSession) => unanswered.some((each) => each.session === entry.session);
  const name = (entry: RuntimeSession) => entry.agent ? names.get(entry.agent) ?? 'An agent outside your view' : 'Unattached session';
  const at = (entry: RuntimeSession) => '#/canvas' + (entry.agent ? '/' + encodeURIComponent(entry.agent) : '');
  const columns: Column<RuntimeSession>[] = [
    { head: 'Agent', cell: (entry) => <a href={at(entry)} aria-current={entry.agent !== null && entry.agent === shown ? 'page' : undefined}>{name(entry)}</a> },
    { head: 'Computer', cell: (entry) => <span className="sec">{entry.machine_name ?? entry.machine}</span> },
    { head: 'State', cell: (entry) => silent(entry) ? <span className="why-not">Its runner did not answer; last reported {entry.last_reported}</span> : entry.shown === 'running' ? <><span className="dot s-active" />Running</> : 'Starting, not yet confirmed' },
    { head: 'Since', cell: (entry) => <span className="sec">{clock(entry.first_report_at)}</span> },
  ];
  return <aside className="running-list" aria-label="Running now">
    {teams.refused ? <p className="why-not">Teams cannot be read, so sessions are listed without their team. {teams.refused}</p> : null}
    <Listing<RuntimeSession> groups={groups} columns={columns} id={(entry) => entry.session} href={at}
      words={(entry) => name(entry) + ' ' + (entry.machine_name ?? entry.machine)} noun="running sessions" empty="Nothing is running."
      holds={(items) => items.length.toLocaleString('en-AU') + ' running'}
      selected={sessions.find((entry) => entry.agent !== null && entry.agent === shown)?.session ?? null} select={() => undefined} open={(entry) => navigate(at(entry).slice(1))}
      tools={<WhoseSelect whose={whose} set={setWhose} teams={teams.list} admin={admin} />} />
  </aside>;
}
