import { operationId, Refused, request } from '../../api';
import type { DelegateBody, Grant, GrantModel } from '../../generated/grants';
import { isRecord, strings } from '../network/contract';
import { actionWords } from '../grants/action-words';
import { resourceWords } from './agent-resource-names';

export interface GrantChoices { grants: Grant[]; model: GrantModel | null; problem: unknown }
/** A grant the boss holds, with the acts it can give an agent one by one, or why it gives none. */
export interface GrantOption { grant: Grant; actions: string[]; reason: string }
export interface AgentGrant { operation: string; source: string; resource: { kind: string; id: string }; relation: string; actions: string[]; window: { starts_at: number; ends_at: number | null }; body: DelegateBody | null; granted: string | null }
const operation = (value: unknown): value is string => typeof value === 'string' && /^op-[0-9a-f]{32}$/.test(value);
const grantId = (value: unknown): value is string => typeof value === 'string' && /^grant-[0-9a-f]{32}$/.test(value);
const second = (value: unknown): value is number => typeof value === 'number' && Number.isSafeInteger(value) && value >= 0;
const resource = (value: unknown): value is { kind: string; id: string } => isRecord(value) && typeof value.kind === 'string' && Boolean(value.kind) && typeof value.id === 'string' && Boolean(value.id);
const window = (value: unknown): value is { starts_at: number; ends_at: number | null } => isRecord(value) && second(value.starts_at) && (value.ends_at === null || second(value.ends_at) && value.ends_at > value.starts_at);
function readableGrant(value: unknown): value is Grant {
  return isRecord(value) && grantId(value.id) && typeof value.holder === 'string' && /^(person|agent|op)-[0-9a-f]{32}$/.test(value.holder)
    && resource(value.resource) && typeof value.relation === 'string' && strings(value.actions) && isRecord(value.pass_on)
    && (value.pass_on.kind === 'use_only' || value.pass_on.kind === 'to' && strings(value.pass_on.actions) && strings(value.pass_on.recipients))
    && isRecord(value.standing) && typeof value.standing.stands === 'boolean' && window(value.window)
    && (value.effective_ends_at === null || second(value.effective_ends_at));
}
export async function readGrantChoices(): Promise<GrantChoices> {
  try {
    const [answer, model] = await Promise.all([request<unknown>('/grants'), request<unknown>('/grants/model')]);
    if (!isRecord(answer) || !Array.isArray(answer.grants) || !answer.grants.every(readableGrant)
      || !isRecord(model) || !second(model.version) || !isRecord(model.relations) || !Object.values(model.relations).every(strings)) {
      throw new Refused(200, { refusal: 'GrantsUnreadable', reason: 'Lys did not return readable grants and their actions. No access is selected.' });
    }
    return { grants: answer.grants, model: model as unknown as GrantModel, problem: null };
  } catch (problem) { return { grants: [], model: null, problem }; }
}
/** The prefix of the relation that carries one action alone. */
const ONE = 'only.';
/** The acts Lys says no agent may hold, or null when it does not say. */
function withheldFromAgents(model: GrantModel | null): string[] | null {
  const withheld: unknown = isRecord(model) ? (model as unknown as Record<string, unknown>).withheld_from_agents : null;
  return strings(withheld) ? withheld : null;
}
/** The acts of `grant` an agent may be given one by one: held, passable to an agent, carried alone by an only.<action> relation and never withheld from agents. An app's kind gives an agent nothing. */
function agentActions(model: GrantModel | null, grant: Grant, passable: string[], withheld: string[]): string[] {
  if (!model || grant.resource.kind.includes('.')) return [];
  return grant.actions.filter((action) => passable.includes(action) && !withheld.includes(action)
    && JSON.stringify(model.relations[ONE + action] ?? null) === JSON.stringify([action])).sort();
}
export function grantOptions(choices: GrantChoices, boss: string, caller: string, name: string): GrantOption[] {
  const withheld = withheldFromAgents(choices.model);
  return choices.grants.filter((grant) => grant.holder === boss).map((grant) => {
    const pass = grant.pass_on;
    const offered = pass.kind === 'to' && withheld ? agentActions(choices.model, grant, pass.actions, withheld) : [];
    const reason = boss !== caller ? 'Only ' + name + ' can pass this on.' : !grant.standing.stands ? 'This access no longer stands.'
      : pass.kind !== 'to' ? 'This access cannot be passed on.' : !pass.recipients.includes('agent') ? 'This access cannot be given to an agent.'
        : !withheld ? 'Lys did not say which actions no agent may hold, so none is offered.'
          : !offered.length ? 'Nothing in this access can be given to an agent.' : '';
    return { grant, actions: reason ? grant.actions : offered, reason };
  });
}
export function grantWords(option: GrantOption, names: Map<string, string>, model: GrantModel | null = null, actions: string[] = option.actions): string {
  const where = resourceWords(option.grant.resource, names).words;
  if (model?.action_sentences) return actionWords(model, option.grant.resource, actions) + ' · ' + where;
  const known: Record<string, string> = { view: 'read', edit: 'change', grant: 'give access to' };
  const words = [...actions].sort((left, right) => (left === 'view' ? -1 : right === 'view' ? 1 : left.localeCompare(right))).map((action) => known[action] ?? action).join(' and ');
  return words.charAt(0).toUpperCase() + words.slice(1) + ' ' + where;
}
/** One grant per chosen action, each through its only.<action> relation. `selected` names grants whose every offered action is chosen ("everything here", never editor), or maps a grant to the actions chosen on it. */
export function newAgentGrants(options: GrantOption[], selected: string[] | Record<string, string[]>): AgentGrant[] {
  const now = Math.floor(Date.now() / 1000);
  const picks: [string, string[] | null][] = Array.isArray(selected) ? selected.map((id) => [id, null]) : Object.entries(selected);
  return picks.flatMap(([id, actions]) => {
    const option = options.find((entry) => entry.grant.id === id);
    const chosen = actions ?? option?.actions ?? [];
    if (!option || option.reason || !chosen.length || chosen.some((action) => !option.actions.includes(action))
      || option.grant.effective_ends_at !== null && option.grant.effective_ends_at <= now) {
      throw new Refused(0, { refusal: 'GrantChoiceUnavailable', reason: 'The selected access can no longer be passed on. Nothing has been sent.' });
    }
    const span = { starts_at: Math.max(now, option.grant.window.starts_at), ends_at: option.grant.effective_ends_at };
    return [...new Set(chosen)].sort().map((action) => ({ operation: operationId(), source: id, resource: option.grant.resource, relation: ONE + action, actions: [action],
      window: span, body: null, granted: null }));
  });
}
export function agentGrantsOf(value: unknown, agent: string | null, responsible: string | null): AgentGrant[] {
  const bad = (): never => { throw new Refused(0, { refusal: 'PendingAgentGrantsUnreadable', reason: 'The saved access requests cannot be read. Their outcomes must be checked before adding another agent.' }); };
  if (!Array.isArray(value)) return bad();
  const operations = new Set<string>(); const given = new Set<string>(); let unfinished = false;
  return value.map((entry: unknown) => {
    if (!isRecord(entry) || !operation(entry.operation) || !grantId(entry.source) || !resource(entry.resource) || typeof entry.relation !== 'string' || !entry.relation
      || !strings(entry.actions) || !entry.actions.length || !window(entry.window) || !(entry.granted === null || grantId(entry.granted))
      || operations.has(entry.operation) || given.has(entry.source + ' ' + entry.relation) || unfinished && entry.granted !== null) return bad();
    operations.add(entry.operation); given.add(entry.source + ' ' + entry.relation); unfinished ||= entry.granted === null;
    let body: DelegateBody | null = null;
    if (entry.body !== null) {
      const candidate = entry.body;
      if (!isRecord(candidate) || Object.keys(candidate).some((key) => !['operation', 'route', 'source', 'recipient', 'responsible', 'resource', 'relation', 'pass_on', 'window'].includes(key))
        || !agent || !responsible || candidate.operation !== entry.operation || candidate.route !== 'browser' || candidate.source !== entry.source
        || candidate.recipient !== agent || candidate.responsible !== responsible || !resource(candidate.resource) || candidate.resource.kind !== entry.resource.kind || candidate.resource.id !== entry.resource.id
        || candidate.relation !== entry.relation || !isRecord(candidate.pass_on) || candidate.pass_on.kind !== 'use_only' || !window(candidate.window)
        || candidate.window.starts_at !== entry.window.starts_at || candidate.window.ends_at !== entry.window.ends_at) return bad();
      body = { operation: entry.operation, route: 'browser', source: entry.source, recipient: agent, responsible, resource: entry.resource, relation: entry.relation, pass_on: { kind: 'use_only' }, window: entry.window };
    } else if (entry.granted !== null) return bad();
    return { operation: entry.operation, source: entry.source, resource: entry.resource, relation: entry.relation, actions: entry.actions, window: entry.window, body, granted: entry.granted };
  });
}
export async function delegateAgentGrant(entry: AgentGrant, caller: string): Promise<string> {
  const body = entry.body;
  const mismatch = (): never => { throw new Refused(200, { refusal: 'GrantReceiptMismatch', reason: 'Lys did not confirm this exact access request. Its original details are saved; no completion is claimed.' }); };
  if (!body) return mismatch();
  const answer = await request<unknown>('/grants', body);
  if (!isRecord(answer) || answer.operation !== body.operation || !grantId(answer.grant) || !isRecord(answer.receipt) || answer.receipt.caller !== caller) return mismatch();
  const grant = await request<unknown>('/grants/' + encodeURIComponent(answer.grant));
  if (!readableGrant(grant) || grant.id !== answer.grant || grant.operation !== body.operation || grant.issuer !== caller || grant.holder !== body.recipient
    || grant.responsible !== body.responsible || grant.source !== body.source || grant.resource.kind !== body.resource.kind || grant.resource.id !== body.resource.id
    || grant.relation !== body.relation || grant.pass_on.kind !== 'use_only' || grant.window.starts_at !== body.window.starts_at || grant.window.ends_at !== body.window.ends_at
    || grant.actions.length !== entry.actions.length || !entry.actions.every((action) => grant.actions.includes(action))) return mismatch();
  if (!grant.standing.stands) throw new Refused(200, { refusal: 'GrantNoLongerStands', reason: 'The selected access was recorded but no longer stands. Its original request is kept; the agent has not been started.' });
  return grant.id;
}
