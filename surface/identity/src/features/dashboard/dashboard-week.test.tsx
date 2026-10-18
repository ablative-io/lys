/**
 * The Dashboard's "This week" panel: what the person's agents cost, from each agent's own figure as the service answers
 * it.
 */
import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';
import type { DashboardAgent } from '../dashboard/contract';
import { Spend, weekSpend, weekTotal } from '../dashboard/Spend';
import { money } from '../../shell/count';
import type { AccountWindows, Used } from '../usage/contract';

const monday = new Date(2026, 9, 12, 0, 0).getTime();
const week = (figure: number | null, more: Partial<Used> = {}): Used => ({ unit: 'dollars', period: 'week', since_ms: monday,
  ...(figure === null ? { figure: null, unavailable: 'no cost was reported this week' } : { figure, unavailable: null }), ...more } as Used);
const today = (figure: number): Used => ({ unit: 'dollars', period: 'day', since_ms: monday, figure, unavailable: null });
const row = (name: string, figures: Used[] | null, accounts: AccountWindows[] = []): DashboardAgent => ({
  agent: { id: 'agent-' + name, display_name: name, state: 'active' }, teams: [], sessions: [],
  usage: figures === null ? { refusal: 'BudgetsUnavailable', reason: 'the budgets store is closed' }
    : { agent: 'agent-' + name, used: [], receipts: [], last_reported_ms: null, figures, accounts },
  budget: { holder: { kind: 'agent', id: name }, limits: [], warn_at: null, zone: 'UTC', version: 0,
    by: null, at: null, used: [], unavailable: [], within: [], unconfirmed: [] }, goals: { goals: [] },
} as DashboardAgent);
const page = (rows: DashboardAgent[]) => {
  const host = document.createElement('div');
  host.innerHTML = renderToStaticMarkup(<Spend rows={rows} />);
  return host;
};
const lines = (host: Element) => [...host.querySelectorAll('tr[data-spend]')].map((tr) => [...tr.querySelectorAll('td')].map((td) => td.querySelector('summary')?.textContent ?? td.textContent));

describe('This week on the Dashboard', () => {
  it('writes money in US dollars with two decimals in Australian English, never the browser locale', () => {
    expect([money(12.5), money(0), money(1234.567), money(0.004)]).toEqual(['US$12.50', 'US$0.00', 'US$1,234.57', 'US$0.00']);
  });

  it('lists every agent dearest first, takes the week and never the day, and says a total from what was reported', () => {
    const host = page([row('Quiet', [today(9), week(0)]), row('Scribe', [today(1), week(12.5)]), row('Courier', [week(3)])]);
    expect(lines(host)).toEqual([['Scribe', 'US$12.50'], ['Courier', 'US$3.00'], ['Quiet', 'US$0.00']]);
    expect(host.querySelector('[data-spend="total"]')?.textContent).toBe('US$15.50');
    expect(host.querySelector('.dash-spend-total')?.textContent).toBe('US$15.50 since 12 Oct');
    expect(host.querySelector('[data-spend="coverage"]')).toBeNull();
  });

  it('never counts a missing figure as nothing: the total says how many agents it covers in full, and each line says why', () => {
    const rows = [row('Scribe', [week(10)]), row('Courier', [week(null, { reported: { figure: 2, missing: 3 } } as Partial<Used>)]),
      row('Silent', [week(null)]), row('Unread', null), row('Old', [today(4)])];
    expect(rows.map((each) => weekSpend(each).kind)).toEqual(['spent', 'partial', 'none', 'unread', 'none']);
    expect(weekTotal(rows)).toEqual({ total: 12, short: 4, priced: 2 });
    const host = page(rows);
    expect(host.querySelector('[data-spend="total"]')?.textContent).toBe('US$12.00');
    expect(host.querySelector('[data-spend="coverage"]')?.textContent).toBe('from 1 of 5 agents in full');
    expect(lines(host)).toEqual([['Scribe', 'US$10.00'], ['Courier', 'at least US$2.00some of its use gave no price'], ['Silent', 'Nothing reported'],
      ['Unread', 'Could not be read'], ['Old', 'Nothing reported']]);
  });

  it('says a plan pays when an agent’s account reported plan windows, and US dollars at reported prices otherwise', () => {
    const plan: AccountWindows = { account: 'cherie', at_ms: monday, windows: [{ duration_minutes: 300, used_percent: 10, resets_at_ms: monday + 1 }] };
    expect(page([row('Helper', [week(4.2)], [plan])]).querySelector('.dash-spend-note')?.textContent).toContain('On a plan, you pay the plan’s price.');
    expect(page([row('Helper', [week(4.2)])]).querySelector('.dash-spend-note')?.textContent).toBe('In US dollars, at the prices the agents’ programs report.');
  });

  it('says nothing was reported, never US$0.00, when no agent gave a figure', () => {
    const host = page([row('Silent', [week(null)]), row('Unread', null)]);
    expect(host.querySelector('[data-spend="total"]')?.textContent).toBe('Nothing reported yet');
    const idle = page([row('Silent', [week(null)]), row('Idle', [week(0)])]);
    expect([idle.querySelector('[data-spend="total"]')?.textContent, idle.querySelector('[data-spend="coverage"]')?.textContent]).toEqual(['US$0.00', 'from 1 of 2 agents in full']);
  });

  it('with no agent, says so and links to adding one', () => {
    const host = page([]);
    expect(host.querySelector('[data-spend="total"]')?.textContent).toBe('US$0.00');
    expect(host.querySelector('.you-scroll a')?.getAttribute('href')).toBe('#/agents/new');
  });
});
