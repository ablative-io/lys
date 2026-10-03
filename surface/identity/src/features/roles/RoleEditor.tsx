/** Create a role or publish a new version; existing holders are never moved by this form. */
import { useState } from 'react';
import type { FormEvent } from 'react';
import { operationId } from '../../api';
import type { GrantModel } from '../../generated/grants';
import { field } from '../people/RecordedForm';
import { roleWordsOf, sameWords } from './contract';
import type { GrantTemplate, Role } from './contract';
import { useRoleChange } from './useRoleChange';
import { DirectoryChangeStatus as ChangeStatus } from './ChangeStatus';

export function RoleEditor({ role, person, model, changed }: { role?: Role; person: string; model: GrantModel; changed: (answer: Role) => void }) {
  const current = role?.versions.find((version) => version.number === role.latest);
  const [rows, setRows] = useState((current?.grant_templates ?? []).map((template, id) => ({ id, template })));
  const [failure, setFailure] = useState('');
  const path = role ? '/roles/' + encodeURIComponent(role.id) + '/versions' : '/roles';
  const change = useRoleChange<Role>('lys.pending.role.' + person + '.' + (role?.id ?? 'new'), path, (answer, body) => {
    const words = roleWordsOf(body);
    return words !== null && answer.id === (role?.id ?? body.operation) && answer.versions.some((version) => version.made_by === person && sameWords(version, words));
  }, changed);
  const submit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault(); if (change.blocked) return; setFailure('');
    try {
      const data = new FormData(event.currentTarget);
      const templates: GrantTemplate[] = rows.map(({ id }) => {
        const days = field(data, 'days-' + id);
        if (days !== '' && (!Number.isSafeInteger(Number(days)) || Number(days) <= 0)) throw new Error('A template duration must be a positive whole number of days.');
        return { resource: { kind: field(data, 'kind-' + id), id: field(data, 'resource-' + id) }, relation: field(data, 'relation-' + id), days: days === '' ? null : Number(days) };
      });
      change.submit({ operation: operationId(), ...(role ? {} : { name: field(data, 'name') }),
        responsibilities: field(data, 'responsibilities'), goals: field(data, 'goals'), practice: field(data, 'practice'),
        profile: field(data, 'profile'), note: field(data, 'note'), grant_templates: templates });
    } catch (error) { setFailure(String(error)); }
  };
  return <form className="card recorded-form" aria-label={role ? 'Publish role version' : 'Create role'} onSubmit={submit}>
    <h2>{role ? 'Save a new version of ' + role.name : 'Create a role'}</h2>
    <p>Describe the work and the access normally needed. Saving a version leaves existing assignments on their current version and grants no access.</p>
    <fieldset className="form-grid" disabled={change.blocked} style={{ border: 0, padding: 0 }}>
      {!role ? <label className="field wide">Role name<input name="name" required maxLength={100} /></label> : null}
      {(['responsibilities', 'goals', 'practice'] as const).map((name) => <label className="field wide" key={name}>{name === 'responsibilities' ? 'Responsibilities' : name === 'goals' ? 'Goals' : 'How the work is done'}<textarea name={name} rows={6} required maxLength={4000} defaultValue={current?.[name] ?? ''} /></label>)}
      <label className="field wide">Starting instructions for an agent<textarea name="profile" rows={6} maxLength={4000} defaultValue={current?.profile ?? ''} /></label>
      <table className="usage-table" aria-label="Access templates"><thead><tr><th>Type of thing</th><th>Name of the thing</th><th>Access level</th><th>Usual days</th><th>Change</th></tr></thead>
        <tbody>
          {rows.map(({ id, template }) => <tr key={id} aria-label={'Access template ' + (id + 1)}>
            <td><input name={'kind-' + id} aria-label="Type of thing this access covers" required defaultValue={template.resource.kind} /></td>
            <td><input name={'resource-' + id} aria-label="Name of the thing this access covers" required defaultValue={template.resource.id} /></td>
            <td><select name={'relation-' + id} aria-label="Access level" required defaultValue={template.relation}><option value="">Choose access</option>{Object.entries(model.relations).map(([name, actions]) => <option key={name} value={name}>{name} · {actions.join(', ')}</option>)}</select></td>
            <td><input name={'days-' + id} aria-label="Usual duration in days, empty for no expiry of its own" type="number" min={1} step={1} defaultValue={template.days ?? ''} /></td>
            <td><button className="btn" type="button" onClick={() => setRows((values) => values.filter((row) => row.id !== id))}>Remove template</button></td>
          </tr>)}
          {rows.length ? null : <tr><td colSpan={5} className="dim">No access template. A template grants nothing by itself.</td></tr>}
        </tbody>
        <tfoot><tr><td colSpan={5} className="usage-add"><button className="btn" type="button" onClick={() => setRows((values) => [...values, { id: Math.max(-1, ...values.map((value) => value.id)) + 1, template: { resource: { kind: '', id: '' }, relation: '', days: null } }])}>Add access template</button></td></tr></tfoot>
      </table>
      <label className="field wide">Reason for this version<textarea name="note" rows={2} required maxLength={500} /></label>
      <p><button className="btn primary" type="submit">{role ? 'Save this version' : 'Create role'}</button></p>
    </fieldset>
    {failure ? <p role="alert">{failure}</p> : null}<ChangeStatus change={change} />
  </form>;
}
