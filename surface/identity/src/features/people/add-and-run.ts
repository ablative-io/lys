import { operationId, Refused, request } from '../../api';
import type { Login } from '../../generated';
import { agentRequestOf, addAgent } from './add-agent-request';
import type { PendingAgent } from './add-agent-request';
import { isRecord, pendingMachineOf, readableMachine, strings } from '../network/contract';
import type { NetworkView, PendingMachine } from '../network/contract';
import { recordComputer, validComputerName } from '../network/AddMachine';
import { pendingStartOf, profileRequest, startRequest } from '../runtime/StartAgent';
import type { Pending } from '../runtime/StartAgent';
import { readChoices } from '../provisioning/choices';
import type { Choices } from '../provisioning/choices';
import type { HarnessDescription, ProvisioningProfile } from '../provisioning/Provisioning';

export interface AddAndRun {
  version: 1; person: string; registration: PendingAgent; settings: Record<string, unknown>;
  step: 'registration' | 'profile' | 'review' | 'computer' | 'start'; machine: PendingMachine; pending: Pending | null;
}
const unreadable = (): never => { throw new Refused(0, { refusal: 'PendingAddAndRunUnreadable', reason: 'The retained add-and-run request cannot be read. Resolve its original outcome before starting another.' }); };
function description(value: unknown): value is HarnessDescription {
  if (!isRecord(value) || !isRecord(value.models) || !isRecord(value.models.further_encoding) || !isRecord(value.permissions) || !isRecord(value.mcp)) return false;
  const models = value.models; const encoding = value.models.further_encoding;
  return typeof models.minimum === 'number' && Number.isSafeInteger(models.minimum) && models.minimum >= 0
    && (models.maximum === null || typeof models.maximum === 'number' && Number.isSafeInteger(models.maximum) && models.maximum >= models.minimum)
    && (encoding.kind === 'array' || encoding.kind === 'delimited' && typeof encoding.separator === 'string')
    && strings(value.permissions.modes) && strings(value.permissions.rule_forms) && strings(value.mcp.transports)
    && typeof value.mcp.working_directory === 'boolean' && typeof value.mcp.handle_variables === 'boolean'
    && Array.isArray(value.mcp.channel_policies) && value.mcp.channel_policies.every((policy) => policy === 'off' || policy === 'wake') && typeof value.rendering_contract === 'string';
}
export function profileFromSettings(value: Record<string, unknown>): ProvisioningProfile {
  const harness = value.harness; const permissions = value.permissions;
  if (!isRecord(harness) || typeof harness.name !== 'string' || typeof harness.program !== 'string' || !harness.program.startsWith('/') || harness.program.includes('\0')
    || typeof harness.package !== 'string' || !description(harness.description) || !strings(value.model_access) || !value.model_access.length
    || !strings(value.tools) || value.tools.length || !strings(value.skills) || value.skills.length || !Array.isArray(value.mcp_servers) || value.mcp_servers.length
    || typeof value.instructions !== 'string' || typeof value.note !== 'string' || !isRecord(permissions) || typeof permissions.default_mode !== 'string'
    || (value.instructions_mode !== undefined && value.instructions_mode !== 'keep' && value.instructions_mode !== 'append' && value.instructions_mode !== 'replace')) return unreadable();
  return { version: 0, operation: '', harness: { name: harness.name, program: harness.program, package: harness.package, description: harness.description },
    model_access: value.model_access, permissions: { default_mode: permissions.default_mode }, tools: [], skills: [], mcp_servers: [],
    instructions: value.instructions, instructions_mode: value.instructions_mode ?? (value.instructions ? 'append' : 'keep'), note: value.note, set_by: '', set_at: 0, session: null };
}
export function readAddAndRun(key: string, person: string): AddAndRun | null {
  const raw = sessionStorage.getItem(key);
  if (raw === null) return null;
  try {
    const value: unknown = JSON.parse(raw);
    if (!isRecord(value) || value.version !== 1 || value.person !== person || !isRecord(value.settings)
      || !['registration', 'profile', 'review', 'computer', 'start'].includes(String(value.step))) return unreadable();
    const registration = agentRequestOf(value.registration); profileFromSettings(value.settings);
    if (!isRecord(value.machine)) return unreadable();
    const machine = pendingMachineOf({ version: 1, ...value.machine });
    if (!machine || machine.legacy || !validComputerName(machine.body.name) || machine.body.kind !== 'Computer' || machine.body.runtime !== 'lys-runner'
      || machine.body.slots !== 0 || machine.body.may_reach.length || (machine.body.may_run_roles ?? []).length) return unreadable();
    if (value.step === 'registration') {
      if (value.pending !== null || machine.phase !== 'machine' || machine.body.may_run.length) return unreadable();
      return { version: 1, person, registration, settings: value.settings, step: 'registration', machine, pending: null };
    }
    if (!registration.agent || !registration.activated || machine.body.may_run.length !== 1 || machine.body.may_run[0] !== registration.agent) return unreadable();
    const prefix = '/agents/' + encodeURIComponent(registration.agent);
    const pending = pendingStartOf(value.pending, 'lys.pending.agent-start.' + person + '.' + registration.agent, prefix);
    if (value.step === 'start' && (machine.phase !== 'runner' || !machine.machine)) return unreadable();
    const expected = value.step === 'computer' ? 'start' : value.step;
    if (pending.stage !== expected || pending.machine !== machine.body.operation || pending.legacyKey !== undefined) return unreadable();
    return { version: 1, person, registration, settings: value.settings, step: value.step as AddAndRun['step'], machine, pending };
  } catch { return unreadable(); }
}
export function newAddAndRun(registration: PendingAgent, settings: Record<string, unknown>, name: string, person: string): AddAndRun {
  profileFromSettings(settings);
  if (!validComputerName(name)) return unreadable();
  return { version: 1, person, registration, settings, step: 'registration', pending: null,
    machine: { body: { operation: operationId(), name, kind: 'Computer', runtime: 'lys-runner', slots: 0, may_run: [], may_run_roles: [], may_reach: [] }, phase: 'machine', legacy: false, machine: null } };
}
export function addAndRunStep(current: AddAndRun): string {
  if (current.step === 'registration') return current.registration.activated ? 'joining the team' : current.registration.agent ? 'activating the agent' : 'registering the agent';
  if (current.step === 'profile') return 'saving the settings';
  if (current.step === 'review') return 'approving the settings';
  if (current.step === 'computer') return current.machine.phase === 'machine' ? 'adding this computer' : 'recording this computer’s runner';
  return 'starting the agent';
}
export class AddAndRunFailure extends Error {
  constructor(readonly step: string, readonly problem: unknown) {
    const code = problem instanceof Refused ? problem.refusal.refusal : problem instanceof Error ? problem.message.split(':', 1)[0] : '';
    super('Lys could not confirm ' + step + '.' + (code === 'RunnerStartUnconfirmed' || code === 'RunnerDidNotRun' ? ' Lys admitted the start, but no runner ran it.' : code === 'RunnerStartEnded' ? ' The runner confirmed this session already ended.' : ''));
  }
}
export async function addAndRun(initial: AddAndRun, login: Login, keep: (next: AddAndRun) => void): Promise<string> {
  let current = initial;
  const save = (next: AddAndRun) => { keep(next); current = next; };
  try {
    save(current);
    if (current.step === 'registration') {
      const agent = await addAgent(current.registration, current.person, (registration) => save({ ...current, registration }), login);
      const machine = { ...current.machine, body: { ...current.machine.body, may_run: [agent] } };
      save({ ...current, machine, step: 'profile', pending: { stage: 'profile', path: '/agents/' + agent + '/provisioning', body: { ...current.settings, operation: operationId(), from_version: 0 }, machine: machine.body.operation, version: 0 } });
    }
    const agent = current.registration.agent;
    if (!agent) return unreadable();
    const prefix = '/agents/' + encodeURIComponent(agent);
    if (current.step === 'profile') {
      if (!current.pending) return unreadable();
      const confirmed = await profileRequest(agent, current.pending);
      save({ ...current, step: 'review', pending: { stage: 'review', path: prefix + '/provisioning/' + confirmed.version + '/review', body: { operation: operationId() }, machine: current.machine.body.operation, version: confirmed.version } });
    }
    if (current.step === 'review') {
      if (!current.pending) return unreadable();
      const confirmed = await profileRequest(agent, current.pending);
      save({ ...current, step: 'computer', pending: { stage: 'start', path: prefix + '/start-command', body: { operation: operationId(), machine: current.machine.body.operation }, machine: current.machine.body.operation, version: confirmed.version } });
    }
    if (current.step === 'computer') {
      await recordComputer(current.machine, current.person, (machine) => save({ ...current, machine }));
      save({ ...current, step: 'start' });
    }
    if (!current.pending || current.step !== 'start') return unreadable();
    const answer = await startRequest(agent, current.pending);
    if (!answer.runner) throw new Refused(200, { refusal: 'RunnerDidNotRun', reason: 'Lys admitted the start, but no runner ran it. No running session is claimed.' });
    if (answer.runner.state !== 'running') throw new Refused(200, { refusal: 'RunnerStartEnded', reason: 'The runner confirmed this session already ended. No running session is claimed.' });
    return agent;
  } catch (problem) { throw new AddAndRunFailure(addAndRunStep(current), problem); }
}
export interface FirstRunOptions { choices: Choices | null; network: NetworkView | null; problem: unknown }
export async function firstRunChoices(): Promise<FirstRunOptions> {
  let network: NetworkView | null = null;
  try {
    const answer = await request<unknown>('/network');
    if (!isRecord(answer) || !Array.isArray(answer.machines) || !answer.machines.every(readableMachine) || typeof answer.reports_served !== 'boolean') throw new Refused(200, { refusal: 'NetworkUnreadable', reason: 'The service did not name readable computers and their admission rules.' });
    network = { machines: answer.machines, reports_served: answer.reports_served };
    if (network.machines.some((machine) => machine.state === 'in_use')) return { choices: null, network, problem: null };
    return { choices: await readChoices(network), network, problem: null };
  } catch (problem) { return { choices: null, network, problem }; }
}
