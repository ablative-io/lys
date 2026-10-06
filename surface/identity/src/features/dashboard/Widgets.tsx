/**
 * The Dashboard's widgets: what waits for the person said item by item (the requests they may decide, the drafts their
 * agents prepared), how much each agent has used against its limits, and every goal with its deadline. Each is one card
 * that scrolls inside itself; a part that could not be read says its refusal by name, never nothing.
 */
import type { ReactNode } from 'react';
import { Refused } from '../../api';
import type { Draft } from '../drafts/contract';
import { day } from '../file/time';
import type { AccessRequest } from '../requests/contract';
import { refusedPart } from './contract';
import type { DashboardAgent } from './contract';
import { budgetLine, tightest } from './words';

/** One widget: a titled card, how many it holds, and where its whole list lives. */
function Widget({ label, count, href, children }: { label: string; count: ReactNode; href: string; children: ReactNode }) {
  return <section className="dash-widget" aria-label={label}>
    <div className="section-h"><span>{label}</span><b className="dash-count">{count}</b><a href={href}>Open</a></div>
    <div className="you-scroll">{children}</div>
  </section>;
}

/** What a widget holds, apart from the card around it: the Dashboard puts it in a card, the canvas in a window a person places. */
export interface Part { label: string; count: ReactNode; href: string; body: ReactNode; /** Its one figure, where that is not its count. */ figure?: ReactNode; /** That figure as a level from 0 to 100, when it is one. */ percent?: number }
const part = (label: string, count: ReactNode, href: string, body: ReactNode): Part => ({ label, count, href, body });
const card = (made: Part) => <Widget label={made.label} count={made.count} href={made.href}>{made.body}</Widget>;

function Unread({ what, refused }: { what: string; refused: Refused }) {
  return <p className="why-not" role="alert">{what} could not be read. <small className="refusal-name" title={refused.refusal.reason}>{refused.refusal.refusal}</small></p>;
}

/** The requests still waiting, the ones this person may decide first, each said by who asked, for what and why. */
export function requestsPart({ requests, me }: { requests: AccessRequest[] | Refused; me: string }) {
  if (requests instanceof Refused) return part("Requests", "?", "#/requests", <><Unread what="Requests" refused={requests} /></>);
  const mine = (entry: AccessRequest) => entry.can_decide ?? entry.approvers.some((each) => each.id === me);
  const waiting = requests.filter((entry) => entry.state === 'waiting').sort((left, right) => Number(mine(right)) - Number(mine(left)) || left.asked_at - right.asked_at);
  return part("Requests", waiting.length, "#/requests", <>
    <table className="dash-table"><tbody>
      {waiting.map((entry) => <tr key={entry.id} data-request={entry.id}>
        <td>{entry.asked_by_name ?? 'Name unavailable'}</td>
        <td className="sec">{entry.relation}: {entry.actions.join(', ')}<div className="note">{entry.why}</div></td>
        <td className="you-act">{mine(entry) ? <a className="btn" href="#/requests">Decide</a> : <span className="dim">{entry.approvers.map((each) => each.display_name).join(', ') || 'No one can decide'}</span>}</td>
      </tr>)}
      {waiting.length ? null : <tr className="empty"><td className="dim">No request is waiting.</td></tr>}
    </tbody></table>
  </>);
}

/** The drafts agents prepared for this person to approve, each said by the agent, what it would do and the agent's note. */
export function draftsPart({ drafts }: { drafts: Draft[] | Refused }) {
  if (drafts instanceof Refused) return part("Drafts", "?", "#/access/drafts", <><Unread what="Drafts" refused={drafts} /></>);
  return part("Drafts", drafts.length, "#/access/drafts", <>
    <table className="dash-table"><tbody>
      {drafts.map((draft) => <tr key={draft.id} data-draft="waiting">
        <td>{draft.agent?.display_name ?? 'An agent outside your view'}</td>
        <td className="sec">{draft.target.action} on {draft.target.kind}<div className="note">{draft.note}</div></td>
        <td className="you-act"><a className="btn" href="#/access/drafts">Decide</a></td>
      </tr>)}
      {drafts.length ? null : <tr className="empty"><td className="dim">No draft is waiting.</td></tr>}
    </tbody></table>
  </>);
}

/** Each agent that has a limit, the nearest to its limit first, with a bar for how near; agents with none are counted in one line. */
export function budgetPart({ rows }: { rows: DashboardAgent[] }) {
  const limited = rows.flatMap((row) => {
    if (refusedPart(row.budget) || refusedPart(row.usage)) return [{ row, near: 2, line: budgetLine(row.budget, row.usage) }];
    return row.budget.limits.length ? [{ row, near: tightest(row.budget) ?? -1, line: budgetLine(row.budget, row.usage) }] : [];
  }).sort((left, right) => right.near - left.near);
  const known = limited.filter((each) => !each.line.refused);
  const reached = known.filter((each) => each.near >= 1).length;
  const free = rows.length - limited.length;
  return part("Budget", reached ? reached + ' at limit' : known.length + ' limited', "#/people", <>
    <table className="dash-table"><tbody>
      {limited.map(({ row, near, line }) => <tr key={row.agent.id} data-budget={row.agent.id}>
        <td className="dash-name">{row.agent.display_name}</td>
        <td className="sec">{line.refused ? <small className="refusal-name why-not">{line.words}</small> : <>
          <span className="dash-bar" role="img" aria-label={near < 0 ? 'Nothing reported yet' : Math.round(near * 100) + '% of the nearest limit'}>
            <span className={near >= 1 ? 'full' : ''} style={{ width: Math.max(0, Math.min(1, near)) * 100 + '%' }} /></span>
          {line.words}</>}</td>
      </tr>)}
      {free ? <tr className="empty"><td className="dim" colSpan={2}>{limited.length ? free + (free === 1 ? ' other agent has' : ' other agents have') + ' no limit.' : 'No agent has a limit.'}</td></tr> : null}
    </tbody></table>
  </>);
}

const ORDER = { missed: 0, open: 1, met: 2, dropped: 3 };
const STANDING = { open: 'Open', met: 'Met', missed: 'Missed', dropped: 'Dropped' };

/** Every active goal of every agent: the missed first, then the open by deadline, then the met. */
export function goalsPart({ rows }: { rows: DashboardAgent[] }) {
  const unread = rows.filter((row) => refusedPart(row.goals));
  const goals = rows.flatMap((row) => refusedPart(row.goals) ? [] : row.goals.goals.filter((item) => item.goal.active).map((item) => ({ row, item })))
    .sort((left, right) => ORDER[left.item.standing] - ORDER[right.item.standing] || (left.item.goal.deadline ?? Infinity) - (right.item.goal.deadline ?? Infinity));
  const open = goals.filter((each) => each.item.standing === 'open').length;
  const missed = goals.filter((each) => each.item.standing === 'missed').length;
  return part("Goals", open + ' open' + (missed ? ', ' + missed + ' missed' : ''), "#/people", <>
    <table className="dash-table"><tbody>
      {goals.map(({ row, item }) => <tr key={row.agent.id + '/' + item.goal.id} data-goal={item.standing}>
        <td>{row.agent.display_name}</td>
        <td className="sec">{item.goal.words}</td>
        <td className="sec you-act">{STANDING[item.standing]}{item.goal.deadline === null ? '' : ', due ' + day(item.goal.deadline)}</td>
      </tr>)}
      {unread.map((row) => <tr key={row.agent.id} data-goal="unread"><td>{row.agent.display_name}</td>
        <td className="why-not" colSpan={2}>Its goals could not be read. {refusedPart(row.goals) ? <small className="refusal-name">{row.goals.refusal}</small> : null}</td></tr>)}
      {goals.length || unread.length ? null : <tr className="empty"><td className="dim">No goal is set.</td></tr>}
    </tbody></table>
  </>);
}

export const RequestsWidget = (props: Parameters<typeof requestsPart>[0]) => card(requestsPart(props));
export const DraftsWidget = (props: Parameters<typeof draftsPart>[0]) => card(draftsPart(props));
export const BudgetWidget = (props: Parameters<typeof budgetPart>[0]) => card(budgetPart(props));
export const GoalsWidget = (props: Parameters<typeof goalsPart>[0]) => card(goalsPart(props));
