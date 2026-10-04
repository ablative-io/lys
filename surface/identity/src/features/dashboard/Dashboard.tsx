/**
 * The front page: the person's whole system at a glance, filling the screen with each panel scrolling inside itself.
 * Left, the agents under their teams with where each runs, its budget and its goals, started and stopped in their own
 * rows. Right, what waits for the person and what is running now. Watch puts one running agent's live terminal in a
 * small picture over the page's bottom right corner, which stays while the page reads itself again.
 */
import { useEffect, useRef, useState } from 'react';
import { Navigate } from 'react-router';
import { Refused, api, request, useLive } from '../../api';
import type { MeView } from '../../generated';
import { refreshLive } from '../../live';
import { readTogether } from '../../reads';
import { singular } from '../../shell/count';
import type { Entry } from '../people/directory';
import { agentsOf, buildTree } from '../people/tree';
import type { Node } from '../people/tree';
import type { RuntimeSession } from '../runtime/RuntimeSessions';
import { readDrafts } from '../drafts/contract';
import type { Draft } from '../drafts/contract';
import type { AccessRequest } from '../requests/contract';
import { Terminal } from '../runtime/Terminal';
import '../runtime/terminal.css';
import { Gate } from '../signin/Gate';
import type { Team } from '../teams/contract';
import { AgentTree, RunningNow, entryOf, liveOf, whereOf } from './AgentTree';
import type { Asked, RowContext } from './AgentTree';
import { orRefused } from './board';
import { readDashboard, refusedPart } from './contract';
import type { DashboardAgent, DashboardAnswer, Waiting } from './contract';
import { readCord, stoppedWhy } from './cord';
import type { CordView } from './cord';
import { CordLine, StopEverythingButton, StopEverythingRow, pullPending } from './StopEverything';
import { BudgetWidget, DraftsWidget, GoalsWidget, RequestsWidget } from './Widgets';
import './dashboard.css';

interface Ready { kind: 'active'; me: MeView; answer: DashboardAnswer; teams: Team[] | Refused; cord: CordView | Refused; requests: AccessRequest[] | Refused; drafts: Draft[] | Refused }

type Read = Ready | { kind: 'registered' };

async function readPage(): Promise<Read> {
  const me = await api.me();
  if (me.person.state === 'registered') return { kind: 'registered' };
  const { answer, teams, cord, requests, drafts } = await readTogether({
    requests: orRefused<AccessRequest[]>(request<{ requests: AccessRequest[] }>('/requests').then((list) => list.requests), 'RequestsUnreadable'),
    drafts: orRefused(readDrafts('waiting'), 'DraftsUnreadable'),
    cord: readCord(),
    answer: readDashboard(),
    teams: request<{ teams: Team[] }>('/teams').then((list) => list.teams, (problem: unknown) => problem instanceof Refused ? problem : new Refused(0, { refusal: 'TeamsUnreadable', reason: String(problem) })),
  });
  return { kind: 'active', me, answer, teams, cord, requests, drafts };
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
  return missing.length ? { ...tree, branches: [...tree.branches, { team: null, members: missing.map((entry) => ({ entry, branches: [] })) }] } : tree;
}

const WAITING: [keyof Waiting, string, string][] = [
  ['requests', 'requests to decide', '#/requests'], ['drafts', 'drafts to approve', '#/access/drafts'], ['reviews', 'reviews due', '#/reviews'],
];

/** The three counts, one in the singular; a count that could not be read says its refusal's name in place of a number, never zero. */
function WaitingTable({ waiting }: { waiting: Waiting }) {
  const none = WAITING.every(([key]) => waiting[key] === 0);
  return <table className="dash-table" aria-label="Waiting for you">
    <tbody>{none ? <tr className="empty"><td className="dim">Nothing is waiting.</td></tr>
      : WAITING.map(([key, words, href]) => {
        const count = waiting[key];
        return <tr key={key} data-waiting={key}>
          <td className="dash-num">{refusedPart(count) ? <small className="refusal-name why-not" title={count.reason}>{count.refusal}</small> : count}</td>
          <td><a href={href}>{count === 1 ? singular(words) : words}</a></td>
        </tr>;
      })}</tbody>
  </table>;
}

function Page({ data, reload, watching, watch }: { data: Ready; reload: () => void; watching: string | null; watch: (row: DashboardAgent, session: RuntimeSession) => void }) {
  const [asked, setAsked] = useState<Asked | null>(null);
  // A pull with no confirmed answer in this tab opens its row by itself, so it is asked about and never sent twice.
  const [pulling, setPulling] = useState(pullPending);
  // A started row waits only for the next read of the page. When that read arrives the row says what is so: Watch and Stop if it runs, Start again if it does not.
  useEffect(() => { setAsked((held) => held?.what === 'started' ? null : held); }, [data]);
  // A retired agent leaves the agents list, but a session of its that has not stopped still stands in Running now.
  const every = data.answer.agents;
  const rows = every.filter((row) => row.agent.state !== 'retired');
  const byId = new Map(every.map((row) => [row.agent.id, row]));
  const teams = data.teams instanceof Refused ? [] : data.teams;
  const context: RowContext = {
    rows: byId, me: data.me.person.id, asked, ask: setAsked, changed: reload, watching, stopped: stoppedWhy(data.cord),
    watch: (agent, id) => { const row = byId.get(agent); const session = liveOf(row).find((each) => each.session === id); if (row && session) watch(row, session); },
  };
  const listed = { running: every.flatMap(liveOf).length, unread: every.filter((row) => refusedPart(row.sessions)).length };
  return <><CordLine cord={data.cord} changed={reload} listed={listed} /><div className="dash-body">
    <section className="you-panel dash-agents" aria-label="Agents">
      <div className="section-h"><span>Agents</span></div>
      {data.teams instanceof Refused ? <p className="why-not">Teams cannot be read, so your agents are listed without their teams. <small className="refusal-name">{data.teams.refusal.refusal}</small></p> : null}
      <div className="you-scroll"><AgentTree branches={treeOf(data.me, rows, teams).branches} context={context} /></div>
    </section>
    <div className="dash-side">
      <section className="you-panel" aria-label="Waiting for you">
        <div className="section-h"><span>Waiting for you</span></div>
        <div className="you-scroll"><WaitingTable waiting={data.answer.waiting} /></div>
      </section>
      <section className="you-panel" aria-label="Running now">
        <div className="section-h"><span>Running now</span><StopEverythingButton cord={data.cord} open={() => setPulling(true)} /></div>
        <div className="you-scroll"><RunningNow rows={every} context={context}
          first={pulling ? <StopEverythingRow columns={3} close={() => setPulling(false)} changed={reload} /> : null} /></div>
      </section>
    </div>
    <div className="dash-widgets">
      <RequestsWidget requests={data.requests} me={data.me.person.id} />
      <DraftsWidget drafts={data.drafts} />
      <BudgetWidget rows={rows} />
      <GoalsWidget rows={rows} />
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
      <button type="button" className="btn" data-act="close-picture" onClick={close}>Close</button>
    </header>
    <Terminal key={watched.session.session} session={watched.session.session} agent={watched.agent} bare />
  </aside>;
}

export function Dashboard() {
  const [version, setVersion] = useState(0);
  const read = useLive(readPage, 'dashboard:' + version);
  // While the page reads itself again after a start, a stop or a change, what it last showed stays; it never blanks.
  const last = useRef<typeof read | null>(null);
  if (read.status === 'ok') last.current = read;
  const load = read.status === 'loading' && last.current ? last.current : read;
  const [watched, setWatched] = useState<Watched | null>(null);
  // The picture keeps the session it shows. When a later read no longer lists that session and its agent has exactly one
  // live session that was not there while the watched one ran, the agent was restarted, and the picture moves to the new
  // one; a session that was already running beside it is another piece of work, and the terminal says how the watched one ended.
  const beside = useRef<Set<string>>(new Set());
  const live = load.status === 'ok' && load.data.kind === 'active' && watched ? liveOf(load.data.answer.agents.find((row) => row.agent.id === watched.agent)) : [];
  if (watched && live.some((each) => each.session === watched.session.session)) for (const each of live) beside.current.add(each.session);
  const shown = watched && live.length === 1 && !beside.current.has(live[0].session) ? { ...watched, session: live[0] } : watched;
  return <div className={'page fill dash-page' + (shown ? ' dash-watching' : '')}>
    <div className="head"><div><h1>Dashboard</h1></div>{read.status === 'refused' ? <button type="button" className="btn" onClick={refreshLive}>Reconnect</button> : null}</div>
    <Gate load={load} title="your dashboard" ok={(data) => data.kind === 'registered' ? <Navigate replace to="/me" />
      : <Page data={data} reload={() => setVersion((v) => v + 1)} watching={shown?.session.session ?? null}
        watch={(row, session) => { beside.current = new Set(liveOf(row).map((each) => each.session)); setWatched({ agent: row.agent.id, name: row.agent.display_name, session }); }} />} />
    {shown ? <Picture watched={shown} close={() => setWatched(null)} /> : null}
  </div>;
}
