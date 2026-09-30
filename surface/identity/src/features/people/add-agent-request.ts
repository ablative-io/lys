import { operationId, Refused, request } from '../../api';
import { confirmReceipt } from './recorded-receipt';

export interface PendingAgent { name: string; register: string; activate: string; agent: string | null }
function object(value: unknown): value is Record<string, unknown> { return value !== null && typeof value === 'object' && !Array.isArray(value); }
function mismatch(words: string): never { throw new Refused(200, { refusal: 'UnconfirmedReceipt', reason: words }); }

export function readAgentRequest(key: string): PendingAgent | null {
  const saved = sessionStorage.getItem(key);
  if (saved === null) return null;
  const value: unknown = JSON.parse(saved);
  if (!object(value) || typeof value.name !== 'string' || !value.name.trim()
    || typeof value.register !== 'string' || !/^op-[0-9a-f]{32}$/.test(value.register)
    || typeof value.activate !== 'string' || !/^op-[0-9a-f]{32}$/.test(value.activate)
    || !(value.agent === null || (typeof value.agent === 'string' && /^agent-[0-9a-f]{32}$/.test(value.agent)))) {
    throw new Error('The saved registration cannot be read. Check its outcome before adding another agent.');
  }
  return { name: value.name, register: value.register, activate: value.activate, agent: value.agent };
}

export function newAgentRequest(name: string): PendingAgent {
  return { name, register: operationId(), activate: operationId(), agent: null };
}

export async function addAgent(initial: PendingAgent, person: string, keep: (pending: PendingAgent) => void): Promise<string> {
  let pending = initial;
  keep(pending);
  if (!pending.agent) {
    const answer = await request<unknown>('/agents', { operation: pending.register, display_name: pending.name });
    confirmReceipt(answer, pending.register, '/agents');
    if (!object(answer) || typeof answer.agent !== 'string' || !/^agent-[0-9a-f]{32}$/.test(answer.agent)
      || answer.responsible !== person || !object(answer.receipt) || answer.receipt.identity !== answer.agent) {
      return mismatch('The registration did not confirm the agent and the person it answers to. Its original request is saved.');
    }
    pending = { ...pending, agent: answer.agent };
    keep(pending);
  }
  if (!pending.agent) return mismatch('The registration did not return an agent.');
  const path = '/identities/' + encodeURIComponent(pending.agent) + '/transitions';
  const answer = await request<unknown>(path, { operation: pending.activate, transition: 'activate', reason: 'Agent registration' });
  confirmReceipt(answer, pending.activate, path);
  if (!object(answer) || !object(answer.receipt) || answer.receipt.identity !== pending.agent || answer.receipt.change_kind !== 5) {
    return mismatch('Activation did not confirm this agent. Its original request is saved.');
  }
  return pending.agent;
}
