/** An agent's budget and goals on its own file, in the words a person running agents uses, keeping what they set. */
import { act } from 'react';
import { describe, expect, it } from 'vitest';
import { $, $$, choose, click, mount, text, unmountAll } from './harness';
import { SCRIBE, SERVICE, ok, refused } from './fixtures';
import type { Route } from './fixtures';
import type { Budget, BudgetBody, GoalItem, Receipt } from '../src/features/usage/contract';

import { budgetsView } from './budget-fixtures';

const budgets = '/budgets/agent/' + SCRIBE;
const usage = '/agents/' + SCRIBE + '/usage';
const goals = '/agents/' + SCRIBE + '/goals';
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
    ...Object.fromEntries(set.map((goal) => ['POST /goals/' + goal.id + '/active', (body: unknown) => {
      const found = set.find((each) => each.id === goal.id) as Goal;
      found.active = (body as { active: boolean }).active;
      return ok(found);
    }])),
    ...Object.fromEntries(set.map((goal) => ['POST /goals/' + goal.id + '/words', (body: unknown) => {
      const found = set.find((each) => each.id === goal.id) as Goal;
      found.words = (body as { words: string }).words;
      return ok(found);
    }])),
  };
}

async function type(selector: string, value: string): Promise<void> {
  const input = document.querySelector<HTMLInputElement>(selector);
  if (!input) throw new Error('no ' + selector);
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')?.set?.call(input, value);
    input.dispatchEvent(new Event('input', { bubbles: true }));
  });
}

async function enter(selector: string): Promise<void> {
  const form = document.querySelector<HTMLInputElement>(selector)?.form;
  await act(async () => { form?.requestSubmit(); });
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

describe('Budget', () => {
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
    const { posted } = await mount(file, routes);
    await click($$('button').find((b) => b.textContent === '+ Add a limit') ?? null);
    await type('input[name="amount"]', '500');
    await choose($('select[aria-label="Period"]'), 'week');
    await click($$('button').find((b) => b.textContent === '+ Add another limit') ?? null);
    await choose($$('select[aria-label="Unit"]')[1], 'plan_percent');
    await choose($$('select[aria-label="Period"]')[1], 'week');
    await type('li:nth-child(2) input[name="amount"]', '50');
    await type('input[name="warn_at"]', '80');
    expect($('.usage-summary')?.textContent).toBe('Stops at $500 a week or 50% of the weekly plan, whichever comes first; tells you at 80%.');
    await click($$('button').find((b) => b.textContent === 'Save budget') ?? null);
    expect(posted.find((call) => call.path === 'PUT ' + budgets)?.body).toEqual({ limits: [weekly, plan], warn_at: 80, act: 'stop', version: 0 });
    expect(text()).toContain('Budget saved as version 1.');
    await reload(routes);
    expect($('.usage-summary')?.textContent).toContain('$500 a week or 50% of the weekly plan');
  });

  it('shows what is used beside each limit, and never asks for a time zone', async () => {
    await mount(file, keeping({ budget: { ...empty, limits: [weekly], version: 1 }, used: [{ unit: 'dollars', period: 'week', figure: 212, since_ms: 1 }] }));
    expect($('.usage-used')?.textContent).toBe('$212 of $500 this week');
    expect(text()).not.toMatch(/time zone|Measure|Counted each/);
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
      version: 1,
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

  it('says what happened at a limit in plain words', async () => {
    const crossing = { operation: 'op-c', holder: { kind: 'agent' as const, id: SCRIBE }, measure: 'tokens' as const, version: 1, limit: 40000, figure: 40000, act: 'compact' as const, agent: SCRIBE, at_ms: 1790000000000 };
    await mount(file, keeping({ receipts: [{ crossing, acted: { stands: 'delivered', words: 'sent', at_ms: 1 } }] }));
    expect($('.usage-event')?.textContent).toMatch(/^Hit 40,000 tokens at .+\. Lys asked to compact the session\. Lys hasn't heard back yet\.$/);
    expect(text()).not.toContain('cannot be known');
  });

  it('names a refusal and keeps nothing', async () => {
    const routes = { ...keeping(), ['PUT ' + budgets]: refused(403, 'not_permitted', 'you are not responsible for this agent') };
    await mount(file, routes);
    await click($$('button').find((b) => b.textContent === '+ Add a limit') ?? null);
    await type('input[name="amount"]', '500');
    await click($$('button').find((b) => b.textContent === 'Save budget') ?? null);
    expect($('[role="alert"]')?.textContent).toContain('not_permitted');
    await reload(routes);
    expect($('.usage-summary')?.textContent).toBe('No limit: this agent can spend without stopping.');
  });

  it('shows missing runner tracking as incomplete', async () => {
    await mount(file, keeping({ reported: null }));
    expect(text()).toContain('Tracking is incomplete');
  });

  it('draws no analytics dashboard', async () => {
    await mount(file, { ...keeping(reached(null)), [budgets]: ok(budgetsView(holder, [tokens], { used: [{ unit: 'tokens', period: 'day', figure: 1200, since_ms: 0, unavailable: null }] })) });
    expect(document.querySelectorAll('section.usage svg, section.usage canvas')).toHaveLength(0);
  });
});

describe('Goals', () => {
  const standing: Goal = { id: 'op-' + '1'.repeat(32), words: 'Keep main green', deadline: null, active: true };
  const old: Goal = { id: 'op-' + '2'.repeat(32), words: 'Land the install', deadline: null, active: false };

  it('adds a goal by typing it and pressing Enter, with no kind and no deadline asked', async () => {
    const routes = keeping();
    const { posted } = await mount(file, routes);
    expect($('form[aria-label="Add a goal"] select')).toBeNull();
    expect($('input[name="deadline"]')).toBeNull();
    await type('input[name="new-goal"]', 'Keep main green');
    await enter('input[name="new-goal"]');
    const body = posted.find((call) => call.path === goals)?.body as Record<string, unknown>;
    expect(body.words).toBe('Keep main green');
    expect(body).not.toHaveProperty('deadline');
    await reload(routes);
    expect(($('.usage-goal input[name="words"]') as HTMLInputElement | null)?.value).toBe('Keep main green');
  });

  it('takes a deadline only when asked for one', async () => {
    const { posted } = await mount(file, keeping());
    await click($$('button').find((b) => b.textContent === 'Add a deadline') ?? null);
    await type('input[name="deadline"]', '2026-10-31');
    await type('input[name="new-goal"]', 'Ship the plan');
    await enter('input[name="new-goal"]');
    expect((posted.find((call) => call.path === goals)?.body as { deadline?: number }).deadline).toBe(Math.floor(Date.parse('2026-10-31') / 1000));
  });

  it('rewords a goal in place and switches it off without losing it', async () => {
    const { posted } = await mount(file, keeping({ goals: [standing, old] }));
    await type('.usage-goal input[name="words"]', 'Keep main green all night');
    await enter('.usage-goal input[name="words"]');
    await act(async () => { ($('.usage-goal input[name="words"]') as HTMLInputElement).dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true })); });
    expect(posted.find((call) => call.path === '/goals/' + standing.id + '/words')?.body).toMatchObject({ words: 'Keep main green all night' });
    await click($('.usage-goal input[type="checkbox"]'));
    expect(posted.find((call) => call.path === '/goals/' + standing.id + '/active')?.body).toMatchObject({ active: false });
    expect($('.usage-inactive summary')?.textContent).toBe('2 switched off');
  });
});
