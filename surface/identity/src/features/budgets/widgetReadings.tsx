import type { DashboardAgent } from '../dashboard/contract';
import { refusedPart } from '../dashboard/contract';
import { clockMs } from '../file/time';
import type { Used } from '../usage/contract';
import { tokenPresentations } from './tokenReadings';

/** Provenance accompanies the retained figure; a later empty call is not a new reading. */
export interface ContextReading {
  session?: string | null;
  read_at_ms: number | null;
  reported_none: number;
  completed_call?: boolean | null;
  model: string | null;
  context_tokens?: number | null;
  window_tokens?: number | null;
  live_sessions?: SessionContext[];
}

export interface SessionContext {
  session: string;
  read_at_ms: number | null;
  reported_none: number;
  completed_call?: boolean | null;
  model: string | null;
  context_tokens: number | null;
  window_tokens: number | null;
  figure: number | null;
  unavailable: string | null;
}

export type ContextUsed = Used & { source?: string; context?: ContextReading };

export interface ContextPresentation {
  figure: number | null;
  display: string;
  read_at_ms: number | null;
  reported_none: number | null;
  words: string;
  unavailable: string[];
  session: string | null;
  model: string | null;
  context_tokens: number | null;
  window_tokens: number | null;
  live_sessions: SessionContext[];
}

const calls = (count: number): string => count + (count === 1 ? ' call' : ' calls') + ' gave no context figure';
const instant = (at: unknown): at is number => typeof at === 'number' && Number.isSafeInteger(at)
  && Number.isFinite(new Date(at).getTime());
const count = (value: unknown): value is number => Number.isSafeInteger(value) && (value as number) >= 0;

function metadataGap(reading: ContextReading): string | null {
  if (reading.read_at_ms !== null && !instant(reading.read_at_ms)) return 'the context read time is invalid';
  if (!count(reading.reported_none)) return 'the count of calls that reported no context is invalid';
  if (reading.model !== null && (typeof reading.model !== 'string' || !reading.model.length)) return 'the context model is invalid';
  if (reading.session !== undefined && reading.session !== null && (typeof reading.session !== 'string' || !reading.session.length)) return 'the context session is invalid';
  if (reading.context_tokens !== undefined && reading.context_tokens !== null && !count(reading.context_tokens)) return 'the reported context token count is invalid';
  if (reading.window_tokens !== undefined && reading.window_tokens !== null && (!count(reading.window_tokens) || reading.window_tokens === 0)) return 'the declared context window is invalid';
  return null;
}

function unreadable(reason: string): ContextPresentation {
  return { figure: null, display: 'Context unreadable', read_at_ms: null, reported_none: null,
    words: 'The context reading could not be used: ' + reason, unavailable: [reason], session: null, model: null,
    context_tokens: null, window_tokens: null, live_sessions: [] };
}

function missingDisplay(used: ContextUsed): string {
  const model = used.context?.model;
  const prefix = 'no context window is declared for model ';
  if (model && used.unavailable === prefix + model) return model + ' window not declared';
  if (!model && used.unavailable?.startsWith(prefix)) {
    const named = used.unavailable.slice(prefix.length);
    if (named && named !== '(not named)') return named + ' window not declared';
  }
  if (used.unavailable === "the call's model is declared as side work, which never sets an agent's context") return 'Model declared as side work';
  if (used.unavailable === "the call's profile context window declaration was not retained") return 'Profile context window not retained';
  if (used.unavailable && used.unavailable !== 'context_percent was not reported') return 'Context percentage unavailable';
  return 'Not reported';
}

/** The service names the current live session; choosing another live session never compares their percentages. */
export function contextPresentation(rows: DashboardAgent[], selectedSession?: string): ContextPresentation | null {
  if (rows.length > 1) return { ...unreadable('context is reported per session'), display: 'Context is per session',
    words: 'Choose an agent session', unavailable: [] };
  for (const row of rows) if (!refusedPart(row.usage)) {
    const figures = row.usage.figures;
    if (figures !== undefined && (!Array.isArray(figures) || figures.some((used) => !used || typeof used !== 'object'))) return unreadable('the reported figures are not a list of usage records');
  }
  const readings = rows.flatMap((row) => refusedPart(row.usage) ? [] : (row.usage.figures ?? [])
    .filter((used) => used.unit === 'context_percent' && used.period === null)
    .map((used) => ({ used: used as ContextUsed, agent: row.agent.display_name })));
  if (!readings.length) return null;
  if (readings.length !== 1) return unreadable('more than one current context reading was reported for the agent');
  let used = readings[0].used;
  const metadata = used.context;
  if (metadata !== undefined && (!metadata || typeof metadata !== 'object')) return unreadable('the context reading is invalid');
  if (metadata) {
    const gap = metadataGap(metadata);
    if (gap) return unreadable(gap);
  }
  const live = metadata?.live_sessions ?? [];
  if (!Array.isArray(live)) return unreadable('the live context sessions are not a list');
  const names = new Set<string>();
  for (const reading of live) {
    if (!reading || typeof reading.session !== 'string' || !reading.session.length || names.has(reading.session)) return unreadable('a live context session is missing or repeated');
    names.add(reading.session);
    const gap = metadataGap(reading);
    if (gap) return unreadable(gap);
  }
  if (selectedSession === undefined && live.length && metadata?.session !== null && metadata?.session !== undefined && !names.has(metadata.session)) return unreadable('the reported current session is not live');
  if (selectedSession !== undefined) {
    const selected = live.find((reading) => reading.session === selectedSession);
    if (!selected) return { ...unreadable('the selected session is no longer live'), session: selectedSession, live_sessions: live };
    used = { ...used, figure: selected.figure, unavailable: selected.unavailable, context: { ...selected, live_sessions: live } } as ContextUsed;
  }
  const reading = used.context;
  const session = reading?.session ?? null;
  const contextTokens = reading?.context_tokens ?? null;
  const windowTokens = reading?.window_tokens ?? null;
  if (used.figure !== null && (!Number.isFinite(used.figure) || used.figure < 0 || used.figure > 100)) return unreadable('the context percentage is invalid');
  if (used.figure !== null && contextTokens !== null && windowTokens !== null && contextTokens > windowTokens) return unreadable('the reported context is larger than the declared window');
  // The tokens over the declared window when both are reported; an older reading carries neither, and its service figure stands.
  const figure = used.figure === null ? null
    : contextTokens !== null && windowTokens !== null ? (session !== null ? contextTokens * 100 / windowTokens : null)
      : used.figure;
  const readAt = reading?.read_at_ms ?? null;
  const noMessages = session !== null && readAt === null && contextTokens === null && reading?.completed_call === false
    && (used.unavailable === 'context_percent was not reported' || used.unavailable?.startsWith('context counters not reported:'));
  const missing = missingDisplay(used);
  const missingWindow = missing === 'Not reported' && windowTokens === null && (contextTokens !== null || used.figure !== null);
  // A session is named by which it is, never by its identifier: the picker beside the reading tells sessions apart.
  const words = [session === null ? 'current session not reported' : selectedSession !== undefined && session !== metadata?.session ? 'chosen session' : 'current session',
    readAt !== null ? 'read ' + clockMs(readAt) : contextTokens !== null || used.figure !== null ? 'read time not reported' : '',
    noMessages ? '' : reading ? calls(reading.reported_none) : 'how many calls gave no context figure is not reported',
    contextTokens === null && used.figure !== null ? 'context tokens not reported' : '',
    missingWindow ? 'declared context window not reported' : '']
    .filter(Boolean).join(' · ');
  const display = figure !== null ? figure.toLocaleString('en-AU', { maximumFractionDigits: 1 }) + '%'
    : noMessages ? 'No messages yet' : missing !== 'Not reported' ? missing : missingWindow ? 'Declared context window not reported' : 'Not reported';
  return { figure, display, read_at_ms: readAt, reported_none: reading?.reported_none ?? null, words,
    unavailable: used.unavailable && !noMessages ? [readings[0].agent + ': ' + used.unavailable] : [], session,
    model: reading?.model ?? null, context_tokens: contextTokens, window_tokens: windowTokens, live_sessions: live };
}

/** The time is machine-readable as well as visible, so a retained reading is never presented as fresh. */
export function ContextReadingNote({ reading }: { reading: ContextPresentation }) {
  const text = reading.read_at_ms === null ? null : 'read ' + clockMs(reading.read_at_ms);
  const position = text === null ? -1 : reading.words.indexOf(text);
  return <span className="dim" data-context-reading>{position < 0 ? reading.words : <>
    {reading.words.slice(0, position)}<time dateTime={new Date(reading.read_at_ms!).toISOString()}>{text}</time>
    {reading.words.slice(position + text!.length)}</>}</span>;
}

/** An epoch-aligned token interval remains component reports for each agent, never a provider quota. */
export function fiveHourPresentation(rows: DashboardAgent[]): ReturnType<typeof tokenPresentations> {
  return tokenPresentations(rows, 'five_hour');
}
