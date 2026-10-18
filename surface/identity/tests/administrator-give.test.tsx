/** Give an own agent access without first granting it to the administrator. */
import { act } from 'react';
import { beforeEach, expect, it } from 'vitest';
import { $, choose, click, mount, pick, type, unmountAll } from './harness';
import { ADA, DIRECTORY, RECEIPTS, SCRIBE, SERVICE, ok, refused } from './fixtures';
import type { Route } from './fixtures';

beforeEach(() => sessionStorage.clear());

const KIND = 'cambium.workspace';
const routes: Record<string, Route> = {
  ...SERVICE,
  '/grants': ok({ grants: [], revision: 0 }),
  '/apps': ok({ apps: [{ id: 'cambium', name: 'Cambium', state: 'approved' }] }),
  '/apps/cambium/schema': ok({ app: 'cambium', version: 1, schema: { kinds: { [KIND]: {
    actions: ['read', 'write'], relations: { member: ['read'], owner: ['read', 'write'] }, agents: ['read'], parents: [],
  } } } }),
  '/grants/model': ok({ version: 1, relations: {}, withheld_from_agents: [], action_sentences: {}, kinds: {
    [KIND]: { relations: { member: ['read'], owner: ['read', 'write'] }, agents: ['read'] },
  } }),
  'POST /grants': (body: unknown) => ok({ operation: (body as { operation: string }).operation,
    grant: 'grant-recorded', index: RECEIPTS[4].receipt.log.index, receipt: { ...RECEIPTS[4].receipt, caller: ADA, revision: 1 } }),
};

function form(): HTMLElement {
  const page = $('form[aria-label="Give access"]');
  if (!(page instanceof HTMLElement)) throw new Error('No Give access form');
  return page;
}

it('gives membership to an own active agent from standing, with its mode and pass-on terms', async () => {
  const { posted } = await mount('#/access/issue', routes);
  const page = form();
  await choose(page.querySelector('select[aria-label="Give to"]'), 'agent');
  await pick(page, 'Find one of your agents', 'Scribe', 'Scribe');
  await choose(page.querySelector('select[name="kind"]'), KIND);
  await click(page.querySelector('.level-picker input[type="radio"][value="member"]'));
  await type(page.querySelector('input[name="resource_id"]'), 'ablative');
  await choose(page.querySelector('select[name="mode"]'), 'by_draft');
  await act(async () => { page.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })); });
  expect(page.textContent).toContain('Change recorded.');
  expect(posted).toHaveLength(1);
  expect(posted[0]).toMatchObject({ path: '/grants', body: {
    source: 'administrator', recipient: SCRIBE, responsible: ADA, relation: 'member',
    resource: { kind: KIND, id: 'ablative' }, mode: 'by_draft', pass_on: { kind: 'use_only' },
    window: { ends_at: null },
  } });
  expect((posted[0]?.body as Record<string, unknown>).holder).toBeUndefined();
});

it('offers only the signed-in person’s active agents and clears a person choice when switching', async () => {
  await mount('#/access/issue', routes);
  const page = form();
  await pick(page, 'Find a person', 'Ada', 'Ada (test person)');
  await choose(page.querySelector('select[aria-label="Give to"]'), 'agent');
  expect(page.querySelector('input[name="holder"]')).toBeNull();
  for (const person of DIRECTORY.people) {
    for (const agent of person.agents) {
      await type(page.querySelector('input[aria-label="Find one of your agents"]'), agent.display_name);
      expect([...page.querySelectorAll('.picker li button')].map((button) => button.textContent))
        .toEqual(agent.id === SCRIBE ? ['Scribe'] : []);
    }
  }
});

it('shows the server’s refusal when a gift is withheld from an agent', async () => {
  const { posted } = await mount('#/access/issue', {
    ...routes, 'POST /grants': refused(403, 'WithheldFromAgents', 'WithheldFromAgents: an agent cannot be given write'),
  });
  const page = form();
  await choose(page.querySelector('select[aria-label="Give to"]'), 'agent');
  await pick(page, 'Find one of your agents', 'Scribe', 'Scribe');
  await choose(page.querySelector('select[name="kind"]'), KIND);
  await click(page.querySelector('.level-picker input[type="radio"][value="owner"]'));
  await type(page.querySelector('input[name="resource_id"]'), 'ablative');
  await act(async () => { page.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })); });
  expect(posted).toHaveLength(1);
  expect(page.textContent).toContain('WithheldFromAgents');
});

it('keeps an uncertain standing gift across reload and checks the original operation', async () => {
  const first = await mount('#/access/issue', { ...routes, 'POST /grants': ok({}) });
  const page = form();
  await choose(page.querySelector('select[aria-label="Give to"]'), 'agent');
  await pick(page, 'Find one of your agents', 'Scribe', 'Scribe');
  await choose(page.querySelector('select[name="kind"]'), KIND);
  await click(page.querySelector('.level-picker input[type="radio"][value="member"]'));
  await type(page.querySelector('input[name="resource_id"]'), 'ablative');
  await act(async () => { page.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })); });
  expect(page.textContent).toContain('UnconfirmedReceipt');
  unmountAll();
  const next = await mount('#/access/issue', routes);
  expect(next.posted).toHaveLength(0);
  await click(form().querySelector('button[aria-label="Check original change"]'));
  expect(next.posted).toEqual(first.posted);
  expect(form().textContent).toContain('Change recorded.');
  expect(sessionStorage.getItem('lys.pending.root-grant')).toBeNull();
});
