import { act } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { $$, choose, click, mount, settle } from './harness';
import { ADA, ME, SERVICE, SCRIBE, ok } from './fixtures';
import type { Route } from './fixtures';
import type { CannotGiveAnswer, CannotGiveItem, PeopleView } from '../src/generated';
import type { Grant, GrantModel, PassOn } from '../src/generated/grants';

// DIRECTORY-024 R3: the delegation form shows exactly the cannot-give list the
// service answers (conformance 2.4). The answers are R1's fixture as the
// service writes it: Ada is P1, the scribe is her agent A1, Cara is P2.

beforeEach(() => sessionStorage.clear());

const hex = (n: number) => n.toString(16).padStart(32, '0');
const A1 = SCRIBE;
const P2 = 'person-' + hex(3);
const [G1, G2, G3, G10] = [1, 2, 3, 10].map((n) => 'grant-' + hex(0x100 + n));

const grant = (id: string, relation: string, actions: string[], pass_on: PassOn, source: string | null, on = 'p'): Grant => ({
  id, issuer: ADA, holder: ADA, responsible: ADA, resource: { kind: 'project', id: on }, relation, actions, pass_on, source,
  window: { starts_at: 1_759_000_000, ends_at: null }, model_version: 1, operation: 'op-' + hex(500), revoked: false,
  revoked_at: null, revoked_revision: null, last_use: { seen: false, recorded: 0, source: 'reported' },
  standing: { stands: true }, effective_ends_at: null,
});

const GRANTS: Grant[] = [
  grant(G1, 'alder', ['view'], { kind: 'to', actions: ['view'], recipients: ['person', 'agent'] }, null),
  grant(G2, 'birch', ['comment', 'view'], { kind: 'use_only' }, null),
  grant(G3, 'cedar', ['edit', 'view'], { kind: 'use_only' }, 'grant-' + hex(0x100)),
  grant(G10, 'alder', ['view'], { kind: 'to', actions: ['view'], recipients: ['agent'] }, null, 's'),
];

/** The fixture model: the relation names say nothing of their actions. */
const MODEL: GrantModel = { action_sentences: { view: 'View this resource', edit: 'Edit this resource', grant: 'Give access to this resource' }, version: 1, relations: { alder: ['view'], birch: ['comment', 'view'], cedar: ['edit', 'view'], damson: ['comment', 'edit', 'grant', 'view'] } };

const PEOPLE: PeopleView = {
  scope: 'directory',
  people: [
    { id: ADA, display_name: 'Ada (test person)', state: 'active', agents: [{ id: A1, display_name: 'Scribe', state: 'active' }] },
    { id: P2, display_name: 'Cara (test person)', state: 'active', agents: [] },
  ],
};

const g2: CannotGiveItem = { subject: 'grant', grant: G2, reason: 'use_only', source: false };
const g3: CannotGiveItem = { subject: 'grant', grant: G3, reason: 'lent_to_you', source: false };
const g10: CannotGiveItem = { subject: 'grant', grant: G10, reason: 'agents_only', source: false };
const damson: CannotGiveItem = { subject: 'relation', relation: 'damson', reason: 'above_what_you_hold', source: false };
const signIn: CannotGiveItem = { subject: 'sign_in_identity', reason: 'sign_in_identity', source: false };

const answer = (recipient: string, items: unknown[]): CannotGiveAnswer => ({ source: G1, recipient, items: items as CannotGiveItem[] });
const FOR_A1 = answer(A1, [g2, g3, damson, signIn]);
const FOR_P2 = answer(P2, [g2, g3, damson]);
const FOR_P2_AGENTS_ONLY = answer(P2, [g2, g3, g10, damson]);

const path = (recipient: string) => `/grants/cannot-give?route=browser&source=${G1}&recipient=${recipient}`;

const routes = (answers: Record<string, CannotGiveAnswer>): Record<string, Route> => ({
  ...SERVICE,
  '/me': ok(ME),
  '/directory/people': ok(PEOPLE),
  '/grants': ok({ grants: GRANTS, revision: 9 }),
  '/grants/model': ok(MODEL),
  ...Object.fromEntries(Object.entries(answers).map(([recipient, a]) => [path(recipient), ok(a)])),
});

const open = async (answers: Record<string, CannotGiveAnswer>) => {
  const m = await mount('#/me?tab=account', routes(answers));
  await click($$(`[data-act="delegate"][data-g="${G1}"]`)[0] ?? null);
  return m;
};

const rows = () => $$('#drawer [data-cannot-give]').map((row) => [row.querySelector('b')?.textContent, row.dataset.cannotGive]);
const pairs = (a: CannotGiveAnswer) => a.items.map((item) => [subjectOf(item), item.reason]);
const subjectOf = (item: CannotGiveItem): string => {
  switch (item.subject) {
    case 'grant': return `${GRANTS.find((g) => g.id === item.grant)?.relation} of project:${GRANTS.find((g) => g.id === item.grant)?.resource.id}`;
    case 'service_account': return 'Service account';
    case 'relation': return `${item.relation} of project:p`;
    case 'sign_in_identity': return 'Your sign-in identities';
  }
};

describe('What you cannot give, as the service answers it (conformance 2.4)', () => {
  it('CANNOT_GIVE_SCREEN: shows the answer for A1, row for item, in its order', async () => {
    const { requests } = await open({ [A1]: FOR_A1 });
    expect(requests.filter((r) => r === path(A1))).toHaveLength(1);
    expect(FOR_A1.items).toHaveLength(4);
    expect(rows()).toHaveLength(4);
    expect(rows()).toEqual([
      ['birch of project:p', 'use_only'],
      ['cedar of project:p', 'lent_to_you'],
      ['damson of project:p', 'above_what_you_hold'],
      ['Your sign-in identities', 'sign_in_identity'],
    ]);
    expect(rows()).toEqual(pairs(FOR_A1));
  });

  it('CANNOT_GIVE_SCREEN_NO_OTHER: shows nothing the answer does not list, whatever the grants it holds would say', async () => {
    await open({ [A1]: answer(A1, [g2, g3, signIn]) });
    expect(rows()).toHaveLength(3);
    expect(rows().some(([subject]) => subject?.startsWith('damson'))).toBe(false);
  });

  it('CANNOT_GIVE_SCREEN_RECIPIENT: asks again, once, when To changes, and shows that answer', async () => {
    const { requests } = await open({ [A1]: FOR_A1, [P2]: FOR_P2 });
    expect(rows()).toHaveLength(4);
    await choose($$('#dTo')[0] ?? null, P2);
    expect(requests.filter((r) => r.startsWith('/grants/cannot-give') && r.includes('recipient=' + P2))).toHaveLength(1);
    expect(rows()).toEqual(pairs(FOR_P2));
    expect(rows()).toHaveLength(3);
    expect(rows().some(([, reason]) => reason === 'sign_in_identity')).toBe(false);
  });

  it('CANNOT_GIVE_SCREEN_AGENTS_ONLY: says a grant passes on only to agents in the words given', async () => {
    await open({ [A1]: FOR_A1, [P2]: FOR_P2_AGENTS_ONLY });
    await choose($$('#dTo')[0] ?? null, P2);
    expect(rows()).toHaveLength(4);
    const agentsOnly = $$('#drawer [data-cannot-give="agents_only"]');
    expect(agentsOnly).toHaveLength(1);
    expect(agentsOnly[0].querySelector('b')?.textContent).toBe('alder of project:s');
    expect(agentsOnly[0].querySelector('.note')?.textContent).toBe('This can be passed on only to an agent.');
  });

  it('CANNOT_GIVE_SCREEN_STALE: an answer for a recipient no longer chosen is discarded', async () => {
    await open({ [A1]: FOR_A1 });
    expect(rows()).toEqual(pairs(FOR_A1));
    const served = globalThis.fetch;
    const held: { url: string; deliver: () => void }[] = [];
    vi.stubGlobal('fetch', (input: string, init?: RequestInit) => {
      const url = String(input);
      if (!url.includes('/grants/cannot-give')) return served(input, init);
      const recipient = new URL(url, 'http://surface.test').searchParams.get('recipient');
      const body = recipient === P2 ? FOR_P2 : FOR_A1;
      return new Promise<Response>((resolve) => {
        held.push({ url, deliver: () => resolve(new Response(JSON.stringify(body), { status: 200, headers: { 'content-type': 'application/json' } })) });
      });
    });
    await choose($$('#dTo')[0] ?? null, P2);
    await choose($$('#dTo')[0] ?? null, A1);
    expect(held.map((h) => new URL(h.url, 'http://surface.test').searchParams.get('recipient'))).toEqual([P2, A1]);
    await act(async () => held[1].deliver());
    await settle();
    await act(async () => held[0].deliver());
    await settle();
    expect(rows()).toHaveLength(4);
    expect(rows()).toEqual(pairs(FOR_A1));
  });

  it('CANNOT_GIVE_SCREEN_UNKNOWN_REASON: refuses the whole answer by name, showing no row', async () => {
    const borrowed = { subject: 'grant', grant: G3, reason: 'borrowed', source: false };
    await open({ [A1]: answer(A1, [g2, damson, borrowed, signIn]) });
    expect(rows()).toHaveLength(0);
    const refusals = $$('#drawer [data-refusal]');
    expect(refusals).toHaveLength(1);
    expect(refusals[0].dataset.refusal).toBe('unknown_cannot_give_reason');
    expect(refusals[0].querySelector('b')?.textContent).toBe('unknown_cannot_give_reason');
  });
});
