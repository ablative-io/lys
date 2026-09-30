/** An agent's budget as a list of limits, the first reached acting, with the words the page says about it. */

export type Unit = 'dollars' | 'plan_percent' | 'tokens' | 'running_ms' | 'context_percent';
export type LimitPeriod = 'five_hour' | 'day' | 'week' | 'month';
export type Limit = { unit: Unit; amount: number; period: LimitPeriod | null };
export type LimitAct = 'pause' | 'stop' | 'tell' | 'compact' | 'notice';
export type Unavailable = { unit: Unit; reason: string };
export type LimitsBudget = { limits: Limit[]; warn_at: number | null; act: LimitAct; zone: string; version: number; by: string; at: number };
export type LimitsView = LimitsBudget & { unavailable: Unavailable[] };
export type Used = { unit: Unit; period: LimitPeriod | null; figure: number; since_ms: number };

export const UNITS: Record<Unit, string> = { dollars: 'dollars', plan_percent: '% of my plan', tokens: 'tokens', running_ms: 'minutes running', context_percent: '% of its context' };

/** The periods each unit is counted over: a plan's share is counted over the plan's own windows. */
export const PERIODS: Record<Unit, LimitPeriod[]> = {
  dollars: ['day', 'week', 'month'],
  tokens: ['day', 'week', 'month'],
  running_ms: ['day', 'week', 'month'],
  plan_percent: ['five_hour', 'week'],
  context_percent: [],
};

export const PERIOD_WORDS: Record<LimitPeriod, string> = { five_hour: '5-hour window', day: 'day', week: 'week', month: 'month' };

export const ACT_WORDS: Record<LimitAct, string> = { pause: 'Pause this agent', stop: 'Stop this agent', tell: 'Just tell me', compact: 'Compact its session', notice: 'Type a notice into its session' };

const THIS: Record<LimitPeriod, string> = { five_hour: 'this 5-hour window', day: 'today', week: 'this week', month: 'this month' };

const ADJECTIVE: Record<LimitPeriod, string> = { five_hour: '5-hour', day: 'daily', week: 'weekly', month: 'monthly' };

/** An amount as a person reads it: $500, 50%, 2,000,000 tokens. */
export function amount(unit: Unit, value: number): string {
  const figure = value.toLocaleString(undefined, { maximumFractionDigits: unit === 'dollars' ? 2 : 0 });
  if (unit === 'dollars') return '$' + figure;
  if (unit === 'plan_percent' || unit === 'context_percent') return figure + '%';
  if (unit === 'running_ms') return (value / 60000).toLocaleString(undefined, { maximumFractionDigits: 0 }) + ' minutes running';
  return figure + ' tokens';
}

/** One limit in words: "$500 a week", "50% of the weekly plan". */
export function limitWords(limit: Limit): string {
  if (limit.unit === 'context_percent' || limit.period === null) return amount(limit.unit, limit.amount) + ' of its context';
  if (limit.unit === 'plan_percent') return amount(limit.unit, limit.amount) + ' of the ' + ADJECTIVE[limit.period] + ' plan';
  return amount(limit.unit, limit.amount) + ' a ' + (limit.period === 'five_hour' ? '5-hour window' : limit.period);
}

/** The whole budget in one sentence, as the page says it under the limits. */
export function summary(limits: Limit[], act: LimitAct, warnAt: number | null): string {
  if (!limits.length) return 'No limit: this agent can spend without stopping.';
  const verb = { pause: 'Pauses', stop: 'Stops', tell: 'Tells you', compact: 'Compacts its session', notice: 'Types a notice into its session' }[act];
  const joined = limits.map(limitWords).join(' or ');
  const either = limits.length > 1 ? ', whichever comes first' : '';
  const warn = warnAt !== null && act !== 'tell' ? '; tells you at ' + warnAt + '%' : '';
  return verb + ' at ' + joined + either + warn + '.';
}

/** What is used against a limit, beside it: "$212 of $500 this week". */
export function usedWords(limit: Limit, used: Used[]): string | null {
  const found = used.find((each) => each.unit === limit.unit && each.period === limit.period);
  if (!found) return null;
  return amount(limit.unit, found.figure) + ' of ' + amount(limit.unit, limit.amount) + (limit.period ? ' ' + THIS[limit.period] : ' now');
}

export type Goal = { id: string; words: string; deadline: number | null; active: boolean };
export type GoalList = { goals: Goal[] };

/** What happened at a crossing, as a person would say it: "Hit 40,000 tokens at 3:12 pm. Lys asked the session to compact, and hasn't heard back yet." */
export function eventWords(unit: Unit, figure: number, atMs: number, act: LimitAct, stands: string | null): string {
  const when = new Date(atMs).toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' });
  const asked = { pause: 'pause this agent', stop: 'stop this agent', tell: 'tell the responsible person', compact: 'compact the session', notice: 'type a notice into the session' }[act];
  const outcome = stands === 'confirmed' || stands === 'told' ? 'Done.'
    : stands === 'refused' ? 'The session refused it.'
    : stands === 'uncertain' ? 'Lys cannot tell whether it happened, and will not ask again.'
    : "Lys hasn't heard back yet.";
  return 'Hit ' + amount(unit, figure) + ' at ' + when + '. Lys asked to ' + asked + '. ' + outcome;
}
