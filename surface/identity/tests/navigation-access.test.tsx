import { describe, expect, it } from 'vitest';
import { $, $$, mount, press, settle, text, type, unmountAll } from './harness';
import { ME, SERVICE, ok, refused } from './fixtures';
import type { Route } from './fixtures';

const ANSWERS = {
  ...SERVICE,
  '/network': ok({ machines: [], total: 0, next: null }),
  '/secrets': ok({ secrets: [] }),
  '/configuration': ok({}),
};
const NAVS = ['canvas', 'people', 'waiting', 'access', 'secrets', 'network', 'apps', 'settings'];
// Each refused read: the rail entries it marks, and the Search rows it hides. Jobs and the access map have no rail
// entry of their own (CONSENSUS.md, 18 October 2026), so their refusals show only in Search and the keys. The rail's
// Settings opens your own account, which nothing refuses, and Apps is decided by its own read (domain 05's ask).
const DENIALS: [string, string[], string[]][] = [
  ['/runtime/live', ['canvas'], ['/canvas']], ['/roles', [], ['/roles']], ['/grants', ['access'], ['/access', '/graph']],
  ['/secrets', ['secrets'], ['/secrets']], ['/network', ['network'], ['/network']], ['/configuration', [], ['/settings']], ['/apps', ['apps'], ['/apps']],
];

/** Search's name for each address a denial can hide: Search offers a page off the rail only once something is typed. */
const NAMES: Record<string, string> = { '/canvas': 'Running agents', '/roles': 'Jobs', '/access': 'Access', '/graph': 'Access map', '/secrets': 'Secrets', '/network': 'Computers', '/settings': 'Settings', '/apps': 'Apps' };
const searchFor = async (name: string) => {
  await press('k', { metaKey: true }, document.body);
  await type($('#palIn'), name);
};
const offered = (address: string) => $('#palList [data-to="#' + address + '"]') !== null;

const shut = (nav: string) => $('#rail a[data-nav="' + nav + '"]')?.dataset.shut;

describe('navigation follows the answering API', () => {
  it.each(DENIALS)('marks only the entries refused by %s, and keeps them from Search and the keys', async (path, hidden, rows) => {
    await mount('#/me', { ...ANSWERS, [path]: refused(403, 'NotAdmitted', 'NotAdmitted: this read is not admitted') });
    // Every entry is drawn whatever the answer; the refused ones are marked and say why in their title.
    expect($$('#rail a[data-nav]').map((entry) => entry.dataset.nav)).toEqual(expect.arrayContaining(NAVS));
    expect($$('#rail a[data-shut]').map((entry) => entry.dataset.nav).sort()).toEqual([...hidden].sort());
    for (const nav of hidden) {
      expect(shut(nav)).toBe('refused');
      expect($('#rail a[data-nav="' + nav + '"]')?.title).toContain('not open to you: this read is not admitted.');
      expect($('#rail a[data-nav="' + nav + '"]')?.title).not.toContain('NotAdmitted');
    }
    for (const address of rows) {
      await searchFor(NAMES[address]);
      expect(offered(address), address).toBe(false);
      await press('Escape', {}, document.body);
    }
    if (rows.includes('/roles')) {
      await press('g', {}, document.body);
      await press('o', {}, document.body);
      expect(location.hash).toBe('#/me');
    }
  });

  it('offers each of those pages in Search by name when nothing is refused, so their absence above is the refusal', async () => {
    await mount('#/me', ANSWERS);
    for (const [address, name] of Object.entries(NAMES)) {
      await searchFor(name);
      expect(offered(address), address).toBe(true);
      await press('Escape', {}, document.body);
    }
  });

  it('draws every entry, unmarked, while its check has not answered', async () => {
    // Held until the end of the test, then answered, so no read is left in flight for the next test to join.
    const held: (() => void)[] = [];
    const later = (answer: Route) => (() => new Promise((resolve) => { held.push(() => resolve(answer)); })) as unknown as Route;
    await mount('#/me', { ...ANSWERS, '/network': later(ANSWERS['/network']), '/configuration': later(ANSWERS['/configuration']) });
    expect(held.length).toBeGreaterThan(0);
    expect($$('#rail a[data-nav]').map((entry) => entry.dataset.nav)).toEqual(['dashboard', 'waiting', 'people', 'canvas', 'access', 'network', 'apps', 'secrets', 'settings', 'me']);
    expect($$('#rail a[data-shut]')).toHaveLength(0);
    for (const answer of held) answer();
    await settle();
  });

  it('says a check that failed for another reason could not be checked, and still offers the page', async () => {
    await mount('#/me', { ...ANSWERS, '/secrets': refused(500, 'StoreUnreadable', 'StoreUnreadable: the secrets store could not be read') });
    expect($$('#rail a[data-shut]').map((entry) => entry.dataset.nav)).toEqual(['secrets']);
    expect(shut('secrets')).toBe('unchecked');
    expect($('#rail a[data-nav="secrets"]')?.title).toBe('Secrets (g v): could not be checked: the secrets store could not be read. Open it to try again.');
    await searchFor('Secrets');
    expect(offered('/secrets')).toBe(true);
  });

  it('opens Waiting on you on Home whatever the requests read answers, and decides Access by its own read', async () => {
    await mount('#/me', { ...ANSWERS, '/grants': refused(403, 'NotAdmitted', 'grants are not admitted') });
    expect(shut('access')).toBe('refused');
    expect(shut('waiting')).toBeUndefined();
    unmountAll();
    // Waiting on you is Home's section (DIRECTORY-123 R5): a refused requests read is said in the section, never by
    // shutting the rail's way to it.
    await mount('#/me', { ...ANSWERS, '/requests': refused(403, 'NotAdmitted', 'requests are not admitted') });
    expect(shut('waiting')).toBeUndefined();
    expect($('#rail a[data-nav="waiting"]')?.getAttribute('href')).toBe('#/?section=waiting-on-you');
    expect(shut('access')).toBeUndefined();
  });

  it('marks the acting places for a person who is not active, in words, and leaves Waiting on you open', async () => {
    await mount('#/me', { ...ANSWERS, '/me': ok({ ...ME, person: { ...ME.person, state: 'registered' } }) });
    expect(['people', 'access'].map(shut)).toEqual(['refused', 'refused']);
    expect($('#rail a[data-nav="people"]')?.title).toContain('your account is not active yet');
    expect(shut('waiting')).toBeUndefined();
  });

  it('keeps People and agents when the personal API answers after directory admission is refused', async () => {
    await mount('#/me', { ...ANSWERS, '/directory/people': refused(403, 'NotAdmitted', 'directory access refused') });
    expect($('#rail a[data-nav="people"]')).not.toBeNull();
    expect(shut('people')).toBeUndefined();
    unmountAll();
    await mount('#/me', { ...ANSWERS, '/directory/people': refused(403, 'NotAdmitted', 'directory access refused'), '/people': refused(403, 'NoPerson', 'the login has no bound person') });
    expect(shut('people')).toBe('refused');
  });

  it('answers a hidden Configuration URL with the API name and reason before exposing its controls', async () => {
    await mount('#/settings', { ...ANSWERS, '/configuration': refused(403, 'NotAdmitted', 'configuration is restricted to this service administrator') });
    // The rail's own check is asked once the entry page has answered, so it is waited for; "absent" used to pass while it was still loading.
    await settle();
    // The rail's Settings is your own account, which is open to everyone, so it stays unmarked.
    expect(shut('settings')).toBeUndefined();
    expect(text()).toContain('NotAdmitted');
    expect(text()).toContain('configuration is restricted to this service administrator');
    expect($('[data-dock="left"]')).toBeNull();
    expect($('[data-labels="labels"]')).toBeNull();
  });

  it('does not infer access from the directory scope or an empty list', async () => {
    await mount('#/me', { ...ANSWERS, '/directory/people': refused(403, 'NotAdmitted', 'directory access refused') });
    for (const nav of NAVS) expect($('#rail a[data-nav="' + nav + '"]')).not.toBeNull();
  });

  it('reads a formerly allowed entry again before mounting its controls', async () => {
    let admitted = true;
    await mount('#/me', { ...ANSWERS, '/roles': () => admitted ? ok({ roles: [] }) : refused(403, 'NotAdmitted', 'role access was removed') });
    await searchFor('Jobs');
    expect(offered('/roles')).toBe(true);
    await press('Escape', {}, document.body);
    admitted = false;
    await press('g', {}, document.body);
    await press('o', {}, document.body);
    expect(text()).toContain('NotAdmitted');
    expect(text()).toContain('role access was removed');
    expect($('.page h1')?.textContent).not.toBe('Roles');
  });

  // Domain 03's ask: the four Access views say a refused read in place, under their own head and tabs.
  it.each(['#/access', '#/access/can', '#/model', '#/resources'])('opens %s when the grants read is refused, so the view says it in place', async (hash) => {
    await mount(hash, { ...ANSWERS, '/grants': refused(403, 'NotAdmitted', 'grants are not admitted') });
    expect(text()).not.toContain('You do not have access to this');
    if (hash === '#/model' || hash === '#/resources') expect($('nav[aria-label="Access views"]')).not.toBeNull();
    else expect(text()).toContain('grants are not admitted');
  });

  // The ways to ask and decide are decided by their own reads, and say a refused one in place too.
  it.each([
    ['#/requests', '/requests'], ['#/reviews', '/reviews'], ['#/access/drafts', '/drafts?state=waiting'],
  ])('opens %s when its own read is refused, so the view says it in place', async (hash, route) => {
    await mount(hash, { ...ANSWERS, [route]: refused(403, 'NotAdmitted', 'this list is not admitted') });
    expect(text()).not.toContain('You do not have access to this');
    expect($('nav[aria-label="Access views"]')).not.toBeNull();
  });

  it('still sends a person with no session to sign in from Requests', async () => {
    await mount('#/requests', { ...ANSWERS, '/requests': refused(401, 'NoSession', 'no session') });
    expect($('nav[aria-label="Access views"]')).toBeNull();
    expect($('.page h1')?.textContent).toBe('Sign in');
  });

  it('still sends a person with no session to sign in from an Access view', async () => {
    await mount('#/model', { ...ANSWERS, '/grants': refused(401, 'NoSession', 'no session') });
    expect($('nav[aria-label="Access views"]')).toBeNull();
    expect($('.page h1')?.textContent).toBe('Sign in');
  });

  it('keeps service accounts under Settings, read by their page alone', async () => {
    const { requests } = await mount('#/settings/service-accounts', { ...ANSWERS, '/configuration': refused(403, 'NotAdmitted', 'configuration is restricted') });
    expect($('.page h1')?.textContent).toBe('Settings');
    expect($('nav[aria-label="Settings views"]')).not.toBeNull();
    expect(text()).toContain('Keep a list of accounts used for work.');
    expect(requests).toContain('/service-accounts');
  });
});
