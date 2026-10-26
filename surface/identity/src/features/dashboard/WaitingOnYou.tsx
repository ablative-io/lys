/**
 * What waits on the person, as one list: `GET /waiting`'s answer drawn as it comes, oldest first, each line decided
 * where it stands (Tom, 18 October 2026: nothing is decided on another page). A request entry carries the request as
 * `GET /requests` answers it and its row approves or declines it; an agent's draft carries the draft as `GET /drafts`
 * answers it and its row approves or declines it; an app action carries the same controls as Drafts, and one check row
 * keeps exactly the shown grants of one agent. A request or a
 * draft sent without its record, by an older service, says so in its row and opens its page. The number in the heading
 * is how many lines there are, so the number and the list can never disagree, and it is the rail's number too. A part
 * the service could not read says so on its own line where it would stand, never as nothing, and while one is unread
 * no number is shown.
 */
import { useEffect, useState } from 'react';
import { Refused } from '../../api';
import { DraftDecide } from '../drafts/DraftDecide';
import { ProductDraftDecide } from '../drafts/ProductDrafts';
import { KeepAll } from '../reviews/KeepGrant';
import { DecisionForm } from '../requests/DecisionForm';
import { Unread } from './Unread';
import { PLACE, UNREAD, WAITING_HOME, focusWaiting, isRefusedPart, opens, waitingCount, waitingSettled } from './waiting';
import type { WaitingEntry, WaitingLine } from './waiting';

/** A source grant in words on the dashboard, by its place in the request's list; the requests page names what each is on. */
const sourceWords = (_source: string, index: number): string => 'Access you hold, ' + (index + 1);

/** What a row says when the service sent a request or a draft without its record, so it is decided on its own page. */
const UNSENT: Partial<Record<WaitingEntry['kind'], string>> = {
  request: 'This Lys did not send the request with it, so it is decided on its page.',
  agent_draft: 'This Lys did not send the draft with it, so it is decided on its page.',
  product_draft: 'This Lys did not send the app action with it, so it is decided on its page.',
  review: 'This Lys did not send the grants with it, so they are checked on their page.',
};

/** One entry's row: decided where it stands when it carries its record, else its Open to the page it stands for. */
function Row({ line, me, changed }: { line: WaitingEntry; me: string; changed: () => void }) {
  if (line.kind === 'request' && line.request) return <tr data-waiting-entry="request" data-request={line.id}>
    <td>{line.about}{line.request.why ? <div className="note">{line.request.why}</div> : null}</td>
    <td className="you-act dash-decide"><DecisionForm entry={line.request} person={me} canIssueRoot={line.request.can_issue_root === true} changed={changed} describe={sourceWords} /></td>
  </tr>;
  if (line.kind === 'agent_draft' && line.draft) return <tr data-waiting-entry="agent_draft" data-draft="waiting">
    <td>{line.about}</td>
    <td className="you-act dash-decide"><DraftDecide draft={line.draft} person={me} changed={changed} /></td>
  </tr>;
  if (line.kind === 'product_draft' && line.product_draft) return <tr data-waiting-entry="product_draft">
    <td>{line.about}<details><summary>Prepared action</summary><pre>{line.product_draft.words}</pre></details></td>
    <td className="you-act dash-decide"><ProductDraftDecide draft={line.product_draft} person={me} changed={changed} /></td>
  </tr>;
  if (line.kind === 'review' && line.review) {
    const review = line.review;
    return <tr data-waiting-entry="review">
      <td><details><summary>{line.about}</summary><ul>{review.grants.map((grant) => <li key={grant.id}>
        {grant.relation} on {grant.resource} · last checked {new Date(grant.last_check * 1000).toLocaleDateString()}
      </li>)}</ul></details></td>
      <td className="you-act dash-decide"><KeepAll agent={review.agent.id} grants={review.grants.map((grant) => grant.id)} person={me} changed={changed} />
        <a className="btn" href={'#/file/' + encodeURIComponent(review.agent.id) + '/access'}>Open</a></td>
    </tr>;
  }
  const { href, word } = opens(line);
  const unsent = UNSENT[line.kind];
  return <tr data-waiting-entry={line.kind}>
    <td>{line.about}{unsent ? <div className="note" data-waiting="unsent">{unsent}</div> : null}</td>
    <td className="you-act"><a className="btn" href={href}>{word}</a></td>
  </tr>;
}

export function WaitingOnYou({ waiting, me, changed = () => undefined }: { waiting: WaitingLine[] | Refused; me: string; changed?: () => void }) {
  const [completed, setCompleted] = useState<{ source: typeof waiting; ids: Set<string> }>({ source: waiting, ids: new Set() });
  const lines = waiting instanceof Refused ? waiting : waiting.filter((line) => isRefusedPart(line) || completed.source !== waiting || !completed.ids.has(line.kind + ':' + line.id));
  const done = (line: WaitingEntry) => {
    setCompleted((held) => ({ source: waiting, ids: new Set([...(held.source === waiting ? held.ids : []), line.kind + ':' + line.id]) }));
    waitingSettled(line.kind, line.id);
    changed();
  };
  useEffect(() => {
    const focus = () => { if (window.location.hash === WAITING_HOME) focusWaiting(); };
    focus();
    window.addEventListener('hashchange', focus);
    return () => window.removeEventListener('hashchange', focus);
  }, []);
  const count = lines instanceof Refused ? null : waitingCount(lines);
  return <section id="waiting-on-you" tabIndex={-1} className="you-panel dash-waiting" aria-label="Waiting on you">
    <div className="section-h"><span>Waiting on you</span>{count ? <b className="dash-count" data-waiting="count">{count}</b> : null}</div>
    <div className="you-scroll"><table className="dash-table"><tbody>
      {lines instanceof Refused
        ? <tr data-waiting="all"><td colSpan={2}><Unread part={lines} words="What waits on you could not be read" /></td></tr>
        : lines.map((line) => isRefusedPart(line)
          // A part that could not be read keeps its Open, so the place it stands for is still one press away.
          ? <tr key={'unread-' + line.kind} data-waiting={line.kind}><td><Unread part={line} words={UNREAD[line.kind]} /></td>
            <td className="you-act"><a className="btn" href={PLACE[line.kind]}>Open</a></td></tr>
          : <Row key={line.kind + line.id} line={line} me={me} changed={() => done(line)} />)}
      {count === 0 ? <tr className="empty"><td className="dim" colSpan={2}>Nothing is waiting on you.</td></tr> : null}
    </tbody></table></div>
    <footer><a href="#/requests">Requests</a> · <a href="#/reviews">Checks</a> · <a href="#/access/drafts">Drafts</a></footer>
  </section>;
}
