/** The retained change and a named refusal remain visible alongside a role form. */
import type { RoleChange } from './useRoleChange';
export function ChangeStatus({ change }: { change: RoleChange }) {
  return <>{change.pending ? <div role="status"><p>This change has no confirmed answer. Its original details are retained.</p><button className="btn" type="button" disabled={change.busy} onClick={change.retry}>Check original change</button></div> : null}
    {change.failure ? <p role="alert" className="why-not">{change.failure}</p> : null}
    {change.done ? <p role="status">Change recorded.</p> : null}</>;
}
