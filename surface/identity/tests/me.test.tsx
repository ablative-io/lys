import { describe, expect, it } from 'vitest';
import { $, $$, mount, press, text, unmountAll, unreachable } from './harness';
import { ADA, BEA, COURIER, DIRECTORY, GRANTS, ISSUER, LEDGER_G, SCRIBE, SERVICE, ok, refused } from './fixtures';
import type { Grant } from '../src/generated/grants';

/** The grants that stand on an agent's Access tab, each by its relation and resource. */
const standing = () => [...document.querySelectorAll('table[aria-label="Grants"] tbody tr[data-grant]')].filter((tr) => !tr.textContent?.includes('void'))
  .map((tr) => `${tr.querySelector('td[data-col="Relation"]')?.textContent} of ${tr.querySelector<HTMLElement>('td[data-col="On"] span')?.title}`);

describe('You', () => {
  it('shows the signed-in person from /me', async () => {
    const { requests } = await mount('#/me');
    expect(requests).toEqual(expect.arrayContaining(['/me', '/people']));
    expect($('.eyebrow')?.textContent).toBe('Signed in as');
    expect($('.page h1')?.textContent).toBe('Ada (test person)');
  });

  it('keeps sign-in identities and service accounts as separate lists (conformance 1.1, 1.3)', async () => {
    await mount('#/me?tab=account');
    const signIn = $('#signin-identities');
    const service = $('#service-accounts');
    expect(signIn?.querySelector('h2')?.textContent).toBe('Sign-in identities');
    expect(signIn?.textContent).toContain(new URL(ISSUER).host);
    expect(signIn?.textContent).toContain('this session');
    expect(signIn?.textContent).toContain('Never lent to an agent.');
    expect(service?.querySelector('h2')?.textContent).toBe('Your service-account records');
    expect(service?.textContent).toContain('No service-account records were returned for you.');
    expect(signIn?.contains(service ?? null)).toBe(false);
  });

  it('lists your agents that still stand on the Dashboard, and what each holds on its own page', async () => {
    await mount('#/dashboard');
    const agents = $$('tr[data-href]').map((tr) => tr.querySelector('td')?.textContent);
    expect(agents).toEqual(["Scribe", "Courier", "Archivist"]);
    expect(text()).not.toContain('finance-readonly');
    unmountAll();
    // What an agent holds has one home, the Access tab of its own page; the Dashboard does not repeat it.
    await mount('#/file/' + SCRIBE + '/access');
    expect(standing()).toEqual(['viewer of project:identity']);
    expect(text()).not.toContain('finance-readonly');
    unmountAll();
    await mount('#/file/' + COURIER + '/access');
    expect(standing()).toEqual([]);
  });

  it('opens an agent from the keyboard on the Dashboard, and every control is reachable (9.3)', async () => {
    await mount('#/dashboard');
    expect(unreachable()).toEqual([]);
    $(`tr[data-href="#/file/${SCRIBE}"]`)?.focus();
    await press('Enter');
    expect(location.hash).toBe('#/file/' + SCRIBE);
  });

  it('names the refusal when the login is bound to no person', async () => {
    await mount('#/me', { ...SERVICE, '/me': refused(403, 'NoPerson', 'NoPerson: the signed-in login is bound to no person') });
    expect($('.why-not b')?.textContent).toBe('NoPerson');
  });
});

const CAL = 'person-' + '3'.padStart(32, '0');
const CAL_G = 'grant-' + '41'.padStart(32, '0');

/**
 * The service as the administrator reads it, with one more active person, Cal,
 * who holds a root grant Ada issued: a grant admission admits, so the service
 * answers it standing, and only You's holder check keeps it out of What you hold.
 */
function withCal(): { routes: typeof SERVICE; grants: Grant[] } {
  const base = GRANTS.find((g) => g.id === LEDGER_G) as Grant;
  const calG: Grant = {
    ...base, id: CAL_G, issuer: ADA, holder: CAL, responsible: CAL, resource: { kind: 'project', id: 'atlas' }, relation: 'viewer', actions: ['view'],
    pass_on: { kind: 'use_only' }, source: null, window: { starts_at: base.window.starts_at, ends_at: null }, standing: { stands: true }, effective_ends_at: null,
  };
  const people = { ...DIRECTORY, people: [...DIRECTORY.people, { id: CAL, display_name: 'Cal (test person)', state: 'active' as const, agents: [] }] };
  const grants = [...GRANTS, calG];
  return { routes: { ...SERVICE, '/directory/people': ok(people), '/grants': ok({ grants, revision: 7 }) }, grants };
}

describe('Personal scope', () => {
  it('row_1_5_an_administrators_people_shows_others_while_you_shows_only_the_persons_own', async () => {
    const { routes, grants } = withCal();
    expect(grants.filter((g) => g.holder === BEA)).toEqual([]);
    await mount('#/people', routes);
    const names = $$('tbody tr[data-pick] td:first-child').map((td) => td.textContent);
    expect(names).toContain('Cal (test person)');
    expect(names).toContain('Bea (test person)');

    unmountAll();
    await mount('#/dashboard', routes);
    expect($$('tr[data-href]').map((tr) => tr.querySelector('td')?.textContent)).toEqual(['Scribe', 'Courier', 'Archivist']);
    unmountAll();
    await mount('#/me?tab=account', routes);
    const holds = [...document.querySelectorAll('table[aria-label="Grants"] tbody tr[data-grant]')].map((tr) => `${tr.querySelector('td[data-col="Relation"]')?.textContent} of ${tr.querySelector<HTMLElement>('td[data-col="On"] span')?.title}`);
    expect(holds).toEqual(['owner of project:identity', 'viewer of project:ledger']);
    expect(holds).not.toContain('viewer of project:atlas');
    // Secrets have their place on the rail; Account does not repeat it.
    expect($$('.card').find((c) => c.querySelector('h2')?.textContent === 'Secrets available to you')).toBeUndefined();
    expect(document.querySelector('.you-account a[href="#/secrets"]')).toBeNull();
    // The grant table has the full width, in its own scrolling box, and the account's sections sit in one row under it.
    expect(document.querySelector('.you-account.grid2')).toBeNull();
    expect(document.querySelector('.you-holds-scroll table[aria-label="Grants"]')).not.toBeNull();
    expect(document.querySelector('.you-account-row #signin-identities')).not.toBeNull();
    // A person ends their other sign-ins here.
    expect(document.querySelector('.you-account-row section[aria-label="Your signed-in sessions"]')).not.toBeNull();
  });
});

const AGENTS_G = 'grant-' + '42'.padStart(32, '0');
const NONE_G = 'grant-' + '43'.padStart(32, '0');

/**
 * The service as the first administrator reads it after install: Ada also holds
 * the editor root grant on the directory's agents the install issues, and one
 * grant whose recorded actions are empty, so it lets her take no action.
 */
function withInstallGrants(): typeof SERVICE {
  const base = GRANTS.find((g) => g.id === LEDGER_G) as Grant;
  const agentsG: Grant = {
    ...base, id: AGENTS_G, resource: { kind: 'directory', id: 'agents' }, relation: 'editor', actions: ['edit', 'view'],
    pass_on: { kind: 'to', actions: ['edit', 'view'], recipients: ['service_account'] },
  };
  const noneG: Grant = { ...base, id: NONE_G, resource: { kind: 'project', id: 'atlas' }, relation: 'auditor', actions: [] };
  return { ...SERVICE, '/grants': ok({ grants: [...GRANTS, agentsG, noneG], revision: 7 }) };
}

describe('What you hold', () => {
  const cell = (tr: Element, col: string) => tr.querySelector('td[data-col="' + col + '"]')?.textContent ?? '';
  const rows = () => [...document.querySelectorAll('table[aria-label="Grants"] tbody tr[data-grant]')].map((tr) => [cell(tr, 'Allows'), cell(tr, 'Relation'), (tr.querySelector<HTMLElement>('td[data-col="On"] span')?.title ?? ''), ((pills) => pills.length === 1 ? 'root' : (pills[pills.length - 2].textContent ?? '').split(' · ')[0])([...tr.querySelectorAll('.chain .pill')])]);

  it('starts each row with what the grant lets the person do, from the actions it carries, then the relation, object and source', async () => {
    await mount('#/me?tab=account', withInstallGrants());
    expect(rows()).toEqual([
      ['Everything here', 'owner', 'project:identity', 'root'],
      ['View this resource', 'viewer', 'project:ledger', 'root'],
      ['Edit this resource; View this resource', 'editor', 'directory:agents', 'root'],
      ['No action', 'auditor', 'project:atlas', 'root'],
    ]);
  });
});
