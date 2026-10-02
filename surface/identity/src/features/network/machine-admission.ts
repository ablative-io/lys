import { Refused, request } from '../../api';
import { isRecord, readableMachine, strings } from './contract';
import type { Machine } from './contract';

export interface MachineAdmission { operation: string; agent: string; allow: boolean }
export type PendingAdmissions = Record<string, MachineAdmission>;
const operation = /^op-[0-9a-f]{32}$/;
const refused = (name: string, reason: string): never => { throw new Refused(0, { refusal: name, reason }); };

export function savedAdmissions(key: string, agent: string): PendingAdmissions {
  const raw = sessionStorage.getItem(key);
  if (raw === null) return {};
  let value: unknown;
  try { value = JSON.parse(raw); } catch { return refused('PendingMachineAdmissionUnreadable', 'The saved computer permission cannot be read. Resolve it before making another change.'); }
  if (!isRecord(value)) return refused('PendingMachineAdmissionUnreadable', 'The saved computer permissions are not requests.');
  const pending: PendingAdmissions = {};
  for (const [machine, body] of Object.entries(value)) {
    if (!operation.test(machine) || !isRecord(body) || typeof body.operation !== 'string' || !operation.test(body.operation)
      || body.agent !== agent || typeof body.allow !== 'boolean') return refused('PendingMachineAdmissionUnreadable', 'A saved permission does not name this agent, computer and operation.');
    pending[machine] = { operation: body.operation, agent, allow: body.allow };
  }
  return pending;
}

export function confirmAdmission(value: unknown, machine: string, body: MachineAdmission, person: string): Machine {
  const recorded = isRecord(value) ? value.recorded : undefined;
  if (!isRecord(value) || !readableMachine(value.machine) || value.machine.id !== machine || !isRecord(recorded)
    || recorded.operation !== body.operation || recorded.machine !== machine || recorded.agent !== body.agent || recorded.allow !== body.allow
    || recorded.by !== person || typeof recorded.at !== 'number' || !Number.isSafeInteger(recorded.at) || recorded.at < 0
    || (recorded.original_may_run !== undefined && !strings(recorded.original_may_run))) {
    return refused('MachineAdmissionReceiptMismatch', 'The answer did not confirm this computer permission. Its original request is kept.');
  }
  return value.machine;
}

export async function machineAdmissionServed(): Promise<boolean> {
  return machineAdmissionOf(await request<unknown>('/openapi.json'));
}

export function machineAdmissionOf(document: unknown): boolean {
  if (!isRecord(document) || !isRecord(document.paths)) return refused('MachineAdmissionSchemaUnreadable', 'The served route descriptions could not be read.');
  const route = document.paths['/network/machines/{id}/agents'];
  if (route === undefined) return false;
  const resolve = (initial: unknown): Record<string, unknown> => {
    let value = initial;
    for (let depth = 0; depth < 8; depth += 1) {
      if (!isRecord(value)) break;
      if (value.$ref === undefined) return value;
      if (typeof value.$ref !== 'string' || !value.$ref.startsWith('#/')) break;
      const parts = value.$ref.slice(2).split('/').map((part) => part.replace(/~1/g, '/').replace(/~0/g, '~'));
      value = document;
      for (const part of parts) value = isRecord(value) ? value[part] : undefined;
    }
    return refused('MachineAdmissionSchemaUnreadable', 'The computer permission route has an unreadable schema.');
  };
  const schema = (value: unknown) => {
    const content = resolve(value).content;
    if (!isRecord(content) || !isRecord(content['application/json'])) return refused('MachineAdmissionSchemaUnreadable', 'The computer permission route does not describe JSON.');
    return resolve(content['application/json'].schema);
  };
  const post = resolve(resolve(route).post);
  const body = schema(post.requestBody);
  const responses = resolve(post.responses);
  const answer = schema(responses['200']);
  const fields = body.properties; const required = body.required;
  const answerFields = answer.properties; const answerRequired = answer.required;
  if (!isRecord(fields) || !strings(required) || !isRecord(answerFields) || !strings(answerRequired)
    || !['operation', 'agent', 'allow'].every((field) => required.includes(field))
    || resolve(fields.operation).type !== 'string' || resolve(fields.agent).type !== 'string' || resolve(fields.allow).type !== 'boolean'
    || !['machine', 'recorded'].every((field) => answerRequired.includes(field))
    || resolve(answerFields.machine).type !== 'object' || resolve(answerFields.recorded).type !== 'object') {
    return refused('MachineAdmissionSchemaUnreadable', 'The computer permission request or receipt is incomplete.');
  }
  return true;
}
