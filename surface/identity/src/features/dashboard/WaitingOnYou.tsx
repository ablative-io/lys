/**
 * What waits on the person, as one list: `GET /waiting`'s answer drawn as it comes, oldest first, each line in the
 * service's words with the button that decides it. The number in the heading is how many lines there are, so the
 * number and the list can never disagree, and it is the rail's number too. A part the service could not read says so
 * on its own line where it would stand, never as nothing, and while one is unread no number is shown.
 */
import { Refused } from '../../api';
import { Unread } from './Unread';
import { PLACE, UNREAD, isRefusedPart, opens, waitingCount } from './waiting';
import type { WaitingLine } from './waiting';

export function WaitingOnYou({ waiting }: { waiting: WaitingLine[] | Refused }) {
  const count = waiting instanceof Refused ? null : waitingCount(waiting);
  return <section className="you-panel dash-waiting" aria-label="Waiting on you">
    <div className="section-h"><span>Waiting on you</span>{count ? <b className="dash-count" data-waiting="count">{count}</b> : null}</div>
    <div className="you-scroll"><table className="dash-table"><tbody>
      {waiting instanceof Refused
        ? <tr data-waiting="all"><td colSpan={2}><Unread part={waiting} words="What waits on you could not be read" /></td></tr>
        : waiting.map((line) => {
          // A part that could not be read keeps its Open, so the place it stands for is still one press away.
          if (isRefusedPart(line)) return <tr key={'unread-' + line.kind} data-waiting={line.kind}><td><Unread part={line} words={UNREAD[line.kind]} /></td>
            <td className="you-act"><a className="btn" href={PLACE[line.kind]}>Open</a></td></tr>;
          const { href, word } = opens(line);
          return <tr key={line.kind + line.id} data-waiting-entry={line.kind}>
            <td>{line.about}</td>
            <td className="you-act"><a className="btn" href={href}>{word}</a></td>
          </tr>;
        })}
      {count === 0 ? <tr className="empty"><td className="dim" colSpan={2}>Nothing is waiting on you.</td></tr> : null}
    </tbody></table></div>
  </section>;
}
