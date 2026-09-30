/** An agent's budget and goals on its own file, in the words a person running agents uses, keeping what they set. */
import { act } from 'react';
import { describe, expect, it } from 'vitest';
import { $, $$, choose, click, mount, text, unmountAll } from './harness';
import { SCRIBE, SERVICE, ok, refused } from './fixtures';
import type { Route } from './fixtures';
import type { Receipt } from '../src/features/usage/contract';
import type { Goal, Limit, LimitsView, Used } from '../src/features/usage/limits';

const budgets = '/budgets/agent/' + SCRIBE;
const usage = '/agents/' + SCRIBE + '/usage';
const goals = '/agents/' + SCRIBE + '/goals';
const file = '#/file/' + SCRIBE + '/budgets';

const empty: LimitsView = { limits: [], warn_at: null, act: 'stop', zone: 'Australia/Sydney', version: 0, by: SCRIBE, at: 0, unavailable: [] };

/** A service that keeps the budget and goals it is given, as the identity service does. */
function keeping(options: { budget?: LimitsView; used?: Used[]; receipts?: Receipt[]; reported?: number | null; goals?: Goal[] } = {}): Record<string, Route> {
  let budget = options.budget ?? empty;
  const set: Goal[] = [...(options.goals ?? [])];
  return {
    ...SERVICE,
    [budgets]: () => ok(budget),
    ['PUT ' + budgets]: (body) => {
      const given = body as { limits: Limit[]; warn_at: number | null; act: LimitsView['act']; version: number };
      budget = { ...budget, ...given, version: given.version + 1 };
      return ok(budget);
    },
    [usage]: ok({ agent: SCRIBE, receipts: options.receipts ?? [], last_reported_ms: options.reported === undefined ? 1790000000000 : options.reported, used: options.used ?? [] }),
    [goals]: () => ok({ goals: set }),
    ['POST ' + goals]: (body) => {
      const given = body as { operation: string; words: string; deadline?: number };
      const goal: Goal = { id: given.operation, words: given.words, deadline: given.deadline ?? null, active: true };
      set.push(goal);
      return ok(goal);
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

const weekly: Limit = { unit: 'dollars', amount: 500, period: 'week' };
const plan: Limit = { unit: 'plan_percent', amount: 50, period: 'week' };

describe('Budget', () => {
  it('lives on the agent\'s own file: the old address opens its Budgets and goals tab, and the rail has no Usage page', async () => {
    await mount('#/usage/' + SCRIBE, keeping());
    expect(location.hash).toBe(file);
    expect($('.tabs a.on')?.textContent).toBe('Budgets and goals');
    expect($('section[aria-label="Budgets"]')).not.toBeNull();
    expect($('section[aria-label="Goals"]')).not.toBeNull();
    expect($('a[href="#/usage"]')).toBeNull();
  });

  it('sets "$500 a week or 50% of the weekly plan, whichever comes first" and keeps it across a reload', async () => {
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

  it('keeps every existing measure and act', async () => {
    await mount(file, keeping({ budget: { ...empty, limits: [{ unit: 'context_percent', amount: 80, period: null }], act: 'compact', version: 1 } }));
    expect($('.usage-summary')?.textContent).toBe('Compacts its session at 80% of its context.');
    expect($$('select[aria-label="Unit"] option').map((o) => o.getAttribute('value'))).toEqual(['dollars', 'plan_percent', 'tokens', 'running_ms', 'context_percent']);
    expect($$('select[aria-label="When a limit is hit"] option').map((o) => o.getAttribute('value'))).toEqual(['pause', 'stop', 'tell', 'compact', 'notice']);
  });

  it('offers only the units Lys has a reported figure for, and says why the others are missing', async () => {
    await mount(file, keeping({ budget: { ...empty, unavailable: [{ unit: 'dollars', reason: 'Codex reports no cost.' }] } }));
    await click($$('button').find((b) => b.textContent === '+ Add a limit') ?? null);
    expect($$('select[aria-label="Unit"] option').map((o) => o.getAttribute('value'))).not.toContain('dollars');
    expect(text()).toContain('dollars: Codex reports no cost.');
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
    await mount(file, keeping({ budget: { ...empty, limits: [weekly], version: 1 } }));
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
