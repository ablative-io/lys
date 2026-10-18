/**
 * One editor for the spending limit: the card says what its chosen act does, and the Budget table leaves that limit's
 * row out on an agent's tab. Kept here because this folder is the one of domain 06 the test runner includes.
 */
import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';
import { REACHED, SpendingLimit, spendingIndex, withSpending } from '../usage/SpendingLimit';
import { UsageBudgets } from '../usage/UsageBudgets';
import type { BudgetsView, Limit } from '../usage/contract';

const dollars = (act: Limit['act']): Limit => ({ unit: 'dollars', amount: 20, period: 'week', act });
const tokens: Limit = { unit: 'tokens', amount: 1000, period: 'day', act: 'notice' };
const view = (limits: Limit[], kind: 'agent' | 'team' = 'agent'): BudgetsView => ({ holder: { kind, id: 'a' }, limits, warn_at: null, zone: 'UTC', version: 3,
  by: null, at: null, unavailable: [], within: [], unconfirmed: [],
  used: limits.map((limit) => ({ unit: limit.unit, period: limit.period, since_ms: null, figure: 1, unavailable: null })) });
const host = (markup: string) => { const div = document.createElement('div'); div.innerHTML = markup; return div; };
const card = (limits: Limit[]) => host(renderToStaticMarkup(<SpendingLimit budgets={view(limits)} changed={() => undefined} />));
const table = (limits: Limit[], kind: 'agent' | 'team' = 'agent') => host(renderToStaticMarkup(
  <UsageBudgets budgets={view(limits, kind)} receipts={[]} changed={() => undefined} apart={kind === 'agent' ? spendingIndex(limits) : -1} />));

describe('The spending limit has one editor', () => {
  it('says what the limit’s own act does, never "the session ends" for a limit that only tells', () => {
    expect(card([dollars('tell')]).querySelector('[data-reached-act]')?.textContent).toBe('Applies as soon as it is saved. When it is reached, the responsible person is told; the session keeps running. Leave the amount empty and save to remove it.');
    expect(card([dollars('stop')]).querySelector('[data-reached-act]')?.textContent).toContain('the running session ends.');
    expect(card([dollars('tell')]).querySelector<HTMLSelectElement>('select[name="spending_act"] option[selected]')?.value).toBe('tell');
    // A new limit holds: its act begins as ending the session, and the sentence says so.
    expect(card([]).querySelector('[data-reached-act]')?.getAttribute('data-reached-act')).toBe('stop');
    expect(Object.keys(REACHED).sort()).toEqual(['compact', 'notice', 'stop', 'tell']);
  });

  it('saves the act chosen in the card, and keeps every other limit as it was', () => {
    expect(withSpending([tokens, dollars('stop')], 30, 'month', 'tell')).toEqual([tokens, { ...dollars('stop'), amount: 30, period: 'month', act: 'tell' }]);
    expect(withSpending([tokens], 5, 'day', 'compact')).toEqual([tokens, { unit: 'dollars', amount: 5, period: 'day', act: 'compact' }]);
    expect(withSpending([tokens, dollars('stop')], null, 'week')).toEqual([tokens]);
  });

  it('leaves the spending limit’s row out of an agent’s table, and keeps every row on a team’s', () => {
    const agent = table([tokens, dollars('tell')]);
    expect(agent.querySelectorAll('tbody tr').length).toBe(1);
    expect(agent.querySelector('tbody input')?.getAttribute('aria-label')).toBe('Amount of 1,000 tokens a day');
    expect(agent.querySelector('caption')?.textContent).toContain('The spending limit is set above.');
    expect([...agent.querySelectorAll('thead th')].map((th) => th.textContent)[0]).toBe('Limit');
    expect(table([dollars('tell')]).querySelector('tbody td.dim')?.textContent).toBe('No other limit is set for this agent.');
    expect(table([tokens, dollars('tell')], 'team').querySelectorAll('tbody tr').length).toBe(2);
  });
});
