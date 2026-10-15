/** An agent's variables (AGENTS-001 R5): the map with each value's revision, author and expiry, the names that have expired, and a form that patches one key from the revision read; a stale read is refused by name and never overwrites blind. */
import { useState } from 'react';
import { useRoleChange } from '../roles/useRoleChange';
import { ChangeStatus } from '../roles/ChangeStatus';
import { Act } from '../../shell/Act';
import { clock } from '../file/time';
import { IdentityName } from '../people/Words';
import type { VariablesRead } from './contract';

type Props = { agent: string; variables: VariablesRead; changed: (words: string) => void };

const COLUMNS = ['18%', '30%', '15%', '23%', '14%'];

/** A name the service keeps for a variable: a lower-case letter, then lower-case letters, digits or _, at most 64. */
const NAME = /^[a-z][a-z0-9_]{0,63}$/;

/** Who set a value: a person or agent by name; anything else as written. */
const Author = ({ id }: { id: string }) => /^(person|agent)-[0-9a-f]{32}$/.test(id) ? <IdentityName id={id} /> : <>{id}</>;

/** A datetime-local value as seconds since the epoch, or null when it is empty or not a time. */
function seconds(local: string): number | null {
  const ms = Date.parse(local);
  return local && Number.isFinite(ms) ? Math.floor(ms / 1000) : null;
}

/** A value as typed: JSON when it parses as JSON, else the text itself. */
function parsed(given: string): unknown {
  try { return JSON.parse(given) as unknown; } catch { return given; }
}

export function Variables({ agent, variables, changed }: Props) {
  const path = '/agents/' + encodeURIComponent(agent) + '/variables';
  const key = 'lys.pending.variables.' + agent;
  const [name, setName] = useState('');
  const [value, setValue] = useState('');
  const [expiry, setExpiry] = useState('');
  const patch = useRoleChange<VariablesRead>(key + '.set', path,
    (answer, body) => typeof answer?.revision === 'number' && answer.revision === Number(body.revision) + 1,
    () => changed('Variable set.'));
  const remove = useRoleChange<VariablesRead>(key + '.remove', path,
    (answer, body) => typeof answer?.revision === 'number' && answer.revision === Number(body.revision) + 1,
    () => changed('Variable removed.'));
  const until = seconds(expiry);
  const why = name && !NAME.test(name) ? 'A name starts with a lower-case letter and has only lower-case letters, digits and _, for example ward_list.'
    : expiry && (until === null || until <= Date.now() / 1000) ? 'Keep until must be a time still to come, or empty.' : '';
  const valid = NAME.test(name) && value.trim().length > 0 && !why;
  const entries = Object.entries(variables.values);
  return <section className="card usage-variables" aria-label="Variables"><h3>Variables</h3>
    <p className="note">Values kept for this agent, which it and its work can read. If someone changed them after this page was read, a change here is refused rather than overwriting theirs; reload and try again.</p>
    <table className="usage-list usage-table">
      <colgroup>{COLUMNS.map((width, index) => <col key={index} style={{ width }} />)}</colgroup>
      <thead><tr><th>Name</th><th>Value</th><th>Set by</th><th>Kept until</th><th>Change</th></tr></thead>
      <tbody>
        {entries.map(([held, variable]) => <tr key={held}>
          <td>{held}</td>
          <td><code>{typeof variable.value === 'string' ? variable.value : JSON.stringify(variable.value)}</code></td>
          <td><Author id={variable.author} /></td>
          <td>{variable.expires_at ? clock(variable.expires_at) : 'Until removed'}</td>
          <td><Act symbol="decline" name={'Remove the variable ' + held} word="Remove" disabled={remove.blocked} onClick={() => remove.submit({ revision: variables.revision, values: { [held]: null } })} /></td>
        </tr>)}
        {entries.length ? null : <tr><td colSpan={5} className="dim">No variable is set for this agent.</td></tr>}
        {variables.expired.length ? <tr><td colSpan={5} className="dim">No longer kept, their time having passed: {variables.expired.join(', ')}</td></tr> : null}
      </tbody>
      <tfoot><tr>
        <td><input form="set-variable" name="name" aria-label="Variable name" placeholder="Name, for example ward" autoComplete="off" spellCheck={false} value={name} required disabled={patch.blocked} onChange={(event) => setName(event.target.value)} /></td>
        <td><input form="set-variable" name="value" aria-label="Variable value" placeholder="Its value" autoComplete="off" value={value} required disabled={patch.blocked} onChange={(event) => setValue(event.target.value)} /></td>
        <td className="dim">You</td>
        <td><input form="set-variable" type="datetime-local" name="expires_at" aria-label="Keep until, empty to keep it until removed" title="Keep until (empty: until removed)" value={expiry} disabled={patch.blocked} onChange={(event) => setExpiry(event.target.value)} /></td>
        <td><Act form="set-variable" symbol="add" type="submit" name="Set this variable" disabled={patch.blocked || !valid} /></td>
      </tr>{why ? <tr><td colSpan={5}><p className="why-not" role="alert">{why}</p></td></tr> : null}</tfoot>
    </table>
    <form id="set-variable" aria-label="Set a variable" onSubmit={(event) => {
      event.preventDefault();
      if (patch.blocked || !valid) return;
      const body: Record<string, unknown> = { revision: variables.revision, values: { [name]: parsed(value) } };
      if (until !== null) body.expires_at = until;
      patch.submit(body);
    }}>
      <ChangeStatus change={patch} />
    </form>
    <ChangeStatus change={remove} />
  </section>;
}
