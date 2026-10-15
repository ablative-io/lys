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
import { answeredNo } from '../../kept';

type Placement = { kind: 'new'; machine: PendingMachine } | { kind: 'existing'; computer: Machine; admission: MachineAdmission | null };
export interface AddAndRun {
  version: 3; person: string; registration: PendingAgent; settings: Record<string, unknown>;
  step: 'registration' | 'profile' | 'review' | 'computer' | 'admission' | 'start'; placement: Placement; pending: Pending | null;
}
const operation = /^op-[0-9a-f]{32}$/;
export function placementComputer(placement: Placement): { id: string; name: string } {
  return placement.kind === 'new' ? { id: placement.machine.body.operation, name: placement.machine.body.name } : placement.computer;
}
const unreadable = (field: string): never => { throw new Refused(0, { refusal: 'PendingAddAndRunUnreadable', reason: 'The add-and-run request cannot be read: ' + field + '. Check that field before trying again.' }); };
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
  if (!isRecord(harness)) return unreadable('harness must be an object');
  if (typeof harness.name !== 'string' || !harness.name.trim()) return unreadable('harness.name must name the program');
  if (typeof harness.program !== 'string' || !harness.program.length || /[\x00-\x1f\x7f]/.test(harness.program)
    || !(harness.program.startsWith('/') || /^[A-Za-z0-9_.+-]+$/.test(harness.program))) return unreadable('harness.program must be an absolute path or a bare command name');
  if (typeof harness.package !== 'string') return unreadable('harness.package must be text');
  if (!description(harness.description)) return unreadable('harness.description must describe the reported models, permissions and tools');
  if (!strings(value.model_access) || !value.model_access.length) return unreadable('model_access must name at least one model');
  if (!strings(value.tools) || value.tools.length) return unreadable('tools must be an empty list for this start');
  if (!strings(value.skills) || value.skills.length) return unreadable('skills must be an empty list for this start');
  if (!Array.isArray(value.mcp_servers) || value.mcp_servers.length) return unreadable('mcp_servers must be an empty list for this start');
  if (typeof value.instructions !== 'string') return unreadable('instructions must be text');
  if (typeof value.note !== 'string') return unreadable('note must be text');
  if (!isRecord(permissions) || typeof permissions.default_mode !== 'string') return unreadable('permissions.default_mode must be text');
  if (value.instructions_mode !== undefined && value.instructions_mode !== 'keep' && value.instructions_mode !== 'append' && value.instructions_mode !== 'replace') return unreadable('instructions_mode must be keep, append or replace');
  if (value.working_folder !== undefined && (typeof value.working_folder !== 'string' || !value.working_folder.startsWith('/'))) return unreadable('working_folder must be an absolute path');
  if (value.resume !== undefined && (typeof value.resume !== 'string' || !/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/.test(value.resume))) return unreadable('resume must be a 36-character session or thread id');
  return { version: 0, operation: '', harness: { name: harness.name, program: harness.program, package: harness.package, description: harness.description },
    model_access: value.model_access, permissions: { default_mode: permissions.default_mode }, tools: [], skills: [], mcp_servers: [],
    instructions: value.instructions, instructions_mode: value.instructions_mode ?? (value.instructions ? 'append' : 'keep'), note: value.note, set_by: '', set_at: 0, session: null,
    ...(typeof value.working_folder === 'string' ? { working_folder: value.working_folder } : {}),
    ...(typeof value.resume === 'string' ? { resume: value.resume } : {}) };
}
function placementOf(value: unknown, registration: PendingAgent, step: AddAndRun['step']): Placement {
  if (!isRecord(value)) return unreadable('placement must be an object');
  if (value.kind === 'new') {
    if (!isRecord(value.machine) || 'computer' in value || 'admission' in value || step === 'admission') return unreadable('placement.machine must describe a new computer without an existing admission');
    const machine = pendingMachineOf({ ...value.machine, version: 1 });
    if (!machine || !validComputerName(machine.body.name) || machine.body.kind !== 'Computer' || machine.body.runtime !== 'lys-runner'
      || machine.body.slots !== 0 || machine.body.may_reach.length || (machine.body.may_run_roles ?? []).length) return unreadable('placement.machine.body must name a computer, runner and valid zero-slot scope');
    if (step === 'registration' ? machine.phase !== 'machine' || machine.body.may_run.length
      : machine.body.may_run.length !== 1 || machine.body.may_run[0] !== registration.agent) return unreadable('placement.machine.body.may_run must name this registered agent at this stage');
    if (step === 'start' && (machine.phase !== 'runner' || !machine.machine)) return unreadable('placement.machine.phase must confirm the runner before start');
    return { kind: 'new', machine };
  }
  if (value.kind !== 'existing' || 'machine' in value || step === 'computer' || !readableMachine(value.computer)
    || !operation.test(value.computer.id) || value.computer.kind !== 'Computer' || value.computer.state !== 'in_use' || value.computer.runtime !== 'lys-runner') return unreadable('placement.computer must name an in-use computer with a runner');
  const admission = value.admission;
  if (step === 'admission' || step === 'start') {
    if (!isRecord(admission) || typeof admission.operation !== 'string' || !operation.test(admission.operation)
      || admission.agent !== registration.agent || admission.allow !== true || !registration.agent) return unreadable('placement.admission must name this agent and its allowance operation');
    return { kind: 'existing', computer: value.computer, admission: { operation: admission.operation, agent: registration.agent, allow: true } };
  }
  if (admission !== null) return unreadable('placement.admission must be null before the allowance stage');
  return { kind: 'existing', computer: value.computer, admission: null };
}
function addAndRunOf(value: unknown, person: string): AddAndRun {
  // One format is read. A saved request in any other shape is said to be unreadable, never rewritten into this one.
  if (!isRecord(value) || value.version !== 3 || value.person !== person || !isRecord(value.settings) || 'machine' in value
    || !['registration', 'profile', 'review', 'computer', 'admission', 'start'].includes(String(value.step))
    || !isRecord(value.registration) || value.registration.version !== 1) return unreadable('request.version, person, step, settings and registration must describe this flow');
  const registration = agentRequestOf(value.registration, person); profileFromSettings(value.settings);
  const step = value.step as AddAndRun['step'];
  const placement = placementOf(value.placement, registration, step);
  if (step === 'registration') {
    if (value.pending !== null) return unreadable('pending must be null before registration');
    return { version: 3, person, registration, settings: value.settings, step, placement, pending: null };
  }
  if (!registration.agent || !registration.activated || registration.grants.some((entry) => entry.granted === null)) return unreadable('registration.agent, activated and grants must be confirmed before proceeding');
  const prefix = '/agents/' + encodeURIComponent(registration.agent);
  const pending = pendingStartOf(value.pending, prefix);
  const expected = step === 'computer' || step === 'admission' ? 'start' : step;
  if (pending.stage !== expected || pending.machine !== placementComputer(placement).id
    || placement.kind === 'existing' && placement.admission?.operation === pending.body.operation) return unreadable('pending.stage, machine and operation must match the selected placement');
  return { version: 3, person, registration, settings: value.settings, step, placement, pending };
}
export function readAddAndRun(key: string, person: string): AddAndRun | null {
  const raw = sessionStorage.getItem(key);
  if (raw === null) return null;
  try { return addAndRunOf(JSON.parse(raw), person); }
  catch (error) { if (error instanceof Refused) throw error; return unreadable('stored request JSON must describe this add-and-run flow'); }
}
export const CONNECT_COMPUTER = 'Connect your own computer first: Network → Add a computer → download its installer';
function requireComputerAccess(computer: Machine | undefined, person: string, administrator: boolean): void {
  if (!administrator && (!computer || computer.named_by !== person)) throw new Refused(0, {
    refusal: 'ComputerConnectionRequired', reason: CONNECT_COMPUTER,
  });
}
export function newAddAndRun(registration: PendingAgent, settings: Record<string, unknown>, name: string, person: string, computer?: Machine, administrator = false): AddAndRun {
  requireComputerAccess(computer, person, administrator);
  profileFromSettings(settings);
  const placement: Placement = computer ? placementOf({ kind: 'existing', computer, admission: null }, registration, 'registration')
    : { kind: 'new', machine: { body: { operation: operationId(), name, kind: 'Computer', runtime: 'lys-runner', slots: 0, may_run: [], may_run_roles: [], may_reach: [] }, phase: 'machine', machine: null } };
  if (placement.kind === 'new' && !validComputerName(name)) return unreadable('placement.machine.body.name must be a valid computer name');
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
    const refusedWith = answeredNo(problem) && problem instanceof Refused ? problem.refusal.reason : '';
    super((refusedWith ? 'Lys refused ' + step + ': ' + refusedWith : 'Lys could not confirm ' + step + '.') + (code === 'RunnerStartUnconfirmed' || code === 'RunnerDidNotRun' ? ' Lys admitted the start, but no runner ran it.' : code === 'RunnerStartEnded' ? ' The runner confirmed this session already ended.' : ''));
  }
}
export async function addAndRun(initial: AddAndRun, login: Login, keep: (next: AddAndRun) => void, administrator = false): Promise<string> {
  let current = initial;
  const save = (next: AddAndRun) => { keep(next); current = next; };
  try {
    requireComputerAccess(current.placement.kind === 'existing' ? current.placement.computer : undefined, current.person, administrator);
    save(current);
    if (current.step === 'registration') {
      const agent = await addAgent(current.registration, current.person, (registration) => save({ ...current, registration }), login);
      const placement: Placement = current.placement.kind === 'new' ? { kind: 'new', machine: { ...current.placement.machine, body: { ...current.placement.machine.body, may_run: [agent] } } } : current.placement;
      save({ ...current, placement, step: 'profile', pending: { stage: 'profile', path: '/agents/' + agent + '/provisioning', body: { ...current.settings, operation: operationId(), from_version: 0 }, machine: placementComputer(placement).id, version: 0 } });
    }
    const agent = current.registration.agent;
    if (!agent) return unreadable('registration.agent must be confirmed before provisioning');
    const prefix = '/agents/' + encodeURIComponent(agent);
    if (current.step === 'profile') {
      if (!current.pending) return unreadable('pending must name the profile request');
      const confirmed = await profileRequest(agent, current.pending);
      save({ ...current, step: 'review', pending: { stage: 'review', path: prefix + '/provisioning/' + confirmed.version + '/review', body: { operation: operationId() }, machine: placementComputer(current.placement).id, version: confirmed.version } });
    }
    if (current.step === 'review') {
      if (!current.pending) return unreadable('pending must name the review request');
      const confirmed = await profileRequest(agent, current.pending);
      const placement: Placement = current.placement.kind === 'existing' ? { ...current.placement, admission: { operation: operationId(), agent, allow: true } } : current.placement;
      const machine = placementComputer(placement).id;
      save({ ...current, placement, step: placement.kind === 'new' ? 'computer' : 'admission', pending: { stage: 'start', path: prefix + '/start-command', body: { operation: operationId(), machine }, machine, version: confirmed.version } });
    }
    if (current.step === 'computer') {
      if (current.placement.kind !== 'new') return unreadable('placement.kind must be new before recording a computer');
      await recordComputer(current.placement.machine, current.person, (machine) => save({ ...current, placement: { kind: 'new', machine } }));
      save({ ...current, step: 'start' });
    }
    if (current.step === 'admission') {
      const placement = current.placement;
      if (placement.kind !== 'existing' || !placement.admission) return unreadable('placement.admission must be an existing computer allowance');
      const answer = await request<unknown>('/network/machines/' + encodeURIComponent(placement.computer.id) + '/agents', placement.admission);
      const machine = confirmAdmission(answer, placement.computer.id, placement.admission, current.person);
      if (machine.state !== 'in_use' || machine.kind !== 'Computer' || machine.runtime !== 'lys-runner' || !machine.may_run.some((entry) => entry.id === agent && entry.state === 'active')) {
        throw new Refused(200, { refusal: 'MachineAdmissionReceiptMismatch', reason: 'The allowance did not confirm this active agent on the chosen computer. Its original request is kept; no start has been sent.' });
      }
      save({ ...current, placement: { ...placement, computer: machine }, step: 'start' });
    }
    if (!current.pending || current.step !== 'start') return unreadable('pending.stage must name the start request');
    const answer = await startRequest(agent, current.pending);
    if (!answer.runner) throw new Refused(200, { refusal: 'RunnerDidNotRun', reason: 'Lys admitted the start, but no runner ran it. No running session is claimed.' });
    if (answer.runner.state !== 'running') throw new Refused(200, { refusal: 'RunnerStartEnded', reason: 'The runner confirmed this session already ended. No running session is claimed.' });
    return agent;
  } catch (problem) { throw new AddAndRunFailure(addAndRunStep(current), problem); }
}
export type RunnerChoice = { machine: Machine; state: 'asking' | 'confirmed' } | { machine: Machine; state: 'unavailable'; reason: string };
export interface FirstRunOptions { choices: Choices | null; network: NetworkView | null; computers: Machine[]; runners: RunnerChoice[]; loading: boolean; problem: unknown }
export function runnerProblemReason(problem: unknown): string {
  return problem instanceof Refused ? problem.refusal.reason : problem instanceof Error ? problem.message : String(problem);
}
export async function firstRunChoices(capability: AddAndRunCapability, person: string, administrator: boolean, publish?: (options: FirstRunOptions) => void): Promise<FirstRunOptions> {
  let network: NetworkView | null = null;
  let runners: RunnerChoice[] = [];
  try {
    const answer = await request<unknown>('/network');
    if (!isRecord(answer) || !Array.isArray(answer.machines) || !answer.machines.every(readableMachine) || typeof answer.reports_served !== 'boolean') throw new Refused(200, { refusal: 'NetworkUnreadable', reason: 'The service did not name readable computers and their admission rules.' });
    network = { machines: answer.machines, reports_served: answer.reports_served, host_name: typeof answer.host_name === 'string' && answer.host_name.trim() ? answer.host_name : null };
    const inUse = network.machines.filter((machine) => machine.state === 'in_use' && (administrator || machine.named_by === person));
    if (!inUse.length) return { choices: administrator ? await readChoices(network) : null, network, computers: [], runners, loading: false, problem: null };
    if (capability.machineAdmission !== true) return { choices: null, network, computers: [], runners, loading: false, problem: capability.admissionProblem };
    const eligible = inUse.filter((machine) => machine.kind === 'Computer' && machine.runtime === 'lys-runner' && operation.test(machine.id));
    runners = eligible.map((machine) => ({ machine, state: 'asking' }));
    let choices: Choices | null = null;
    let choicesPending = true;
    let choicesProblem: unknown = null;
    const snapshot = (): FirstRunOptions => {
      const computers = runners.flatMap((entry) => entry.state === 'confirmed' ? [entry.machine] : []);
      const asking = runners.some((entry) => entry.state === 'asking');
      const problem = choicesProblem ?? (computers.length || asking ? null : new Refused(0, { refusal: 'LocalRunnerUnconfirmed', reason: 'No in-use computer has a confirmed Lys runner, on this computer or dialled in from another.' }));
      return { choices: computers.length && choices ? { ...choices, machines: computers } : null, network, computers, runners, loading: choicesPending || !computers.length && asking, problem };
    };
    publish?.(snapshot());
    // Programme choices and each runner answer are independent reads. A slow
    // computer cannot hold back a confirmed computer or replace its answer.
    await Promise.all([
      readChoices(network).then((answer) => { choices = answer; }, (problem: unknown) => { choicesProblem = problem; }).then(() => { choicesPending = false; publish?.(snapshot()); }),
      ...eligible.map(async (machine) => {
        let answer: RunnerChoice;
        try {
          const held = await request<unknown>('/network/machines/' + encodeURIComponent(machine.id) + '/runner');
          // Lys's own runner counts, held by this install or dialled in from the computer itself; a socket Lys does not hold is not a place to start an agent.
          confirmRunner(held, machine.id, false);
          if (!isRecord(held) || held.answers !== true) throw new Refused(200, {
            refusal: 'RunnerNotAnswering',
            reason: machine.name + ' is not answering right now; start its computer, then try again',
          });
          if (isRecord(held) && isRecord(held.runner) && held.runner.kind === 'socket') throw new Refused(200, { refusal: 'RunnerNotLys', reason: 'The computer’s runner is a socket Lys does not hold. Agents start only through Lys’s own runner, on this computer or dialled in.' });
          answer = { machine, state: 'confirmed' };
        } catch (problem) { answer = { machine, state: 'unavailable', reason: runnerProblemReason(problem) }; }
        runners = runners.map((entry) => entry.machine.id === machine.id ? answer : entry);
        publish?.(snapshot());
      }),
    ]);
    return snapshot();
  } catch (problem) { return { choices: null, network, computers: [], runners, loading: false, problem }; }
}
