/** DIRECTORY-051 R5: plain budget and goal controls that keep what they set and never show an unknown act as done. */
import { act } from 'react';
import { describe, expect, it } from 'vitest';
import { $, choose, click, mount, text, unmountAll } from './harness';
import { SCRIBE, SERVICE, ok, refused } from './fixtures';
import type { Route } from './fixtures';
import type { Budget, BudgetBody, GoalItem, Receipt } from '../src/features/usage/contract';

import { budgetsView } from './budget-fixtures';

const budgets = '/budgets/agent/' + SCRIBE;
const usage = '/agents/' + SCRIBE + '/usage';
const goals = '/agents/' + SCRIBE + '/goals';
const holder = { kind: 'agent' as const, id: SCRIBE };
const file = '#/file/' + SCRIBE + '/budgets';

/** A service that keeps the budgets and goals it is given, as the identity service does. */
function keeping(receipts: Receipt[] = [], reported: number | null = 1790000000000): Record<string, Route> {
  let kept = budgetsView(holder);
  const set: GoalItem[] = [];
  return {
    ...SERVICE,
    [budgets]: () => ok(kept),
    ['PUT ' + budgets]: (body) => {
      const given = body as BudgetBody;
      expect(given.version).toBe(kept.version);
      expect(Object.keys(given).sort()).toEqual(['limits', 'version', 'warn_at']);
      kept = { ...kept, ...given, version: given.version + 1, by: SCRIBE, at: 1790000000,
        used: given.limits.map((limit) => ({ unit: limit.unit, period: limit.period, figure: 0, since_ms: limit.period ? 0 : null, unavailable: null })) };
      return ok(kept);
    },
    [usage]: ok({ agent: SCRIBE, used: [], receipts, last_reported_ms: reported }),
    [goals]: () => ok({ goals: set }),
    ['POST ' + goals]: (body) => {
      const given = body as { operation: string; kind: GoalItem['goal']['kind']; words: string; deadline: number };
      const item: GoalItem = { goal: { id: given.operation, kind: given.kind, words: given.words, deadline: given.deadline, active: true, evidence: null }, standing: 'open' };
      set.push(item);
      return ok(item);
    },
  };
}

async function type(selector: string, value: string): Promise<void> {
  const input = document.querySelector<HTMLInputElement | HTMLTextAreaElement>(selector);
  if (!input) throw new Error('no ' + selector);
  const kind = input instanceof HTMLTextAreaElement ? HTMLTextAreaElement : HTMLInputElement;
  await act(async () => {
    Object.getOwnPropertyDescriptor(kind.prototype, 'value')?.set?.call(input, value);
    input.dispatchEvent(new Event('input', { bubbles: true }));
  });
}

async function submit(form: string): Promise<void> {
  await click($('form[aria-label="' + form + '"] button[type="submit"]'));
}

async function reload(routes: Record<string, Route>): Promise<void> {
  unmountAll();
  document.body.innerHTML = '';
  await mount(file, routes);
}

const reached = (stands: Receipt['acted']): Receipt[] => [{
  crossing: { operation: 'op-crossing', holder, measure: 'tokens', version: 1, limit: 1000, figure: 1200, limit_index: 0, warning: false, act: 'stop', agent: SCRIBE, at_ms: 1790000000000 },
  acted: stands,
}];

const tokens: Budget = { holder, measure: 'tokens', limit: 1000, period: { length: 'day', zone: 'UTC' }, act: 'stop', version: 1, by: SCRIBE, at: 1790000000 };

describe('Usage', () => {
  it('lives on the agent\'s own file: the old address opens its Budgets and goals tab, and the rail has no Usage page', async () => {
    await mount('#/usage/' + SCRIBE, keeping());
    expect(location.hash).toBe(file);
    expect($('.tabs a.on')?.textContent).toBe('Budgets and goals');
    expect($('section[aria-label="Budgets"]')).not.toBeNull();
    expect($('section[aria-label="Goals"]')).not.toBeNull();
    expect($('a[href="#/usage"]')).toBeNull();
  });

  it('gives a goal a box several lines tall', async () => {
    await mount(file, keeping());
    const what = $('form[aria-label="Set a goal"] textarea[name="words"]') as HTMLTextAreaElement | null;
    expect(what?.rows).toBeGreaterThanOrEqual(4);
    expect(what?.maxLength).toBe(500);
  });

  it('shows a deadline-free goal without inventing a date', async () => {
    const item: GoalItem = { goal: { id: 'op-standing-aim', kind: 'goal', words: 'Keep the directory available', deadline: null, active: true, evidence: null }, standing: 'open' };
    await mount(file, { ...keeping(), [goals]: ok({ goals: [item] }) });
    const row = $('section[aria-label="Goals"] tbody tr');
    expect(row?.children[1]?.textContent).toBe(item.goal.words);
    expect(row?.children[2]?.textContent).toBe('No deadline');
  });

  it('changes a budget on the agent\'s own file and keeps it across a reload', async () => {
    const routes = keeping();
    const { requests } = await mount(file, routes);
    await choose($('form[aria-label="Set a budget"] select'), 'tokens');
    await type('input[name="limit"]', '5000');
    await submit('Set a budget');
    expect(requests).toContain('PUT ' + budgets);
    expect(text()).toContain('Budget kept as version 1.');
    await reload(routes);
    expect($('section[aria-label="Budgets"] table')?.textContent).toContain('5000');
  });

  it('keeps a goal entered in the screen across a reload', async () => {
    const routes = keeping();
    await mount(file, routes);
    await type('textarea[name="words"]', 'Land the Usage screen');
    await type('input[name="deadline"]', '2026-10-01T12:00');
    await submit('Set a goal');
    await reload(routes);
    expect($('section[aria-label="Goals"] table')?.textContent).toContain('Land the Usage screen');
    expect($('section[aria-label="Goals"] table')?.textContent).toContain('Open');
  });

  it('keeps both existing actions and the holder warning when adding a limit', async () => {
    const limits = [{ unit: 'tokens' as const, amount: 400, period: 'week' as const, act: 'tell' as const },
      { unit: 'tokens' as const, amount: 500, period: 'week' as const, act: 'stop' as const }];
    const kept = { ...budgetsView(holder), limits, warn_at: 80, version: 7 };
    let sent: BudgetBody | null = null;
    await mount(file, { ...keeping(), [budgets]: ok(kept), ['PUT ' + budgets]: (body) => {
      sent = body as BudgetBody;
      return ok({ ...kept, ...sent, version: 8 });
    } });
    await type('input[name="limit"]', '600');
    await submit('Set a budget');
    expect(sent).toEqual({ limits: [...limits, { unit: 'tokens', amount: 600, period: 'day', act: 'tell' }], warn_at: 80, version: 7 });
  });

  it('names an unavailable figure without displaying it as a measured zero', async () => {
    await mount(file, { ...keeping(), [budgets]: ok(budgetsView(holder, [tokens], {
      used: [{ unit: 'tokens', period: 'day', figure: null, since_ms: 0, unavailable: 'runner token report is missing' }],
    })) });
    expect($('section[aria-label="Budgets"] tbody tr')?.children[4]?.textContent).toBe('Unavailable: runner token report is missing');
  });

  it('matches a receipt to its own limit and excludes warning receipts', async () => {
    const limits = [tokens, { ...tokens, limit: 1500, act: 'tell' as const }];
    const receipts = [...reached({ stands: 'confirmed', words: 'exit seen', at_ms: 1 }),
      { ...reached(null)[0], crossing: { ...reached(null)[0].crossing, limit_index: 1, warning: true } }];
    await mount(file, { ...keeping(receipts), [budgets]: ok(budgetsView(holder, limits, {
      used: limits.map(() => ({ unit: 'tokens', period: 'day', figure: 1200, since_ms: 0, unavailable: null })),
    })) });
    const rows = document.querySelectorAll('section[aria-label="Budgets"] tbody tr');
    expect(rows[0]?.children[4]?.textContent).toContain('Its act was confirmed');
    expect(rows[1]?.children[4]?.textContent).toBe('Held: not reached.');
  });

  it('says in plain words that a budget was reached and its act confirmed', async () => {
    await mount(file, { ...keeping(reached({ stands: 'confirmed', words: 'exit seen', at_ms: 1 })), [budgets]: ok(budgetsView(holder, [tokens], { used: [{ unit: 'tokens', period: 'day', figure: 1200, since_ms: 0, unavailable: null }] })) });
    expect(text()).toContain('Reached at 1200. Its act was confirmed: end the session.');
  });

  it('never shows an uncertain act as confirmed', async () => {
    await mount(file, { ...keeping(reached({ stands: 'uncertain', words: 'runner restarted', at_ms: 1 })), [budgets]: ok(budgetsView(holder, [tokens], { used: [{ unit: 'tokens', period: 'day', figure: 1200, since_ms: 0, unavailable: null }] })) });
    expect(text()).toContain('cannot be known');
    expect(text()).not.toContain('confirmed:');
  });

  it('shows no values to a caller who may not read them', async () => {
    await mount(file, { ...keeping(), [budgets]: refused(403, 'not_permitted', 'you are not responsible for this agent') });
    expect(text()).toContain('not_permitted');
    expect($('section[aria-label="Budgets"]')).toBeNull();
  });

  it('keeps nothing a caller may not change, and names the refusal', async () => {
    const routes = { ...keeping(), ['PUT ' + budgets]: refused(403, 'not_permitted', 'you are not responsible for this agent') };
    await mount(file, routes);
    await type('input[name="limit"]', '5000');
    await submit('Set a budget');
    expect($('[role="alert"]')?.textContent).toContain('not_permitted');
    await reload(routes);
    expect(text()).toContain('No budget is set for this agent.');
  });

  it('shows missing runner tracking as incomplete', async () => {
    await mount(file, keeping([], null));
    expect(text()).toContain('Tracking is incomplete');
  });

  it('draws no analytics dashboard', async () => {
    await mount(file, { ...keeping(reached(null)), [budgets]: ok(budgetsView(holder, [tokens], { used: [{ unit: 'tokens', period: 'day', figure: 1200, since_ms: 0, unavailable: null }] })) });
    expect(document.querySelectorAll('section.usage svg, section.usage canvas')).toHaveLength(0);
  });
});
