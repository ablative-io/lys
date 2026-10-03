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

/** The holders table's last row: who, until when, and the button that assigns the latest saved version. */
function AssignRole({ role, person, identities, changed }: { role: Role; person: string; identities: { id: string; name: string }[]; changed: (answer: RoleAnswer) => void }) {
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
  return <form aria-label="Assign role" onSubmit={submit}>
    <fieldset className="usage-add-row" disabled={change.blocked} style={{ border: 0, padding: 0, margin: 0, gridTemplateColumns: 'minmax(0, 2fr) minmax(0, 1.4fr) auto auto' }}>
      <Picker name="holder" label="Find a person or agent" options={identities} />
      <input type="datetime-local" name="expires" aria-label="Role until, your local time" required={!noExpiry} disabled={noExpiry} />
      <label className="tick">No expiry<input type="checkbox" checked={noExpiry} onChange={(event) => setNoExpiry(event.target.checked)} /></label>
      <span><button className="btn primary" type="submit">Assign role</button></span>
    </fieldset>{failure ? <p role="alert">{failure}</p> : null}<ChangeStatus change={change} />
  </form>;
}

function VersionDifference({ before, after }: { before: RoleVersion; after: RoleVersion }) {
  return <table><thead><tr><th>Part</th><th>Version {before.number}</th><th>Version {after.number}</th></tr></thead><tbody>
    {(['responsibilities', 'goals', 'practice', 'profile'] as const).map((part) => <tr key={part}><th>{PART[part]}</th><td style={{ whiteSpace: 'pre-wrap' }}>{before[part] || 'Empty'}</td><td style={{ whiteSpace: 'pre-wrap' }}>{after[part] || 'Empty'}</td></tr>)}
    <tr><th>Access templates</th><td>{before.grant_templates.map((template) => `${template.relation} of ${template.resource.kind}:${template.resource.id} (${template.days === null ? 'no own expiry' : template.days + ' days'})`).join('; ') || 'None'}</td><td>{after.grant_templates.map((template) => `${template.relation} of ${template.resource.kind}:${template.resource.id} (${template.days === null ? 'no own expiry' : template.days + ' days'})`).join('; ') || 'None'}</td></tr>
  </tbody></table>;
}

/** One holder as a row; a move or an ending is reviewed in a row of its own beneath it, and names the exact assignment. */
function HolderRow({ role, holder, person, admin, changed }: { role: Role; holder: RoleHolder; person: string; admin: boolean; changed: (answer: RoleAnswer) => void }) {
  const [confirm, setConfirm] = useState<'move' | 'end' | null>(null);
  const before = role.versions.find((version) => version.number === holder.version);
  const after = role.versions.find((version) => version.number === role.latest);
  const base = '/roles/' + encodeURIComponent(role.id) + '/holders/' + encodeURIComponent(holder.holder);
  const key = (action: string) => 'lys.pending.role-' + action + '.' + person + '.' + role.id + '.' + holder.assignment;
  const move = useRoleChange<RoleAnswer>(key('move'), base + '/move', (answer, body) => 'role' in answer && answer.role === role.id && answer.holder.assignment === body.assignment && answer.holder.holder === holder.holder
    && answer.to.number === body.to_version && answer.from.number === body.from_version && answer.holder.ends_at === holder.ends_at, changed);
  const end = useRoleChange<RoleAnswer>(key('end'), base + '/end', (answer, body) => 'id' in answer && answer.id === role.id && answer.holders.some((entry) => entry.assignment === body.assignment && entry.state === 'ended'), changed);
  const open = confirm ?? (move.pending ? 'move' : end.pending ? 'end' : null);
  const change = open === 'move' ? move : end;
  const titles = { move: 'Review changes in version ' + role.latest, end: 'Remove this role assignment' };
  const name = holder.display_name ?? 'Name unavailable';
  return <>
    <tr aria-label={'Holder ' + name}>
      <td><a href={'#/file/' + holder.holder}>{name}</a></td>
      <td>Version {holder.version}{holder.behind ? <div className="note">A newer version is available.</div> : null}</td>
      <td>{holder.state}</td>
      <td>{holder.ends_at === null ? 'No expiry' : clock(holder.ends_at)}</td>
      <td>{clock(holder.assigned_at)} by <IdentityName id={holder.assigned_by} />
        {holder.moves.map((entry, index) => <div className="note" key={index}>Version {entry.from} → {entry.to}, {clock(entry.at)}, by <IdentityName id={entry.by} /></div>)}
        {holder.ended_at !== null ? <div className="note">Ended {clock(holder.ended_at)} by {holder.ended_by ? <IdentityName id={holder.ended_by} /> : 'Name unavailable'}</div> : null}</td>
      {admin ? <td>{holder.state !== 'holding' ? null : !holder.assignment ? <span className="why-not">The server must identify this assignment before it can be changed safely.</span> : open ? null : <>
        {holder.behind ? <button className="btn" type="button" onClick={() => setConfirm('move')}>{titles.move}</button> : null}
        <button className="btn" type="button" onClick={() => setConfirm('end')}>{titles.end}</button></>}
        <ChangeStatus change={move} /><ChangeStatus change={end} /></td> : null}
    </tr>
    {open && admin && holder.assignment ? <tr className="review-row"><td colSpan={6}>
      <h4>{titles[open]} for {name}?</h4>
      {open === 'move' && before && after ? <VersionDifference before={before} after={after} /> : null}
      <p>{open === 'move' ? 'The expiry stays ' + (holder.ends_at === null ? 'unset' : clock(holder.ends_at)) + '. Existing grants are not changed.' : 'The assignment stays in its history. This does not retire the identity or revoke separately issued grants.'}</p>
      {!change.pending ? <><button className="btn primary" type="button" disabled={change.blocked || (open === 'move' && (!before || !after))} onClick={() => change.submit(open === 'move' ? { assignment: holder.assignment, from_version: holder.version, to_version: role.latest } : { assignment: holder.assignment })}>{open === 'move' ? 'Apply version ' + role.latest : 'Yes, remove this assignment'}</button>{' '}<button className="btn" type="button" disabled={change.busy} onClick={() => setConfirm(null)}>Cancel</button></> : null}
    </td></tr> : null}
  </>;
}

export function RoleHolders({ role, person, admin, identities, changed }: { role: Role; person: string; admin: boolean; identities: { id: string; name: string }[]; changed: (answer: RoleAnswer) => void }) {
  return <section className="card" aria-label="Holders"><h2>People and agents assigned this role</h2><p className="note">Each person or agent keeps their assigned version until an administrator applies another. A version change never extends its expiry.</p>
    <table className="usage-table"><thead><tr><th>Who</th><th>Version</th><th>State</th><th>Until</th><th>Assigned</th>{admin ? <th>Change</th> : null}</tr></thead>
      <tbody>
        {role.holders.map((holder) => <HolderRow key={holder.assignment ?? holder.holder + ':' + holder.assigned_at} role={role} holder={holder} person={person} admin={admin} changed={changed} />)}
        {role.holders.length ? null : <tr><td colSpan={admin ? 6 : 5} className="dim">No holders yet.</td></tr>}
      </tbody>
      {admin ? <tfoot><tr><td colSpan={6} className="usage-add"><AssignRole key={role.id + ':' + role.holders.length} role={role} person={person} identities={identities} changed={changed} /></td></tr></tfoot> : null}
    </table>
  </section>;
}
