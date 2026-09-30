/** Personal confirmation preserves the enforced policy and trusts the subsequent read. */
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
  it('shows both full policies, confirms the exact version, and reads the stored result', async () => {
    let confirmed = false;
    const world = await mount('#/file/' + ADA + '/budgets', routes({
      [path]: () => ok(confirmed ? { holder: requested.holder, budgets: [{ ...requested, version: 3 }], unconfirmed: [] } : pending),
      ['POST ' + path + '/confirm']: () => { confirmed = true; return ok({ ...requested, version: 3 }); },
    }));
    expect(text()).toContain('Budgets for Ada (test person)');
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
    expect(world.requests.filter((entry) => entry === path)).toHaveLength(2);
    expect(document.querySelector('article[aria-label="Pending tokens"]')).toBeNull();
    expect(text()).toContain('Enforced budget');
  });
  it('keeps the owner read-only', async () => {
    const world = await mount('#/file/' + ADA + '/budgets', routes({ '/directory/people': refused(403, 'NotAdmitted', 'administrator only'), '/people': ok(OWN) }));
    expect(text()).toContain('An administrator must confirm this change');
    expect(document.querySelector('section[aria-label="Personal budgets"] button.primary')).toBeNull();
    expect(world.posted).toEqual([]);
  });
  for (const [status, kind, reason] of [[503, 'BudgetsUnavailable', 'upgrade_pending'], [409, 'BudgetVersionConflict', 'budget changed'], [500, 'BudgetsUnavailable', 'outcome unreadable']] as const) {
    it('reads the budgets again after ' + kind + ' ' + status, async () => {
      const world = await mount('#/file/' + ADA + '/budgets', routes({ ['POST ' + path + '/confirm']: refused(status, kind, reason) }));
      await click(confirm());
      expect(text()).toContain(kind + ': ' + reason);
      expect(text()).toContain('Currently enforced');
      expect(text()).toContain('Requested change');
      expect(world.requests.filter((entry) => entry === path)).toHaveLength(2);
      expect(world.posted).toHaveLength(1);
    });
  }
  it('does not infer completion from a successful POST if the fresh read says pending', async () => {
    const world = await mount('#/file/' + ADA + '/budgets', routes({ ['POST ' + path + '/confirm']: ok({ ...requested, version: 3 }) }));
    await click(confirm());
    expect(world.requests.filter((entry) => entry === path)).toHaveLength(2);
    expect(text()).toContain('Currently enforced');
    expect(text()).toContain('Requested change');
  });
  it('sends once while confirmation is pending and never shows success before the read', async () => {
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
  it('names a failed read after confirmation without pretending the requested change is enforced', async () => {
    let confirmed = false;
    await mount('#/file/' + ADA + '/budgets', routes({
      [path]: () => confirmed ? refused(503, 'BudgetsUnavailable', 'cannot read recorded budget') : ok(pending),
      ['POST ' + path + '/confirm']: () => { confirmed = true; return ok({ ...requested, version: 3 }); },
    }));
    await click(confirm());
    expect(text()).toContain('cannot read recorded budget');
    expect(document.querySelector('article[aria-label="Pending tokens"]')).toBeNull();
    expect(text()).not.toContain('Enforced budget');
  });

  it('removes stale confirmation controls while the authoritative follow-up read is pending', async () => {
    await mount('#/file/' + ADA + '/budgets', routes());
    let answer: ((value: Response) => void) | undefined;
    const pendingRead = new Promise<Response>((resolve) => { answer = resolve; });
    const original = fetch;
    let posts = 0;
    vi.stubGlobal('fetch', (input: RequestInfo | URL, init?: RequestInit) => {
      if (init?.method === 'POST') {
        posts += 1;
        return Promise.resolve(new Response(JSON.stringify(requested), { status: 200 }));
      }
      if (String(input).endsWith(path)) return pendingRead;
      return original(input, init);
    });
    await click(confirm());
    expect(posts).toBe(1);
    expect(confirm()).toBeNull();
    expect(text()).not.toContain('Enforced budget');
    if (!answer) throw new Error('fresh read resolver absent');
    const release = answer;
    await act(async () => release(new Response(JSON.stringify(pending), { status: 200 })));
    expect(text()).toContain('Requested change');
    expect(confirm()).not.toBeNull();
  });

});
