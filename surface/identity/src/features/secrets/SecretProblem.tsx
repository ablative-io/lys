/**
 * A failure on the secrets screens, said in plain words with its code and the service's own reason kept under Error
 * details. Two names are the screens' own rather than the service's: an answer that did not confirm what was asked,
 * and a request that was never answered. Each is said as what it means for the change, never as "Lys refused".
 */
import { Refused } from '../../api';
import { ErrorWords } from '../people/Words';

const OWN: Record<string, string> = {
  UnconfirmedAnswer: 'Lys answered, but its answer did not confirm this change. Check whether it was made before trying again.',
  Unanswered: 'Lys did not answer, so it is not known whether this change was made. Check before trying again.',
  PendingNotRecorded: 'This tab could not keep a record of the change, so it was not sent. Nothing was changed.',
};

export function SecretProblem({ problem }: { problem: unknown }) {
  if (problem === null || problem === undefined || problem === '') return null;
  if (typeof problem === 'string') return <p className="why-not" role="alert">{problem}</p>;
  const own = problem instanceof Refused ? OWN[problem.refusal.refusal] : undefined;
  if (!own) return <ErrorWords problem={problem} />;
  const refused = problem as Refused;
  return <div className="why-not" role="alert">
    <p>{own}</p>
    <details><summary>Error details</summary><p className="refusal-name" style={{ overflowWrap: 'anywhere' }}>{refused.refusal.refusal + ': ' + refused.refusal.reason}</p></details>
  </div>;
}
