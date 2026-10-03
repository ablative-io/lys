/** The one account of a kept change beside its form: that it has no confirmed answer and how to check it, a refusal in plain words, and that it was recorded. */
import type { RoleChange } from './useRoleChange';
import { ErrorWords } from '../people/Words';

export function ChangeStatus({ change }: { change: RoleChange }) {
  return <>{change.pending ? <div role="status"><p>This change has no confirmed answer. Its original details are retained.</p><button className="btn" type="button" disabled={change.busy} onClick={change.retry}>Check whether Lys saved it</button></div> : null}
    {change.failure ? <ErrorWords problem={change.failure} /> : null}
    {change.done ? <p role="status">Change recorded.</p> : null}</>;
}
