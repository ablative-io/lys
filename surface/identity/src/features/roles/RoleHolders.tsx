/** Assignments and reviewed version moves name the exact holding so retries cannot affect a successor. */
import { useState } from 'react';
import { Picker } from '../../shell/Picker';
import type { FormEvent } from 'react';
import { operationId } from '../../api';
import { field } from '../people/RecordedForm';
import { clock } from '../file/time';
import type { Role, RoleAnswer, RoleHolder, RoleVersion } from './contract';
import { useRoleChange } from './useRoleChange';
import { DirectoryChangeStatus as ChangeStatus } from './ChangeStatus';
import { IdentityName, PART } from '../people/Words';

export function AssignRole({ role, person, identities, changed }: { role: Role; person: string; identities: { id: string; name: string }[]; changed: (answer: RoleAnswer) => void }) {
  const [noExpiry, setNoExpiry] = useState(false);
  const [failure, setFailure] = useState('');
  const change = useRoleChange<Role>('lys.pending.role-assignment.' + person + '.' + role.id, '/roles/' + encodeURIComponent(role.id) + '/holders',
    (answer, body) => answer.id === role.id && answer.holders.some((holder) => holder.assignment === body.operation && holder.holder === body.holder && holder.ends_at === body.ends_at && holder.assigned_by === person), changed);
  const submit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault(); if (change.blocked) return;
    const data = new FormData(event.currentTarget); const holder = field(data, 'holder');
    const ends = noExpiry ? null : Date.parse(field(data, 'expires')) / 1000;
    if (!identities.some((identity) => identity.id === holder) || (ends !== null && (!Number.isSafeInteger(ends) || ends <= Date.now() / 1000))) { setFailure('Choose a person or agent and a future end date, or select No expiry.'); return; }
    change.submit({ operation: operationId(), holder, ends_at: ends });
  };
  return <form className="card recorded-form" aria-label="Assign role" onSubmit={submit}>
    <h2>Assign {role.name}</h2><p>This assigns the latest saved version to the selected person or agent. Access grants are separate.</p>
    <fieldset disabled={change.blocked} style={{ border: 0, padding: 0 }}>
      <div className="field">Person or agent<Picker name="holder" label="Find a person or agent" options={identities} /></div>
      <label className="field">Role until (your local time)<input type="datetime-local" name="expires" required={!noExpiry} disabled={noExpiry} /></label>
      <label><input type="checkbox" checked={noExpiry} onChange={(event) => setNoExpiry(event.target.checked)} /> No expiry</label>
      <p><button className="btn primary" type="submit">Assign role</button></p>
    </fieldset>{failure ? <p role="alert">{failure}</p> : null}<ChangeStatus change={change} />
  </form>;
}

function VersionDifference({ before, after }: { before: RoleVersion; after: RoleVersion }) {
  return <table><thead><tr><th>Part</th><th>Version {before.number}</th><th>Version {after.number}</th></tr></thead><tbody>
    {(['responsibilities', 'goals', 'practice', 'profile'] as const).map((part) => <tr key={part}><th>{PART[part]}</th><td style={{ whiteSpace: 'pre-wrap' }}>{before[part] || 'Empty'}</td><td style={{ whiteSpace: 'pre-wrap' }}>{after[part] || 'Empty'}</td></tr>)}
    <tr><th>Access templates</th><td>{before.grant_templates.map((template) => `${template.relation} of ${template.resource.kind}:${template.resource.id} (${template.days === null ? 'no own expiry' : template.days + ' days'})`).join('; ') || 'None'}</td><td>{after.grant_templates.map((template) => `${template.relation} of ${template.resource.kind}:${template.resource.id} (${template.days === null ? 'no own expiry' : template.days + ' days'})`).join('; ') || 'None'}</td></tr>
  </tbody></table>;
}

function HolderAction({ role, holder, person, action, changed }: { role: Role; holder: RoleHolder; person: string; action: 'move' | 'end'; changed: (answer: RoleAnswer) => void }) {
  const [confirm, setConfirm] = useState(false);
  const before = role.versions.find((version) => version.number === holder.version);
  const after = role.versions.find((version) => version.number === role.latest);
  const base = '/roles/' + encodeURIComponent(role.id) + '/holders/' + encodeURIComponent(holder.holder);
  const change = useRoleChange<RoleAnswer>('lys.pending.role-' + action + '.' + person + '.' + role.id + '.' + holder.assignment, base + '/' + action, (answer, body) => {
    if (action === 'end') return 'id' in answer && answer.id === role.id && answer.holders.some((entry) => entry.assignment === body.assignment && entry.state === 'ended');
    return 'role' in answer && answer.role === role.id && answer.holder.assignment === body.assignment && answer.holder.holder === holder.holder
      && answer.to.number === body.to_version && answer.from.number === body.from_version && answer.holder.ends_at === holder.ends_at;
  }, changed);
  if (!holder.assignment) return <p className="why-not">The server must identify this assignment before it can be changed safely.</p>;
  const title = action === 'move' ? 'Review changes in version ' + role.latest : 'Remove this role assignment';
  return <div>{confirm || change.pending ? <>
    <h4>{title} for {holder.display_name ?? 'Name unavailable'}?</h4>
    {action === 'move' && before && after ? <VersionDifference before={before} after={after} /> : null}
    <p>{action === 'move' ? 'The expiry stays ' + (holder.ends_at === null ? 'unset' : clock(holder.ends_at)) + '. Existing grants are not changed.' : 'The assignment stays in its history. This does not retire the identity or revoke separately issued grants.'}</p>
    {!change.pending ? <><button className="btn primary" disabled={change.blocked || (action === 'move' && (!before || !after))} onClick={() => change.submit(action === 'move' ? { assignment: holder.assignment, from_version: holder.version, to_version: role.latest } : { assignment: holder.assignment })}>{action === 'move' ? 'Apply version ' + role.latest : 'Yes, remove this assignment'}</button>{' '}<button className="btn" disabled={change.busy} onClick={() => setConfirm(false)}>Cancel</button></> : null}
  </> : <button className="btn" onClick={() => setConfirm(true)}>{title}</button>}<ChangeStatus change={change} /></div>;
}

export function RoleHolders({ role, person, admin, changed }: { role: Role; person: string; admin: boolean; changed: (answer: RoleAnswer) => void }) {
  return <section className="card"><h2>People and agents assigned this role</h2><p>Each person or agent keeps their assigned version until an administrator applies another. A version change never extends its expiry.</p>
    {role.holders.length ? role.holders.map((holder) => <article key={holder.assignment ?? holder.holder + ':' + holder.assigned_at}>
      <h3><a href={'#/file/' + holder.holder}>{holder.display_name ?? 'Name unavailable'}</a> · Version {holder.version} · {holder.state}</h3>
      <p>{holder.ends_at === null ? 'No expiry.' : 'Until ' + clock(holder.ends_at) + '.'} {holder.behind ? 'A newer version is available.' : ''}</p>
      {admin && holder.state === 'holding' ? <div>{holder.behind ? <HolderAction role={role} holder={holder} person={person} action="move" changed={changed} /> : null}<HolderAction role={role} holder={holder} person={person} action="end" changed={changed} /></div> : null}
      <details><summary>Assignment history</summary><p>Assigned {clock(holder.assigned_at)} by <IdentityName id={holder.assigned_by} />.</p>{holder.moves.map((move, index) => <p key={index}>Version {move.from} → {move.to}, {clock(move.at)}, by <IdentityName id={move.by} />.</p>)}{holder.ended_at !== null ? <p>Ended {clock(holder.ended_at)} by {holder.ended_by ? <IdentityName id={holder.ended_by} /> : 'Name unavailable'}.</p> : null}</details>
    </article>) : <p>No holders yet.</p>}
  </section>;
}
