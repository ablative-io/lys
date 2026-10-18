/**
 * What waits on the person, as one list: `GET /waiting`'s answer drawn as it comes, oldest first, each line decided
 * where it stands (Tom, 18 October 2026: nothing is decided on another page). A request entry carries the request as
 * `GET /requests` answers it and its row approves or declines it; an agent's draft carries the draft as `GET /drafts`
 * answers it and its row approves or declines it; an app action and a check due open their own pages. A request or a
 * draft sent without its record, by an older service, says so in its row and opens its page. The number in the heading
 * is how many lines there are, so the number and the list can never disagree, and it is the rail's number too. A part
 * the service could not read says so on its own line where it would stand, never as nothing, and while one is unread
 * no number is shown.
 */
import { Refused } from '../../api';
import { DraftDecide } from '../drafts/DraftDecide';
import { DecisionForm } from '../requests/DecisionForm';
import { Unread } from './Unread';
import { PLACE, UNREAD, isRefusedPart, opens, waitingCount } from './waiting';
import type { WaitingEntry, WaitingLine } from './waiting';

/** A source grant in words on the dashboard, by its place in the request's list; the requests page names what each is on. */
const sourceWords = (_source: string, index: number): string => 'Access you hold, ' + (index + 1);

/** What a row says when the service sent a request or a draft without its record, so it is decided on its own page. */
const UNSENT: Partial<Record<WaitingEntry['kind'], string>> = {
  request: 'This Lys did not send the request with it, so it is decided on its page.',
  agent_draft: 'This Lys did not send the draft with it, so it is decided on its page.',
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
  const { href, word } = opens(line);
  const unsent = UNSENT[line.kind];
  return <tr data-waiting-entry={line.kind}>
    <td>{line.about}{unsent ? <div className="note" data-waiting="unsent">{unsent}</div> : null}</td>
    <td className="you-act"><a className="btn" href={href}>{word}</a></td>
  </tr>;
}

export function WaitingOnYou({ waiting, me, changed = () => undefined }: { waiting: WaitingLine[] | Refused; me: string; changed?: () => void }) {
  const count = waiting instanceof Refused ? null : waitingCount(waiting);
  return <section className="you-panel dash-waiting" aria-label="Waiting on you">
    <div className="section-h"><span>Waiting on you</span>{count ? <b className="dash-count" data-waiting="count">{count}</b> : null}</div>
    <div className="you-scroll"><table className="dash-table"><tbody>
      {waiting instanceof Refused
        ? <tr data-waiting="all"><td colSpan={2}><Unread part={waiting} words="What waits on you could not be read" /></td></tr>
        : waiting.map((line) => isRefusedPart(line)
          // A part that could not be read keeps its Open, so the place it stands for is still one press away.
          ? <tr key={'unread-' + line.kind} data-waiting={line.kind}><td><Unread part={line} words={UNREAD[line.kind]} /></td>
            <td className="you-act"><a className="btn" href={PLACE[line.kind]}>Open</a></td></tr>
          : <Row key={line.kind + line.id} line={line} me={me} changed={changed} />)}
      {count === 0 ? <tr className="empty"><td className="dim" colSpan={2}>Nothing is waiting on you.</td></tr> : null}
    </tbody></table></div>
  </section>;
}
