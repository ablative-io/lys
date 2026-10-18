/**
 * One formatter for every budget figure: money in US dollars with two decimals, everything in Australian English
 * whatever the browser's locale, and a reached limit said with its unit. Kept here because this folder is the one of
 * domain 06 the test runner includes.
 */
import { describe, expect, it, vi } from 'vitest';
import { amount, limitWords, usedWords } from '../usage/budgetWords';
import { standing } from '../usage/contract';
import type { BudgetsView, Limit, Receipt } from '../usage/contract';

const view = (limit: Limit): BudgetsView => ({ holder: { kind: 'agent', id: 'a' }, limits: [limit], warn_at: null, zone: 'UTC', version: 2, by: null, at: null,
  used: [{ unit: limit.unit, period: limit.period, since_ms: 0, figure: 1, unavailable: null }], unavailable: [], within: [], unconfirmed: [] });
const crossed = (limit: Limit, figure: number): Receipt => ({ acted: null, crossing: { operation: 'o', holder: { kind: 'agent', id: 'a' }, measure: limit.unit, version: 2,
  limit: limit.amount, figure, limit_index: 0, warning: false, act: limit.act, agent: 'a', at_ms: 10 } });

describe('Budget figures in words', () => {
  it('writes money as US$ with two decimals, and every figure in Australian English', () => {
    expect([amount('dollars', 12.5), amount('dollars', 500), amount('tokens', 2_000_000), amount('plan_percent', 50), amount('running_ms', 5_400_000)])
      .toEqual(['US$12.50', 'US$500.00', '2,000,000 tokens', '50%', '90 minutes running']);
    expect(limitWords({ unit: 'dollars', amount: 500, period: 'week' })).toBe('US$500.00 a week');
    expect(usedWords({ unit: 'dollars', amount: 500, period: 'week', act: 'tell' }, { unit: 'dollars', period: 'week', since_ms: 0, figure: 212, unavailable: null }))
      .toBe('US$212.00 of US$500.00 this week');
  });

  it('ignores a browser that writes numbers another way', () => {
    const native = Number.prototype.toLocaleString;
    const spy = vi.spyOn(Number.prototype, 'toLocaleString').mockImplementation(function (this: number, locales?: Intl.LocalesArgument, options?: Intl.NumberFormatOptions) {
      return native.call(this, locales ?? 'de-DE', options);
    });
    expect(amount('tokens', 1234.5)).toBe('1,235 tokens');
    expect(amount('dollars', 1234.5)).toBe('US$1,234.50');
    expect(spy).toHaveBeenCalled();
  });

  it('says a reached limit with its unit, never a bare number', () => {
    const dollars: Limit = { unit: 'dollars', amount: 500, period: 'week', act: 'stop' };
    const minutes: Limit = { unit: 'running_ms', amount: 3_600_000, period: 'day', act: 'tell' };
    expect(standing(view(dollars), 0, [crossed(dollars, 512.34)]).words).toBe('Reached at US$512.34. Nothing is known yet of its act (end the session).');
    expect(standing(view(minutes), 0, [crossed(minutes, 3_660_000)]).words).toBe('Reached at 61 minutes running. Nothing is known yet of its act (tell the responsible person).');
  });
});
