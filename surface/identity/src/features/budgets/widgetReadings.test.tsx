import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';
import type { DashboardAgent } from '../dashboard/contract';
import { clockMs } from '../file/time';
import type { Used } from '../usage/contract';
import { ContextReadingNote, contextPresentation } from './widgetReadings';
import type { ContextUsed, SessionContext } from './widgetReadings';

const at = 1_792_000_000_000;
const model = 'claude-opus-5-5';
const session = 'current-session';
const row = (name: string, figures: Used[]): DashboardAgent => ({
  agent: { id: name, display_name: name, state: 'active' }, teams: [], sessions: [],
  usage: { agent: name, used: [], receipts: [], last_reported_ms: at + 600_000, figures },
  budget: { holder: { kind: 'agent', id: name }, limits: [], warn_at: null, zone: 'UTC', version: 0,
    by: null, at: null, used: [], unavailable: [], within: [], unconfirmed: [] }, goals: { goals: [] },
} as DashboardAgent);
const context = (figure: number | null, changes: Partial<ContextUsed> = {}): ContextUsed => ({
  unit: 'context_percent', period: null, since_ms: null,
  ...(figure === null ? { figure, unavailable: 'context_percent was not reported' } : { figure, unavailable: null }),
  source: 'model', context: { session, read_at_ms: figure === null ? null : at, reported_none: 2, model,
    completed_call: figure !== null,
    context_tokens: figure === null ? null : 58_795, window_tokens: 1_000_000, live_sessions: [] }, ...changes,
} as ContextUsed);
const older: SessionContext = { session: 'older-session', read_at_ms: at - 600_000, reported_none: 8, model,
  context_tokens: 980_000, window_tokens: 1_000_000, figure: 98, unavailable: null };
const current: SessionContext = { session, read_at_ms: at, reported_none: 2, model,
  context_tokens: 58_795, window_tokens: 1_000_000, figure: 5, unavailable: null };
const rendered = (reading: ReturnType<typeof contextPresentation>): HTMLElement => {
  const host = document.createElement('div');
  host.innerHTML = reading ? renderToStaticMarkup(<ContextReadingNote reading={reading} />) : '';
  return host;
};

describe('Current live-session context readings', () => {
  it('shows 58795 over the declared million as 5.9 percent with its reading time, because context is the latest session reading', () => {
    const reading = contextPresentation([row('Agent', [context(5)])]);
    expect(reading).toEqual({ figure: 5.8795, display: '5.9%', session, model, context_tokens: 58_795,
      window_tokens: 1_000_000, live_sessions: [], read_at_ms: at, reported_none: 2,
      words: 'current session · read ' + clockMs(at) + ' · 2 calls gave no context figure', unavailable: [] });
    const host = rendered(reading);
    expect(host.textContent).toBe(reading?.words);
    expect(host.querySelector('time')?.getAttribute('datetime')).toBe(new Date(at).toISOString());
    expect(host.textContent).not.toContain(clockMs(at + 600_000));
  });

  it('keeps the service-selected current session instead of taking an older live session with a higher percentage', () => {
    const used = context(5, { context: { ...current, live_sessions: [older, current] } });
    const reading = contextPresentation([row('Agent', [used])]);
    expect(reading).toMatchObject({ session, figure: 5.8795, read_at_ms: at, reported_none: 2 });
    expect(reading?.display).not.toBe('98%');
    expect(reading?.words).not.toContain('10 calls');
  });

  it('uses only the explicitly picked live session, including its own reading time and later empty calls', () => {
    const reading = contextPresentation([row('Agent', [context(5, { context: { ...current, live_sessions: [older, current] } })])], older.session);
    expect(reading).toMatchObject({ session: older.session, figure: 98, display: '98%', read_at_ms: older.read_at_ms,
      reported_none: 8, words: 'chosen session · read ' + clockMs(at - 600_000) + ' · 8 calls gave no context figure' });
  });

  it('names a picked session that ended instead of silently switching to another session', () => {
    const reading = contextPresentation([row('Agent', [context(5, { context: { ...current, live_sessions: [current] } })])], older.session);
    expect(reading).toMatchObject({ session: older.session, figure: null, display: 'Context unreadable',
      words: 'The context reading could not be used: the selected session is no longer live' });
  });

  it('keeps context per session across agents instead of adding percentages or taking their maximum', () => {
    const reading = contextPresentation([row('One', [context(5)]), row('Other', [context(98)])]);
    expect(reading).toMatchObject({ figure: null, reported_none: null, display: 'Context is per session', words: 'Choose an agent session' });
  });

  it('names an undeclared model window because a missing declaration cannot supply a percentage', () => {
    const used = context(null, { unavailable: 'no context window is declared for model ' + model,
      context: { ...current, window_tokens: null, live_sessions: [] } });
    const reading = contextPresentation([row('Agent', [used])]);
    expect(reading).toMatchObject({ figure: null, display: model + ' window not declared', window_tokens: null, read_at_ms: at });
    expect(reading?.unavailable).toEqual(['Agent: no context window is declared for model ' + model]);
    expect(reading?.display).not.toContain('%');
  });

  it("shows the service's own figure, never a guessed denominator, and names the missing declaration when an old record has tokens but no window", () => {
    const reading = contextPresentation([row('Agent', [context(5, { context: { ...current, window_tokens: null } })])]);
    expect(reading).toMatchObject({ figure: 5, display: '5%', context_tokens: 58_795, window_tokens: null });
    expect(reading?.words).toContain('declared context window not reported');
    expect(rendered(reading).textContent).toBe(reading?.words);
  });

  it('says No messages yet because the selected session has not completed its first context reading', () => {
    expect(contextPresentation([row('Agent', [context(null)])])).toMatchObject({ session, figure: null,
      display: 'No messages yet', read_at_ms: null, words: 'current session' });
  });

  it('retains side-work and oversized-context reasons rather than calling their models undeclared', () => {
    for (const unavailable of [
      "the call's model is declared as side work, which never sets an agent's context",
      "the call's context of 120 tokens is larger than the window of 100 declared for its model",
    ]) {
      const reading = contextPresentation([row('Agent', [context(null, { unavailable })])]);
      expect(reading?.display).toBe(unavailable.includes('side work') ? 'Model declared as side work' : 'Context percentage unavailable');
      expect(reading?.unavailable).toEqual(['Agent: ' + unavailable]);
      expect(reading?.display).not.toContain('window not declared');
    }
  });

  it('names a missing read time rather than using the latest usage-call timestamp', () => {
    const reading = contextPresentation([row('Agent', [context(5, { context: { ...current, read_at_ms: null, reported_none: 0 } })])]);
    expect(reading?.words).toBe('current session · read time not reported · 0 calls gave no context figure');
    expect(rendered(reading).querySelector('time')).toBeNull();
  });

  it("shows an older scalar-only answer's own figure and names absent provenance without inventing a session, denominator, reading time or empty-call count", () => {
    const scalar: Used = { unit: 'context_percent', period: null, since_ms: null, figure: 5, unavailable: null };
    const reading = contextPresentation([row('Agent', [scalar])]);
    expect(reading).toMatchObject({ figure: 5, display: '5%', read_at_ms: null, reported_none: null, context_tokens: null, window_tokens: null,
      words: 'current session not reported · read time not reported · how many calls gave no context figure is not reported · context tokens not reported · declared context window not reported' });
    expect(rendered(reading).textContent).toBe(reading?.words);
    expect(contextPresentation([])).toBeNull();
  });

  it('uses an undeclared model named by the refusal when older answers carry no metadata', () => {
    const reading = contextPresentation([row('Agent', [{ unit: 'context_percent', period: null,
      since_ms: null, figure: null, unavailable: 'no context window is declared for model ' + model }])]);
    expect(reading).toMatchObject({ figure: null, display: model + ' window not declared', reported_none: null });
    expect(reading?.display).not.toContain('%');
  });

  it('names invalid provenance without substituting zero for a missing call count', () => {
    const reading = contextPresentation([row('Agent', [context(5, { context: { ...current, reported_none: -1 } })])]);
    expect(reading).toMatchObject({ figure: null, display: 'Context unreadable', reported_none: null,
      words: 'The context reading could not be used: the count of calls that reported no context is invalid' });
    expect(rendered(reading).textContent).not.toContain('0 calls');
  });

  it('names a reading beyond its declared window and percentages beyond 100 instead of rendering valid context', () => {
    expect(contextPresentation([row('Agent', [context(101)])])).toMatchObject({ figure: null,
      words: 'The context reading could not be used: the context percentage is invalid' });
    expect(contextPresentation([row('Agent', [context(5, { context: { ...current, window_tokens: 100 } })])]))
      .toMatchObject({ figure: null, words: 'The context reading could not be used: the reported context is larger than the declared window' });
  });

  it('names actual side-work raw input without incorrectly calling its declared side-work model undeclared', () => {
    const reading = contextPresentation([row('Agent', [context(null, {
      unavailable: "the call's model is declared as side work, which never sets an agent's context",
      context: { ...current, window_tokens: null },
    })])]);
    expect(reading).toMatchObject({ figure: null, display: 'Model declared as side work', context_tokens: 58_795,
      read_at_ms: at, model, window_tokens: null });
    expect(reading?.words).not.toContain('declared context window not reported');
  });

  it('preserves a legitimately unavailable oversized reading time, model, raw counts and picker rather than rejecting its provenance', () => {
    const unavailable = "the call's context of 58795 tokens is larger than the window of 100 declared for its model";
    const reading = contextPresentation([row('Agent', [context(null, { unavailable,
      context: { ...current, window_tokens: 100, live_sessions: [{ ...current, window_tokens: 100, figure: null, unavailable }] },
    })])]);
    expect(reading).toMatchObject({ figure: null, display: 'Context percentage unavailable', read_at_ms: at,
      model, session, context_tokens: 58_795, window_tokens: 100, unavailable: ['Agent: ' + unavailable] });
    expect(reading?.live_sessions).toHaveLength(1);
  });
});
