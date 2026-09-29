/** One retained team act remains recoverable even when its member or team state has since changed. */
import { useState } from 'react';
import { operationId } from '../../api';
import type { Login } from '../../generated';
import { useRoleChange } from '../roles/useRoleChange';
import { ChangeStatus } from '../roles/ChangeStatus';
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
export function TeamActions({ team, person, login, members, administrator, changed }: { team: Team; person: string; login: Login; members: Member[]; administrator: boolean; changed: (message: string) => void }) {
  const key = 'lys.pending.team.' + person + '.' + team.id;
  const [initial] = useState(() => { try { return { action: held(key, team.id), error: '' }; } catch (error) { return { action: null, error: String(error) }; } });
  const [action, setAction] = useState<Action | null>(initial.action);
  const [member, setMember] = useState('');
  const label = (id: string) => members.find((entry) => entry.id === id)?.display_name ?? id;
  if (initial.error) return <p role="alert">{initial.error}</p>;
  if (action) return <Act team={team} login={login} action={action} memberName={action.member ? label(action.member) : null} storageKey={key} changed={changed} cancel={() => setAction(null)} />;
  if (team.state === 'retired') return null;
  return <><label className="field">Add a member<select value={member} onChange={(event) => setMember(event.target.value)}><option value="">Choose a person or agent</option>{members.filter((entry) => entry.state !== 'retired' && !team.members.includes(entry.id)).map((entry) => <option key={entry.id} value={entry.id}>{entry.display_name}</option>)}</select></label>
    <button className="btn" disabled={!member} onClick={() => setAction({ act: 'added', member })}>Add member</button>
    {team.members.map((id) => <button className="btn" key={id} onClick={() => setAction({ act: 'removed', member: id })}>Remove {label(id)}</button>)}
    {administrator ? (team.held ?? []).map((held) => <button className="btn" key={'confirm:' + held.member} onClick={() => setAction({ act: 'confirmed', member: held.member })}>Allow {label(held.member)} to take part</button>) : null}
    <button className="btn danger" onClick={() => setAction({ act: 'retired', member: null })}>Retire team</button>
  </>;
}
function Act({ team, login, action, memberName, storageKey, changed, cancel }: { team: Team; login: Login; action: Action; memberName: string | null; storageKey: string; changed: (message: string) => void; cancel: () => void }) {
  const change = useRoleChange<TeamChanged>(storageKey, pathOf(team.id, action), (answer, body) => answer.id === team.id && answer.recorded?.operation === body.operation && answer.recorded.act === action.act && answer.recorded.member === action.member && sameLogin(answer.recorded.by, login), () => changed('Your team change was recorded. The list shows the team as it stands now.'));
  const word = action.act === 'added' ? 'add member' : action.act === 'removed' ? 'remove member' : action.act === 'confirmed' ? 'allow member to take part' : 'retire team';
  const question = action.act === 'confirmed' ? 'Allow ' + memberName + ' to take part in ' + team.name + '?' : action.act === 'added' ? 'Add ' + memberName + ' to ' + team.name + '?' : action.act === 'removed' ? 'Remove ' + memberName + ' from ' + team.name + '?' : 'Retire ' + team.name + '?';
  return <section aria-label="Confirm team change"><p>{question} This does not grant or revoke access.</p>
    <button className="btn primary" disabled={change.blocked} onClick={() => change.submit({ operation: operationId(), ...(action.act === 'added' ? { member: action.member } : {}) })}>Confirm {word}</button>
    <button className="btn" disabled={change.busy || change.pending} onClick={cancel}>Cancel</button><ChangeStatus change={change} />
  </section>;
}
