/**
 * The front page: the person's whole system at a glance, filling the screen with each panel scrolling inside itself.
 * Left, the agents under their teams with where each runs, its budget and its goals, started and stopped in their own
 * rows; each is said once, there. Right, what waits on the person, what needs a look, and what the week cost. Watch puts one running agent's live terminal in a
 * small picture over the page's bottom right corner, which stays while the page reads itself again.
 */
import { useEffect, useRef, useState } from 'react';
import { Navigate, useSearchParams } from 'react-router';
import { Refused, api, request, useLive } from '../../api';
import type { MeView } from '../../generated';
import { refreshLive } from '../../live';
import { readTogether } from '../../reads';
import type { Entry } from '../people/directory';
import { agentsOf, buildTree } from '../people/tree';
import type { Node } from '../people/tree';
import type { RuntimeSession } from '../runtime/RuntimeSessions';
import { Terminal } from '../runtime/Terminal';
import '../runtime/terminal.css';
import { Gate, SignIn } from '../signin/Gate';
import type { Team } from '../teams/contract';
import type { BudgetsView } from '../usage/contract';
import { AgentTree, entryOf, liveOf, whereOf } from './AgentTree';
import { GettingStarted, readStanding } from './GettingStarted';
import type { Standing } from './GettingStarted';
import type { Asked, RowContext } from './AgentTree';
import { orRefused } from './board';
import { readDashboard, refusedPart } from './contract';
import type { DashboardAgent, DashboardAnswer } from './contract';
import { readCord, stoppedWhy } from './cord';
import type { CordView } from './cord';
import { CordLine, StopEverythingButton, StopEverythingRow, pullPending } from './StopEverything';
import { Exceptions } from './Exceptions';
import { WaitingOnYou } from './WaitingOnYou';
import { readWaiting } from './waiting';
import type { WaitingLine } from './waiting';
import './dashboard.css';
import { Act } from '../../shell/Act';
import { Alarms } from './Alarms';
import { Unread } from './Unread';
import { Spend } from './Spend';
import { DashOptionStrip, DashboardOption, dashOptionOf } from './DashboardOptions';
import type { DashOption } from './DashboardOptions';

export interface Ready { kind: 'active'; me: MeView; answer: DashboardAnswer; standing: Standing; teams: Team[] | Refused; cord: CordView | Refused; waiting: WaitingLine[] | Refused; budgets: BudgetsView | Refused }

type Read = Ready | { kind: 'registered' };

async function readPage(): Promise<Read> {
  const me = await api.me();
  if (me.person.state === 'registered') return { kind: 'registered' };
  const { answer, teams, cord, waiting, standing, budgets } = await readTogether({
    standing: readStanding(),
    // One read for everything waiting on the person, the same the rail counts.
    waiting: orRefused(readWaiting(), 'WaitingUnreadable'),
    cord: readCord(),
    answer: readDashboard(),
    teams: request<{ teams: Team[] }>('/teams').then((list) => list.teams, (problem: unknown) => problem instanceof Refused ? problem : new Refused(0, { refusal: 'TeamsUnreadable', reason: String(problem) })),
    // The person's own limits on the whole week, against which the week's total is said.
    budgets: request<BudgetsView>('/budgets/person/' + encodeURIComponent(me.person.id)).then((view) => view, (problem: unknown) => problem instanceof Refused ? problem : new Refused(0, { refusal: 'BudgetsUnreadable', reason: String(problem) })),
  });
  return { kind: 'active', me, answer, teams, cord, waiting, standing, budgets };
}

/**
 * The agents as the tree on the person: each team under its owner, a lead's team under the lead, an agent under every
 * team it is in. Membership is the dashboard's own answer; `/teams` gives each team its name and owner. A team led by
 * someone outside this tree is shown under the person, and an agent in no team that can be named is listed loose.
 */
function treeOf(me: MeView, rows: DashboardAgent[], teams: Team[]): Node {
  const person: Entry = { id: me.person.id, display_name: me.person.display_name, state: me.person.state, kind: 'person', role: null, person: null };
  const agents: Entry[] = rows.map((row) => ({ ...entryOf(row), person: me.person }));
  const owners = new Set([person.id, ...agents.map((agent) => agent.id)]);
  const shown = teams.filter((team) => team.state === 'active').map((team) => ({
    ...team, owner: owners.has(team.owner) ? team.owner : person.id,
    members: rows.filter((row) => Array.isArray(row.teams) && row.teams.includes(team.id)).map((row) => row.agent.id),
  }));
  const tree = buildTree(person, shown, [person, ...agents]);
  const placed = new Set(agentsOf(tree).map((entry) => entry.id));
  const missing = agents.filter((agent) => !placed.has(agent.id));
  const complete = missing.length ? { ...tree, branches: [...tree.branches, { team: null, members: missing.map((entry) => ({ entry, branches: [] })) }] } : tree;
  const trim = (node: Node): Node => ({ ...node, branches: node.branches.map(branch => ({ ...branch, members: branch.members.map(trim) })).filter(branch => branch.members.some(member => member.entry.kind === 'agent' || member.branches.length)) });
  return trim(complete);
}

function Page({ data, reload, watching, watch, option, fixture }: { data: Ready; reload: () => void; watching: string | null; watch: (row: DashboardAgent, session: RuntimeSession) => void; option: DashOption | null; fixture: boolean }) {
  const [asked, setAsked] = useState<Asked | null>(null);
  // A pull with no confirmed answer in this tab opens its row by itself, so it is asked about and never sent twice.
  const [pulling, setPulling] = useState(pullPending);
  // A started row waits only for the next read of the page. When that read arrives the row says what is so: Watch and Stop if it runs, Start again if it does not.
  useEffect(() => { setAsked((held) => held?.what === 'started' ? null : held); }, [data]);
  // A retired agent leaves the agents list, but one with a session that has not stopped stays in the table, so its Stop is still there.
  const every = data.answer.agents;
  const rows = every.filter((row) => row.agent.state !== 'retired');
  const listedRows = every.filter((row) => row.agent.state !== 'retired' || liveOf(row).length);
  const byId = new Map(every.map((row) => [row.agent.id, row]));
  const teams = data.teams instanceof Refused ? [] : data.teams;
  const context: RowContext = {
    rows: byId, me: data.me.person.id, asked, ask: setAsked, changed: reload, watching, stopped: stoppedWhy(data.cord),
    watch: (agent, id) => { const row = byId.get(agent); const session = liveOf(row).find((each) => each.session === id); if (row && session) watch(row, session); },
  };
  const listed = { running: every.flatMap(liveOf).length, unread: every.filter((row) => refusedPart(row.sessions)).length };
  // Stop all is offered while anything runs, or while what runs cannot be read, when the off switch is needed most; never with nothing running.
  const stoppable = listed.running > 0 || listed.unread > 0 || data.cord instanceof Refused;
  const branches = treeOf(data.me, listedRows, teams).branches;
  // Under an option the page is one of the three dashboards being decided between; the cord line and the pull row stay the same.
  if (option) return <><CordLine cord={data.cord} changed={reload} listed={listed} />
    {pulling ? <table className="dash-table dash-cord-ask"><tbody><StopEverythingRow columns={1} close={() => setPulling(false)} changed={reload} /></tbody></table> : null}
    <DashboardOption option={option} shown={{ data, context, branches, reload, fixture, stoppable: stoppable || pulling, openStop: () => setPulling(true) }} />
  </>;
  return <><CordLine cord={data.cord} changed={reload} listed={listed} /><div className="dash-body">
    <GettingStarted person={data.me.person.id} agents={rows.length} standing={data.standing} reread={reload} />
    <section className="you-panel dash-agents" aria-label="Agents">
      <div className="section-h"><span>Agents</span>{stoppable || pulling ? <StopEverythingButton cord={data.cord} open={() => setPulling(true)} /> : null}</div>
      {pulling ? <table className="dash-table dash-cord-ask"><tbody><StopEverythingRow columns={1} close={() => setPulling(false)} changed={reload} /></tbody></table> : null}
      {data.teams instanceof Refused ? <Unread part={data.teams} words="Teams could not be read, so your agents are listed without their teams." /> : null}
      <div className="you-scroll"><AgentTree branches={branches} context={context} /></div>
    </section>
    <div className="dash-side">
      <WaitingOnYou waiting={data.waiting} me={data.me.person.id} changed={reload} />
      <Exceptions rows={rows} />
      <Spend rows={rows} budgets={data.budgets} />
    </div>
    <div className="dash-widgets">
      <Alarms summary={data.answer.alarms} agents={every} />
    </div>
  </div></>;
}

interface Watched { agent: string; name: string; session: RuntimeSession }

/** The one picture: a running agent's live terminal over the page's bottom right corner. Closing it leaves the agent running. */
function Picture({ watched, close }: { watched: Watched; close: () => void }) {
  return <aside className="dash-picture" aria-label={'Watching ' + watched.name}>
    <header className="dash-picture-bar">
      <h2>{watched.name}</h2>
      <span className="sec">{whereOf(watched.session)}</span>
      <a className="btn" data-act="full-size" href={'#/canvas/' + encodeURIComponent(watched.agent)}>Open full size</a>
      <Act symbol="close" name="Close" data-act="close-picture" onClick={close} />
    </header>
    <Terminal key={watched.session.session} session={watched.session.session} agent={watched.agent} bare />
  </aside>;
}

/** True in a dev build (the dev server), where the option strip is drawn and `&fixture=cherie` draws the page from her fixture. */
const DEV = process.env.NODE_ENV !== 'production';

export function Dashboard() {
  const [version, setVersion] = useState(0);
  const read = useLive(readPage, 'dashboard:' + version);
  const [query] = useSearchParams();
  const option = dashOptionOf(query.get('option'));
  // Dev builds only: Cherie's fixture stands in for the service, so the page can be looked at signed out.
  const asFixture = DEV && query.get('fixture') === 'cherie';
  const [fixture, setFixture] = useState<Ready | null>(null);
  useEffect(() => {
    if (!asFixture) { setFixture(null); return; }
    void import('./dashboard-cherie.fixture').then((mod) => setFixture(mod.cherie().data));
  }, [asFixture]);
  // While the page reads itself again after a start, a stop or a change, what it last showed stays; it never blanks.
  const last = useRef<typeof read | null>(null);
  if (read.status === 'ok') last.current = read;
  const load: typeof read = fixture ? { status: 'ok', data: fixture } : read.status === 'loading' && last.current ? last.current : read;
  const [watched, setWatched] = useState<Watched | null>(null);
  // The picture keeps the session it shows. When a later read no longer lists that session and its agent has exactly one
  // live session that was not there while the watched one ran, the agent was restarted, and the picture moves to the new
  // one; a session that was already running beside it is another piece of work, and the terminal says how the watched one ended.
  const beside = useRef<Set<string>>(new Set());
  const live = load.status === 'ok' && load.data.kind === 'active' && watched ? liveOf(load.data.answer.agents.find((row) => row.agent.id === watched.agent)) : [];
  if (watched && live.some((each) => each.session === watched.session.session)) for (const each of live) beside.current.add(each.session);
  const shown = watched && live.length === 1 && !beside.current.has(live[0].session) ? { ...watched, session: live[0] } : watched;
  if (!fixture && read.status === 'refused' && read.refused.status === 401) {
    last.current = null;
    return <SignIn />;
  }
  return <div className={'page fill dash-page' + (shown ? ' dash-watching' : '')}>
    {/* No heading: the rail names the page. Reconnect stands alone when the feed is lost. */}
    {read.status === 'refused' && read.feed && last.current ? <div className="dash-reconnect"><Act symbol="retry" name="Reconnect" word="Reconnect" onClick={refreshLive} /></div> : null}
    {DEV ? <DashOptionStrip option={option} /> : null}
    <Gate load={load} title="your dashboard" ok={(data) => data.kind === 'registered' ? <Navigate replace to="/me" />
      : <Page data={data} reload={() => setVersion((v) => v + 1)} watching={shown?.session.session ?? null} option={option} fixture={fixture !== null}
        watch={(row, session) => { beside.current = new Set(liveOf(row).map((each) => each.session)); setWatched({ agent: row.agent.id, name: row.agent.display_name, session }); }} />} />
    {shown ? <Picture watched={shown} close={() => setWatched(null)} /> : null}
  </div>;
}
