/** A period in which some calls reported nothing: the usage widget says what was reported and how many reported none, never a total and never a nought. */
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { $, $$, click, mount } from './harness';
import { SCRIBE, SERVICE, dashboard, ok } from './fixtures';
import type { Route } from './fixtures';
import { mockTerminal } from './terminal-double';
import { summed } from '../src/features/runtime/canvas-widgets';
import type { DashboardAgent } from '../src/features/dashboard/contract';
vi.mock('@gespenst/core', () => ({ createTerminal: mockTerminal }));

const session = 'op-' + '7'.repeat(32);
const running = { session, agent: SCRIBE, machine: 'machine-one', machine_name: 'Test runner', runtime: 'sh', shown: 'running', last_reported: 'running', first_report_at: 1790000000, last_report_at: 1790000001, what: 'process 4242', stopped: null, reported_by: 'test runner' };
const routes: Record<string, Route> = { ...SERVICE, '/runtime/live': ok({ sessions: [running], unanswered: [] }) };
const gap = 'call c9 ended Unrecorded, answered HTTP 429, and its response reported no token figures: what it spent is not known';
const figures = [
  { unit: 'tokens', period: 'day', since_ms: null, figure: null, unavailable: gap, reported: { figure: 1180, missing: 1 } },
  { unit: 'tokens', period: 'week', since_ms: null, figure: null, unavailable: gap, reported: { figure: 5400, missing: 3 } },
  { unit: 'dollars', period: 'day', since_ms: null, figure: null, unavailable: 'dollars have not been reported' },
];
const usage = { agent: SCRIBE, used: [], receipts: [], last_reported_ms: 1790000000000, figures, accounts: [] };

beforeEach(() => {
  vi.stubGlobal('ResizeObserver', class { observe() {} disconnect() {} });
  localStorage.clear();
});

describe('A period some calls reported nothing in', () => {
  it('is summed as no figure, with what was reported and the count of those that reported none kept beside it', () => {
    const row = (name: string, more: unknown[]) => ({ agent: { id: name, display_name: name }, usage: { ...usage, agent: name, figures: more } }) as unknown as DashboardAgent;
    const lines = summed([row('One', figures), row('Two', [figures[0]])]);
    expect(lines.map((line) => [line.unit + '/' + line.period, line.figure, line.partial ?? null, line.missing.length])).toEqual([
      ['tokens/day', null, { figure: 2360, missing: 2 }, 2], ['tokens/week', null, { figure: 5400, missing: 3 }, 1], ['dollars/day', null, null, 1]]);
  });

  it('shows on the usage widget as what was reported and how many calls reported none, and a figure nobody reported still says so', async () => {
    localStorage.setItem('lys.canvas', JSON.stringify({ boxes: { ['session:' + session]: { x: 200, y: 200, w: 440, h: 34 } }, open: [], view: { x: 24, y: 24 },
      widgets: [{ id: 'widget:u', kind: 'usage', x: 700, y: 300, w: 340, h: 250, view: 'detail' }], links: [{ id: 'link:u', from: 'agent:' + SCRIBE, to: 'widget:u' }] }));
    await mount('#/canvas', { ...routes, '/dashboard': ok(dashboard({ [SCRIBE]: { usage } })) });
    expect($$('[data-widget] tr[data-figure]').map((row) => row.textContent)).toEqual([
      'Tokens today1,180 tokens reported, 1 call reported none', 'Tokens this week5,400 tokens reported, 3 calls reported none', 'Dollars todayNot reported']);
    expect($('[data-widget] tr[data-figure="tokens"] [data-partial]')?.getAttribute('title')).toBe('Scribe: ' + gap);
    expect($('[data-widget] tr[data-figure="dollars"] [data-partial]')).toBeNull();
    // On its medium face the same is said: the reported amount over its name and the count.
    await click($('[data-widget] [data-act="widget-view"]'));
    await click($('[data-widget] [data-act="widget-view"]'));
    await click($('[data-widget] [data-act="widget-view"]'));
    const stat = $('[data-widget] [data-stat="tokens/day"]');
    expect([$('[data-widget]')?.getAttribute('data-view'), stat?.querySelector('b')?.textContent, stat?.querySelector('.sec')?.textContent]).toEqual(['faces', '1,180 tokens reported', 'Tokens today, 1 call reported none']);
  });
});
