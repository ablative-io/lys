import { StartAgent } from '../runtime/StartAgent';
import { ReviewProfile } from './ReviewProfile';
/** An agent's recorded provisioning is versioned explicitly; a saved profile is not a runtime application receipt. */
import { useState } from 'react';
import { api, request, useLoad } from '../../api';
import { clock } from '../file/time';
import { Gate } from '../signin/Gate';
import { ProfileEditor } from './ProfileEditor';

export type Setting = string | number | boolean | { handle: string };
export interface McpServer { name: string; url?: string; command?: { program: string; args?: string[]; cwd?: string; env?: Record<string, Setting> }; channel?: 'off' | 'wake' }
export interface DeclaredHarness { kind: 'claude_code' | 'codex'; program: string; package: string }
export interface Permissions { allow?: string[]; deny?: string[]; ask?: string[]; default_mode?: string; additional_directories?: string[] }
export interface ProvisioningProfile { reviewed_by?: string | null; reviewed_at?: number | null; self_reviewed?: boolean; version: number; operation: string; model_access: string[]; tools: string[]; skills: string[]; mcp_servers: McpServer[]; instructions: string; note: string; set_by: string; set_at: number; harness?: DeclaredHarness | null; permissions?: Permissions | null; skill_pins?: { name: string; len: number; sha256: string }[] }
export interface ProvisioningAnswer { agent: string; recorded?: { operation: string; version: number } | null; profile: ProvisioningProfile | null; versions: { version: number; set_by: string; set_at: number; note: string }[]; enforced: boolean }
const pathOf = (id: string) => '/agents/' + encodeURIComponent(id) + '/provisioning';
const setting = (value: Setting) => typeof value === 'object' ? 'the handle on ' + value.handle : String(value);
const serverLine = (server: McpServer) => server.command ? server.command.program + (server.command.args?.length ? ' ' + JSON.stringify(server.command.args) : '') + (server.command.cwd ? ' in ' + server.command.cwd : '') + Object.entries(server.command.env ?? {}).map(([name, value]) => ' · ' + name + ' = ' + setting(value)).join('') : server.url ?? '';
const listed = (title: string, entries: string[] | undefined) => <div key={title}><h3>{title}</h3>{entries?.length ? <ul>{entries.map((entry, index) => <li key={index}>{entry}</li>)}</ul> : <p>None recorded.</p>}</div>;

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
        {(['model_access', 'tools', 'skills'] as const).map((name) => <div key={name}><h3>{name === 'model_access' ? 'Model access' : name === 'tools' ? 'Tools' : 'Skills'}</h3>{answer.profile?.[name].length ? <ul>{answer.profile[name].map((entry, index) => <li key={index}>{entry}</li>)}</ul> : <p>None recorded.</p>}</div>)}
        <h3>Harness</h3><p>{answer.profile.harness ? (answer.profile.harness.kind === 'codex' ? 'Codex' : 'Claude Code') + ' · ' + answer.profile.harness.program + ' · package ' + answer.profile.harness.package : 'None declared: a start is refused until one is.'}</p>
        {answer.profile.skill_pins?.length ? <><h3>Skill texts pinned</h3><ul>{answer.profile.skill_pins.map((pin) => <li key={pin.name}>{pin.name} · {pin.len} bytes · {pin.sha256.slice(0, 12)}</li>)}</ul></> : null}
        <h3>MCP servers</h3>{answer.profile.mcp_servers.length ? <ul>{answer.profile.mcp_servers.map((server) => <li key={server.name}>{server.name} · {serverLine(server)}{server.channel === 'wake' ? ' · wakes the agent' : ''}</li>)}</ul> : <p>None recorded.</p>}
        <h3>Permissions</h3>{answer.profile.permissions ? <>{listed('Allow', answer.profile.permissions.allow)}{listed('Deny', answer.profile.permissions.deny)}{listed('Ask', answer.profile.permissions.ask)}{listed('Additional directories', answer.profile.permissions.additional_directories)}<p>Permission mode: {answer.profile.permissions.default_mode ?? 'not set'}.</p></> : <p>None recorded.</p>}
        <h3>Instructions</h3><p style={{ whiteSpace: 'pre-wrap' }}>{answer.profile.instructions || 'None recorded.'}</p>
      </section> : <p>No provisioning profile has been recorded.</p>}
      {authority.status === 'ok' && answer.profile ? <ReviewProfile key={id + ':' + revision} agent={id} person={authority.data.me.person.id} profile={answer.profile} changed={(message) => { setNotice(message); setRevision((value) => value + 1); }} /> : null}
      {authority.status === 'ok' && authority.data.people.scope === 'directory' ? <ProfileEditor key={id + ':' + revision} id={id} path={pathOf(id)} person={authority.data.me.person.id} profile={answer.profile} changed={() => { setNotice('Provisioning profile recorded.'); setRevision((value) => value + 1); }} /> : null}
      {answer.profile?.reviewed_by ? <StartAgent agent={id} /> : <p>Review the current profile before preparing a start.</p>}
      {answer.versions.length ? <details className="card"><summary>Profile history ({answer.versions.length})</summary>{answer.versions.map((version) => <p key={version.version}>Version {version.version} · {clock(version.set_at)} · {version.note}</p>)}</details> : null}
    </>} />
    {authority.status === 'refused' ? <p className="why-not">{authority.refused.refusal.refusal}: {authority.refused.refusal.reason}</p> : null}
  </>;
}
