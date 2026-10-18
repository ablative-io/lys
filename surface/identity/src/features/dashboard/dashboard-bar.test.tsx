/**
 * A limit's bar on the Dashboard and the canvas Budget widget: three bands, the warn colour from where the budget's
 * warning begins.
 */
import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';
import { BudgetBar, LimitBar } from '../dashboard/Widgets';
import type { BudgetsView, Limit, Used } from '../usage/contract';

const meter = (markup: string) => {
  const host = document.createElement('div');
  host.innerHTML = markup;
  return host.querySelector('meter');
};
const budget = (limits: Limit[], figures: (number | null)[], warn: number | null = null): BudgetsView => ({
  holder: { kind: 'agent', id: 'a' }, limits, warn_at: warn, zone: 'UTC', version: 1, by: null, at: null, unavailable: [], within: [], unconfirmed: [],
  used: limits.map((each, index) => (figures[index] === null ? { unit: each.unit, period: each.period, since_ms: null, figure: null, unavailable: 'not reported' }
    : { unit: each.unit, period: each.period, since_ms: null, figure: figures[index], unavailable: null }) as Used),
});
const dollars: Limit = { unit: 'dollars', amount: 500, period: 'week', act: 'stop' };
const context: Limit = { unit: 'context_percent', amount: 40, period: null, act: 'compact' };

describe('A limit’s bar', () => {
  it('is a meter whose low band ends where the warning begins and whose high band ends just under the limit', () => {
    const bar = meter(renderToStaticMarkup(<LimitBar near={0.9} warn={0.8} />));
    expect([bar?.getAttribute('min'), bar?.getAttribute('max'), bar?.getAttribute('low'), bar?.getAttribute('optimum'), bar?.getAttribute('value')]).toEqual(['0', '1', '0.8', '0', '0.9']);
    expect(Number(bar?.getAttribute('high'))).toBeLessThan(1);
    expect(Number(bar?.getAttribute('high'))).toBeGreaterThan(0.99);
    expect(bar?.getAttribute('aria-label')).toBe('90% of the nearest limit, near it');
    expect(bar?.hasAttribute('title')).toBe(false);
  });

  it('says reached at the limit, holds a value past it at the end, and says nothing extra below the warning', () => {
    expect(meter(renderToStaticMarkup(<LimitBar near={1.4} warn={0.8} />))?.getAttribute('aria-label')).toBe('140% of the nearest limit, reached');
    expect(meter(renderToStaticMarkup(<LimitBar near={1.4} warn={0.8} />))?.getAttribute('value')).toBe('1');
    expect(meter(renderToStaticMarkup(<LimitBar near={0.5} warn={0.8} />))?.getAttribute('aria-label')).toBe('50% of the nearest limit');
  });

  it('takes the budget’s own warn_at, and draws nothing for a context limit or a limit with no figure', () => {
    expect(meter(renderToStaticMarkup(<BudgetBar budget={budget([dollars], [400], 60)} />))?.getAttribute('low')).toBe('0.6');
    expect(meter(renderToStaticMarkup(<BudgetBar budget={budget([dollars], [400])} />))?.getAttribute('low')).toBe('0.8');
    expect(renderToStaticMarkup(<BudgetBar budget={budget([context], [95])} />)).toBe('');
    expect(renderToStaticMarkup(<BudgetBar budget={budget([dollars], [null])} />)).toBe('');
    // A context limit beside a spend limit never makes the bar: it is the spend limit's 20%.
    expect(meter(renderToStaticMarkup(<BudgetBar budget={budget([context, dollars], [95, 100])} />))?.getAttribute('value')).toBe('0.2');
  });
});
