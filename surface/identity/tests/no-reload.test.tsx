/** No screen offers a button to read again: a page reads because it opened or because something changed. */
import { budgetsView } from './budget-fixtures';
import type { Budget } from '../src/features/usage/contract';
import { act } from 'react';
import { beforeEach, describe, expect, it } from 'vitest';
import { $, $$, click, mount, text } from './harness';
import { ADA, SCRIBE, SERVICE, ok, refused } from './fixtures';
import type { Route } from './fixtures';

const again = /refresh|reload|read .* again/i;
const reloaders = () => $$('button').map((button) => button.textContent ?? '').filter((words) => again.test(words));
const memory = { agent: SCRIBE, home: false, memories: [], skipped: [], last_given: null, visible_to: { agent: SCRIBE, responsible: ADA, administrator: true }, notes_shown: false };
const budget = '/budgets/person/' + ADA;
const agentRoutes: Record<string, Route> = {
  ...SERVICE,
  ['/agents/' + SCRIBE + '/memory']: ok(memory),
  ['/agents/' + SCRIBE + '/policy']: ok({ agent: SCRIBE, policy: null, digest: null, applies: 'applies on the agent\'s next launch' }),
  ['/agents/' + SCRIBE + '/refusals']: ok({ agent: SCRIBE, refusals: [], read_from: [] }),
  ['/agents/' + SCRIBE + '/stops']: refused(503, 'StopsUnavailable', 'Stop log could not be read'),
  [budget]: refused(503, 'BudgetsUnavailable', 'cannot read'),
};
beforeEach(() => sessionStorage.clear());

describe('No read-again buttons', () => {
  for (const hash of ['#/file/' + SCRIBE + '/memory', '#/file/' + SCRIBE + '/policy', '#/file/' + SCRIBE + '/record', '#/file/' + ADA + '/budgets', '#/roles', '#/service-accounts']) {
    it('offers none on ' + hash, async () => {
      await mount(hash, agentRoutes);
      expect(reloaders()).toEqual([]);
    });
  }
  it('offers none on the teams list', async () => {
    await mount('#/people', agentRoutes);
    await click($('[data-kind="teams"]'));
    expect(reloaders()).toEqual([]);
  });
  it('shows the stop answer and the suspended file together, with nothing to press', async () => {
    let stopped = false;
    const path = '/agents/' + SCRIBE + '/stop';
    const { requests } = await mount('#/file/' + SCRIBE, { ...agentRoutes,
      ['/directory/agents/' + SCRIBE]: () => ok({ ...(SERVICE['/directory/agents/' + SCRIBE] as { body: object }).body, state: stopped ? 'suspended' : 'active' }),
      ['POST ' + path]: (body) => { stopped = true; return ok({ agent: SCRIBE, operation: (body as { operation: string }).operation, state: 'suspended', by: ADA, at: 1790000200, certificates_withdrawn: [], credentials_ended: [], credentials_refused: null, sessions_asked: [], reason: 'leaked its key' }); } });
    await click($('[data-act="stop"]'));
    const reason = $('form[aria-label="Confirm emergency stop"] input');
    if (!(reason instanceof HTMLInputElement)) throw new Error('Reason field missing');
    await act(async () => { Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')?.set?.call(reason, 'leaked its key'); reason.dispatchEvent(new Event('input', { bubbles: true })); });
    await click($$('form[aria-label="Confirm emergency stop"] button[type="submit"]')[0] ?? null);
    expect(requests.filter((entry) => entry === '/directory/agents/' + SCRIBE)).toHaveLength(1);
    expect($('#state')?.textContent).toBe('Suspended in this stop answer');
    expect($('[aria-label="Emergency stop recorded"]')).not.toBeNull();
    expect(reloaders()).toEqual([]);
  });
  it('keeps a refused confirmation\'s reason on screen without reading the budgets again', async () => {
    const effective: Budget = { holder: { kind: 'person', id: ADA }, measure: 'tokens', limit: 100, period: { length: 'day', zone: 'UTC' }, act: 'stop', version: 1, by: 'administrator', at: 1 };
    const requested = { ...effective, limit: 200, version: 2, by: ADA, at: 2 };
    const { requests } = await mount('#/file/' + ADA + '/budgets', { ...agentRoutes, [budget]: ok(budgetsView(effective.holder, [requested], { unconfirmed: [{ requested, effective, reason: 'waiting' }] })), ['POST ' + budget + '/confirm']: refused(409, 'BudgetVersionConflict', 'budget changed') });
    await click($('section[aria-label="Personal budgets"] button.primary'));
    expect(requests.filter((entry) => entry === budget)).toHaveLength(1);
    expect(text()).toContain('budget changed');
    expect(reloaders()).toEqual([]);
  });
});
