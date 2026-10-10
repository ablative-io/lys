/** An agent's variables (AGENTS-001 R5): the map with each value's revision, author and expiry, the names that have expired, and a form that patches one key from the revision read; a stale read is refused by name and never overwrites blind. */
import { useState } from 'react';
import { useRoleChange } from '../roles/useRoleChange';
import { ChangeStatus } from '../roles/ChangeStatus';
import { Act } from '../../shell/Act';
import { clock } from '../file/time';
import type { VariablesRead } from './contract';

type Props = { agent: string; variables: VariablesRead; changed: (words: string) => void };

const COLUMNS = ['20%', '36%', '10%', '20%', '14%'];

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
  const valid = /^[a-z][a-z0-9_]{0,63}$/.test(name) && value.trim().length > 0;
  const entries = Object.entries(variables.values);
  return <section className="card usage-variables" aria-label="Variables"><h3>Variables</h3>
    <p className="note">Revision {variables.revision}. A set carries the revision read; a stale one is refused by name.</p>
    <table className="usage-list usage-table">
      <colgroup>{COLUMNS.map((width, index) => <col key={index} style={{ width }} />)}</colgroup>
      <thead><tr><th>Name</th><th>Value</th><th>Revision</th><th>Who, and until</th><th>Change</th></tr></thead>
      <tbody>
        {entries.map(([held, variable]) => <tr key={held}>
          <td>{held}</td>
          <td><code>{typeof variable.value === 'string' ? variable.value : JSON.stringify(variable.value)}</code></td>
          <td>{variable.revision}</td>
          <td>{variable.author}{variable.expires_at ? ', until ' + clock(variable.expires_at) : ''}</td>
          <td><Act symbol="decline" name={'Remove the variable ' + held} word="Remove" disabled={remove.blocked} onClick={() => remove.submit({ revision: variables.revision, values: { [held]: null } })} /></td>
        </tr>)}
        {entries.length ? null : <tr><td colSpan={5} className="dim">No variable is set for this agent.</td></tr>}
        {variables.expired.length ? <tr><td colSpan={5} className="dim">Expired: {variables.expired.join(', ')}</td></tr> : null}
      </tbody>
      <tfoot><tr>
        <td><input form="set-variable" name="name" aria-label="Variable name" value={name} required disabled={patch.blocked} onChange={(event) => setName(event.target.value)} /></td>
        <td><input form="set-variable" name="value" aria-label="Variable value" value={value} required disabled={patch.blocked} onChange={(event) => setValue(event.target.value)} /></td>
        <td></td>
        <td><input form="set-variable" name="expires_at" aria-label="Expires at, seconds since the epoch" value={expiry} disabled={patch.blocked} onChange={(event) => setExpiry(event.target.value)} /></td>
        <td><Act form="set-variable" symbol="add" type="submit" name="Set this variable" disabled={patch.blocked || !valid} /></td>
      </tr></tfoot>
    </table>
    <form id="set-variable" aria-label="Set a variable" onSubmit={(event) => {
      event.preventDefault();
      if (patch.blocked || !valid) return;
      const body: Record<string, unknown> = { revision: variables.revision, values: { [name]: parsed(value) } };
      if (/^\d+$/.test(expiry)) body.expires_at = Number(expiry);
      patch.submit(body);
    }}>
      <ChangeStatus change={patch} />
    </form>
    <ChangeStatus change={remove} />
  </section>;
}
