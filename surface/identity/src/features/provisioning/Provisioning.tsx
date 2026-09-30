import { StartAgent } from '../runtime/StartAgent';
import { ReviewProfile } from './ReviewProfile';
/** An agent's recorded provisioning is versioned explicitly; a saved profile is not a runtime application receipt. */
import { useState } from 'react';
import { api, request, useLoad } from '../../api';
import { clock } from '../file/time';
import { entries } from '../people/directory';
import { readChoices } from './choices';
import { Gate } from '../signin/Gate';
import { ProfileEditor } from './ProfileEditor';

export type Setting = string | number | boolean | { handle: string };
export interface McpServer { name: string; url?: string; command?: { program: string; args?: string[]; cwd?: string; env?: Record<string, Setting> }; channel?: 'off' | 'wake' }
export interface HarnessDescription {
  models: { minimum: number; maximum: number | null; further_encoding: { kind: 'array' } | { kind: 'delimited'; separator: string } };
  permissions: { modes: string[]; rule_forms: string[] };
  mcp: { transports: string[]; working_directory: boolean; handle_variables: boolean; channel_policies: ('off' | 'wake')[] };
  rendering_contract: string;
}
export interface DeclaredHarness { name: string; description: HarnessDescription; program: string; package: string }
export interface Permissions { allow?: string[]; deny?: string[]; ask?: string[]; default_mode?: string; additional_directories?: string[] }
export interface ProvisioningProfile { reviewed_by?: string | null; reviewed_at?: number | null; self_reviewed?: boolean; version: number; operation: string; model_access: string[]; tools: string[]; skills: string[]; mcp_servers: McpServer[]; instructions: string; note: string; set_by: string; set_at: number; harness?: DeclaredHarness | null; permissions?: Permissions | null; skill_pins?: { name: string; len: number; sha256: string }[]; runs_on?: string; writable?: string }
export interface ProvisioningAnswer { agent: string; recorded?: { operation: string; version: number } | null; profile: ProvisioningProfile | null; versions: { version: number; set_by: string; set_at: number; note: string }[]; enforced: boolean }
const pathOf = (id: string) => '/agents/' + encodeURIComponent(id) + '/provisioning';

/** The agent's start first, then its settings as the one form they are read and changed in, then approval; each reloads on its own write's answer. */
export function Provisioning({ id }: { id: string }) {
  const [revision, setRevision] = useState(0);
  const [notice, setNotice] = useState('');
  const load = useLoad(async () => {
    const [answer, me, people, choices] = await Promise.all([request<ProvisioningAnswer>(pathOf(id)), api.me(), api.people(), readChoices()]);
    if (answer.agent !== id || typeof answer.enforced !== 'boolean') throw new Error('Provisioning answer did not name this agent and its application state.');
    return { answer, me, people, choices };
  }, 'provisioning:' + id + ':' + revision);
  const after = (message: string) => { setNotice(message); setRevision((value) => value + 1); };
  return <Gate load={load} title="Settings" ok={({ answer, me, people, choices }) => {
    const names = new Map(entries(people).map((entry) => [entry.id, entry.display_name]));
    const name = (who: string) => names.get(who) ?? who;
    const profile = answer.profile ? { ...answer.profile, set_by: name(answer.profile.set_by), reviewed_by: answer.profile.reviewed_by ? name(answer.profile.reviewed_by) : answer.profile.reviewed_by } : null;
    return <>
      {notice ? <p role="status">{notice}</p> : null}
      <StartAgent agent={id} profile={profile} />
      {profile ? <p className="note">Version {profile.version}, saved {clock(profile.set_at)} by {profile.set_by}.</p> : null}
      <ProfileEditor key={id + ':' + revision} readOnly={people.scope !== 'directory'} id={id} path={pathOf(id)} person={me.person.id} profile={answer.profile} choices={choices} changed={() => after('Settings saved as a new version. A start is refused until it is approved.')} />
      {profile ? <ReviewProfile key={id + ':r' + revision} agent={id} person={me.person.id} profile={profile} changed={after} /> : null}
      {answer.versions.length > 1 ? <details className="card"><summary>Earlier versions ({answer.versions.length - 1})</summary>{answer.versions.filter((version) => version.version !== profile?.version).map((version) => <p key={version.version}>Version {version.version} · {clock(version.set_at)} · {name(version.set_by)} · {version.note}</p>)}</details> : null}
    </>;
  }} />;
}
