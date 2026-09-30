/** Confirmation preserves the enforced policy until a matching answer advances its version. */
import { act } from 'react';
import { describe, expect, it, vi } from 'vitest';
import { $, click, mount, text } from './harness';
import { ADA, OWN, SERVICE, ok, refused } from './fixtures';
import type { Route } from './fixtures';
import type { Budget } from '../src/features/usage/contract';
const path = '/budgets/person/' + ADA;
const effective: Budget = { holder: { kind: 'person', id: ADA }, measure: 'tokens', limit: 100, period: { length: 'day', zone: 'Australia/Melbourne' }, act: 'stop', version: 1, by: 'administrator', at: 1 };
const requested: Budget = { ...effective, limit: 200, period: { length: 'week', zone: 'Australia/Melbourne' }, act: 'tell', version: 2, by: ADA, at: 2 };
const pending = { holder: requested.holder, budgets: [requested], unconfirmed: [{ requested, effective, reason: 'An earlier stricter budget remains effective until administrator confirmation.' }] };
const confirm = () => $('section[aria-label="Personal budgets"] button.primary');
const routes = (more: Record<string, Route> = {}): Record<string, Route> => ({ ...SERVICE, [path]: ok(pending), ...more });
describe('personal budgets', () => {
  it('shows both full policies and applies the policy returned for the exact requested version', async () => {
    const confirmed = { ...requested, limit: 150, version: 3 };
    const world = await mount('#/file/' + ADA + '/budgets', routes({
      ['POST ' + path + '/confirm']: ok(confirmed),
    }));
    expect(text()).toContain('Limits across Ada (test person)’s agents');
    const before = $('section[aria-label="Currently enforced"]');
    expect(before?.textContent).toContain('100');
    expect(before?.textContent).toContain('Each day, Australia/Melbourne');
    expect(before?.textContent).toContain('End the session');
    const after = $('section[aria-label="Requested change"]');
    expect(after?.textContent).toContain('200');
    expect(after?.textContent).toContain('Each week, Australia/Melbourne');
    expect(after?.textContent).toContain('Tell the responsible person');
    await click(confirm());
    expect(world.posted).toEqual([{ path: path + '/confirm', body: { measure: 'tokens', version: 2 } }]);
    expect(world.requests.filter((entry) => entry === path)).toHaveLength(1);
    expect(document.querySelector('article[aria-label="Pending tokens"]')).toBeNull();
    expect(text()).toContain('Enforced budget');
    const enforced = $('section[aria-label="Personal budgets"] article');
    expect(enforced?.textContent).toContain('150');
    expect(enforced?.textContent).toContain('Each week, Australia/Melbourne');
    expect(enforced?.textContent).toContain('Version3');
    expect(confirm()).toBeNull();
  });
  it('keeps the owner read-only', async () => {
    const world = await mount('#/file/' + ADA + '/budgets', routes({ '/directory/people': refused(403, 'NotAdmitted', 'administrator only'), '/people': ok(OWN) }));
    expect(text()).toContain('An administrator must confirm this change');
    expect(document.querySelector('section[aria-label="Personal budgets"] button.primary')).toBeNull();
    expect(world.posted).toEqual([]);
  });
  for (const [status, kind, reason] of [[503, 'BudgetsUnavailable', 'upgrade_pending'], [409, 'BudgetVersionConflict', 'budget changed'], [500, 'BudgetsUnavailable', 'outcome unreadable']] as const) {
    it('keeps the pending policy and names ' + kind + ' ' + status + ' without another read', async () => {
      const world = await mount('#/file/' + ADA + '/budgets', routes({ ['POST ' + path + '/confirm']: refused(status, kind, reason) }));
      await click(confirm());
      expect(text()).toContain(kind + ': ' + reason);
      expect(text()).toContain('Currently enforced');
      expect(text()).toContain('Requested change');
      expect(world.requests.filter((entry) => entry === path)).toHaveLength(1);
      expect(world.posted).toHaveLength(1);
    });
  }
  for (const [what, answer] of [
    ['an unchanged version', requested],
    ['another person', { ...requested, holder: { ...requested.holder, id: ADA + '-other' }, version: 3 }],
    ['another holder kind', { ...requested, holder: { ...requested.holder, kind: 'agent' }, version: 3 }],
    ['another measure', { ...requested, measure: 'running_ms', version: 3 }],
  ] as const) {
    it('does not treat ' + what + ' as confirmation', async () => {
      const world = await mount('#/file/' + ADA + '/budgets', routes({ ['POST ' + path + '/confirm']: ok(answer) }));
      await click(confirm());
      expect(world.requests.filter((entry) => entry === path)).toHaveLength(1);
      expect(text()).toContain('The answer did not confirm the requested budget.');
      expect(text()).toContain('Currently enforced');
      expect(text()).toContain('Requested change');
      expect(text()).not.toContain('Enforced budget');
      expect(confirm()).not.toBeNull();
    });
  }
  it('sends once while confirmation is pending and never shows success before the answer', async () => {
    await mount('#/file/' + ADA + '/budgets', routes());
    let answer: ((value: Response) => void) | undefined;
    const response = new Promise<Response>((resolve) => { answer = resolve; });
    const original = fetch;
    let sent = 0;
    vi.stubGlobal('fetch', (input: RequestInfo | URL, init?: RequestInit) => {
      if (init?.method === 'POST') { sent += 1; return response; }
      return original(input, init);
    });
    const button = confirm();
    if (!button) throw new Error('confirmation button absent');
    await act(async () => {
      button.dispatchEvent(new MouseEvent('click', { bubbles: true }));
      button.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    });
    expect(sent).toBe(1);
    expect((button as HTMLButtonElement).disabled).toBe(true);
    expect(text()).toContain('stored result is not yet known');
    if (!answer) throw new Error('response resolver absent');
    const release = answer;
    await act(async () => release(new Response(JSON.stringify(requested), { status: 200 })));
    expect(text()).toContain('Currently enforced');
  });
  it('uses a confirmed answer without depending on a subsequent read', async () => {
    let confirmed = false;
    const world = await mount('#/file/' + ADA + '/budgets', routes({
      [path]: () => confirmed ? refused(503, 'BudgetsUnavailable', 'cannot read recorded budget') : ok(pending),
      ['POST ' + path + '/confirm']: () => { confirmed = true; return ok({ ...requested, version: 3 }); },
    }));
    await click(confirm());
    expect(world.requests.filter((entry) => entry === path)).toHaveLength(1);
    expect(text()).not.toContain('cannot read recorded budget');
    expect(document.querySelector('article[aria-label="Pending tokens"]')).toBeNull();
    expect(text()).toContain('Enforced budget');
  });

  it('keeps confirmation unavailable until the pending answer confirms the requested version', async () => {
    const world = await mount('#/file/' + ADA + '/budgets', routes());
    let answer: ((value: Response) => void) | undefined;
    const pendingAnswer = new Promise<Response>((resolve) => { answer = resolve; });
    const original = fetch;
    let posts = 0;
    vi.stubGlobal('fetch', (input: RequestInfo | URL, init?: RequestInit) => {
      if (init?.method === 'POST') {
        posts += 1;
        return pendingAnswer;
      }
      return original(input, init);
    });
    const button = confirm();
    if (!button) throw new Error('confirmation button absent');
    await act(async () => button.click());
    expect(posts).toBe(1);
    expect((button as HTMLButtonElement).disabled).toBe(true);
    expect(text()).not.toContain('Enforced budget');
    expect(text()).toContain('Currently enforced');
    if (!answer) throw new Error('confirmation resolver absent');
    const release = answer;
    await act(async () => release(new Response(JSON.stringify({ ...requested, version: 3 }), { status: 200 })));
    expect(confirm()).toBeNull();
    expect(text()).not.toContain('Requested change');
    expect(text()).toContain('Enforced budget');
    expect(world.requests.filter((entry) => entry === path)).toHaveLength(1);
  });

});
