/** The one account of a kept change beside its form: that it has no confirmed answer and how to check it, a refusal in plain words, and that it was recorded. */
import type { RoleChange } from './useRoleChange';
import { ErrorWords } from '../people/Words';
import { Act } from '../../shell/Act';

export function ChangeStatus({ change }: { change: RoleChange }) {
  return <>{change.pending ? <div role="status"><p>This change has no confirmed answer. Its original details are retained.</p><Act symbol="again" name="Check whether Lys saved it" word="Check" disabled={change.busy} onClick={change.retry} /></div> : null}
    {change.failure ? <ErrorWords problem={change.failure} /> : null}
    {change.done ? <p role="status">Change recorded.</p> : null}</>;
}
