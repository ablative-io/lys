/** An agent's recorded provisioning is versioned explicitly; a saved profile is not a runtime application receipt. */
import { useState } from 'react';
import { Act } from '../../shell/Act';
import { api, operationId, request, useLoad } from '../../api';
import { readChoices } from './choices';
import { ErrorWords, DirectoryGate as Gate } from '../people/Words';
import { ProfileEditor } from './ProfileEditor';
import { entries } from '../people/directory';
import type { PeopleView } from '../../generated';

export type Setting = string | number | boolean | { handle: string };
export interface McpServer { name: string; url?: string; command?: { program: string; args?: string[]; cwd?: string; env?: Record<string, Setting> }; channel?: 'off' | 'wake' }
export interface SeatProgram {
  executable: string; executable_sha256: string; arguments: string[];
  seat_file: string; seat_file_sha256: string; adapter: string;
}
export interface HarnessDescription {
  seat_program?: SeatProgram;
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
    answer.profile?.harness?.description.rendering_contract === 'seat-program/v1'
      ? <SeatProgramReview key={id + ":" + answer.profile.version} id={id} profile={answer.profile} canReview={maySetSettings(people, me, id)} />
      : <ProfileEditor key={id + ':' + String(answer.profile?.version ?? 0)} readOnly={!maySetSettings(people, me, id)} id={id} profile={answer.profile} choices={choices} people={people} saved={saved} />
  } />;
}

function SeatProgramReview({ id, profile, canReview }: { id: string; profile: ProvisioningProfile; canReview: boolean }) {
  const [reviewed, setReviewed] = useState(Boolean(profile.reviewed_by));
  const [pending, setPending] = useState(false);
  const [problem, setProblem] = useState<unknown>(null);
  const seat = profile.harness?.description.seat_program;
  if (!seat) return <ErrorWords problem={new Error('SeatProgramMissing: the harness description has no seat_program.')} />;
  const review = async () => {
    setPending(true); setProblem(null);
    try {
      const answer = await request<ProvisioningAnswer>(pathOf(id) + '/' + profile.version + '/review', { operation: operationId() });
      if (answer.agent !== id || answer.profile?.version !== profile.version || !answer.profile.reviewed_by) throw new Error('SeatProgramReviewUnconfirmed: the service did not record a reviewer.');
      setReviewed(true);
    } catch (error) { setProblem(error); } finally { setPending(false); }
  };
  return <section className="card" aria-label="Reviewed seat program">
    <h2>{profile.harness?.name}</h2><dl>
      <dt>Executable</dt><dd>{seat.executable}</dd>
      <dt>Executable SHA-256</dt><dd>{seat.executable_sha256}</dd>
      <dt>Arguments, in order</dt><dd><ol>{seat.arguments.map((argument, index) => <li key={index}><code>{JSON.stringify(argument)}</code></li>)}</ol></dd>
      <dt>Seat file</dt><dd>{seat.seat_file}</dd>
      <dt>Seat file SHA-256</dt><dd>{seat.seat_file_sha256}</dd>
      <dt>Adapter</dt><dd>{seat.adapter}</dd>
    </dl>
    {reviewed ? <p>Reviewed</p> : canReview && <Act symbol="approve" name="Review these settings" word="Review" disabled={pending} onClick={() => void review()} />}
    {problem !== null && <ErrorWords problem={problem} />}
  </section>;
}
