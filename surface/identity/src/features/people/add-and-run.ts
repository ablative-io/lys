import { operationId, Refused, request } from '../../api';
import type { Login } from '../../generated';
import { agentRequestOf, addAgent } from './add-agent-request';
import type { PendingAgent } from './add-agent-request';
import { confirmRunner, isRecord, pendingMachineOf, readableMachine, strings } from '../network/contract';
import type { Machine, NetworkView, PendingMachine } from '../network/contract';
import { confirmAdmission } from '../network/machine-admission';
import type { MachineAdmission } from '../network/machine-admission';
import type { AddAndRunCapability } from './registration-capability';
import { recordComputer, validComputerName } from '../network/AddMachine';
import { pendingStartOf, profileRequest, startRequest } from '../runtime/start-requests';
import type { Pending } from '../runtime/start-requests';
import { readChoices } from '../provisioning/choices';
import type { Choices } from '../provisioning/choices';
import type { HarnessDescription, ProvisioningProfile } from '../provisioning/Provisioning';

type Placement = { kind: 'new'; machine: PendingMachine } | { kind: 'existing'; computer: Machine; admission: MachineAdmission | null };
export interface AddAndRun {
  version: 3; person: string; registration: PendingAgent; settings: Record<string, unknown>;
  step: 'registration' | 'profile' | 'review' | 'computer' | 'admission' | 'start'; placement: Placement; pending: Pending | null;
}
const operation = /^op-[0-9a-f]{32}$/;
export function placementComputer(placement: Placement): { id: string; name: string } {
  return placement.kind === 'new' ? { id: placement.machine.body.operation, name: placement.machine.body.name } : placement.computer;
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
    || (value.instructions_mode !== undefined && value.instructions_mode !== 'keep' && value.instructions_mode !== 'append' && value.instructions_mode !== 'replace')
    || (value.working_folder !== undefined && (typeof value.working_folder !== 'string' || !value.working_folder.startsWith('/')))) return unreadable();
  return { version: 0, operation: '', harness: { name: harness.name, program: harness.program, package: harness.package, description: harness.description },
    model_access: value.model_access, permissions: { default_mode: permissions.default_mode }, tools: [], skills: [], mcp_servers: [],
    instructions: value.instructions, instructions_mode: value.instructions_mode ?? (value.instructions ? 'append' : 'keep'), note: value.note, set_by: '', set_at: 0, session: null,
    ...(typeof value.working_folder === 'string' ? { working_folder: value.working_folder } : {}) };
}
function placementOf(value: unknown, registration: PendingAgent, step: AddAndRun['step']): Placement {
  if (!isRecord(value)) return unreadable();
  if (value.kind === 'new') {
    if (!isRecord(value.machine) || 'computer' in value || 'admission' in value || step === 'admission') return unreadable();
    const machine = pendingMachineOf({ ...value.machine, version: 1 });
    if (!machine || machine.legacy || !validComputerName(machine.body.name) || machine.body.kind !== 'Computer' || machine.body.runtime !== 'lys-runner'
      || machine.body.slots !== 0 || machine.body.may_reach.length || (machine.body.may_run_roles ?? []).length) return unreadable();
    if (step === 'registration' ? machine.phase !== 'machine' || machine.body.may_run.length
      : machine.body.may_run.length !== 1 || machine.body.may_run[0] !== registration.agent) return unreadable();
    if (step === 'start' && (machine.phase !== 'runner' || !machine.machine)) return unreadable();
    return { kind: 'new', machine };
  }
  if (value.kind !== 'existing' || 'machine' in value || step === 'computer' || !readableMachine(value.computer)
    || !operation.test(value.computer.id) || value.computer.kind !== 'Computer' || value.computer.state !== 'in_use' || value.computer.runtime !== 'lys-runner') return unreadable();
  const admission = value.admission;
  if (step === 'admission' || step === 'start') {
    if (!isRecord(admission) || typeof admission.operation !== 'string' || !operation.test(admission.operation)
      || admission.agent !== registration.agent || admission.allow !== true || !registration.agent) return unreadable();
    return { kind: 'existing', computer: value.computer, admission: { operation: admission.operation, agent: registration.agent, allow: true } };
  }
  if (admission !== null) return unreadable();
  return { kind: 'existing', computer: value.computer, admission: null };
}
function addAndRunOf(value: unknown, person: string): AddAndRun {
  if (!isRecord(value) || (value.version !== 1 && value.version !== 2 && value.version !== 3) || value.person !== person || !isRecord(value.settings)
    || !['registration', 'profile', 'review', 'computer', 'admission', 'start'].includes(String(value.step))) return unreadable();
  if (value.version === 3 && (!isRecord(value.registration) || value.registration.version !== 1)) return unreadable();
  const registration = agentRequestOf(value.registration, person); profileFromSettings(value.settings);
  const step = value.step as AddAndRun['step'];
  if (value.version === 1 && (step === 'admission' || 'placement' in value)) return unreadable();
  if (value.version !== 1 && 'machine' in value) return unreadable();
  const placement = placementOf(value.version === 1 ? { kind: 'new', machine: value.machine } : value.placement, registration, step);
  if (step === 'registration') {
    if (value.pending !== null) return unreadable();
    return { version: 3, person, registration, settings: value.settings, step, placement, pending: null };
  }
  if (!registration.agent || !registration.activated || registration.grants.some((entry) => entry.granted === null)) return unreadable();
  const prefix = '/agents/' + encodeURIComponent(registration.agent);
  const pending = pendingStartOf(value.pending, 'lys.pending.agent-start.' + person + '.' + registration.agent, prefix);
  const expected = step === 'computer' || step === 'admission' ? 'start' : step;
  if (pending.stage !== expected || pending.machine !== placementComputer(placement).id || pending.legacyKey !== undefined
    || placement.kind === 'existing' && placement.admission?.operation === pending.body.operation) return unreadable();
  return { version: 3, person, registration, settings: value.settings, step, placement, pending };
}
export function readAddAndRun(key: string, person: string): AddAndRun | null {
  const raw = sessionStorage.getItem(key);
  if (raw === null) return null;
  let value: unknown; let current: AddAndRun;
  try { value = JSON.parse(raw); current = addAndRunOf(value, person); }
  catch { return unreadable(); }
  if (isRecord(value) && value.version !== 3) {
    try { sessionStorage.setItem(key, JSON.stringify(current)); }
    catch { throw new Refused(0, { refusal: 'PendingAddAndRunMigrationFailed', reason: 'The earlier request could not be saved in the current format. No request has been sent; its original record is kept.' }); }
  }
  return current;
}
export function newAddAndRun(registration: PendingAgent, settings: Record<string, unknown>, name: string, person: string, computer?: Machine): AddAndRun {
  profileFromSettings(settings);
  const placement: Placement = computer ? placementOf({ kind: 'existing', computer, admission: null }, registration, 'registration')
    : { kind: 'new', machine: { body: { operation: operationId(), name, kind: 'Computer', runtime: 'lys-runner', slots: 0, may_run: [], may_run_roles: [], may_reach: [] }, phase: 'machine', legacy: false, machine: null } };
  if (placement.kind === 'new' && !validComputerName(name)) return unreadable();
  return { version: 3, person, registration, settings, step: 'registration', pending: null, placement };
}
export function addAndRunStep(current: AddAndRun): string {
  if (current.step === 'registration') return current.registration.activated ? current.registration.grants.some((entry) => entry.granted === null) ? 'giving selected access' : 'joining the team' : current.registration.agent ? 'activating the agent' : 'registering the agent';
  if (current.step === 'profile') return 'saving the settings';
  if (current.step === 'review') return 'approving the settings';
  if (current.step === 'computer' && current.placement.kind === 'new') return current.placement.machine.phase === 'machine' ? 'adding this computer' : 'recording this computer’s runner';
  if (current.step === 'admission') return 'allowing the agent on this computer';
  return 'starting the agent';
}
export class AddAndRunFailure extends Error {
  constructor(readonly step: string, readonly problem: unknown) {
    const code = problem instanceof Refused ? problem.refusal.refusal : problem instanceof Error ? problem.message.split(':', 1)[0] : '';
    // A refusal is a definite answer, so its own sentence is shown; only an
    // unknown outcome is "could not confirm".
    const refusedWith = problem instanceof Refused && problem.status >= 400 && problem.status < 500 ? problem.refusal.reason : '';
    super((refusedWith ? 'Lys refused ' + step + ': ' + refusedWith : 'Lys could not confirm ' + step + '.') + (code === 'RunnerStartUnconfirmed' || code === 'RunnerDidNotRun' ? ' Lys admitted the start, but no runner ran it.' : code === 'RunnerStartEnded' ? ' The runner confirmed this session already ended.' : ''));
  }
}
export async function addAndRun(initial: AddAndRun, login: Login, keep: (next: AddAndRun) => void): Promise<string> {
  let current = initial;
  const save = (next: AddAndRun) => { keep(next); current = next; };
  try {
    save(current);
    if (current.step === 'registration') {
      const agent = await addAgent(current.registration, current.person, (registration) => save({ ...current, registration }), login);
      const placement: Placement = current.placement.kind === 'new' ? { kind: 'new', machine: { ...current.placement.machine, body: { ...current.placement.machine.body, may_run: [agent] } } } : current.placement;
      save({ ...current, placement, step: 'profile', pending: { stage: 'profile', path: '/agents/' + agent + '/provisioning', body: { ...current.settings, operation: operationId(), from_version: 0 }, machine: placementComputer(placement).id, version: 0 } });
    }
    const agent = current.registration.agent;
    if (!agent) return unreadable();
    const prefix = '/agents/' + encodeURIComponent(agent);
    if (current.step === 'profile') {
      if (!current.pending) return unreadable();
      const confirmed = await profileRequest(agent, current.pending);
      save({ ...current, step: 'review', pending: { stage: 'review', path: prefix + '/provisioning/' + confirmed.version + '/review', body: { operation: operationId() }, machine: placementComputer(current.placement).id, version: confirmed.version } });
    }
    if (current.step === 'review') {
      if (!current.pending) return unreadable();
      const confirmed = await profileRequest(agent, current.pending);
      const placement: Placement = current.placement.kind === 'existing' ? { ...current.placement, admission: { operation: operationId(), agent, allow: true } } : current.placement;
      const machine = placementComputer(placement).id;
      save({ ...current, placement, step: placement.kind === 'new' ? 'computer' : 'admission', pending: { stage: 'start', path: prefix + '/start-command', body: { operation: operationId(), machine }, machine, version: confirmed.version } });
    }
    if (current.step === 'computer') {
      if (current.placement.kind !== 'new') return unreadable();
      await recordComputer(current.placement.machine, current.person, (machine) => save({ ...current, placement: { kind: 'new', machine } }));
      save({ ...current, step: 'start' });
    }
    if (current.step === 'admission') {
      const placement = current.placement;
      if (placement.kind !== 'existing' || !placement.admission) return unreadable();
      const answer = await request<unknown>('/network/machines/' + encodeURIComponent(placement.computer.id) + '/agents', placement.admission);
      const machine = confirmAdmission(answer, placement.computer.id, placement.admission, current.person);
      if (machine.state !== 'in_use' || machine.kind !== 'Computer' || machine.runtime !== 'lys-runner' || !machine.may_run.some((entry) => entry.id === agent && entry.state === 'active')) {
        throw new Refused(200, { refusal: 'MachineAdmissionReceiptMismatch', reason: 'The allowance did not confirm this active agent on the chosen computer. Its original request is kept; no start has been sent.' });
      }
      save({ ...current, placement: { ...placement, computer: machine }, step: 'start' });
    }
    if (!current.pending || current.step !== 'start') return unreadable();
    const answer = await startRequest(agent, current.pending);
    if (!answer.runner) throw new Refused(200, { refusal: 'RunnerDidNotRun', reason: 'Lys admitted the start, but no runner ran it. No running session is claimed.' });
    if (answer.runner.state !== 'running') throw new Refused(200, { refusal: 'RunnerStartEnded', reason: 'The runner confirmed this session already ended. No running session is claimed.' });
    return agent;
  } catch (problem) { throw new AddAndRunFailure(addAndRunStep(current), problem); }
}
export interface FirstRunOptions { choices: Choices | null; network: NetworkView | null; computers: Machine[]; problem: unknown }
export async function firstRunChoices(capability: AddAndRunCapability): Promise<FirstRunOptions> {
  let network: NetworkView | null = null;
  try {
    const answer = await request<unknown>('/network');
    if (!isRecord(answer) || !Array.isArray(answer.machines) || !answer.machines.every(readableMachine) || typeof answer.reports_served !== 'boolean') throw new Refused(200, { refusal: 'NetworkUnreadable', reason: 'The service did not name readable computers and their admission rules.' });
    network = { machines: answer.machines, reports_served: answer.reports_served };
    const inUse = network.machines.filter((machine) => machine.state === 'in_use');
    if (!inUse.length) return { choices: await readChoices(network), network, computers: [], problem: null };
    if (capability.machineAdmission !== true) return { choices: null, network, computers: [], problem: capability.admissionProblem };
    const eligible = inUse.filter((machine) => machine.kind === 'Computer' && machine.runtime === 'lys-runner' && operation.test(machine.id));
    const runners = await Promise.all(eligible.map(async (machine) => {
      try { confirmRunner(await request<unknown>('/network/machines/' + encodeURIComponent(machine.id) + '/runner'), machine.id, true); return { machine, problem: null }; }
      catch (problem) { return { machine: null, problem }; }
    }));
    const computers = runners.flatMap((entry) => entry.machine ? [entry.machine] : []);
    const problem = runners.find((entry) => entry.problem)?.problem ?? (computers.length ? null : new Refused(0, { refusal: 'LocalRunnerUnconfirmed', reason: 'No in-use computer has a confirmed local Lys runner. Add the agent here, then confirm its computer and runner in Network.' }));
    return { choices: computers.length ? await readChoices(network) : null, network, computers, problem };
  } catch (problem) { return { choices: null, network, computers: [], problem }; }
}
