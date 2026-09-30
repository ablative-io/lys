/** Read versioned roles, publish deliberate changes and manage explicitly identified holdings. */
import { useState } from 'react';
import { useParams } from 'react-router';
import { api, request, useLoad } from '../../api';
import { Gate } from '../signin/Gate';
import { clock } from '../file/time';
import type { Role } from './contract';
import { RoleEditor } from './RoleEditor';
import { AssignRole, RoleHolders } from './RoleHolders';

export function Roles() {
  const { id } = useParams();
  const [revision, setRevision] = useState(0);
  const [editing, setEditing] = useState(false);
  const [notice, setNotice] = useState('');
  const changed = () => { setEditing(false); setNotice('Role change recorded.'); setRevision((value) => value + 1); };
  const load = useLoad(() => request<{ roles: Role[] }>('/roles'), 'roles:' + revision);
  const authority = useLoad(async () => ({ people: await api.people(), me: await api.me(), model: await api.model() }), 'role-authority');
  const admin = authority.status === 'ok' && authority.data.people.scope === 'directory';
  return <div className="page"><div className="head"><div><div className="eyebrow">Directory</div><h1>Roles</h1><p className="sub">Responsibilities, working practices and access templates, with a history for every version.</p></div></div>
    {notice ? <p role="status">{notice}</p> : null}
    <Gate load={load} title="Roles" ok={({ roles }) => {
      const role = id ? roles.find((entry) => entry.id === id) : undefined;
      const latest = role?.versions.find((version) => version.number === role.latest);
      return <>
        <nav aria-label="Roles"><a className="btn" href="#/roles">All roles</a>{roles.map((entry) => <a key={entry.id} className="btn" href={'#/roles/' + encodeURIComponent(entry.id)}>{entry.name} · v{entry.latest}</a>)}</nav>
        {id && !role ? <p className="why-not">This role was not returned by the service.</p> : null}
        {!id && !roles.length ? <p>No roles have been recorded.</p> : null}
        {role && latest ? <>
          <section className="card"><h2>{role.name} · Version {latest.number}</h2><p>Holders stay on their version until moved. A role assignment does not grant access.</p>
            {(['responsibilities', 'goals', 'practice', 'profile'] as const).map((part) => <section key={part}><h3>{part === 'practice' ? 'How the work is done' : part[0].toUpperCase() + part.slice(1)}</h3><p style={{ whiteSpace: 'pre-wrap' }}>{latest[part] || 'None recorded.'}</p></section>)}
            <h3>Access templates</h3>{latest.grant_templates.length ? <ul>{latest.grant_templates.map((template, index) => <li key={index}>{template.relation} of {template.resource.kind}:{template.resource.id} · {template.days === null ? 'no expiry of its own' : template.days + ' days'}</li>)}</ul> : <p>None recorded.</p>}
            <details><summary>Version history</summary>{role.versions.map((version) => <details key={version.number}><summary>Version {version.number} · {clock(version.made_at)}</summary><p>{version.note} · by {version.made_by}</p><dl>{(['responsibilities', 'goals', 'practice', 'profile'] as const).map((part) => <div key={part}><dt>{part}</dt><dd>{version[part] || 'Empty'}</dd></div>)}</dl></details>)}</details>
          </section>
          <RoleHolders role={role} person={authority.status === 'ok' ? authority.data.me.person.id : ''} admin={admin} changed={changed} />
        </> : null}
        {admin && authority.status === 'ok' ? <>
          <button className="btn" onClick={() => setEditing((value) => !value)}>{editing ? 'Close editor' : role ? 'New version' : 'Create a role'}</button>
          {editing ? <RoleEditor key={role?.id ?? 'new'} role={role} person={authority.data.me.person.id} model={authority.data.model} changed={changed} /> : null}
          {role ? <AssignRole role={role} person={authority.data.me.person.id} identities={authority.data.people.people.flatMap((person) => [{ id: person.id, name: person.display_name }, ...person.agents.map((agent) => ({ id: agent.id, name: agent.display_name }))])} changed={changed} /> : null}
        </> : null}
      </>;
    }} />
    {authority.status === 'refused' ? <p className="why-not">{authority.refused.refusal.refusal}: {authority.refused.message}</p> : null}
  </div>;
}
