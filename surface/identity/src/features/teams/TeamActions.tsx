/** One retained team act remains recoverable even when its member or team state has since changed. */
import { useState } from 'react';
import { operationId } from '../../api';
import type { Login } from '../../generated';
import { useRoleChange } from '../roles/useRoleChange';
import { DirectoryChangeStatus as ChangeStatus } from '../roles/ChangeStatus';
import { sameLogin } from './contract';
import type { Member, Team, TeamChanged } from './contract';
type Action = { act: 'added' | 'removed' | 'retired' | 'confirmed'; member: string | null };
const pathOf = (id: string, action: Action) => '/teams/' + encodeURIComponent(id) + (action.act === 'added' ? '/members' : action.act === 'retired' ? '/retire' : '/members/' + encodeURIComponent(action.member ?? '') + (action.act === 'confirmed' ? '/confirm' : '/remove'));
function held(key: string, id: string): Action | null {
  const raw = sessionStorage.getItem(key); if (!raw) return null;
  const value: unknown = JSON.parse(raw);
  if (!value || typeof value !== 'object' || !('path' in value) || typeof value.path !== 'string') throw new Error('The retained team change cannot be read. Resolve its outcome before another change.');
  if (value.path === pathOf(id, { act: 'retired', member: null })) return { act: 'retired', member: null };
  if (value.path === pathOf(id, { act: 'added', member: null }) && 'body' in value && value.body && typeof value.body === 'object' && 'member' in value.body && typeof value.body.member === 'string') return { act: 'added', member: value.body.member };
  const prefix = '/teams/' + encodeURIComponent(id) + '/members/';
  if (value.path.startsWith(prefix) && value.path.endsWith('/remove')) return { act: 'removed', member: decodeURIComponent(value.path.slice(prefix.length, -'/remove'.length)) };
  if (value.path.startsWith(prefix) && value.path.endsWith('/confirm')) return { act: 'confirmed', member: decodeURIComponent(value.path.slice(prefix.length, -'/confirm'.length)) };
  throw new Error('The retained team change names an unexpected route. Resolve its outcome before another change.');
}
/** A team's members as a table: remove on each row, add as the last row. One change is confirmed at a time, above the table. */
export function TeamMembers({ team, person, login, members, name, administrator, manages, changed }: { team: Team; person: string; login: Login; members: Member[]; name: (id: string) => string; administrator: boolean; manages: boolean; changed: (answer: TeamChanged, message: string) => void }) {
  const key = 'lys.pending.team.' + person + '.' + team.id;
  const [initial] = useState(() => { try { return { action: manages ? held(key, team.id) : null, error: '' }; } catch (error) { return { action: null, error: String(error) }; } });
  const [action, setAction] = useState<Action | null>(initial.action);
  const [member, setMember] = useState('');
  const open = manages && team.state !== 'retired' && !action && !initial.error;
  return <section className="card" aria-label="Members"><h3>Members <span className="sec">{team.members.length.toLocaleString('en-AU')}</span></h3>
    {initial.error ? <p role="alert">{initial.error}</p> : null}
    {action ? <Act team={team} login={login} action={action} memberName={action.member ? name(action.member) : null} storageKey={key} changed={changed} cancel={() => setAction(null)} /> : null}
    <table className="usage-table"><thead><tr><th>Member</th><th>Takes part</th>{manages ? <th>Change</th> : null}</tr></thead>
      <tbody>
        {team.members.map((id) => {
          const waiting = team.held?.find((each) => each.member === id);
          return <tr key={id} aria-label={'Member ' + name(id)}>
            <td><a href={'#/file/' + id}>{name(id)}</a></td>
            <td>{waiting ? <span className="why-not">Awaiting administrator confirmation. Team reminders and budget actions do not include this member. {waiting.reason}</span> : 'Yes'}</td>
            {manages ? <td>{open ? <>
              <button className="btn" type="button" aria-label={'Remove ' + name(id)} onClick={() => setAction({ act: 'removed', member: id })}>Remove</button>
              {administrator && waiting ? <button className="btn" type="button" aria-label={'Allow ' + name(id) + ' to take part'} onClick={() => setAction({ act: 'confirmed', member: id })}>Allow to take part</button> : null}
            </> : null}</td> : null}
          </tr>;
        })}
        {team.members.length ? null : <tr><td colSpan={manages ? 3 : 2} className="dim">No members yet.</td></tr>}
      </tbody>
      {open ? <tfoot><tr><td colSpan={3} className="usage-add"><div className="usage-add-row" style={{ gridTemplateColumns: 'minmax(0, 1fr) auto' }}>
        <select name="member" aria-label="Add a member" value={member} onChange={(event) => setMember(event.target.value)}><option value="">Choose a person or agent</option>{members.filter((entry) => entry.state !== 'retired' && !team.members.includes(entry.id)).map((entry) => <option key={entry.id} value={entry.id}>{entry.display_name}</option>)}</select>
        <span><button className="btn" type="button" disabled={!member} onClick={() => setAction({ act: 'added', member })}>Add member</button></span>
      </div></td></tr></tfoot> : null}
    </table>
    {open ? <p><button className="btn danger" type="button" onClick={() => setAction({ act: 'retired', member: null })}>Retire team</button></p> : null}
  </section>;
}
function Act({ team, login, action, memberName, storageKey, changed, cancel }: { team: Team; login: Login; action: Action; memberName: string | null; storageKey: string; changed: (answer: TeamChanged, message: string) => void; cancel: () => void }) {
  const change = useRoleChange<TeamChanged>(storageKey, pathOf(team.id, action), (answer, body) => answer.id === team.id && answer.recorded?.operation === body.operation && answer.recorded.act === action.act && answer.recorded.member === action.member && sameLogin(answer.recorded.by, login), (answer) => changed(answer, 'Your team change was recorded. The list shows the team returned with that change.'));
  const word = action.act === 'added' ? 'add member' : action.act === 'removed' ? 'remove member' : action.act === 'confirmed' ? 'allow member to take part' : 'retire team';
  const question = action.act === 'confirmed' ? 'Allow ' + memberName + ' to take part in ' + team.name + '?' : action.act === 'added' ? 'Add ' + memberName + ' to ' + team.name + '?' : action.act === 'removed' ? 'Remove ' + memberName + ' from ' + team.name + '?' : 'Retire ' + team.name + '?';
  return <section aria-label="Confirm team change"><p>{question} This does not grant or revoke access.</p>
    <button className="btn primary" disabled={change.blocked} onClick={() => change.submit({ operation: operationId(), ...(action.act === 'added' ? { member: action.member } : {}) })}>Confirm {word}</button>
    <button className="btn" disabled={change.busy || change.pending} onClick={cancel}>Cancel</button><ChangeStatus change={change} />
  </section>;
}
