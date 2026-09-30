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
    <fieldset disabled={change.blocked} style={{ border: 0, padding: 0 }}>
      {!role ? <label className="field">Role name<input name="name" required maxLength={100} /></label> : null}
      {(['responsibilities', 'goals', 'practice'] as const).map((name) => <label className="field" key={name}>{name === 'responsibilities' ? 'Responsibilities' : name === 'goals' ? 'Goals' : 'How the work is done'}<textarea name={name} required maxLength={4000} defaultValue={current?.[name] ?? ''} /></label>)}
      <details><summary>Starting instructions and suggested access</summary>
        <label className="field">Starting instructions for an agent<textarea name="profile" maxLength={4000} defaultValue={current?.profile ?? ''} /></label>
        {rows.map(({ id, template }) => <fieldset key={id}><legend>Access template {id + 1}</legend>
          <label className="field">Type of thing this access covers<input name={'kind-' + id} required defaultValue={template.resource.kind} /></label>
          <label className="field">Name of the thing this access covers<input name={'resource-' + id} required defaultValue={template.resource.id} /></label>
          <label className="field">Access level<select name={'relation-' + id} required defaultValue={template.relation}><option value="">Choose access</option>{Object.entries(model.relations).map(([name, actions]) => <option key={name} value={name}>{name} · {actions.join(', ')}</option>)}</select></label>
          <label className="field">Usual duration in days<input name={'days-' + id} type="number" min={1} step={1} defaultValue={template.days ?? ''} /></label>
          <p className="note">An empty duration records no expiry of its own. A template grants nothing by itself.</p>
          <button className="btn" type="button" onClick={() => setRows((values) => values.filter((row) => row.id !== id))}>Remove template</button>
        </fieldset>)}
        <button className="btn" type="button" onClick={() => setRows((values) => [...values, { id: Math.max(-1, ...values.map((value) => value.id)) + 1, template: { resource: { kind: '', id: '' }, relation: '', days: null } }])}>Add access template</button>
      </details>
      <label className="field">Reason for this version<textarea name="note" required maxLength={500} /></label>
      <button className="btn primary" type="submit">{role ? 'Save this version' : 'Create role'}</button>
    </fieldset>
    {failure ? <p role="alert">{failure}</p> : null}<ChangeStatus change={change} />
  </form>;
}
