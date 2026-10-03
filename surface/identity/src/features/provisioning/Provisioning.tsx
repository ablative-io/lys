/** An agent's recorded provisioning is versioned explicitly; a saved profile is not a runtime application receipt. */
import { api, request, useLoad } from '../../api';
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
export interface ProvisioningProfile { reviewed_by?: string | null; reviewed_at?: number | null; self_reviewed?: boolean; version: number; operation: string; model_access: string[]; tools: string[]; skills: string[]; mcp_servers: McpServer[]; instructions: string; note: string; set_by: string; set_at: number; harness?: DeclaredHarness | null; permissions?: Permissions | null; skill_pins?: { name: string; len: number; sha256: string }[]; runs_on?: string; writable?: string; working_folder?: string; instructions_mode?: 'keep' | 'append' | 'replace'; session?: Record<string, unknown> | null }
export interface ProvisioningAnswer { agent: string; recorded?: { operation: string; version: number } | null; profile: ProvisioningProfile | null; versions: { version: number; set_by: string; set_at: number; note: string }[]; enforced: boolean }
const pathOf = (id: string) => '/agents/' + encodeURIComponent(id) + '/provisioning';

/** One start uses the choices the service answers and keeps the current profile's other settings. */
export function Provisioning({ id }: { id: string }) {
  const load = useLoad(async () => {
    const [answer, people, choices] = await Promise.all([request<ProvisioningAnswer>(pathOf(id)), api.people(), readChoices()]);
    if (answer.agent !== id || typeof answer.enforced !== 'boolean') throw new Error('Provisioning answer did not name this agent and its application state.');
    return { answer, people, choices };
  }, 'provisioning:' + id);
  return <Gate load={load} title="Start" ok={({ answer, people, choices }) =>
    <ProfileEditor key={id} readOnly={people.scope !== 'directory'} id={id} profile={answer.profile} choices={choices} people={people} />
  } />;
}
