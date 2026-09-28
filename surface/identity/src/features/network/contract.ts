/** Machine declarations as network_api.rs serves them; declarations are not runtime reports. */
import type { AgentSummary } from '../../generated';
export interface Machine {
  id: string; name: string; kind: string; runtime: string | null; slots: number;
  may_run: AgentSummary[]; may_run_roles: string[]; may_reach: string[]; named_by: string; named_at: number;
  state: 'in_use' | 'retired'; retired_at: number | null; last_report_at: number | null;
}
export interface NetworkView { machines: Machine[]; reports_served: boolean }
export interface NameMachine {
  operation: string; name: string; kind: string; runtime: string | null;
  slots: number; may_run: string[]; may_run_roles: string[]; may_reach: string[];
}
export function matchesMachine(machine: Machine, body: NameMachine, person: string): boolean {
  return machine.id === body.operation && machine.named_by === person && machine.name === body.name
    && machine.kind === body.kind && machine.runtime === body.runtime && machine.slots === body.slots
    && JSON.stringify(machine.may_run.map((agent) => agent.id)) === JSON.stringify(body.may_run)
    && JSON.stringify(machine.may_run_roles) === JSON.stringify(body.may_run_roles)
    && JSON.stringify(machine.may_reach) === JSON.stringify(body.may_reach);
}
export function savedMachine(key: string): NameMachine | null {
  const raw = sessionStorage.getItem(key);
  if (raw === null) return null;
  const value: unknown = JSON.parse(raw);
  const strings = (data: unknown): data is string[] => Array.isArray(data) && data.every((part) => typeof part === 'string');
  if (!value || typeof value !== 'object' || !('operation' in value) || typeof value.operation !== 'string'
    || !/^op-[0-9a-f]{32}$/.test(value.operation) || !('name' in value) || typeof value.name !== 'string'
    || !('kind' in value) || typeof value.kind !== 'string' || !('runtime' in value) || !(value.runtime === null || typeof value.runtime === 'string')
    || !('slots' in value) || typeof value.slots !== 'number' || !Number.isInteger(value.slots) || value.slots < 0
    || !('may_run' in value) || !strings(value.may_run) || !('may_run_roles' in value) || !strings(value.may_run_roles)
    || !('may_reach' in value) || !strings(value.may_reach)) {
    throw new Error('The retained machine registration could not be read. It must be resolved before another is sent.');
  }
  return { operation: value.operation, name: value.name, kind: value.kind, runtime: value.runtime, slots: value.slots, may_run: value.may_run, may_run_roles: value.may_run_roles, may_reach: value.may_reach };
}
