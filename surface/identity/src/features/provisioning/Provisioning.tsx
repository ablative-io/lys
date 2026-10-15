/** An agent's recorded provisioning is versioned explicitly; a saved profile is not a runtime application receipt. */
import { api, request, useLoad } from '../../api';
import { readChoices } from './choices';
import { DirectoryGate as Gate } from '../people/Words';
import { ProfileEditor } from './ProfileEditor';
import { entries } from '../people/directory';
import type { PeopleView } from '../../generated';

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
export interface ProvisioningProfile { reviewed_by?: string | null; reviewed_at?: number | null; self_reviewed?: boolean; version: number; operation: string; model_access: string[]; tools: string[]; skills: string[]; plugins?: string[]; mcp_servers: McpServer[]; instructions: string; note: string; set_by: string; set_at: number; harness?: DeclaredHarness | null; permissions?: Permissions | null; skill_pins?: { name: string; len: number; sha256: string }[]; runs_on?: string; writable?: string; working_folder?: string; resume?: string | null; instructions_mode?: 'keep' | 'append' | 'replace'; session?: (Record<string, unknown> & { requires_controls?: boolean }) | null }
export interface ProvisioningAnswer { agent: string; recorded?: { operation: string; version: number } | null; profile: ProvisioningProfile | null; versions: { version: number; set_by: string; set_at: number; note: string }[]; enforced: boolean }
/** Who may change an agent's settings, as the service decides it (routes.rs, responsible_or_administrator): the administrator, or the person the agent answers to. */
export function maySetSettings(people: PeopleView, me: string, agent: string): boolean {
  return people.scope === 'directory' || entries(people).some((entry) => entry.id === agent && entry.person?.id === me);
}
const pathOf = (id: string) => '/agents/' + encodeURIComponent(id) + '/provisioning';

/** One start uses the choices the service answers and keeps the current profile's other settings. */
/** With `saved`, the form is the set-up under an agent's row: it opens filled, and its one button saves and goes on to the start. */
export function Provisioning({ id, saved }: { id: string; saved?: () => void }) {
  const load = useLoad(async () => {
    const [answer, people, choices, me] = await Promise.all([request<ProvisioningAnswer>(pathOf(id)), api.people(), readChoices(), api.me()]);
    if (answer.agent !== id || typeof answer.enforced !== 'boolean') throw new Error('ProvisioningUnreadable: the answer did not name this agent and its application state.');
    return { answer, people, choices, me: me.person.id };
  }, 'provisioning:' + id);
  return <Gate load={load} title="Start" ok={({ answer, people, choices, me }) =>
    <ProfileEditor key={id + ':' + String(answer.profile?.version ?? 0)} readOnly={!maySetSettings(people, me, id)} id={id} profile={answer.profile} choices={choices} people={people} saved={saved} />
  } />;
}
