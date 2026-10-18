/**
 * Three dashboards to decide between on the page itself, switched by `?option=A|B|C` on the dashboard's own route
 * (absent, today's page). All three follow Tom's rules of 18 Oct: the page is called Dashboard, nothing on it scrolls
 * top to bottom (each part scrolls inside itself and the whole fits the screen), every part keeps one proportion, the
 * alarms and the security events are one line each, and what waits on the person comes first.
 *   A, two questions: "Waiting on you" and "Look out for" across the top, the agents and the week underneath.
 *   B, one column: a one-line strip of the agents, then everything that needs the person as one list.
 *   C, six equal tiles: waiting, look out for, agents, this week, alarms, security, each the same size.
 * Signed out in a dev build, `&fixture=cherie` draws each with Cherie's fixture and asks the service nothing.
 */
import { Link, useSearchParams } from 'react-router';
import { Refused } from '../../api';
import { money } from '../../shell/count';
import { AgentTree, liveOf } from './AgentTree';
import type { RowContext } from './AgentTree';
import type { Alarm } from './Alarms';
import { SecurityEvents } from './SecurityEvents';
import { refusedPart } from './contract';
import type { Ready } from './Dashboard';
import { exceptionsOf } from './Exceptions';
import { Spend, weekSpend } from './Spend';
import { StopEverythingButton } from './StopEverything';
import { WaitingOnYou } from './WaitingOnYou';
import { Unread } from './Unread';
import './dashboard-options.css';

export type DashOption = 'A' | 'B' | 'C';
const OPTIONS: readonly DashOption[] = ['A', 'B', 'C'];
const NAMED: Record<DashOption, string> = { A: 'two questions', B: 'one column', C: 'six equal tiles' };

/** The dashboard option asked for in the query, or null for today's page. */
export const dashOptionOf = (asked: string | null): DashOption | null => OPTIONS.find((each) => each === asked) ?? null;

/** The three letters and the current one; each link keeps the rest of the query, the fixture with it. Dev builds only. */
export function DashOptionStrip({ option }: { option: DashOption | null }) {
  const [query] = useSearchParams();
  const to = (letter: DashOption) => { const next = new URLSearchParams(query); next.set('option', letter); return '?' + next.toString(); };
  return <nav className="dash-options" aria-label="Dashboard options">
    {OPTIONS.map((letter) => <Link key={letter} to={to(letter)} className={'btn' + (letter === option ? ' primary' : '')} aria-current={letter === option ? 'page' : undefined}>{letter}</Link>)}
    <span className="sec">{option ? option + ', ' + NAMED[option] : 'none asked: today’s page'}</span>
  </nav>;
}

interface Shown { data: Ready; context: RowContext; branches: Parameters<typeof AgentTree>[0]['branches']; reload: () => void; fixture: boolean; stoppable: boolean; openStop: () => void }

/** One sentence per open alarm, the agent by name: "Helper’s seat stalled on Cherie’s MacBook Air: <detail>". */
function alarmLines(data: Ready): { id: string; words: string }[] {
  const alarms = data.answer.alarms;
  if (refusedPart(alarms)) return [];
  const names = new Map(data.answer.agents.map((row) => [row.agent.id, row.agent.display_name]));
  return alarms.newest.filter((alarm: Alarm) => alarm.cleared_at === null).map((alarm) => ({
    id: alarm.id,
    words: (names.get(alarm.agent) ?? alarm.agent) + (alarm.kind === 'seat_stalled' ? '’s seat stalled' : '’s session is running ungoverned') + ': ' + alarm.detail,
  }));
}

/** Look out for: what is wrong, one sentence each, exceptions first, then open alarms; the security events as one line. */
function LookOut({ data, fixture, withSecurity }: { data: Ready; fixture: boolean; withSecurity: boolean }) {
  const rows = data.answer.agents.filter((row) => row.agent.state !== 'retired');
  const found = exceptionsOf(rows);
  const alarms = alarmLines(data);
  const unread = refusedPart(data.answer.alarms) ? data.answer.alarms : null;
  const count = found.length + alarms.length;
  return <section className="you-panel dash-lookout" aria-label="Look out for">
    <div className="section-h"><span>Look out for</span><b className="dash-count">{count}</b></div>
    <div className="you-scroll">
      {count === 0 && !unread ? <p className="dim dash-nothing">Nothing to look out for.</p> : null}
      {unread ? <Unread part={unread} words="Alarms could not be read." /> : null}
      <table className="dash-table"><tbody>
        {found.map((each, index) => <tr key={each.agent.id + '/' + index} data-exception={each.kind}>
          <td><span role="img" className={'dot ' + (each.kind === 'near' ? 'dash-dot-warn' : 'dash-dot-danger')} aria-label={each.kind === 'near' ? 'near' : 'reached'} />{each.words}</td>
          <td className="you-act"><a className="btn" href={'#/file/' + encodeURIComponent(each.agent.id) + '/budgets'}>Open</a></td>
        </tr>)}
        {alarms.map((each) => <tr key={each.id} data-alarm={each.id}>
          <td><span role="img" className="dot dash-dot-danger" aria-label="alarm" />{each.words}</td>
          <td className="you-act"><a className="btn" href="#/ops">Open</a></td>
        </tr>)}
      </tbody></table>
      {withSecurity ? (fixture ? <p className="dim dash-security-line">No security events this week.</p> : <div className="dash-security-line"><SecurityEvents /></div>) : null}
    </div>
  </section>;
}

/** The agents as one line: how many, how many running, the off switch while anything runs. */
function AgentsStrip({ data, stoppable, openStop }: { data: Ready; stoppable: boolean; openStop: () => void }) {
  const every = data.answer.agents;
  const rows = every.filter((row) => row.agent.state !== 'retired');
  const running = every.flatMap(liveOf).length;
  const spent = rows.map(weekSpend).reduce((sum, week) => week.kind === 'spent' || week.kind === 'partial' ? sum + week.figure : sum, 0);
  return <div className="dash-strip" aria-label="Agents">
    <b>{rows.length === 1 ? '1 agent' : rows.length + ' agents'}</b>
    <span className="sec">{running === 0 ? 'none running' : running === 1 ? '1 running' : running + ' running'}</span>
    <span className="sec">this week {money(spent)}</span>
    <a className="btn" href="#/people">Open</a>
    {stoppable ? <StopEverythingButton cord={data.cord} open={openStop} /> : null}
  </div>;
}

function Agents({ shown }: { shown: Shown }) {
  const teams = shown.data.teams;
  return <section className="you-panel dash-agents" aria-label="Agents">
    <div className="section-h"><span>Agents</span>{shown.stoppable ? <StopEverythingButton cord={shown.data.cord} open={shown.openStop} /> : null}</div>
    {teams instanceof Refused ? <Unread part={teams} words="Teams could not be read, so your agents are listed without their teams." /> : null}
    <div className="you-scroll"><AgentTree branches={shown.branches} context={shown.context} /></div>
  </section>;
}

function Waiting({ data, changed }: { data: Ready; changed: () => void }) {
  return <WaitingOnYou waiting={data.waiting} me={data.me.person.id} changed={changed} />;
}

function Week({ data }: { data: Ready }) {
  return <Spend rows={data.answer.agents.filter((row) => row.agent.state !== 'retired')} budgets={data.budgets} />;
}

/** Alarms alone, as lines, for the tile that is only them. */
function AlarmsTile({ data }: { data: Ready }) {
  const lines = alarmLines(data);
  const unread = refusedPart(data.answer.alarms) ? data.answer.alarms : null;
  return <section className="you-panel dash-alarms-tile" aria-label="Alarms">
    <div className="section-h"><span>Alarms</span><b className="dash-count">{lines.length}</b></div>
    <div className="you-scroll">
      {unread ? <Unread part={unread} words="Alarms could not be read." /> : lines.length === 0 ? <p className="dim dash-nothing">No open alarms.</p> : null}
      <table className="dash-table"><tbody>{lines.map((each) => <tr key={each.id}><td>{each.words}</td><td className="you-act"><a className="btn" href="#/ops">Open</a></td></tr>)}</tbody></table>
    </div>
  </section>;
}

function SecurityTile({ fixture }: { fixture: boolean }) {
  return <section className="you-panel dash-security-tile" aria-label="Security events">
    <div className="section-h"><span>Security events</span></div>
    <div className="you-scroll">{fixture ? <p className="dim dash-nothing">No security events this week.</p> : <SecurityEvents />}</div>
  </section>;
}

/** The dashboard under an option. */
export function DashboardOption({ option, shown }: { option: DashOption; shown: Shown }) {
  const { data, fixture } = shown;
  if (option === 'A') return <div className="dash-opt dash-opt-a">
    <div className="dash-tile"><Waiting data={data} changed={shown.reload} /></div>
    <div className="dash-tile"><LookOut data={data} fixture={fixture} withSecurity /></div>
    <div className="dash-tile"><Agents shown={shown} /></div>
    <div className="dash-tile"><Week data={data} /></div>
  </div>;
  if (option === 'B') return <div className="dash-opt dash-opt-b">
    <AgentsStrip data={data} stoppable={shown.stoppable} openStop={shown.openStop} />
    <div className="dash-tile dash-column"><Waiting data={data} changed={shown.reload} /><LookOut data={data} fixture={fixture} withSecurity /></div>
  </div>;
  return <div className="dash-opt dash-opt-c">
    <div className="dash-tile"><Waiting data={data} changed={shown.reload} /></div>
    <div className="dash-tile"><LookOut data={data} fixture={fixture} withSecurity={false} /></div>
    <div className="dash-tile"><Agents shown={shown} /></div>
    <div className="dash-tile"><Week data={data} /></div>
    <div className="dash-tile"><AlarmsTile data={data} /></div>
    <div className="dash-tile"><SecurityTile fixture={fixture} /></div>
  </div>;
}

