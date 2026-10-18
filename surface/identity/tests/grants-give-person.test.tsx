/** A grant that may be passed on to people is given to a person from its own Give, and only to the recipients its pass-on names. */
import { beforeEach, describe, expect, it } from 'vitest';
import { $, $$, choose, click, mount, settle } from './harness';
import { DIRECTORY, GRANTS, AGENT_MODEL as MODEL, ROOT_G, SCRIBE, SERVICE as BASE, ok } from './fixtures';
import { parsePendingGrant } from '../src/features/grants/pendingGrant';
import type { DelegateBody } from '../src/generated/grants';
import type { PeopleView } from '../src/generated';

beforeEach(() => sessionStorage.clear());

const CY = 'person-' + 'c'.repeat(32);
const DEE = 'person-' + 'd'.repeat(32);
const PEOPLE: PeopleView = { ...DIRECTORY, people: [...DIRECTORY.people,
  { id: CY, display_name: 'Cy (test person)', state: 'active', agents: [] },
  { id: DEE, display_name: 'Dee (test person)', state: 'suspended', agents: [] }] };
const passing = (recipients: ('person' | 'agent')[]) => GRANTS.map((g) => g.id === ROOT_G ? { ...g, pass_on: { kind: 'to' as const, actions: ['edit', 'view'], recipients } } : g);
const routes = (recipients: ('person' | 'agent')[], more: typeof BASE = {}): typeof BASE => ({ ...BASE, '/grants/model': ok(MODEL),
  '/directory/people': ok(PEOPLE), '/people': ok(PEOPLE), '/grants': ok({ grants: passing(recipients), revision: 7 }), ...more });
const open = async (recipients: ('person' | 'agent')[], more: typeof BASE = {}) => {
  const m = await mount('#/me?tab=account', routes(recipients, more));
  await click($(`[data-act="delegate"][data-g="${ROOT_G}"]`));
  return m;
};
const offered = () => $$('#dTo option').filter((o) => (o as HTMLOptionElement).value).map((o) => ({ id: (o as HTMLOptionElement).value, off: (o as HTMLOptionElement).disabled }));

describe('Give to a person', () => {
  it('shows Give on a grant that passes on to people only, and offers people and none of the agents', async () => {
    await open(['person']);
    expect($('.act-panel h2')?.textContent).toBe('Give part of owner of project:identity to a person');
    expect(offered()).toEqual([{ id: CY, off: false }, { id: DEE, off: true }]);
    expect($('.act-panel')?.textContent).toContain('Lys gives access only to an active person or agent');
  });

  it('offers only agents on a grant that passes on to agents only', async () => {
    await open(['agent']);
    expect(offered().map((each) => each.id)).not.toContain(CY);
    expect(offered().map((each) => each.id)).toContain(SCRIBE);
  });

  it('gives to a person with that person as their own responsible', async () => {
    const m = await open(['person', 'agent'], { 'POST /grants': (body) => ok({ operation: (body as DelegateBody).operation, grant: 'grant-' + 'e'.repeat(32), receipt: { caller: DIRECTORY.people[0].id } }) });
    expect($('.act-panel h2')?.textContent).toBe('Give part of owner of project:identity to someone');
    await choose($('#dTo'), CY);
    await click($('.level-picker input[type="radio"][value="viewer"]'));
    await click($('[data-act="delegatedo"]'));
    await settle();
    const given = m.posted.filter((p) => p.path === '/grants').map((p) => p.body as DelegateBody);
    expect(given).toHaveLength(1);
    expect(given[0]).toMatchObject({ recipient: CY, responsible: CY, relation: 'viewer', source: ROOT_G });
  });
});

describe('A retained give to a person', () => {
  const body = { operation: 'op-' + '1'.repeat(32), route: 'browser', source: ROOT_G, relation: 'viewer', resource: { kind: 'project', id: 'identity' },
    window: { starts_at: 1, ends_at: null }, pass_on: { kind: 'use_only' } };
  it('is read back as held when the person named responsible is its recipient', () => {
    expect(parsePendingGrant({ ...body, recipient: CY, responsible: CY }, DIRECTORY.people[0].id, ROOT_G)).toMatchObject({ recipient: CY, responsible: CY });
  });
  it('is still refused when the responsible is neither the giver nor the recipient', () => {
    expect(parsePendingGrant({ ...body, recipient: CY, responsible: DEE }, DIRECTORY.people[0].id, ROOT_G)).toBeNull();
  });
});
