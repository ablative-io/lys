/** Service-backed DOM acceptance; process qualification and browser rendering need their own evidence. */
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { beforeEach, describe, expect, it } from 'vitest';
import { App } from '../../src/App';
import { serve } from '../harness';
import { SCRIBE, SERVICE, ok, refused } from '../fixtures';
import type { Route } from '../fixtures';
import { budgetsView } from '../budget-fixtures';
import type { BudgetBody, GoalItem } from '../../src/features/usage/contract';

const holder = { kind: 'agent' as const, id: SCRIBE };
const agent = '/agents/' + SCRIBE;
const budget = '/budgets/agent/' + SCRIBE;
const session = 'control-session';
const current = '/runtime/sessions/' + session;
const uncertain = {
  operation: 'possible-delivery', session, request: 'goal_reminder', state: 'uncertain', at: 1,
  text: null, prepared: false, certainty: 'possibly_sent', generation: null, uuid: null,
  reference: null, admitted: false, reconciled: null,
};
const status = {
  generation: 1, phase: 'idle', active: null, context: { state: 'held', crossing: 'crossing', reason: 'delivery_uncertain' },
  boundary: 'boundary', crossing: 'crossing', queued: [],
};

beforeEach(() => sessionStorage.clear());

function routes(extra: Record<string, Route> = {}): Record<string, Route> {
  return {
    ...SERVICE,
    [budget]: ok(budgetsView(holder)),
    [agent + '/usage']: ok({ agent: SCRIBE, used: [], receipts: [], last_reported_ms: 1 }),
    [agent + '/goals']: ok({ goals: [] }),
    [agent + '/provisioning']: ok({ agent: SCRIBE, profile: { session: { requires_controls: true } } }),
    [agent + '/control-sessions']: ok({ agent: SCRIBE, sessions: [session], after: null }),
    [current + '/controls']: ok({ session, control: status }),
    [current + '/control-receipts']: ok({ session, receipts: [uncertain], after: null }),
    ...extra,
  };
}

async function open(answer: Record<string, Route>) {
  const posted: { path: string; body: unknown }[] = [];
  const requests = serve(answer, posted);
  location.hash = '#/file/' + SCRIBE + '/budgets';
  const container = document.createElement('div');
  document.body.appendChild(container);
  const root = createRoot(container);
  await act(async () => { root.render(createElement(App)); });
  return { container, posted, requests, close: async () => {
    await act(async () => root.unmount());
    container.remove();
  } };
}

async function input(container: HTMLElement, selector: string, value: string) {
  const found = container.querySelector(selector);
  if (!(found instanceof HTMLInputElement || found instanceof HTMLTextAreaElement || found instanceof HTMLSelectElement)) throw new Error('control missing: ' + selector);
  const kind = found instanceof HTMLInputElement ? HTMLInputElement : found instanceof HTMLTextAreaElement ? HTMLTextAreaElement : HTMLSelectElement;
  await act(async () => {
    Object.getOwnPropertyDescriptor(kind.prototype, 'value')?.set?.call(found, value);
    found.dispatchEvent(new Event(found instanceof HTMLSelectElement ? 'change' : 'input', { bubbles: true }));
  });
}

async function press(container: HTMLElement, words: string) {
  const found = [...container.querySelectorAll('button')].find((button) => button.textContent === words || button.getAttribute('aria-label') === words);
  if (!found) throw new Error('button missing: ' + words);
  await act(async () => found.click());
}

const words = (container: HTMLElement) => container.textContent ?? '';

describe('Managed controls on the agent file', () => {
  it('keeps a context limit and a goal across a new visit while saying the saved control setup cannot start', async () => {
    let kept = budgetsView(holder);
    const goals: GoalItem[] = [];
    const answer = routes({
      [budget]: () => ok(kept),
      ['PUT ' + budget]: (body) => {
        const given = body as BudgetBody;
        expect(given.limits).toEqual([{ unit: 'context_percent', amount: 80, period: null, act: 'compact' }]);
        kept = { ...kept, ...given, version: kept.version + 1, used: [{ unit: 'context_percent', period: null, figure: 85, since_ms: null, unavailable: null }] };
        return ok(kept);
      },
      [agent + '/goals']: () => ok({ goals }),
      ['POST ' + agent + '/goals']: (body) => {
        const given = body as { operation: string; words: string; kind: 'goal'; deadline: null };
        const goal: GoalItem = { goal: { id: given.operation, words: given.words, kind: given.kind, deadline: given.deadline, active: true, evidence: null }, standing: 'open' };
        goals.push(goal); return ok(goal);
      },
    });
    let view = await open(answer);
    try {
      expect(words(view.container)).toContain('Managed controls are required');
      expect(words(view.container)).toContain('control_adapter_unqualified');
      await input(view.container, 'select[aria-label="Unit"]', 'context_percent');
      await input(view.container, 'input[name="limit"]', '80');
      await input(view.container, 'select[aria-label="When it\'s hit"]', 'compact');
      await press(view.container, 'Add this limit');
      await input(view.container, 'textarea[name="words"]', 'Keep the service available');
      await press(view.container, 'Set this goal');
      expect(kept.version).toBe(1);
      expect(goals).toHaveLength(1);
      expect(view.posted.filter((post) => post.path.includes('/runtime/'))).toEqual([]);
    } finally { await view.close(); }
    view = await open(answer);
    try {
      expect(view.container.querySelector<HTMLInputElement>('input[name="amount-0"]')?.value).toBe('80');
      expect(view.container.querySelector<HTMLSelectElement>('select[name="act-0"]')?.value).toBe('compact');
      expect(words(view.container)).toContain('85% of 80% now');
      expect(view.container.querySelector<HTMLInputElement>('section[aria-label="Goals"] input[name="words"]')?.value).toBe('Keep the service available');
      expect(words(view.container)).toContain('Held: delivery uncertain');
      expect(words(view.container)).not.toContain('Compaction confirmed');
    } finally { await view.close(); }
  });

  it('keeps uncertainty separate from a person decision and retries the same unconfirmed decision', async () => {
    const path = current + '/control-receipts/possible-delivery/reconcile';
    let calls = 0;
    const view = await open(routes({ ['POST ' + path]: (body) => {
      const choice = body as { operation: string; decision: string };
      calls += 1;
      if (calls === 1) return refused(503, 'decision_unconfirmed', 'The decision response was lost.');
      return ok({ ...uncertain, reconciled: { operation: choice.operation, by: 'responsible-person', at: 2, decision: { choice: choice.decision } } });
    } }));
    try {
      expect(words(view.container)).toContain('Unconfirmed');
      expect(words(view.container)).toContain('It will not be resent automatically');
      expect(view.posted.filter((post) => post.path === path)).toEqual([]);
      await press(view.container, 'Record not seen');
      expect(words(view.container)).toContain('decision_unconfirmed');
      const opposite = [...view.container.querySelectorAll<HTMLButtonElement>('button')].find((button) => button.textContent === 'Record seen');
      expect(opposite?.disabled).toBe(true);
      await press(view.container, 'Record not seen');
      const attempts = view.posted.filter((post) => post.path === path);
      expect(attempts).toHaveLength(2);
      expect(attempts[1].body).toEqual(attempts[0].body);
      expect(words(view.container)).toContain('Person recorded not seen');
      expect(words(view.container)).toContain('The harness outcome remains Unconfirmed');
      expect(words(view.container)).not.toContain('Compaction confirmed');
    } finally { await view.close(); }
  });

  it('reads the next receipt page only on request and refuses a page belonging to another session', async () => {
    const path = current + '/control-receipts?after=possible-delivery';
    const view = await open(routes({
      [current + '/control-receipts']: ok({ session, receipts: [uncertain], after: uncertain.operation }),
      [path]: ok({ session: 'another-session', receipts: [], after: null }),
    }));
    try {
      expect(view.requests).not.toContain(path);
      expect(words(view.container)).toContain('Held: delivery uncertain');
      await press(view.container, 'More operations');
      expect(view.requests).toContain(path);
      expect(words(view.container)).toContain('ControlReceiptsUnreadable');
      expect(words(view.container)).not.toContain('No control operation is recorded on this page');
      expect(view.requests.some((request) => request === '/runtime/sessions' || request.includes('/output') || request.includes('/history'))).toBe(false);
      expect(view.posted.filter((post) => post.path.startsWith('/runtime/'))).toEqual([]);
    } finally { await view.close(); }
  });
});
