/** Machine declarations as network_api.rs serves them; declarations are not runtime reports. */
import type { AgentSummary } from '../../generated';
import { Refused } from '../../api';
export interface Machine {
  id: string; name: string; kind: string; runtime: string | null; team?: string | null; slots: number;
  may_run: AgentSummary[]; may_run_roles?: string[]; may_reach: string[]; named_by: string; named_at: number;
  state: 'in_use' | 'retired'; retired_at: number | null; last_report_at: number | null;
}
export interface NetworkView { machines: Machine[]; reports_served: boolean }
export interface NameMachine {
  operation: string; name: string; kind: string; runtime: string | null;
  slots: number; may_run: string[]; may_run_roles?: string[]; may_reach: string[];
}
export function isRecord(value: unknown): value is Record<string, unknown> { return value !== null && typeof value === 'object' && !Array.isArray(value); }
export const strings = (value: unknown): value is string[] => Array.isArray(value) && value.every((part) => typeof part === 'string');
export function readableMachine(value: unknown): value is Machine {
  return isRecord(value) && typeof value.id === 'string' && typeof value.name === 'string' && (value.state === 'in_use' || value.state === 'retired')
    && (value.runtime === null || typeof value.runtime === 'string') && Array.isArray(value.may_run)
    && value.may_run.every((agent) => isRecord(agent) && typeof agent.id === 'string')
    && (value.may_run_roles === undefined || strings(value.may_run_roles));
}
export function matchesMachine(machine: unknown, body: NameMachine, person: string): machine is Machine {
  if (!readableMachine(machine)) return false;
  return machine.id === body.operation && machine.named_by === person && machine.name === body.name
    && machine.kind === body.kind && machine.runtime === body.runtime && machine.slots === body.slots
    && JSON.stringify(machine.may_run.map((agent) => agent.id)) === JSON.stringify(body.may_run)
    && JSON.stringify(machine.may_run_roles ?? []) === JSON.stringify(body.may_run_roles ?? [])
    && JSON.stringify(machine.may_reach) === JSON.stringify(body.may_reach);
}
/** A computer addition kept until its outcome is known. `remote` marks another computer: once it is named, it is given a connection code, never kept. */
export interface PendingMachine { body: NameMachine; phase: 'machine' | 'runner'; machine: Machine | null; remote?: true }
function unreadable(): never { throw new Refused(0, { refusal: 'PendingMachineUnreadable', reason: 'The retained computer addition cannot be read. Resolve its original request before adding another computer.' }); }
function machineBody(value: unknown): NameMachine {
  if (!isRecord(value) || typeof value.operation !== 'string' || !/^op-[0-9a-f]{32}$/.test(value.operation)
    || typeof value.name !== 'string' || !value.name.trim() || typeof value.kind !== 'string'
    || !(value.runtime === null || typeof value.runtime === 'string') || typeof value.slots !== 'number' || !Number.isInteger(value.slots) || value.slots < 0
    || !strings(value.may_run) || !strings(value.may_reach) || (value.may_run_roles !== undefined && !strings(value.may_run_roles))) return unreadable();
  return { operation: value.operation, name: value.name, kind: value.kind, runtime: value.runtime, slots: value.slots,
    may_run: value.may_run, ...(value.may_run_roles !== undefined ? { may_run_roles: value.may_run_roles } : {}), may_reach: value.may_reach };
}
export function savedMachine(key: string): PendingMachine | null {
  const raw = sessionStorage.getItem(key);
  if (raw === null) return null;
  let value: unknown;
  try { value = JSON.parse(raw); } catch { return unreadable(); }
  return pendingMachineOf(value);
}

export function pendingMachineOf(value: unknown): PendingMachine | null {
  // One format is read. A saved addition in any other shape is said to be unreadable, never rewritten into this one.
  if (!isRecord(value) || !('body' in value) || value.version !== 1 || !['machine', 'runner'].includes(String(value.phase))) return unreadable();
  const body = machineBody(value.body);
  const machine = value.machine;
  if (machine !== null && (!readableMachine(machine) || machine.id !== body.operation)) return unreadable();
  if (value.phase !== 'machine' && machine === null) return unreadable();
  // Another computer is kept only while it is being named.
  if (value.remote !== undefined && (value.remote !== true || value.phase !== 'machine')) return unreadable();
  return { body, phase: value.phase as PendingMachine['phase'], machine, ...(value.remote === true ? { remote: true as const } : {}) };
}

export function confirmRunner(value: unknown, machine: string, local: boolean): void {
  const runner = isRecord(value) ? value.runner : undefined;
  if (!isRecord(value) || value.machine !== machine || !isRecord(runner)
    || !(runner.kind === 'lys' || (!local && runner.kind === 'socket' && typeof runner.path === 'string' && runner.path.startsWith('/'))
      || (!local && runner.kind === 'dialled' && typeof runner.key === 'string' && /^[0-9a-f]{64}$/.test(runner.key)))) {
    throw new Refused(200, { refusal: 'RunnerReceiptMismatch', reason: 'The computer’s runner is missing or could not be confirmed. Its addition remains unresolved; no runner is invented.' });
  }
}

/** One grant a machine holds, as GET /network/machine-identities serves it (ACCESS-005 R2). */
export interface MachineGrant {
  grant: string; resource: { kind: string; id: string }; relation: string; actions: string[];
  window: { starts_at: number; ends_at: number | null }; mode: 'outright' | 'by_draft' | 'by_two'; admitted: boolean;
}
/** A machine: the identity a computer's join made from its own key, answering to the administrator who gave the code. */
export interface MachineIdentity {
  identity: string; machine: string; key: string; responsible: string; responsible_name: string | null;
  joined_at: number; state: string; replaced: boolean; grants: MachineGrant[];
}
export interface MachineIdentities { machines: MachineIdentity[]; revision: number }

function readableMachineGrant(value: unknown): value is MachineGrant {
  return isRecord(value) && typeof value.grant === 'string' && isRecord(value.resource) && typeof value.resource.kind === 'string'
    && typeof value.resource.id === 'string' && typeof value.relation === 'string' && strings(value.actions)
    && (value.mode === 'outright' || value.mode === 'by_draft' || value.mode === 'by_two') && typeof value.admitted === 'boolean';
}
function readableMachineIdentity(value: unknown): value is MachineIdentity {
  return isRecord(value) && typeof value.identity === 'string' && /^machine-[0-9a-f]{32}$/.test(value.identity)
    && typeof value.machine === 'string' && typeof value.key === 'string' && typeof value.responsible === 'string'
    && (value.responsible_name === null || typeof value.responsible_name === 'string') && typeof value.joined_at === 'number'
    && typeof value.state === 'string' && typeof value.replaced === 'boolean' && Array.isArray(value.grants) && value.grants.every(readableMachineGrant);
}
/** The machines as the service answered them, or a refusal naming what it did not say. */
export function readMachineIdentities(value: unknown): MachineIdentities {
  if (!isRecord(value) || !Array.isArray(value.machines) || !value.machines.every(readableMachineIdentity) || typeof value.revision !== 'number') {
    throw new Refused(0, { refusal: 'MachinesUnreadable', reason: 'The service did not answer the machines in the shape this page reads.' });
  }
  return { machines: value.machines, revision: value.revision };
}
