import { describe, expect, it } from 'vitest';
import type { DashboardAgent } from '../dashboard/contract';
import { clockMs } from '../file/time';
import { fiveHourPresentation } from './widgetReadings';
import { tokenPresentations, tokenSummary } from './tokenReadings';
import type { TokenComponents, TokensUsed } from './tokenReadings';

const at = Math.floor(1_792_000_000_000 / 18_000_000) * 18_000_000;
const row = (name: string, tokens: TokenComponents | undefined, period: 'day' | 'five_hour' = 'day', since = at): DashboardAgent => ({
  agent: { id: name, display_name: name, state: 'active' }, teams: [], sessions: [], goals: { goals: [] },
  budget: { refusal: 'NotRead', reason: 'The budget is not read in this test.' },
  usage: { agent: name, used: [], receipts: [], last_reported_ms: at, figures: [{ unit: 'tokens', period,
    since_ms: since, figure: 177_540, unavailable: null, ...(tokens ? { tokens } : {}) } as TokensUsed] },
} as DashboardAgent);
const components = (reported = 10): TokenComponents => ({
  input_tokens: { reported, missing_calls: 0 }, output_tokens: { reported: 20, missing_calls: 0 },
  cache_creation_tokens: { reported: 30, missing_calls: 0 }, cache_read_tokens: { reported: 40, missing_calls: 0 },
  calls: 2, sessions: 1, calls_without_session: 0,
});

describe('Per-agent token component reports', () => {
  it('shows new input, output, cache written and cache read because a combined scalar cannot identify those different token classes', () => {
    const readings = tokenPresentations([row('Agent', components())], 'day');
    expect(readings[0].components.map((each) => [each.label, each.words])).toEqual([
      ['New input', '10 tokens reported'], ['Output', '20 tokens reported'],
      ['Cache written', '30 tokens reported'], ['Cache read', '40 tokens reported'],
    ]);
    expect(readings[0].counts).toBe('2 calls · 1 session');
    expect(tokenSummary([row('Agent', components())], 'day')).not.toContain('177,540');
  });

  it('keeps component subtotals and missing calls independently rather than hiding the reported part', () => {
    const tokens = { ...components(), input_tokens: { reported: 100, missing_calls: 2 }, cache_read_tokens: { reported: null, missing_calls: 2 } };
    const reading = tokenPresentations([row('Agent', tokens)], 'day')[0];
    expect(reading.components.find((each) => each.key === 'input_tokens')?.words).toBe('100 tokens reported · 2 calls gave no figure');
    expect(reading.components.find((each) => each.key === 'cache_read_tokens')?.words).toBe('Not reported · 2 calls gave no figure');
    expect(reading.components.find((each) => each.key === 'output_tokens')?.words).toBe('20 tokens reported');
  });

  it('distinguishes an explicitly reported zero from a component no call reported', () => {
    const tokens = { ...components(0), output_tokens: { reported: null, missing_calls: 2 } };
    const reading = tokenPresentations([row('Agent', tokens)], 'day')[0];
    expect(reading.components[0].words).toBe('0 tokens reported');
    expect(reading.components[1].words).toBe('Not reported · 2 calls gave no figure');
  });

  it('keeps agents and their counts separate rather than adding unrelated session usage', () => {
    const readings = tokenPresentations([row('One', components(10)), row('Other', components(90))], 'day');
    expect(readings.map((each) => [each.agent, each.components[0].reported, each.counts])).toEqual([
      ['One', 10, '2 calls · 1 session'], ['Other', 90, '2 calls · 1 session'],
    ]);
    expect(tokenSummary([row('One', components()), row('Other', components())], 'day')).toBe('Token components are per agent');
  });

  it('names missing component metadata instead of falling back to the combined token figure or guessed call counts', () => {
    const reading = tokenPresentations([row('Agent', undefined)], 'day')[0];
    expect(reading.components).toEqual([]);
    expect(reading.counts).toBe('Call and session counts not reported');
    expect(reading.unavailable).toBe('Token components not reported; the combined figure does not identify the components');
    expect(tokenSummary([row('Agent', undefined)], 'day')).not.toContain('177,540');
  });

  it('names an invalid reported component or missing-call count rather than turning it into zero', () => {
    const negative = tokenPresentations([row('Agent', components(-1))], 'day')[0];
    expect(negative.unavailable).toBe('Token figures could not be used: New input has an invalid reported token count');
    expect(negative.components).toEqual([]);
    const missing = { ...components(), input_tokens: { reported: 1, missing_calls: -1 } };
    expect(tokenPresentations([row('Agent', missing)], 'day')[0].unavailable)
      .toBe('Token figures could not be used: New input has an invalid missing-call count');
  });

  it('names contradictory call and session counts and separately names calls with no session', () => {
    const tokens = { ...components(), calls: 3, sessions: 1, calls_without_session: 1 };
    expect(tokenPresentations([row('Agent', tokens)], 'day')[0].counts).toBe('3 calls · 1 session · 1 call named no session');
    expect(tokenPresentations([row('Agent', { ...tokens, sessions: 3 })], 'day')[0].unavailable)
      .toBe('Token figures could not be used: the recorded session count exceeds calls with a session');
  });
});

describe('Five-hour component reports', () => {
  it('names the epoch-aligned interval start without claiming a provider quota or a rolling five-hour window', () => {
    const reading = fiveHourPresentation([row('Agent', components(), 'five_hour')])[0];
    expect(reading.since_ms).toBe(at);
    expect(reading.components).toHaveLength(4);
    expect(tokenSummary([row('Agent', components(), 'five_hour')], 'five_hour')).toContain('since ' + clockMs(at));
    expect(tokenSummary([row('Agent', components(), 'five_hour')], 'five_hour')).not.toContain('%');
  });

  it('retains each agent interval separately because component reports are never combined across agents', () => {
    const readings = fiveHourPresentation([row('One', components(), 'five_hour'), row('Other', components(90), 'five_hour', at + 18_000_000)]);
    expect(readings.map((each) => [each.agent, each.since_ms])).toEqual([['One', at], ['Other', at + 18_000_000]]);
  });

  it('names a missing period start and refuses an invalid epoch boundary without substituting a date', () => {
    const missing = row('Agent', components(), 'five_hour');
    if (!('refusal' in missing.usage)) missing.usage.figures![0].since_ms = null;
    expect(tokenSummary([missing], 'five_hour')).toContain('period start not reported');
    expect(fiveHourPresentation([row('Agent', components(), 'five_hour', at + 1)])[0].unavailable)
      .toBe('Token figures could not be used: the reported period start is invalid');
  });
});
