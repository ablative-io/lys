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

/** A form's submit button, inside it or in the table row whose controls belong to it. */
async function submit(form: string): Promise<void> {
  const found = $('form[aria-label="' + form + '"]');
  await click(found?.querySelector('button[type="submit"]') ?? $('button[type="submit"][form="' + found?.id + '"]'));
}

const addBudget = 'section[aria-label="Budgets"] table > tfoot > tr:last-child';
const addGoal = 'section[aria-label="Goals"] table > tfoot > tr:last-child';

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
  it('lives on the agent\'s own file: the old address opens its Limits and goals tab, and the rail has no Usage page', async () => {
    await mount('#/usage/' + SCRIBE, keeping());
    expect(location.hash).toBe(file);
    expect($('.tabs a.on')?.textContent).toBe('Limits and goals');
    expect($('section[aria-label="Budgets"]')).not.toBeNull();
    expect($('section[aria-label="Goals"]')).not.toBeNull();
    expect($('a[href="#/usage"]')).toBeNull();
  });

  it('adds a limit in the budget table\'s own last row, each control under its own head', async () => {
    await mount(file, { ...keeping(), [budgets]: ok(budgetsView(holder, [tokens])) });
    const heads = [...document.querySelectorAll('section[aria-label="Budgets"] thead th')].map((th) => th.textContent);
    expect(heads).toEqual(['Spend at most', 'Used', 'When it\'s hit', 'Where it stands', 'Change']);
    const cells = [...($(addBudget)?.children ?? [])];
    expect(cells).toHaveLength(5);
    expect(cells.every((cell) => cell.tagName === 'TD' && !cell.hasAttribute('colspan'))).toBe(true);
    expect(cells[0].querySelector('input[name="limit"]')).not.toBeNull();
    expect(cells[0].querySelector('select[aria-label="Unit"]')).not.toBeNull();
    expect(cells[0].querySelector('select[aria-label="Period"]')).not.toBeNull();
    expect(cells[1].textContent).toBe('');
    expect(cells[1].children).toHaveLength(0);
    expect(cells[2].querySelector('select[aria-label="When it\'s hit"]')).not.toBeNull();
    expect(cells[3].textContent).toBe('');
    expect(cells[3].children).toHaveLength(0);
    const add = cells[4].querySelector<HTMLButtonElement>('button[type="submit"]');
    expect([add?.getAttribute('aria-label'), add?.dataset.symbol, add?.textContent]).toEqual(['Add this limit', 'add', '']);
    const form = $('form[aria-label="Set a budget"]');
    expect([...($(addBudget)?.querySelectorAll('input, select, button') ?? [])].every((control) => control.getAttribute('form') === form?.id)).toBe(true);
    expect(text()).not.toContain('New limit');
  });

  it('adds a goal in the goal table\'s own last row, in a box that grows with the words', async () => {
    await mount(file, keeping());
    const heads = [...document.querySelectorAll('section[aria-label="Goals"] thead th')].map((th) => th.textContent);
    expect(heads).toEqual(['Kind', 'What the agent is reminded of', 'Deadline', 'Where it stands', 'Change']);
    const cells = [...($(addGoal)?.children ?? [])];
    expect(cells).toHaveLength(5);
    expect(cells[0].textContent).toBe('goal');
    const what = cells[1].querySelector<HTMLTextAreaElement>('textarea[name="words"]');
    expect(what?.rows).toBe(1);
    expect(what?.classList.contains('usage-grow')).toBe(true);
    expect(cells[2].querySelector('input[name="deadline"]')).not.toBeNull();
    expect(cells[3].textContent).toBe('');
    const set = cells[4].querySelector<HTMLButtonElement>('button[type="submit"]');
    expect([set?.getAttribute('aria-label'), set?.dataset.symbol, set?.textContent]).toEqual(['Set this goal', 'approve', 'Set']);
    const form = $('form[aria-label="Set a goal"]');
    expect([...($(addGoal)?.querySelectorAll('textarea, input, button') ?? [])].every((control) => control.getAttribute('form') === form?.id)).toBe(true);
  });

  it('shows a deadline-free goal without inventing a date', async () => {
    const item: GoalItem = { goal: { id: 'op-standing-aim', kind: 'goal', words: 'Keep the directory available', deadline: null, active: true, evidence: null }, standing: 'open' };
    await mount(file, { ...keeping(), [goals]: ok({ goals: [item] }) });
    const row = $('section[aria-label="Goals"] tbody tr');
    expect(row?.children[1]?.querySelector<HTMLInputElement>('input[name="words"]')?.value).toBe(item.goal.words);
    expect(row?.children[2]?.textContent).toBe('No deadline');
  });

  it('changes a budget on the agent\'s own file and keeps it across a reload', async () => {
    const routes = keeping();
    const { requests } = await mount(file, routes);
    await choose($(addBudget + ' select[aria-label="Unit"]'), 'tokens');
    await type('input[name="limit"]', '5000');
    await submit('Set a budget');
    expect(requests).toContain('PUT ' + budgets);
    expect(text()).toContain('Budget kept as version 1.');
    await reload(routes);
    expect(document.querySelector<HTMLInputElement>('section[aria-label="Budgets"] tbody input[name="amount-0"]')?.value).toBe('5000');
    expect($('section[aria-label="Budgets"] tbody tr')?.children[0]?.textContent).toBe('tokens a day');
    expect($('button[aria-label="Remove 5,000 tokens a day"]')).not.toBeNull();
    // The row carries what a summary sentence would repeat: the amount, its unit and period, and its act.
    const act = document.querySelector<HTMLSelectElement>('section[aria-label="Budgets"] tbody select[name="act-0"]');
    expect(act?.value).toBe('tell');
    expect(act?.selectedOptions[0]?.textContent).toBe('Tell the responsible person');
    expect($('.usage-summary')).toBeNull();
    expect(text()).not.toContain('Tells you at');
    expect($('section[aria-label="Budgets"] caption')).toBeNull();
    expect(text()).not.toMatch(/time zone|Measure|Counted each/);
  });

  it('keeps a goal entered in the screen across a reload', async () => {
    const routes = keeping();
    await mount(file, routes);
    await type('textarea[name="words"]', 'Land the Usage screen');
    expect([...document.querySelectorAll('button')].some((button) => button.textContent === 'Add a deadline')).toBe(false);
    await type('input[name="deadline"]', '2026-10-01T12:00');
    await submit('Set a goal');
    await reload(routes);
    expect([...document.querySelectorAll<HTMLInputElement>('section[aria-label="Goals"] tbody input[name="words"]')].map((input) => input.value)).toContain('Land the Usage screen');
    expect($('section[aria-label="Goals"] table')?.textContent).toContain('Open');
  });

  it('keeps a goal set with no deadline when the deadline box is left empty', async () => {
    const routes = keeping();
    await mount(file, routes);
    expect($(addGoal + ' select')).toBeNull();
    expect(document.querySelector<HTMLInputElement>(addGoal + ' input[name="deadline"]')?.value).toBe('');
    await type('textarea[name="words"]', 'Answer every question in plain words');
    await submit('Set a goal');
    await reload(routes);
    expect([...document.querySelectorAll<HTMLInputElement>('section[aria-label="Goals"] tbody input[name="words"]')].map((input) => input.value)).toContain('Answer every question in plain words');
    expect($('section[aria-label="Goals"] table')?.textContent).toContain('No deadline');
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
    expect($('section[aria-label="Budgets"] caption')?.textContent).toBe('Each limit acts on its own: whichever is reached first acts first.');
    expect(text().split('whichever')).toHaveLength(2);
    await type('input[name="limit"]', '600');
    await submit('Set a budget');
    expect(sent).toEqual({ limits: [...limits, { unit: 'tokens', amount: 600, period: 'day', act: 'tell' }], warn_at: 80, version: 7 });
  });

  it('removes one limit and keeps the others with their acts and the warning', async () => {
    const limits = [{ unit: 'tokens' as const, amount: 400, period: 'week' as const, act: 'tell' as const },
      { unit: 'tokens' as const, amount: 500, period: 'week' as const, act: 'stop' as const }];
    const kept = { ...budgetsView(holder), limits, warn_at: 80, version: 7 };
    let sent: BudgetBody | null = null;
    await mount(file, { ...keeping(), [budgets]: ok(kept), ['PUT ' + budgets]: (body) => {
      sent = body as BudgetBody;
      return ok({ ...kept, ...sent, version: 8 });
    } });
    await click($('button[aria-label="Remove 400 tokens a week"]'));
    expect(sent).toEqual({ limits: [limits[1]], warn_at: 80, version: 7 });
    expect(text()).toContain('Budget kept as version 8.');
  });

  it('offers only the units Lys has a figure for, and says why the others are not offered', async () => {
    await mount(file, { ...keeping(), [budgets]: ok(budgetsView(holder, [], { unavailable: [{ unit: 'dollars', reason: 'no dollar spend is reported for this agent' }] })) });
    expect(document.querySelector<HTMLOptionElement>(addBudget + ' select[aria-label="Unit"] option[value="dollars"]')?.disabled).toBe(true);
    expect(document.querySelector<HTMLOptionElement>(addBudget + ' select[aria-label="Unit"] option[value="tokens"]')?.disabled).toBe(false);
    expect(text()).toContain('No dollar spend is reported for this agent.');
  });

  it('says why a unit has no figure once, in plain words, with no identifier', async () => {
    await mount(file, { ...keeping(), [budgets]: ok(budgetsView(holder, [], { unavailable: [
      { unit: 'dollars', reason: 'dollars have not been reported for agent ' + SCRIBE },
      { unit: 'plan_percent', reason: 'plan window unreported for agent ' + SCRIBE + '; plan window unreported for agent ' + SCRIBE },
    ] })) });
    const said = [...document.querySelectorAll('section[aria-label="Budgets"] .usage-reason')].map((line) => line.textContent);
    expect(said).toEqual(['No dollar spend has been reported for Scribe.', 'No plan window has been reported for Scribe.']);
    const budget = $('section[aria-label="Budgets"]')?.textContent ?? '';
    expect(budget).not.toContain(SCRIBE);
    expect(budget).not.toMatch(/agent-[0-9a-f]/);
    expect(budget).not.toContain('No limit');
    expect(budget.split('No budget is set for this agent.')).toHaveLength(2);
  });

  it('names an unavailable figure without displaying it as a measured zero', async () => {
    await mount(file, { ...keeping(), [budgets]: ok(budgetsView(holder, [tokens], {
      used: [{ unit: 'tokens', period: 'day', figure: null, since_ms: 0, unavailable: 'runner token report is missing' }],
    })) });
    expect($('section[aria-label="Budgets"] tbody tr')?.children[1]?.textContent).toBe('Nothing reported yet');
    expect($('section[aria-label="Budgets"] tbody tr')?.children[3]?.textContent).toBe('Runner token report is missing.');
    expect($('section[aria-label="Budgets"] tbody tr')?.textContent).not.toMatch(/\b0 tokens/);
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
    expect(rows[0]?.children[3]?.textContent).toContain('Its act was confirmed');
    expect(rows[1]?.children[3]?.textContent).toBe('Held: not reached.');
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
    // A control's own symbol is a drawing and not a chart; anything else drawn here would be one.
    const drawn = [...document.querySelectorAll('section.usage svg, section.usage canvas')].filter((each) => !each.closest('button.act'));
    expect(drawn).toHaveLength(0);
    expect(document.querySelectorAll('section.usage button.act svg').length).toBeGreaterThan(0);
  });
});

describe('The saved control transport', () => {
  it('names an unavailable setup without hiding budgets or claiming a terminal transport', async () => {
    await mount(file, { ...keeping(), ['/agents/' + SCRIBE + '/provisioning']: refused(503, 'ProvisioningUnavailable', 'profile store is unavailable') });
    expect(text()).toContain('Control setup unavailable: ProvisioningUnavailable');
    expect($('section[aria-label="Budgets"]')).not.toBeNull();
    expect(text()).not.toContain('Notices use the terminal');
  });

  it('shows terminal notices for an old setup and names the unqualified managed refusal after an explicit choice', async () => {
    for (const required of [false, true]) {
      const routes = { ...keeping(), ['/agents/' + SCRIBE + '/provisioning']: ok({ agent: SCRIBE,
        profile: { session: required ? { requires_controls: true } : null } }) };
      const { posted } = await mount(file, routes);
      expect(text()).toContain(required ? 'Managed controls are required' : 'Notices use the terminal');
      expect(text()).toContain('No managed adapter is qualified');
      if (required) expect(text()).toContain('control_adapter_unqualified');
      expect(posted).toEqual([]);
      unmountAll(); document.body.innerHTML = '';
    }
  });
});

async function managedControlView(control: unknown, receipts: unknown[], extra: Record<string, Route> = {}) {
  const { createRoot } = await import('react-dom/client');
  const { AgentUsage } = await import('../src/features/usage/Usage');
  const { serve } = await import('./harness');
  const session = 'session-old';
  const routes = {
    ...keeping(),
    ['/agents/' + SCRIBE + '/control-sessions']: ok({ agent: SCRIBE, sessions: [session], after: null }),
    ['/runtime/sessions/' + session + '/controls']: ok({ session, control }),
    ['/runtime/sessions/' + session + '/control-receipts']: ok({ session, receipts, after: null }),
    ...extra,
  };
  const posted: { path: string; body: unknown }[] = [];
  const requests = serve(routes, posted);
  const container = document.createElement('div');
  document.body.appendChild(container);
  const root = createRoot(container);
  await act(async () => { root.render(<AgentUsage agent={SCRIBE} />); });
  return { container, requests, posted, close: async () => { await act(async () => root.unmount()); container.remove(); } };
}

const currentControl = {
  generation: 1, phase: 'active', active: 'operation-active', context: { state: 'held', crossing: 'crossing', reason: 'above_limit' },
  boundary: null, crossing: 'crossing', queued: [],
};
const uncertainControl = {
  operation: 'delivery-uncertain', session: 'session-old', request: 'goal_reminder', state: 'uncertain', at: 1,
  text: null, prepared: false, certainty: 'possibly_sent', generation: null, uuid: null, reference: null, admitted: false, reconciled: null,
};

describe('managed control readback', () => {
  it('shows the active hold and unconfirmed delivery without assuming work completed', async () => {
    const view = await managedControlView(currentControl, [uncertainControl]);
    try {
      const panel = view.container.querySelector('section[aria-label="Managed controls"]');
      expect(panel).not.toBeNull();
      expect(panel?.textContent).toContain('Active turn');
      expect(panel?.textContent).toContain('Held: above limit');
      expect(panel?.textContent).toContain('Unconfirmed');
      expect(panel?.textContent).toContain('Possible prior delivery');
      expect(view.requests).toContain('/agents/' + SCRIBE + '/control-sessions');
      expect(view.requests.some((path) => path.endsWith('/runtime/sessions'))).toBe(false);
    } finally { await view.close(); }
  });

  it('names an unknown phase instead of displaying an idle boundary', async () => {
    const view = await managedControlView({ ...currentControl, phase: 'unknown-new-phase' }, []);
    try {
      expect(view.container.textContent).toContain('ControlStatusUnreadable');
      expect(view.container.textContent).not.toContain('Idle boundary');
    } finally { await view.close(); }
  });

  it('keeps one person-decision operation identity across an unconfirmed response and retry', async () => {
    let calls = 0;
    const path = '/runtime/sessions/session-old/control-receipts/delivery-uncertain/reconcile';
    const view = await managedControlView({ ...currentControl, phase: 'closed' }, [uncertainControl], {
      ['POST ' + path]: (body) => {
        calls += 1;
        const decision = body as { operation: string; decision: string };
        if (calls === 1) return refused(503, 'decision_unconfirmed', 'The decision write did not answer.');
        return ok({ ...uncertainControl, reconciled: { operation: decision.operation, by: 'responsible-person', at: 2, decision: { choice: decision.decision } } });
      },
    });
    try {
      const button = () => [...view.container.querySelectorAll('button')].find((element) => element.textContent === 'Record not seen');
      expect(button()).toBeDefined();
      await act(async () => { button()?.click(); });
      expect(view.container.textContent).toContain('decision_unconfirmed');
      await act(async () => { button()?.click(); });
      expect(view.posted.filter((entry) => entry.path === path)).toHaveLength(2);
      const [first, second] = view.posted.filter((entry) => entry.path === path);
      expect(second.body).toEqual(first.body);
      expect(view.container.textContent).toContain('Person recorded not seen');
      expect(view.container.textContent).toContain('Unconfirmed');
    } finally { await view.close(); }
  });
});


describe('resend recovery readback', () => {
  it('reopens an unconfirmed resend under the kept id instead of creating another delivery', async () => {
    const receipt = { ...uncertainControl, reference: { goal: 'goal', occurrence: 'old-occurrence', version: 'goal', prior: null } };
    const lookup = '/goals/goal/resends/delivery-uncertain';
    const resend = '/goals/goal/resend';
    const routes: Record<string, Route> = {
      [lookup]: ok({ goal: 'goal', prior: 'delivery-uncertain', occurrence: { operation: 'kept-resend', session: 'new-session' } }),
      ['POST ' + resend]: (body) => {
        const ask = body as { operation: string; prior: string; session: string };
        expect(ask).toEqual({ operation: 'kept-resend', prior: 'delivery-uncertain', session: 'new-session' });
        return ok({ goal: 'goal', occurrence: ask.operation, operation: 'kept-delivery', session: ask.session, prior: ask.prior,
          reconciliation: { ...receipt, reconciled: { operation: ask.operation, by: 'responsible-person', at: 2, decision: { choice: 'resent', occurrence: ask.operation } } } });
      },
    };
    for (let reload = 0; reload < 2; reload += 1) {
      const view = await managedControlView(currentControl, [receipt], routes);
      try {
        const button = (words: string) => [...view.container.querySelectorAll('button')].find((element) => element.textContent === words);
        await act(async () => { button('Resend options')?.click(); });
        expect(view.requests).toContain(lookup);
        expect(view.container.textContent).toContain('Finish the recorded resend');
        expect(button('Send a new occurrence')).toBeUndefined();
        await act(async () => { button('Finish the recorded resend')?.click(); });
        expect(view.posted.filter((entry) => entry.path === resend)).toHaveLength(1);
        expect(view.container.textContent).toContain('Person recorded resent under occurrence kept-resend');
        expect(view.container.textContent).toContain('Unconfirmed');
      } finally { await view.close(); }
    }
  });
});

describe('running session control readback', () => {
  it('reads only the session explicitly opened, without asking a history route', async () => {
    const { createRoot } = await import('react-dom/client');
    const { MemoryRouter } = await import('react-router');
    const { RunningList } = await import('../src/features/runtime/Sessions');
    const { serve } = await import('./harness');
    const requests = serve({ ...keeping(),
      '/runtime/live': ok({ sessions: [{ session: 'session-old', agent: SCRIBE, machine: 'machine', machine_name: 'Computer', shown: 'running', first_report_at: 1, last_reported: 'running' }], unanswered: [] }),
      '/runtime/sessions/session-old/controls': ok({ session: 'session-old', control: currentControl }),
      '/runtime/sessions/session-old/control-receipts': ok({ session: 'session-old', receipts: [uncertainControl], after: null }),
    });
    const container = document.createElement('div'); document.body.appendChild(container);
    const root = createRoot(container);
    try {
      await act(async () => { root.render(<MemoryRouter><RunningList /></MemoryRouter>); });
      expect(requests).not.toContain('/runtime/sessions/session-old/controls');
      const button = container.querySelector<HTMLButtonElement>('button[aria-label="Read controls for Scribe"]');
      expect(button).not.toBeNull();
      await act(async () => { button?.click(); });
      expect(container.textContent).toContain('Active turn');
      expect(container.textContent).toContain('Held: above limit');
      expect(requests).toContain('/runtime/sessions/session-old/controls');
      expect(requests).toContain('/runtime/sessions/session-old/control-receipts');
      expect(requests.some((path) => path.endsWith('/runtime/sessions'))).toBe(false);
    } finally { await act(async () => root.unmount()); container.remove(); }
  });
});
