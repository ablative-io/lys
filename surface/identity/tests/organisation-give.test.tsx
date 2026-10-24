/**
 * Standing in the organisation is given from two places and lands the same way (24 October 2026): Give access offers
 * Lys's own organisation by name, to a person or one of my agents, with its standing relations alone; an agent's own
 * file offers the same give to whoever holds organisation.grant. Tom's walk of 24 October found neither place.
 */
import { act } from 'react';
import { beforeEach, expect, it } from 'vitest';
import { $, $$, choose, click, mount, pick, settle } from './harness';
import { ADA, DIRECTORY, GRANTS, ME, MODEL, ORGANISATION, RECEIPTS, SCRIBE, SERVICE, ok } from './fixtures';
import type { Route } from './fixtures';
import type { PeopleView } from '../src/generated';

beforeEach(() => sessionStorage.clear());

const CY = 'person-' + 'c'.repeat(32);
const PEOPLE: PeopleView = { ...DIRECTORY, people: [...DIRECTORY.people, { id: CY, display_name: 'Cy (test person)', state: 'active', agents: [] }] };
const ORGANISATION_ACTIONS = ['organisation.administer', 'organisation.admit', 'organisation.grant', 'organisation.read'];
const model = { ...MODEL, relations: { ...MODEL.relations, administrator: ORGANISATION_ACTIONS }, action_sentences: { ...MODEL.action_sentences,
  'organisation.administer': 'Administer this organisation', 'organisation.admit': 'Add people to this organisation',
  'organisation.grant': 'Give access directly in this organisation', 'organisation.read': 'Read this organisation' } };
const GIVEN = 'grant-' + 'f'.repeat(32);
const recorded = (body: unknown) => ok({ operation: (body as { operation: string }).operation, grant: GIVEN, index: RECEIPTS[4].receipt.log.index,
  receipt: { ...RECEIPTS[4].receipt, caller: ADA, revision: 1 } });
const routes: Record<string, Route> = { ...SERVICE, '/grants/model': ok(model), '/apps': ok({ apps: [] }), '/people': ok(PEOPLE), '/directory/people': ok(PEOPLE),
  'POST /grants': recorded, 'POST /grants/roots': recorded };

function form(): HTMLElement {
  const page = $('form[aria-label="Give access"]');
  if (!(page instanceof HTMLElement)) throw new Error('No Give access form');
  return page;
}
const submit = (page: HTMLElement) => act(async () => { page.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })); });
const options = (select: Element | null) => [...(select?.querySelectorAll('option') ?? [])].map((option) => option.textContent);

it('gives one of my agents administrator standing in the organisation from Give access', async () => {
  const { posted } = await mount('#/access/issue', routes);
  const page = form();
  await choose(page.querySelector('select[aria-label="Give to"]'), 'agent');
  await pick(page, 'Find one of your agents', 'Scribe', 'Scribe');
  expect(page.querySelector('select[name="kind"] optgroup[label="Lys"] option[value="organisation"]')?.textContent).toBe('Organisation');
  await choose(page.querySelector('select[name="kind"]'), 'organisation');
  expect(options(page.querySelector('select[name="resource"]'))).toContain('Example organisation');
  await choose(page.querySelector('select[name="resource"]'), ORGANISATION);
  // The organisation's standing relations alone, by sentence: no View, Edit or Own of an organisation.
  expect(page.querySelector('.level-picker input[type="radio"][value="owner"]')).toBeNull();
  expect(page.querySelector('.level-picker')?.textContent).toContain('Add people to this organisation');
  await click(page.querySelector('.level-picker input[type="radio"][value="administrator"]'));
  await submit(page);
  expect(posted).toHaveLength(1);
  expect(posted[0]).toMatchObject({ path: '/grants', body: {
    source: 'administrator', recipient: SCRIBE, responsible: ADA, relation: 'administrator',
    resource: { kind: 'organisation', id: ORGANISATION }, pass_on: { kind: 'use_only' },
  } });
  expect(document.querySelector('.next-step')?.textContent).toContain('Scribe is now administrator of Example organisation.');
});

it('gives a person administrator standing in the organisation as a root grant', async () => {
  const { posted } = await mount('#/access/issue', routes);
  const page = form();
  await pick(page, 'Find a person', 'Cy', 'Cy (test person)');
  await choose(page.querySelector('select[name="kind"]'), 'organisation');
  await choose(page.querySelector('select[name="resource"]'), ORGANISATION);
  await click(page.querySelector('.level-picker input[type="radio"][value="administrator"]'));
  await submit(page);
  expect(posted).toHaveLength(1);
  expect(posted[0]).toMatchObject({ path: '/grants/roots', body: { holder: CY, relation: 'administrator', resource: { kind: 'organisation', id: ORGANISATION } } });
  expect(document.querySelector('.next-step')?.textContent).toContain('Cy (test person) is now administrator of Example organisation.');
});

it('offers no organisation when the service names none for me and no grant on one is held', async () => {
  await mount('#/access/issue', { ...routes, '/me': ok({ ...ME, organisation: undefined }), '/grants': ok({ grants: GRANTS.filter((grant) => grant.resource.kind !== 'organisation'), revision: 7 }) });
  expect($$('select[name="kind"] option').map((option) => (option as HTMLOptionElement).value)).not.toContain('organisation');
});

it('gives the agent administrator standing from its own file, the same give', async () => {
  const { posted } = await mount('#/file/' + SCRIBE + '/access', routes);
  await click($('[data-act="grant"]'));
  const block = $('[data-organisation-standing]');
  expect(block?.textContent).toContain('In Example organisation');
  expect(options(block?.querySelector('select[aria-label="Standing in the organisation"]') ?? null)).toEqual(['Choose standing', expect.stringContaining('administrator')]);
  await choose(block?.querySelector('select[aria-label="Standing in the organisation"]') ?? null, 'administrator');
  await click(block?.querySelector('[data-act="organisation-standing"]') ?? null);
  await settle();
  // The give lands and the file reloads with the grant in its table; the receipt's words are the unit test's (give-standing.test.tsx).
  const gives = posted.filter((each) => each.path === '/grants');
  expect(gives).toHaveLength(1);
  expect(gives[0]).toMatchObject({ body: {
    source: 'administrator', recipient: SCRIBE, responsible: ADA, relation: 'administrator', mode: 'outright',
    resource: { kind: 'organisation', id: ORGANISATION }, pass_on: { kind: 'use_only' }, window: { ends_at: null },
  } });
});

it('shows the organisation give to nobody without organisation.grant', async () => {
  const grants = { grants: GRANTS.filter((grant) => grant.resource.kind !== 'organisation'), revision: 7 };
  await mount('#/file/' + SCRIBE + '/access', { ...routes, '/grants': ok(grants), '/people': ok({ ...PEOPLE, scope: 'own' }), '/directory/people': ok({ ...PEOPLE, scope: 'own' }) });
  await click($('[data-act="grant"]'));
  expect($('[data-organisation-standing]')).toBeNull();
});
