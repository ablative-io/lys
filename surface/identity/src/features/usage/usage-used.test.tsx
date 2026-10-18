/**
 * Used, first on an agent's Limits and goals tab: what it used this week and today, and its account's windows, before
 * any control.
 */
import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';
import { UsedNow, periodWords, windowName } from '../usage/UsedNow';
import type { Used, UsageView } from '../usage/contract';

const fig = (unit: Used['unit'], period: Used['period'], figure: number | null, reported?: { figure: number; missing: number }): Used =>
  (figure === null ? { unit, period, since_ms: 1, figure: null, unavailable: 'none', ...(reported ? { reported } : {}) } : { unit, period, since_ms: 1, figure, unavailable: null }) as Used;
const view = (figures: Used[], more: Partial<UsageView> = {}): UsageView => ({ agent: 'a', used: [], receipts: [], last_reported_ms: Date.UTC(2026, 9, 18, 3, 0), figures, ...more });
const rows = (host: Element) => [...host.querySelectorAll('tr')].map((tr) => [tr.querySelector('th')?.textContent, tr.querySelector('td')?.textContent]);
const page = (usage: UsageView) => { const host = document.createElement('div'); host.innerHTML = renderToStaticMarkup(<UsedNow usage={usage} />); return host; };

describe('Used, on an agent’s Limits and goals', () => {
  it('says this week and today as money, tokens and running time, the week’s own figures and never the day’s', () => {
    const host = page(view([fig('dollars', 'week', 4.2), fig('tokens', 'week', 1204330), fig('running_ms', 'week', 11_400_000),
      fig('dollars', 'day', 0.8), fig('tokens', 'day', 2000), fig('running_ms', 'day', 30_000), fig('tokens', 'five_hour', 9)]));
    expect(rows(host)).toEqual([
      ['This week', 'US$4.20, 1,204,330 tokens, 3 hours 10 minutes running'],
      ['Today', 'US$0.80, 2,000 tokens, under a minute running'],
    ]);
  });

  it('says a missing figure by name, a partial one as at least, and nothing reported only when nothing was', () => {
    expect(periodWords([fig('dollars', 'week', null, { figure: 2, missing: 1 }), fig('tokens', 'week', null), fig('running_ms', 'week', 60_000)], 'week'))
      .toBe('at least US$2.00, no tokens reported, 1 minute running');
    expect(periodWords([fig('dollars', 'week', null)], 'week')).toBeNull();
    expect(rows(page(view([], { last_reported_ms: null })))).toEqual([['This week', 'Nothing reported yet'], ['Today', 'Nothing reported yet']]);
    expect(page(view([], { last_reported_ms: null })).querySelector('.usage-tracking')?.textContent).toContain('Tracking is incomplete');
  });

  it('lists each plan account’s windows with how much is used and when it resets, and says a plan pays', () => {
    const future = Date.now() + 3_600_000;
    const host = page(view([], { accounts: [{ account: 'cherie', at_ms: 1, windows: [{ duration_minutes: 300, used_percent: 12.4, resets_at_ms: future }, { duration_minutes: 10080, used_percent: 40, resets_at_ms: 1 }] }] }));
    const windows = [...host.querySelectorAll('tr[data-window]')].map((tr) => tr.textContent);
    expect(windows.length).toBe(2);
    expect(windows[0]).toMatch(/^5-hour windowcherie12% usedresets /);
    expect(windows[1]).toMatch(/^Weekly windowcherie40% usedreset .*; read again on its next use$/);
    expect(host.querySelector('.usage-used > .note')?.textContent).toContain('on a plan, the plan is what is paid');
    expect([windowName(120), windowName(90)]).toEqual(['2-hour window', '90-minute window']);
  });
});
