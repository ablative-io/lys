import { describe, expect, it } from 'vitest';
import { $, $$, mount, press, text, unmountAll } from './harness';
import { SERVICE, ok, refused } from './fixtures';

const ANSWERS = {
  ...SERVICE,
  '/network': ok({ machines: [], total: 0, next: null }),
  '/secrets': ok({ secrets: [] }),
  '/configuration': ok({}),
};
const NAVS = ['canvas', 'people', 'waiting', 'access', 'secrets', 'network', 'apps', 'settings'];
// Each refused read: the rail entries it hides, and the Search rows it hides. Roles and the access map have no rail
// entry of their own (CONSENSUS.md, 18 October 2026), so their refusals show only in Search and the keys.
const DENIALS: [string, string[], string[]][] = [
  ['/runtime/live', ['canvas'], ['/canvas']], ['/roles', [], ['/roles']], ['/grants', ['access', 'waiting'], ['/access', '/graph', '/requests']],
  ['/secrets', ['secrets'], ['/secrets']], ['/network', ['network'], ['/network']], ['/configuration', ['settings', 'apps'], ['/settings', '/apps']],
];

describe('navigation follows the answering API', () => {
  it.each(DENIALS)('hides only the entries refused by %s, in the rail, palette and keys', async (path, hidden, rows) => {
    await mount('#/me', { ...ANSWERS, [path]: refused(403, 'NotAdmitted', 'this read is not admitted') });
    expect($$('#rail a[data-nav]').map((entry) => entry.dataset.nav)).toEqual(expect.arrayContaining(NAVS.filter((nav) => !hidden.includes(nav))));
    for (const nav of hidden) expect($('#rail a[data-nav="' + nav + '"]')).toBeNull();
    await press('k', { metaKey: true }, document.body);
    expect($$('#palList [data-to]').length).toBeGreaterThan(0);
    for (const address of rows) expect($('#palList [data-to="#' + address + '"]'), address).toBeNull();
    await press('Escape', {}, document.body);
    if (rows.includes('/roles')) {
      await press('g', {}, document.body);
      await press('o', {}, document.body);
      expect(location.hash).toBe('#/me');
    }
  });

  it('keeps People and agents when the personal API answers after directory admission is refused', async () => {
    await mount('#/me', { ...ANSWERS, '/directory/people': refused(403, 'NotAdmitted', 'directory access refused') });
    expect($('#rail a[data-nav="people"]')).not.toBeNull();
    unmountAll();
    await mount('#/me', { ...ANSWERS, '/directory/people': refused(403, 'NotAdmitted', 'directory access refused'), '/people': refused(403, 'NoPerson', 'the login has no bound person') });
    expect($('#rail a[data-nav="people"]')).toBeNull();
  });

  it('answers a hidden Configuration URL with the API name and reason before exposing its controls', async () => {
    await mount('#/settings', { ...ANSWERS, '/configuration': refused(403, 'NotAdmitted', 'configuration is restricted to this service administrator') });
    expect($('#rail a[data-nav="settings"]')).toBeNull();
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
    await press('k', { metaKey: true }, document.body);
    expect($('#palList [data-to="#/roles"]')).not.toBeNull();
    await press('Escape', {}, document.body);
    admitted = false;
    await press('g', {}, document.body);
    await press('o', {}, document.body);
    expect(text()).toContain('NotAdmitted');
    expect(text()).toContain('role access was removed');
    expect($('.page h1')?.textContent).not.toBe('Roles');
  });
});
