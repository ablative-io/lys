import { refreshLive } from '../../live';
import { readTogether } from '../../reads';
/** The Sessions screen: every running agent session the caller may see, as its runner says, each opened to its live terminal with a line to type into, common keys and Stop. A session its runner saw end is not listed; nothing is inferred from a clock. */
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
import { Terminal } from './Terminal';
import './terminal.css';

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

export function RunningSessions() {
  const { session } = useParams();
  const load = useLive(readRunning, 'runtime-live');
  return <div className="page fill">
    <div className="head">
      <div><div className="eyebrow">Running now</div><h1>Running sessions</h1>{load.status === 'refused' ? <button type="button" onClick={refreshLive}>Reconnect</button> : null}<p className="sub">Choose an agent to watch or type in its terminal. Switching leaves the other sessions running.</p></div>
    </div>
    <Gate load={load} title="Running sessions" ok={(data) => <Running {...data} session={session} />} />
  </div>;
}

function Running({ sessions, unanswered, people, me, teams, session }: Awaited<ReturnType<typeof readRunning>> & { session: string | undefined }) {
  const admin = people.scope === 'directory';
  const [whose, setWhose] = useWhose(admin);
  const navigate = useNavigate();
  const agents = new Map(people.people.flatMap((person) => person.agents.map((agent) => [agent.id, { name: agent.display_name, person: person.id }] as const)));
  const names = new Map([...people.people.map((person) => [person.id, person.display_name] as const), ...[...agents].map(([id, agent]) => [id, agent.name] as const)]);
  const held = (entry: RuntimeSession): Held => ({ id: entry.agent ?? entry.session, person: entry.agent ? agents.get(entry.agent)?.person ?? null : null });
  const scoped = sessions.filter((entry) => inWhose(whose, teams.list, me.person.id, held(entry)));
  const groups = groupByTeam(scoped, held, teams.list, whose, (id) => names.get(id) ?? 'someone outside your view');
  // With one running agent and none named in the address, that one is open: there is nothing to choose between.
  const open = sessions.find((entry) => entry.session === session) ?? (!session && sessions.length === 1 ? sessions[0] : undefined);
  const silent = (entry: RuntimeSession) => unanswered.some((each) => each.session === entry.session);
  const name = (entry: RuntimeSession) => entry.agent ? names.get(entry.agent) ?? 'An agent outside your view' : 'Unattached session';
  const columns: Column<RuntimeSession>[] = [
    { head: 'Agent', cell: (entry) => <a href={'#/runtime/' + encodeURIComponent(entry.session)} aria-current={entry.session === session ? 'page' : undefined}>{name(entry)}</a> },
    { head: 'Computer', cell: (entry) => <span className="sec">{entry.machine_name ?? entry.machine}</span> },
    { head: 'State', cell: (entry) => silent(entry) ? <span className="why-not">Its runner did not answer; last reported {entry.last_reported}</span> : entry.shown === 'running' ? <><span className="dot s-active" />Running</> : 'Starting, not yet confirmed' },
    { head: 'Since', cell: (entry) => <span className="sec">{clock(entry.first_report_at)}</span> },
  ];
  return <>
    {unanswered.length ? <div className="why-not" role="alert"><h3>Runners that did not answer</h3><ul>{unanswered.map((entry) => <li key={entry.session}><span className="mono">{entry.session}</span> on {entry.machine}: {entry.refusal}: {entry.reason}</li>)}</ul></div> : null}
    {teams.refused ? <p className="why-not">Teams cannot be read, so sessions are listed without their team. {teams.refused}</p> : null}
    {/* With a terminal open the terminal is the page: the list narrows to the agent and its state, and the terminal takes the rest of the width and the height. */}
    <div className={open ? 'body terminal-open' : 'body'}>
      <Listing<RuntimeSession> groups={groups} columns={open ? [columns[0], columns[2]] : columns} id={(entry) => entry.session} href={(entry) => '#/runtime/' + encodeURIComponent(entry.session)}
        words={(entry) => name(entry) + ' ' + (entry.machine_name ?? entry.machine)} noun="running sessions"
        holds={(items) => items.length.toLocaleString('en-AU') + (items.length === 1 ? ' running' : ' running')}
        selected={open?.session ?? null} select={() => undefined} open={(entry) => navigate('/runtime/' + encodeURIComponent(entry.session))}
        tools={<WhoseSelect whose={whose} set={setWhose} teams={teams.list} admin={admin} />} />
      <div className="detail">{open ? <Terminal key={open.session} session={open.session} agent={open.agent} machine={open.machine_name ?? open.machine} />
        : <div className="card"><h2>{session ? 'Session not returned' : sessions.length ? 'Choose a running agent' : 'No running session was returned.'}</h2><p className="sec">{session ? 'This session is not in the current list. Ask the runners again to refresh it.' : 'Its terminal opens here. Your input and controls use your existing permissions.'}</p></div>}</div>
    </div>
  </>;
}
