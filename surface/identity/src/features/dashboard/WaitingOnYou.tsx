/**
 * What waits on the person, as one list: each request they may decide and each draft their agents prepared, on a line
 * of its own in words, then one line each for app actions held for approval and access due a check, which the service counts. The number
 * in the heading is how many lines there are to act on, counted from the lists themselves, read over every page, so
 * the number and the list can never disagree. Requests only someone else may decide are one line at the foot, never
 * mixed in with the person's own. A part that could not be read says so where it would stand, never as nothing.
 */
import { Refused } from '../../api';
import { counted } from '../../shell/count';
import type { Draft } from '../drafts/contract';
import { resourceWords } from '../grants/action-words';
import type { AccessRequest } from '../requests/contract';
import { refusedPart } from './contract';
import type { PartRefused, Waiting } from './contract';
import { Unread } from './Unread';

/** A grant's relation as the three words a person gives: view, edit, own; any other relation keeps its own name. */
const MAY: Record<string, string> = { viewer: 'view', editor: 'edit', owner: 'own' };

/** Whether the person may decide a request: the service's word when it gives one, else being among its approvers. */
export const mayDecide = (entry: AccessRequest, me: string): boolean => entry.can_decide ?? entry.approvers.some((each) => each.id === me);

/** "Sam asks to edit project Logo"; a relation with no plain word: "Sam asks for auditor access to project Logo". */
export const asks = (entry: AccessRequest): string =>
  (entry.asked_by_name ?? 'Someone outside your view')
  + (MAY[entry.relation] ? ' asks to ' + MAY[entry.relation] + ' ' : ' asks for ' + entry.relation + ' access to ') + resourceWords(entry.resource);

/** How many lines the list holds that the person can act on; null when a list it counts could not be read. */
export function waitingCount(requests: AccessRequest[] | Refused, drafts: Draft[] | Refused, waiting: Waiting, me: string): number | null {
  if (requests instanceof Refused || drafts instanceof Refused) return null;
  const counts = [waiting.product_drafts, waiting.reviews];
  if (counts.some((each) => refusedPart(each))) return null;
  const served = counts.reduce<number>((sum, each) => sum + (typeof each === 'number' ? each : 0), 0);
  return requests.filter((entry) => entry.state === 'waiting' && mayDecide(entry, me)).length + drafts.length + served;
}

/**
 * One counted line the service gave: "1 app action waits for your approval", linked; nothing when there are none. A
 * count that could not be read says so and keeps its Open, so the place it counts is still one press away.
 */
function CountLine({ count, words, unread, href, kind }: { count: number | PartRefused | undefined; words: (count: number) => string; unread: string; href: string; kind: string }) {
  if (count === undefined || count === 0) return null;
  return <tr data-waiting={kind}>{refusedPart(count)
    ? <td><Unread part={count} words={unread} /></td>
    : <td><a href={href}>{words(count)}</a></td>}<td className="you-act"><a className="btn" href={href}>Open</a></td></tr>;
}

export function WaitingOnYou({ requests, drafts, waiting, me }: { requests: AccessRequest[] | Refused; drafts: Draft[] | Refused; waiting: Waiting; me: string }) {
  const open = requests instanceof Refused ? [] : requests.filter((entry) => entry.state === 'waiting').sort((left, right) => left.asked_at - right.asked_at);
  const mine = open.filter((entry) => mayDecide(entry, me));
  const others = open.length - mine.length;
  const count = waitingCount(requests, drafts, waiting, me);
  const nothing = count === 0;
  return <section className="you-panel dash-waiting" aria-label="Waiting on you">
    <div className="section-h"><span>Waiting on you</span>{count ? <b className="dash-count" data-waiting="count">{count}</b> : null}</div>
    <div className="you-scroll"><table className="dash-table"><tbody>
      {requests instanceof Refused ? <tr data-waiting="requests"><td colSpan={2}><Unread part={requests} words="Requests could not be read" /></td></tr> : null}
      {mine.map((entry) => <tr key={entry.id} data-request={entry.id}>
        <td>{asks(entry)}{entry.why ? <div className="note">{entry.why}</div> : null}</td>
        <td className="you-act"><a className="btn" href={'#/requests/' + encodeURIComponent(entry.id)}>Decide</a></td>
      </tr>)}
      {drafts instanceof Refused ? <tr data-waiting="drafts"><td colSpan={2}><Unread part={drafts} words="Drafts could not be read" /></td></tr>
        : drafts.map((draft) => <tr key={draft.id} data-draft="waiting">
          <td>{(draft.agent?.display_name ?? 'An agent outside your view') + ' prepared something for you to approve'}{draft.note ? <div className="note">{draft.note}</div> : null}</td>
          <td className="you-act"><a className="btn" href="#/access/drafts">Decide</a></td>
        </tr>)}
      <CountLine count={waiting.product_drafts} words={(n) => counted(n, 'app actions wait for your approval', 'app action waits for your approval')}
        unread="Whether an app action waits for your approval could not be read" href="#/access/drafts" kind="product-drafts" />
      <CountLine count={waiting.reviews} words={(n) => 'Check the access you gave: ' + counted(n, 'are due', 'is due')}
        unread="Whether the access you gave is due a check could not be read" href="#/reviews" kind="reviews" />
      {nothing ? <tr className="empty"><td className="dim" colSpan={2}>Nothing is waiting on you.</td></tr> : null}
      {others ? <tr className="empty" data-waiting="others"><td className="dim" colSpan={2}><a href="#/requests">{others === 1 ? '1 more request waits' : others + ' more requests wait'} on someone else</a></td></tr> : null}
    </tbody></table></div>
  </section>;
}
