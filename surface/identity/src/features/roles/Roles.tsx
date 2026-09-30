/** Read versioned roles, publish deliberate changes and manage explicitly identified holdings. */
import { useState } from 'react';
import { useParams } from 'react-router';
import { api, request, useLoad } from '../../api';
import { DirectoryGate as Gate, ErrorWords, IdentityName, PART } from '../people/Words';
import { clock } from '../file/time';
import type { Role, RoleAnswer } from './contract';
import { RoleEditor } from './RoleEditor';
import { AssignRole, RoleHolders } from './RoleHolders';

export function Roles() {
  const { id } = useParams();
  const [answers, setAnswers] = useState<Map<string, Role>>(new Map());
  const [editing, setEditing] = useState(false);
  const [notice, setNotice] = useState('');
  const load = useLoad(() => request<{ roles: Role[] }>('/roles'), 'roles');
  const roles = load.status === 'ok' ? [...new Map([...load.data.roles.map((role) => [role.id, role] as const), ...answers]).values()] : [];
  const changed = (answer: RoleAnswer) => {
    const updated = 'id' in answer ? answer : roles.find((role) => role.id === answer.role);
    if (!updated) throw new Error('The saved answer names a role that is not on this page. Open All roles to check the change.');
    const next = 'id' in answer ? answer : { ...updated, holders: updated.holders.map((holder) => holder.assignment === answer.holder.assignment ? answer.holder : holder) };
    setAnswers((values) => new Map([...values, [next.id, next]]));
    setEditing(false); setNotice('Lys saved the role change. The details below came with its answer.');
  };
  const authority = useLoad(async () => ({ people: await api.people(), me: await api.me(), model: await api.model() }), 'role-authority');
  const admin = authority.status === 'ok' && authority.data.people.scope === 'directory';
  return <div className="page"><div className="head"><div><div className="eyebrow">Directory</div><h1>Roles</h1><p className="sub">Describe a job, assign it to people or agents, and review changes before updating their assigned version.</p></div></div>
    {notice ? <p role="status">{notice}</p> : null}
    <Gate load={load} title="Roles" ok={() => {
      const role = id ? roles.find((entry) => entry.id === id) : undefined;
      const latest = role?.versions.find((version) => version.number === role.latest);
      return <>
        <nav aria-label="Roles"><a className="btn" href="#/roles">All roles</a>{roles.map((entry) => <a key={entry.id} className="btn" href={'#/roles/' + encodeURIComponent(entry.id)}>{entry.name} · v{entry.latest}</a>)}</nav>
        {id && !role ? <p className="why-not">This role was not returned by the service.</p> : null}
        {!id && !roles.length ? <p>No roles have been recorded.</p> : null}
        {role && latest ? <>
          <section className="card"><h2>{role.name} · Version {latest.number}</h2><p>Holders stay on their version until moved. A role assignment does not grant access.</p>
            {(['responsibilities', 'goals', 'practice', 'profile'] as const).map((part) => <section key={part}><h3>{PART[part]}</h3><p style={{ whiteSpace: 'pre-wrap' }}>{latest[part] || 'None recorded.'}</p></section>)}
            <h3>Access templates</h3>{latest.grant_templates.length ? <ul>{latest.grant_templates.map((template, index) => <li key={index}>{template.relation} of {template.resource.kind}:{template.resource.id} · {template.days === null ? 'no expiry of its own' : template.days + ' days'}</li>)}</ul> : <p>None recorded.</p>}
            <details><summary>Version history</summary>{role.versions.map((version) => <details key={version.number}><summary>Version {version.number} · {clock(version.made_at)}</summary><p>{version.note} · by <IdentityName id={version.made_by} /></p><dl>{(['responsibilities', 'goals', 'practice', 'profile'] as const).map((part) => <div key={part}><dt>{PART[part]}</dt><dd>{version[part] || 'Empty'}</dd></div>)}</dl></details>)}</details>
          </section>
          <RoleHolders role={role} person={authority.status === 'ok' ? authority.data.me.person.id : ''} admin={admin} changed={changed} />
        </> : null}
        {admin && authority.status === 'ok' ? <>
          <button className="btn" onClick={() => setEditing((value) => !value)}>{editing ? 'Close editor' : role ? 'New version' : 'Create a role'}</button>
          {editing ? <RoleEditor key={(role?.id ?? 'new') + ':' + (role?.latest ?? 0)} role={role} person={authority.data.me.person.id} model={authority.data.model} changed={changed} /> : null}
          {role ? <AssignRole key={role.id + ":" + role.holders.length} role={role} person={authority.data.me.person.id} identities={authority.data.people.people.flatMap((person) => [{ id: person.id, name: person.display_name }, ...person.agents.map((agent) => ({ id: agent.id, name: agent.display_name }))])} changed={changed} /> : null}
        </> : null}
      </>;
    }} />
    {authority.status === 'refused' ? <ErrorWords problem={authority.refused} /> : null}
  </div>;
}
