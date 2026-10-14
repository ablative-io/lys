import type { DashboardAgent } from '../dashboard/contract';
import { refusedPart } from '../dashboard/contract';
import { clockMs } from '../file/time';
import { amount } from '../usage/budgetWords';
import type { Length, Used } from '../usage/contract';

export interface TokenComponent {
  reported: number | null;
  missing_calls: number;
}

export interface TokenComponents {
  input_tokens: TokenComponent;
  output_tokens: TokenComponent;
  cache_creation_tokens: TokenComponent;
  cache_read_tokens: TokenComponent;
  calls: number;
  sessions: number;
  calls_without_session: number;
  refused_calls?: number;
}

export type TokensUsed = Used & { tokens?: TokenComponents };
export const TOKEN_LABELS = { input_tokens: 'New input', output_tokens: 'Output', cache_creation_tokens: 'Cache written', cache_read_tokens: 'Cache read' } as const;
export type TokenKey = keyof typeof TOKEN_LABELS;
export const TOKEN_KEYS = Object.keys(TOKEN_LABELS) as TokenKey[];

export interface TokenPresentation {
  agent: string;
  name: string;
  period: Length;
  since_ms: number | null;
  components: { key: TokenKey; label: string; reported: number | null; missing_calls: number; words: string }[];
  counts: string;
  unavailable: string | null;
  empty: string | null;
}

const count = (value: unknown): value is number => Number.isSafeInteger(value) && (value as number) >= 0;
const namedCount = (value: number, one: string): string => value + ' ' + one + (value === 1 ? '' : 's');

export function componentWords(component: TokenComponent): string {
  return (component.reported === null ? 'Not reported' : amount('tokens', component.reported) + ' reported')
    + (component.missing_calls ? ' · ' + namedCount(component.missing_calls, 'call') + ' reported none' : '');
}

function componentsGap(tokens: TokenComponents): string | null {
  for (const key of TOKEN_KEYS) {
    const component = tokens[key];
    if (!component || typeof component !== 'object') return TOKEN_LABELS[key] + ' was not reported as a component';
    if (component.reported !== null && !count(component.reported)) return TOKEN_LABELS[key] + ' has an invalid reported token count';
    if (!count(component.missing_calls)) return TOKEN_LABELS[key] + ' has an invalid missing-call count';
    if (count(tokens.calls) && component.missing_calls > tokens.calls) return TOKEN_LABELS[key] + ' names more missing calls than recorded calls';
  }
  if (!count(tokens.calls)) return 'the recorded call count is invalid';
  if (tokens.refused_calls !== undefined && (!count(tokens.refused_calls) || tokens.refused_calls > tokens.calls)) return 'the refused call count is invalid';
  if (!count(tokens.sessions)) return 'the recorded session count is invalid';
  if (!count(tokens.calls_without_session) || tokens.calls_without_session > tokens.calls) return 'the count of calls without a session is invalid';
  if (tokens.sessions > tokens.calls - tokens.calls_without_session) return 'the recorded session count exceeds calls with a session';
  return null;
}

/** Components remain per agent because cache reads and new input describe different provider figures. */
export function tokenPresentations(rows: DashboardAgent[], period: Length): TokenPresentation[] {
  return rows.map((row) => {
    const base: TokenPresentation = { agent: row.agent.id, name: row.agent.display_name, period, since_ms: null,
      components: [], counts: 'Call and session counts not reported', unavailable: null, empty: null };
    if (refusedPart(row.usage)) return { ...base, unavailable: row.usage.refusal + ': ' + row.usage.reason };
    const figures = row.usage.figures;
    if (figures !== undefined && (!Array.isArray(figures) || figures.some((used) => !used || typeof used !== 'object'))) {
      return { ...base, unavailable: 'TokenComponentsUnreadable: the reported figures are not a list of usage records' };
    }
    const readings = (row.usage.figures ?? []).filter((used) => used.unit === 'tokens' && used.period === period) as TokensUsed[];
    if (!readings.length) return { ...base, unavailable: 'Token components not reported for this period' };
    if (readings.length !== 1) return { ...base, unavailable: 'TokenComponentsUnreadable: more than one component report was returned for the period' };
    const used = readings[0];
    if (!used.tokens || typeof used.tokens !== 'object') return { ...base, unavailable: 'Token components not reported; the combined figure does not identify the components' };
    const gap = componentsGap(used.tokens);
    if (gap) return { ...base, unavailable: 'TokenComponentsUnreadable: ' + gap };
    if (used.since_ms !== null && (!Number.isSafeInteger(used.since_ms) || !Number.isFinite(new Date(used.since_ms).getTime())
      || (period === 'five_hour' && used.since_ms % 18_000_000 !== 0))) {
      return { ...base, unavailable: 'TokenComponentsUnreadable: the reported period start is invalid' };
    }
    const tokens = used.tokens;
    const counts = namedCount(tokens.calls, 'call') + ' · ' + namedCount(tokens.sessions, 'session')
      + (tokens.calls_without_session ? ' · ' + namedCount(tokens.calls_without_session, 'call') + ' named no session' : '');
    const empty = TOKEN_KEYS.every((key) => tokens[key].reported === null)
      ? 'nothing reported yet' + (tokens.refused_calls ? ' · ' + namedCount(tokens.refused_calls, 'call') + ' refused by the provider'
        : tokens.calls ? ' · ' + namedCount(tokens.calls, 'call') + ' reported no token figures' : '') : null;
    return { ...base, since_ms: used.since_ms, counts, empty,
      components: TOKEN_KEYS.map((key) => ({ key, label: TOKEN_LABELS[key], ...tokens[key], words: componentWords(tokens[key]) })) };
  });
}

export function tokenSummary(rows: DashboardAgent[], period: Length): string {
  const readings = tokenPresentations(rows, period);
  if (readings.length !== 1) return readings.length ? 'Token components are per agent' : 'No agent is counted here';
  const reading = readings[0];
  if (reading.unavailable) return reading.unavailable;
  const start = reading.since_ms === null ? 'period start not reported' : 'since ' + clockMs(reading.since_ms);
  return reading.components.map((each) => each.label + ': ' + each.words).join(' · ') + ' · ' + reading.counts + ' · ' + start;
}
