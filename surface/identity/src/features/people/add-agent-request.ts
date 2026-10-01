import { operationId, Refused, request } from '../../api';
import type { Login } from '../../generated';
import { confirmReceipt } from './recorded-receipt';

export interface PendingAgent {
  name: string; register: string; activate: string; agent: string | null;
  answersTo?: string; team: string | null; membership: string | null; activated: boolean;
}
function object(value: unknown): value is Record<string, unknown> { return value !== null && typeof value === 'object' && !Array.isArray(value); }
function operation(value: unknown): value is string { return typeof value === 'string' && /^op-[0-9a-f]{32}$/.test(value); }
function mismatch(words: string): never { throw new Refused(200, { refusal: 'UnconfirmedReceipt', reason: words }); }

export function readAgentRequest(key: string): PendingAgent | null {
  const saved = sessionStorage.getItem(key);
  if (saved === null) return null;
  const value: unknown = JSON.parse(saved);
  if (!object(value) || typeof value.name !== 'string' || !value.name.trim()
    || !operation(value.register) || !operation(value.activate)
    || !(value.agent === null || (typeof value.agent === 'string' && /^agent-[0-9a-f]{32}$/.test(value.agent)))) {
    throw new Error('The saved registration cannot be read. Check its outcome before adding another agent.');
  }
  const base = { name: value.name, register: value.register, activate: value.activate, agent: value.agent };
  if (!('answersTo' in value) && !('team' in value) && !('membership' in value) && !('activated' in value)) {
    return { ...base, team: null, membership: null, activated: false };
  }
  if ((value.answersTo !== undefined && (typeof value.answersTo !== 'string' || !/^person-[0-9a-f]{32}$/.test(value.answersTo)))
    || !(value.team === null || operation(value.team)) || !(value.membership === null || operation(value.membership))
    || (value.team === null) !== (value.membership === null) || typeof value.activated !== 'boolean'
    || (value.activated && value.agent === null)) {
    throw new Error('The saved person or team choice cannot be read. Check the original request before adding another agent.');
  }
  return { ...base, answersTo: value.answersTo, team: value.team, membership: value.membership, activated: value.activated };
}

export function newAgentRequest(name: string, answersTo: string, team: string): PendingAgent {
  return { name, register: operationId(), activate: operationId(), agent: null, answersTo, team: team || null, membership: team ? operationId() : null, activated: false };
}

export async function addAgent(initial: PendingAgent, caller: string, keep: (pending: PendingAgent) => void, login: Login): Promise<string> {
  let pending = initial;
  keep(pending);
  if (!pending.agent) {
    const answer = await request<unknown>('/agents', { operation: pending.register, display_name: pending.name, ...(pending.answersTo && pending.answersTo !== caller ? { answers_to: pending.answersTo } : {}) });
    confirmReceipt(answer, pending.register, '/agents');
    if (!object(answer) || typeof answer.agent !== 'string' || !/^agent-[0-9a-f]{32}$/.test(answer.agent)
      || answer.responsible !== (pending.answersTo ?? caller) || !object(answer.receipt) || answer.receipt.identity !== answer.agent || answer.receipt.change_kind !== 2) {
      return mismatch('The registration did not confirm the agent and the person it answers to. Its original request is saved.');
    }
    pending = { ...pending, agent: answer.agent };
    keep(pending);
  }
  if (!pending.agent) return mismatch('The registration did not return an agent.');
  if (!pending.activated) {
    const path = '/identities/' + encodeURIComponent(pending.agent) + '/transitions';
    const answer = await request<unknown>(path, { operation: pending.activate, transition: 'activate', reason: 'Agent registration' });
    confirmReceipt(answer, pending.activate, path);
    if (!object(answer) || !object(answer.receipt) || answer.receipt.identity !== pending.agent || answer.receipt.change_kind !== 5) {
      return mismatch('Activation did not confirm this agent. Its original request is saved.');
    }
    pending = { ...pending, activated: true };
    keep(pending);
  }
  const agent = pending.agent;
  if (!agent) return mismatch('Activation did not name this agent.');
  if (pending.team) {
    const answer = await request<unknown>('/teams/' + encodeURIComponent(pending.team) + '/members', { operation: pending.membership, member: agent });
    if (!object(answer) || answer.id !== pending.team || !Array.isArray(answer.members) || !answer.members.includes(agent)
      || !object(answer.recorded) || answer.recorded.operation !== pending.membership || answer.recorded.act !== 'added' || answer.recorded.member !== agent
      || !object(answer.recorded.by) || answer.recorded.by.provider !== login.provider || answer.recorded.by.subject !== login.subject) {
      return mismatch('The team did not confirm this membership. The original request is saved.');
    }
  }
  return agent;
}
