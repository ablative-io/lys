/** An agent's recorded provisioning is versioned explicitly; a saved profile is not a runtime application receipt. */
import { useState } from 'react';
import type { FormEvent } from 'react';
import { api, operationId, request, useLoad } from '../../api';
import { clock } from '../file/time';
import { field } from '../people/RecordedForm';
import { Gate } from '../signin/Gate';
import { useRoleChange } from '../roles/useRoleChange';
import { ChangeStatus } from '../roles/ChangeStatus';

interface Server { name: string; url: string }
export interface ProvisioningProfile { version: number; operation: string; model_access: string[]; tools: string[]; skills: string[]; mcp_servers: Server[]; instructions: string; note: string; set_by: string; set_at: number; reviewed_by: string | null; reviewed_at: number | null; self_reviewed: boolean }
export interface ProvisioningAnswer { agent: string; recorded?: { operation: string; version: number } | null; profile: ProvisioningProfile | null; versions: { version: number; set_by: string; set_at: number; note: string }[]; enforced: boolean }
const pathOf = (id: string) => '/agents/' + encodeURIComponent(id) + '/provisioning';
const lines = (data: FormData, name: string) => field(data, name).split('\n').map((line) => line.trim()).filter(Boolean);

export function Provisioning({ id }: { id: string }) {
  const [revision, setRevision] = useState(0);
  const [notice, setNotice] = useState('');
  const load = useLoad(async () => {
    const answer = await request<ProvisioningAnswer>(pathOf(id));
    if (answer.agent !== id || typeof answer.enforced !== 'boolean') throw new Error('Provisioning answer did not name this agent and its application state.');
    return answer;
  }, 'provisioning:' + id + ':' + revision);
  const authority = useLoad(async () => ({ me: await api.me(), people: await api.people() }), 'provisioning-authority:' + id);
  return <><div className="head"><div><h2>Provisioning</h2><p>Choose the models, tools, skills and instructions for this agent.</p></div><button className="btn" onClick={() => setRevision((value) => value + 1)}>Refresh profile</button></div>
    {notice ? <p role="status">{notice}</p> : null}
    <Gate load={load} title="Provisioning" ok={(answer) => <>
      <p className="note">{answer.enforced ? 'The service reports this profile is enforced.' : 'Recorded only: no runtime is applying this profile yet. Saving it does not start an agent or grant access.'}</p>
      {answer.profile ? <section className="card"><h3>Version {answer.profile.version}</h3><p>Recorded {clock(answer.profile.set_at)} by {answer.profile.set_by}.</p>
        <Reviewed id={id} profile={answer.profile} person={authority.status === 'ok' ? authority.data.me.person.id : null} changed={() => { setNotice('Profile version reviewed.'); setRevision((value) => value + 1); }} />
        {(['model_access', 'tools', 'skills'] as const).map((name) => <div key={name}><h3>{name === 'model_access' ? 'Model access' : name === 'tools' ? 'Tools' : 'Skills'}</h3>{answer.profile?.[name].length ? <ul>{answer.profile[name].map((entry, index) => <li key={index}>{entry}</li>)}</ul> : <p>None recorded.</p>}</div>)}
        <h3>MCP servers</h3>{answer.profile.mcp_servers.length ? <ul>{answer.profile.mcp_servers.map((server) => <li key={server.name}>{server.name} · {server.url}</li>)}</ul> : <p>None recorded.</p>}
        <h3>Instructions</h3><p style={{ whiteSpace: 'pre-wrap' }}>{answer.profile.instructions || 'None recorded.'}</p>
      </section> : <p>No provisioning profile has been recorded.</p>}
      {authority.status === 'ok' && authority.data.people.scope === 'directory' ? <ProfileEditor key={id + ':' + revision} id={id} person={authority.data.me.person.id} profile={answer.profile} changed={() => { setNotice('Provisioning profile recorded.'); setRevision((value) => value + 1); }} /> : null}
      {answer.versions.length ? <details className="card"><summary>Profile history ({answer.versions.length})</summary>{answer.versions.map((version) => <p key={version.version}>Version {version.version} · {clock(version.set_at)} · {version.note}</p>)}</details> : null}
    </>} />
    {authority.status === 'refused' ? <p className="why-not">{authority.refused.refusal.refusal}: {authority.refused.refusal.reason}</p> : null}
  </>;
}

/** A version is reviewed before an agent may be started from it; the service refuses anyone but the person responsible or the administrator. */
function Reviewed({ id, profile, person, changed }: { id: string; profile: ProvisioningProfile; person: string | null; changed: () => void }) {
  const change = useRoleChange<ProvisioningAnswer>('lys.pending.provisioning-review.' + (person ?? '') + '.' + id + '.' + profile.version, pathOf(id) + '/' + profile.version + '/review',
    (answer) => answer.agent === id && answer.recorded?.version === profile.version && answer.profile !== null && (answer.profile.version !== profile.version || answer.profile.reviewed_by !== null), changed);
  if (profile.reviewed_by !== null) {
    return <p>Reviewed {profile.reviewed_at === null ? '' : clock(profile.reviewed_at) + ' '}by {profile.reviewed_by}.{profile.self_reviewed ? <strong> The person who set this version also reviewed it.</strong> : null}</p>;
  }
  return <div role="group" aria-label="Review this version"><p className="why-not">Not reviewed. No agent is started from this version until it is reviewed.</p>
    {person !== null ? <button className="btn primary" disabled={change.blocked} onClick={() => change.submit({ operation: operationId() })}>Review version {profile.version}</button> : null}<ChangeStatus change={change} />
  </div>;
}

function ProfileEditor({ id, person, profile, changed }: { id: string; person: string; profile: ProvisioningProfile | null; changed: () => void }) {
  const [servers, setServers] = useState((profile?.mcp_servers ?? []).map((value, key) => ({ ...value, key })));
  const [nextKey, setNextKey] = useState(servers.length);
  const [error, setError] = useState('');
  const change = useRoleChange<ProvisioningAnswer>('lys.pending.provisioning.' + person + '.' + id, pathOf(id),
    (answer, body) => answer.agent === id && (answer.recorded ? answer.recorded.operation === body.operation && answer.recorded.version === Number(body.from_version) + 1 : answer.profile !== null && answer.profile.operation === body.operation && answer.profile.version === Number(body.from_version) + 1), changed);
  const submit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault(); if (change.blocked) return;
    try {
      const data = new FormData(event.currentTarget);
      const mcp_servers = servers.map(({ key }) => {
        const name = field(data, 'server-name-' + key); const url = field(data, 'server-url-' + key); const parsed = new URL(url);
        if (!name || !['http:', 'https:'].includes(parsed.protocol) || parsed.username || parsed.password) throw new Error('Each MCP server needs a name and an HTTP or HTTPS address without an embedded username or password.');
        return { name, url };
      });
      if (new Set(mcp_servers.map((server) => server.name)).size !== mcp_servers.length) throw new Error('Give each MCP server a different name.');
      setError('');
      change.submit({ operation: operationId(), from_version: profile?.version ?? 0, model_access: lines(data, 'model_access'), tools: lines(data, 'tools'), skills: lines(data, 'skills'), mcp_servers, instructions: field(data, 'instructions'), note: field(data, 'note') });
    } catch (failure) { setError(String(failure)); }
  };
  return <form className="card recorded-form" aria-label="Record provisioning profile" onSubmit={submit}><h3>{profile ? 'Record a new version' : 'Set up a profile'}</h3>
    <fieldset disabled={change.blocked} style={{ border: 0, padding: 0 }}>
      <label className="field">Instructions<textarea name="instructions" defaultValue={profile?.instructions ?? ''} rows={5} placeholder="How this agent should carry out its work" /></label>
      <details><summary>Models, tools, skills and MCP servers</summary><p>One name per line. These declarations do not give permission to use a service.</p>
        <label className="field">Model access<textarea name="model_access" defaultValue={profile?.model_access.join('\n') ?? ''} /></label>
        <label className="field">Tools<textarea name="tools" defaultValue={profile?.tools.join('\n') ?? ''} /></label>
        <label className="field">Skills<textarea name="skills" defaultValue={profile?.skills.join('\n') ?? ''} /></label>
        {servers.map((server) => <div key={server.key}><label className="field">Server name<input name={'server-name-' + server.key} defaultValue={server.name} required /></label><label className="field">Server address<input name={'server-url-' + server.key} type="url" defaultValue={server.url} required /></label><button type="button" className="btn" onClick={() => setServers((values) => values.filter((entry) => entry.key !== server.key))}>Remove server</button></div>)}
        <button type="button" className="btn" onClick={() => { setServers((values) => [...values, { key: nextKey, name: '', url: '' }]); setNextKey((value) => value + 1); }}>Add MCP server</button>
      </details>
      <label className="field">Reason for this version<input name="note" required placeholder="What changed and why" /></label>
      <button type="submit" className="btn primary">Record profile</button>
    </fieldset>{error ? <p role="alert">{error}</p> : null}<ChangeStatus change={change} />
  </form>;
}
