import { operationId, Refused, request } from '../../api';
import type { Login } from '../../generated';
import { confirmReceipt } from './recorded-receipt';
import { agentGrantsOf, delegateAgentGrant } from './agent-grants';
import type { AgentGrant } from './agent-grants';

export interface PendingAgent {
  version: 1; responsible: string | null; grants: AgentGrant[];
  name: string; register: string; activate: string; agent: string | null;
  answersTo?: string; team: string | null; membership: string | null; activated: boolean;
}
function object(value: unknown): value is Record<string, unknown> { return value !== null && typeof value === 'object' && !Array.isArray(value); }
function operation(value: unknown): value is string { return typeof value === 'string' && /^op-[0-9a-f]{32}$/.test(value); }
function mismatch(words: string): never { throw new Refused(200, { refusal: 'UnconfirmedReceipt', reason: words }); }

export function readAgentRequest(key: string, caller: string): PendingAgent | null {
  const saved = sessionStorage.getItem(key);
  if (saved === null) return null;
  return agentRequestOf(JSON.parse(saved), caller);
}

export function agentRequestOf(value: unknown, caller: string): PendingAgent {
  if (!/^person-[0-9a-f]{32}$/.test(caller)) throw new Error('SignedInPersonUnreadable: the caller is not a readable person. No request has been sent.');
  // One format is read. A saved registration in any other shape is said to be unreadable, never rewritten into this one.
  if (!object(value) || value.version !== 1 || typeof value.name !== 'string' || !value.name.trim()
    || !operation(value.register) || !operation(value.activate)
    || !(value.agent === null || (typeof value.agent === 'string' && /^agent-[0-9a-f]{32}$/.test(value.agent)))) {
    throw new Error('The saved registration cannot be read. Check its outcome before adding another agent.');
  }
  const base = { name: value.name, register: value.register, activate: value.activate, agent: value.agent };
  if ((value.answersTo !== undefined && (typeof value.answersTo !== 'string' || !/^(person|agent)-[0-9a-f]{32}$/.test(value.answersTo)))
    || !(value.team === null || operation(value.team)) || !(value.membership === null || operation(value.membership))
    || (value.team === null) !== (value.membership === null) || typeof value.activated !== 'boolean'
    || (value.activated && value.agent === null)) {
    throw new Error('The saved person or team choice cannot be read. Check the original request before adding another agent.');
  }
  const responsible = value.responsible;
  if (!(responsible === null || typeof responsible === 'string' && /^person-[0-9a-f]{32}$/.test(responsible)) || (value.agent === null) !== (responsible === null)) {
    throw new Error('The saved registration format cannot be read.');
  }
  const grants = agentGrantsOf(value.grants, value.agent, responsible as string | null);
  if (grants.some((entry) => entry.operation === value.register || entry.operation === value.activate || entry.operation === value.membership || !value.activated && entry.body !== null)) {
    throw new Error('The saved access request is not bound to this activation.');
  }
  return { ...base, version: 1, responsible: responsible as string | null, grants, answersTo: value.answersTo, team: value.team, membership: value.membership, activated: value.activated };
}

export function newAgentRequest(name: string, answersTo: string, team: string, grants: AgentGrant[] = []): PendingAgent {
  return { version: 1, responsible: null, grants, name, register: operationId(), activate: operationId(), agent: null, answersTo, team: team || null, membership: team ? operationId() : null, activated: false };
}

export async function addAgent(initial: PendingAgent, caller: string, keep: (pending: PendingAgent) => void, login: Login): Promise<string> {
  let pending = agentRequestOf(initial, caller);
  keep(pending);
  if (!pending.agent) {
    const answer = await request<unknown>('/agents', { operation: pending.register, display_name: pending.name, ...(pending.answersTo && pending.answersTo !== caller ? { answers_to: pending.answersTo } : {}) });
    confirmReceipt(answer, pending.register, '/agents');
    const target = pending.answersTo ?? caller;
    const reportsToAgent = target.startsWith('agent-');
    if (!object(answer) || typeof answer.agent !== 'string' || !/^agent-[0-9a-f]{32}$/.test(answer.agent)
      || typeof answer.responsible !== 'string' || !/^person-[0-9a-f]{32}$/.test(answer.responsible)
      || (!reportsToAgent && answer.responsible !== target)
      || (reportsToAgent || 'reports_to' in answer) && (!object(answer.reports_to) || answer.reports_to.id !== target || answer.reports_to.kind !== (reportsToAgent ? 'agent' : 'person'))
      || !object(answer.receipt) || answer.receipt.identity !== answer.agent || answer.receipt.change_kind !== 2) {
      return mismatch('The registration did not confirm the agent and the person it answers to. Its original request is saved.');
    }
    pending = { ...pending, agent: answer.agent, responsible: answer.responsible };
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
  for (let index = 0; index < pending.grants.length; index += 1) {
    let entry = pending.grants[index];
    if (entry.granted !== null) continue;
    if (!pending.responsible) return mismatch('Registration did not name the person accountable for this access request.');
    if (entry.body === null) {
      entry = { ...entry, body: { operation: entry.operation, route: 'browser', source: entry.source, recipient: agent, responsible: pending.responsible,
        resource: entry.resource, relation: entry.relation, pass_on: { kind: 'use_only' }, window: entry.window } };
      pending = { ...pending, grants: pending.grants.map((current, at) => at === index ? entry : current) }; keep(pending);
    }
    const granted = await delegateAgentGrant(entry, caller);
    pending = { ...pending, grants: pending.grants.map((current, at) => at === index ? { ...entry, granted } : current) }; keep(pending);
  }
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
