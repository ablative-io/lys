/**
 * The Dashboard's "Needs a look": only what is wrong, in sentences, and nothing when nothing is. Kept here because this
 * folder is the one of domain 06 the test runner includes.
 */
import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';
import type { DashboardAgent } from '../dashboard/contract';
import { Exceptions, exceptionsOf } from '../dashboard/Exceptions';
import type { Limit, Used } from '../usage/contract';

const limit = (unit: Limit['unit'], amount: number, period: Limit['period']): Limit => ({ unit, amount, period, act: 'tell' });
const used = (unit: Used['unit'], period: Used['period'], figure: number | null): Used =>
  (figure === null ? { unit, period, since_ms: null, figure: null, unavailable: 'not reported' } : { unit, period, since_ms: null, figure, unavailable: null }) as Used;
const row = (name: string, limits: Limit[], figures: (number | null)[], warn: number | null = null, missed: string[] = []): DashboardAgent => ({
  agent: { id: 'agent-' + name, display_name: name, state: 'active' }, teams: [], sessions: [],
  usage: { agent: 'agent-' + name, used: [], receipts: [], last_reported_ms: null },
  budget: { holder: { kind: 'agent', id: name }, limits, warn_at: warn, zone: 'UTC', version: 1, by: null, at: null,
    used: limits.map((each, index) => used(each.unit, each.period, figures[index])), unavailable: [], within: [], unconfirmed: [] },
  goals: { goals: missed.map((words, index) => ({ goal: { id: 'g' + index, kind: 'goal', words, deadline: null, active: true, evidence: null }, standing: 'missed' })) },
} as DashboardAgent);

describe('Needs a look on the Dashboard', () => {
  it('says a reached limit, then a missed goal, then a near one, each once, and leaves out what is fine', () => {
    const rows = [
      row('Near', [limit('dollars', 500, 'week')], [450]),
      row('Fine', [limit('dollars', 500, 'week'), limit('tokens', 1000, 'day')], [100, 500]),
      row('Full', [limit('tokens', 1000, 'day')], [1000], null, ['send the invoice']),
      // Context limits compact by design; a level reached is the limit working, never an exception.
      row('Context', [limit('context_percent', 40, null)], [95]),
      row('Unknown', [limit('dollars', 10, 'day')], [null]),
    ];
    expect(exceptionsOf(rows).map((each) => [each.kind, each.words])).toEqual([
      ['reached', 'Full reached its limit of 1,000 tokens a day'],
      ['missed', 'Full missed a goal: send the invoice'],
      ['near', 'Near has used 90% of its $500 a week'],
    ]);
  });

  it('begins "near" at the budget’s own warn_at, and at 80% when it has none', () => {
    expect(exceptionsOf([row('A', [limit('tokens', 100, 'day')], [79])])).toEqual([]);
    expect(exceptionsOf([row('A', [limit('tokens', 100, 'day')], [80])]).map((each) => each.kind)).toEqual(['near']);
    expect(exceptionsOf([row('A', [limit('tokens', 100, 'day')], [80], 95)])).toEqual([]);
    expect(exceptionsOf([row('A', [limit('tokens', 100, 'day')], [50], 50)]).map((each) => each.kind)).toEqual(['near']);
  });

  it('draws nothing at all when nothing needs a look, and a line with its Open link when something does', () => {
    expect(renderToStaticMarkup(<Exceptions rows={[row('Fine', [limit('tokens', 100, 'day')], [10])]} />)).toBe('');
    const host = document.createElement('div');
    host.innerHTML = renderToStaticMarkup(<Exceptions rows={[row('Full', [limit('tokens', 100, 'day')], [100])]} />);
    expect(host.querySelector('.dash-count')?.textContent).toBe('1');
    expect(host.querySelector('tr[data-exception="reached"] a')?.getAttribute('href')).toBe('#/file/agent-Full/budgets');
  });
});
