/** Editing controls preserve independent limits and recorded goal changes. */
import { act } from 'react';
import type { ReactElement } from 'react';
import { createRoot } from 'react-dom/client';
import type { Root } from 'react-dom/client';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { UsageBudgets } from '../src/features/usage/UsageBudgets';
import { UsageGoals } from '../src/features/usage/UsageGoals';
import type { BudgetBody, GoalItem, Limit } from '../src/features/usage/contract';
import { budgetsView } from './budget-fixtures';
import { SCRIBE, ok, refused } from './fixtures';
import { serve } from './harness';

const roots: Root[] = [];
const path = '/budgets/agent/' + SCRIBE;
const holder = { kind: 'agent' as const, id: SCRIBE };
const goal: GoalItem = { goal: { id: 'op-' + 'a'.repeat(32), kind: 'goal', words: 'Keep the directory available', deadline: null, active: true, evidence: null }, standing: 'open' };
const other: GoalItem = { goal: { ...goal.goal, id: 'op-' + 'b'.repeat(32), words: 'Keep the audit available' }, standing: 'met' };

afterEach(() => {
  for (const root of roots.splice(0)) act(() => root.unmount());
  sessionStorage.clear();
});

async function render(element: ReactElement): Promise<void> {
  const container = document.createElement('div');
  document.body.appendChild(container);
  const root = createRoot(container);
  roots.push(root);
  await act(async () => { root.render(element); });
}

function element<T extends HTMLElement>(selector: string): T {
  const found = document.querySelector<T>(selector);
  if (!found) throw new Error('Missing control: ' + selector);
  return found;
}

async function input(selector: string, value: string): Promise<void> {
  const found = element<HTMLInputElement>(selector);
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')?.set?.call(found, value);
    found.dispatchEvent(new Event('input', { bubbles: true }));
  });
}

async function click(selector: string): Promise<void> {
  await act(async () => { element(selector).click(); });
}

async function choose(selector: string, value: string): Promise<void> {
  await act(async () => {
    const found = element<HTMLSelectElement>(selector);
    found.value = value;
    found.dispatchEvent(new Event('change', { bubbles: true }));
  });
}

const edit = 'form[aria-label="Edit budget limits"]';
const words = 'form[aria-label="Reword goal ' + goal.goal.id + '"]';
const active = 'input[aria-label="Goal active ' + goal.goal.id + '"]';

describe('Usage port', () => {
  it('edits money and minutes while retaining every independent action, zone and other limit', async () => {
    const limits: Limit[] = [
      { unit: 'dollars', amount: 12.5, period: 'week', act: 'tell' },
      { unit: 'running_ms', amount: 120000, period: 'day', act: 'stop', zone: 'Australia/Melbourne' },
      { unit: 'tokens', amount: 500, period: 'month', act: 'notice' },
    ];
    const view = budgetsView(holder, [], { limits, warn_at: 80, version: 7 });
    const posted: { path: string; body: unknown }[] = [];
    const changed = vi.fn();
    serve({ ['PUT ' + path]: (body) => ok({ ...view, ...body as BudgetBody, version: 8 }) }, posted);
    await render(<UsageBudgets budgets={view} receipts={[]} changed={changed} />);
    expect(element<HTMLInputElement>(edit + ' input[name="amount-0"]').value).toBe('12.5');
    expect(element<HTMLInputElement>(edit + ' input[name="amount-1"]').value).toBe('2');
    await input(edit + ' input[name="amount-0"]', '25.75');
    await input(edit + ' input[name="amount-1"]', '3');
    await click(edit + ' button[type="submit"]');
    expect(posted).toEqual([{ path: 'PUT ' + path, body: { limits: [{ ...limits[0], amount: 25.75 }, { ...limits[1], amount: 180000 }, limits[2]], warn_at: 80, version: 7 } }]);
    expect(changed).toHaveBeenCalledWith('Budget kept as version 8.');
  });

  it('keeps a version refusal visible without confirming a budget edit', async () => {
    const changed = vi.fn();
    serve({ ['PUT ' + path]: refused(409, 'BudgetVersionConflict', 'The limits changed elsewhere') });
    await render(<UsageBudgets budgets={budgetsView(holder, [], { limits: [{ unit: 'tokens', amount: 500, period: 'week', act: 'stop' }] })} receipts={[]} changed={changed} />);
    await input(edit + ' input[name="amount-0"]', '600');
    await click(edit + ' button[type="submit"]');
    expect(document.body.textContent).toContain('BudgetVersionConflict');
    expect(changed).not.toHaveBeenCalled();
  });

  it('changes one action and the warning while preserving every amount and the other action', async () => {
    const limits: Limit[] = [{ unit: 'tokens', amount: 400, period: 'week', act: 'tell' }, { unit: 'tokens', amount: 500, period: 'week', act: 'stop' }];
    const view = budgetsView(holder, [], { limits, warn_at: 80, version: 7 });
    const posted: { path: string; body: unknown }[] = [];
    const changed = vi.fn();
    serve({ ['PUT ' + path]: (body) => ok({ ...view, ...body as BudgetBody, version: 8 }) }, posted);
    await render(<UsageBudgets budgets={view} receipts={[]} changed={changed} />);
    await choose(edit + ' select[name="act-0"]', 'notice');
    await input(edit + ' input[name="warn_at"]', '70');
    await click(edit + ' button[type="submit"]');
    expect(posted).toEqual([{ path: 'PUT ' + path, body: { limits: [{ ...limits[0], act: 'notice' }, limits[1]], warn_at: 70, version: 7 } }]);
    expect(changed).toHaveBeenCalledWith('Budget kept as version 8.');
  });

  it('does not confirm an answer that changes an unrelated budget action', async () => {
    const changed = vi.fn();
    const view = budgetsView(holder, [], { limits: [{ unit: 'tokens', amount: 500, period: 'week', act: 'stop' }], version: 7 });
    serve({ ['PUT ' + path]: ok({ ...view, limits: [{ ...view.limits[0], amount: 600, act: 'tell' }], version: 8 }) });
    await render(<UsageBudgets budgets={view} receipts={[]} changed={changed} />);
    await input(edit + ' input[name="amount-0"]', '600');
    await click(edit + ' button[type="submit"]');
    expect(changed).not.toHaveBeenCalled();
    expect(document.body.textContent).toContain('BudgetAnswerUnconfirmed');
    expect(element<HTMLButtonElement>(edit + ' button[type="submit"]').disabled).toBe(true);
    expect(element<HTMLButtonElement>('tr[data-add="budget"] button[type="submit"]').disabled).toBe(true);
  });

  it('refuses an empty or negative edited amount before sending a request', async () => {
    const posted: { path: string; body: unknown }[] = [];
    serve({}, posted);
    await render(<UsageBudgets budgets={budgetsView(holder, [], { limits: [{ unit: 'tokens', amount: 500, period: 'week', act: 'tell' }] })} receipts={[]} changed={vi.fn()} />);
    for (const value of ['', '-1', '0.5', '9007199254740992']) {
      await input(edit + ' input[name="amount-0"]', value);
      expect(element<HTMLButtonElement>(edit + ' button[type="submit"]').disabled).toBe(true);
    }
    expect(posted).toEqual([]);
  });

  it('sets shared-plan percentages only in a reported plan window', async () => {
    const view = budgetsView(holder);
    const posted: { path: string; body: unknown }[] = [];
    const changed = vi.fn();
    serve({ ['PUT ' + path]: (body) => ok({ ...view, ...body as BudgetBody, version: 1 }) }, posted);
    await render(<UsageBudgets budgets={view} receipts={[]} changed={changed} />);
    const form = 'tr[data-add="budget"]';
    await choose(form + ' select', 'plan_percent');
    expect(Array.from(document.querySelectorAll<HTMLSelectElement>(form + ' select'))[1].value).toBe('five_hour');
    await input(form + ' input[name="limit"]', '101');
    expect(element<HTMLButtonElement>(form + ' button[type="submit"]').disabled).toBe(true);
    await input(form + ' input[name="limit"]', '50');
    await click(form + ' button[type="submit"]');
    expect(posted).toEqual([{ path: 'PUT ' + path, body: { limits: [{ unit: 'plan_percent', amount: 50, period: 'five_hour', act: 'tell' }], warn_at: null, version: 0 } }]);
    expect(changed).toHaveBeenCalledWith('Budget kept as version 1.');
  });

  it('names the holder in place of its identifier and says each reason once', async () => {
    const view = budgetsView(holder, [], {
      limits: [{ unit: 'dollars', amount: 5, period: 'day', act: 'tell' }],
      used: [{ unit: 'dollars', period: 'day', figure: null, since_ms: 0, unavailable: 'dollars have not been reported for agent ' + SCRIBE }],
      unavailable: [
        { unit: 'dollars', reason: 'dollars have not been reported for agent ' + SCRIBE },
        { unit: 'plan_percent', reason: 'plan window unreported for agent ' + SCRIBE + '; plan window unreported for agent ' + SCRIBE },
      ],
    });
    serve({});
    await render(<UsageBudgets budgets={view} receipts={[]} changed={vi.fn()} name="pancake" />);
    const page = document.body.textContent ?? '';
    expect(page).not.toContain(SCRIBE);
    expect(element('tbody tr').children[3].textContent).toBe('No dollar spend has been reported for pancake.');
    expect([...document.querySelectorAll('.usage-reason')].map((line) => line.textContent)).toEqual(['No plan window has been reported for pancake.']);
    for (const sentence of ['No dollar spend has been reported for pancake.', 'No plan window has been reported for pancake.']) {
      expect(page.split(sentence)).toHaveLength(2);
    }
  });

  it('rewords one current-shape goal without rewriting its other fields or another goal', async () => {
    const posted: { path: string; body: unknown }[] = [];
    const changed = vi.fn();
    serve({ ['POST /goals/' + goal.goal.id + '/words']: (body) => ok({ ...goal, goal: { ...goal.goal, words: (body as { words: string }).words } }) }, posted);
    await render(<UsageGoals agent={SCRIBE} goals={[goal, other]} changed={changed} />);
    await input(words + ' input[name="words"]', '  Keep all records available  ');
    await click('button[form="reword-' + goal.goal.id + '"]');
    expect(posted).toEqual([{ path: '/goals/' + goal.goal.id + '/words', body: { operation: expect.stringMatching(/^op-[0-9a-f]{32}$/), words: 'Keep all records available' } }]);
    expect(changed).toHaveBeenCalledWith('Goal reworded.');
    expect([...document.querySelectorAll<HTMLInputElement>('input[name=\"words\"]')].map((each) => each.defaultValue)).toContain(other.goal.words);
  });

  it.each([false, true])('switches goal activity to %s with an independent recorded operation', async (next) => {
    const item = { ...goal, goal: { ...goal.goal, active: !next } };
    const posted: { path: string; body: unknown }[] = [];
    const changed = vi.fn();
    serve({ ['POST /goals/' + goal.goal.id + '/active']: (body) => ok({ ...item, goal: { ...item.goal, active: (body as { active: boolean }).active } }) }, posted);
    await render(<UsageGoals agent={SCRIBE} goals={[item, other]} changed={changed} />);
    expect(element<HTMLInputElement>(active).checked).toBe(!next);
    await click(active);
    expect(posted).toEqual([{ path: '/goals/' + goal.goal.id + '/active', body: { operation: expect.stringMatching(/^op-[0-9a-f]{32}$/), active: next } }]);
    expect(changed).toHaveBeenCalledWith(next ? 'Goal switched on.' : 'Goal switched off.');
  });

  it('names a refused goal change and leaves the saved words visible', async () => {
    const changed = vi.fn();
    serve({ ['POST /goals/' + goal.goal.id + '/words']: refused(403, 'NotPermitted', 'The goal cannot be changed by this caller') });
    await render(<UsageGoals agent={SCRIBE} goals={[goal]} changed={changed} />);
    await input(words + ' input[name="words"]', 'Changed words');
    await click('button[form="reword-' + goal.goal.id + '"]');
    expect(document.body.textContent).toContain('NotPermitted');
    expect(changed).not.toHaveBeenCalled();
    expect(document.body.textContent).toContain('Saved words: ' + goal.goal.words);
  });

  it('retains and retries the exact uncertain goal edit after remount', async () => {
    const posted: { path: string; body: unknown }[] = [];
    const changed = vi.fn();
    serve({ ['POST /goals/' + goal.goal.id + '/words']: refused(503, 'GoalsUnavailable', 'The answer is unknown') }, posted);
    await render(<UsageGoals agent={SCRIBE} goals={[goal]} changed={changed} />);
    await input(words + ' input[name="words"]', 'Keep records available');
    await click('button[form="reword-' + goal.goal.id + '"]');
    expect(posted).toHaveLength(1);
    expect(changed).not.toHaveBeenCalled();
    for (const root of roots.splice(0)) act(() => root.unmount());
    document.body.innerHTML = '';
    serve({ ['POST /goals/' + goal.goal.id + '/words']: ok({ ...goal, goal: { ...goal.goal, words: 'Keep records available' } }) }, posted);
    await render(<UsageGoals agent={SCRIBE} goals={[goal]} changed={changed} />);
    await click(words + ' button[type="button"]');
    expect(posted).toHaveLength(2);
    expect(posted[1]).toEqual(posted[0]);
    expect(changed).toHaveBeenCalledWith('Goal reworded.');
  });

  it('never confirms an edit whose answer names another goal', async () => {
    const changed = vi.fn();
    serve({ ['POST /goals/' + goal.goal.id + '/words']: ok({ ...other, goal: { ...other.goal, words: 'Changed words' } }) });
    await render(<UsageGoals agent={SCRIBE} goals={[goal]} changed={changed} />);
    await input(words + ' input[name="words"]', 'Changed words');
    await click('button[form="reword-' + goal.goal.id + '"]');
    expect(changed).not.toHaveBeenCalled();
    expect(document.body.textContent).toContain('The answer did not confirm this recorded change');
  });
});
