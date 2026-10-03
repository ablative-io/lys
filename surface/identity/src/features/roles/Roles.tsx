import { readTogether } from '../../reads';
/** Read versioned roles, publish deliberate changes and manage explicitly identified holdings. */
import { useState } from 'react';
import { useParams } from 'react-router';
import { api, request, useLoad } from '../../api';
import { DirectoryGate as Gate, ErrorWords, IdentityName, PART } from '../people/Words';
import { clock } from '../file/time';
import type { Role, RoleAnswer } from './contract';
import { RoleEditor } from './RoleEditor';
import { RoleHolders } from './RoleHolders';
import { Listing } from '../../shell/Listing';
import type { Column } from '../../shell/Listing';

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
  const authority = useLoad(() => readTogether({ people: api.people(), me: api.me(), model: api.model() }), 'role-authority');
  const admin = authority.status === 'ok' && authority.data.people.scope === 'directory';
  const identities = authority.status === 'ok' ? authority.data.people.people.flatMap((person) => [{ id: person.id, name: person.display_name }, ...person.agents.map((agent) => ({ id: agent.id, name: agent.display_name, detail: 'agent of ' + person.display_name }))]) : [];
  const role = id ? roles.find((entry) => entry.id === id) : undefined;
  const me = authority.status === 'ok' ? authority.data.me.person.id : '';
  const editor = editing && admin && authority.status === 'ok' ? <RoleEditor key={(role?.id ?? 'new') + ':' + (role?.latest ?? 0)} role={role} person={me} model={authority.data.model} changed={changed} /> : null;
  return <div className="page fill"><div className="head"><div><div className="eyebrow">{id ? <a href="#/roles">Roles</a> : 'Directory'}</div><h1>{role ? role.name : 'Roles'}</h1>{id ? null : <p className="sub">Describe a job, assign it to people or agents, and review changes before updating their assigned version.</p>}</div>
    {admin && (!id || role) ? <button className="btn" onClick={() => setEditing((value) => !value)}>{editing ? 'Close editor' : role ? 'New version' : 'Create a role'}</button> : null}</div>
    {notice ? <p role="status">{notice}</p> : null}
    <Gate load={load} title="Roles" ok={() => {
      if (id && !role) return <p className="why-not">This role was not returned by the service.</p>;
      if (role) return <div className="body halves">
        <div className="pane">{editor ?? <RoleText role={role} />}</div>
        <div className="pane"><RoleHolders role={role} person={me} admin={admin} identities={identities} changed={changed} /></div>
      </div>;
      if (editor) return <div className="body one"><div className="pane">{editor}</div></div>;
      const group = [{ id: '', name: 'Roles', lead: null, depth: 0, items: roles, within: roles }];
      const columns: Column<Role>[] = [
        { head: 'Role', cell: (entry) => entry.name },
        { head: 'Version', cell: (entry) => <span className="sec">v{entry.latest}</span> },
        { head: 'Held by', cell: (entry) => <span className="sec">{entry.holders.filter((holder) => holder.state === 'holding').length}</span> },
      ];
      return <div className="body one">
        {roles.length ? <Listing<Role> groups={group} columns={columns} id={(entry) => entry.id} href={(entry) => '#/roles/' + encodeURIComponent(entry.id)} words={(entry) => entry.name}
          noun="roles" holds={(items) => items.length + ' roles'} selected={null} select={() => undefined} open={(entry) => { location.hash = '/roles/' + encodeURIComponent(entry.id); }} />
          : <p>No roles have been recorded.</p>}
      </div>;
    }} />
    {authority.status === 'refused' ? <ErrorWords problem={authority.refused} /> : null}
  </div>;
}

const PARTS = ['responsibilities', 'goals', 'practice', 'profile'] as const;

/** One version's words, its access templates as a table, and the versions as rows: a row's button puts that version's words above. */
function RoleText({ role }: { role: Role }) {
  const [number, setNumber] = useState<number | null>(null);
  const shown = role.versions.find((version) => version.number === (number ?? role.latest));
  if (!shown) return <p className="why-not">Version {number ?? role.latest} of this role was not returned by the service.</p>;
  return <>
    <section className="card" aria-label="Role words"><h2>Version {shown.number}{shown.number === role.latest ? '' : ' (version ' + role.latest + ' is the latest)'}</h2><p className="note">A role assignment does not grant access.</p>
      {PARTS.map((part) => <section key={part}><h3>{PART[part]}</h3><p style={{ whiteSpace: 'pre-wrap' }}>{shown[part] || 'None recorded.'}</p></section>)}
    </section>
    <section className="card" aria-label="Access templates"><h3>Access templates</h3>
      <table className="usage-table"><thead><tr><th>Access level</th><th>Type of thing</th><th>Name of the thing</th><th>Lasts</th></tr></thead><tbody>
        {shown.grant_templates.map((template, index) => <tr key={index}><td>{template.relation}</td><td>{template.resource.kind}</td><td>{template.resource.id}</td><td>{template.days === null ? 'No expiry of its own' : template.days + ' days'}</td></tr>)}
        {shown.grant_templates.length ? null : <tr><td colSpan={4} className="dim">None recorded.</td></tr>}
      </tbody></table>
    </section>
    <section className="card" aria-label="Version history"><h3>Version history</h3>
      <table className="usage-table"><thead><tr><th>Version</th><th>Made</th><th>By</th><th>Note</th><th>Read</th></tr></thead><tbody>
        {role.versions.map((version) => <tr key={version.number} aria-current={version.number === shown.number ? 'true' : undefined}><td>{version.number}</td><td>{clock(version.made_at)}</td><td><IdentityName id={version.made_by} /></td><td>{version.note}</td>
          <td>{version.number === shown.number ? <span className="sec">Shown above</span> : <button className="btn" type="button" onClick={() => setNumber(version.number)}>Read version {version.number}</button>}</td></tr>)}
      </tbody></table>
    </section>
  </>;
}
