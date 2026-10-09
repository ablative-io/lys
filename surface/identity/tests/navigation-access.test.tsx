import { describe, expect, it } from 'vitest';
import { $, $$, mount, press, text, unmountAll } from './harness';
import { SERVICE, ok, refused } from './fixtures';

const ANSWERS = {
  ...SERVICE,
  '/network': ok({ machines: [], total: 0, next: null }),
  '/secrets': ok({ entries: [] }),
  '/configuration': ok({}),
};
const NAVS = ['canvas', 'people', 'roles', 'access', 'graph', 'secrets', 'network', 'settings'];

describe('navigation follows the answering API', () => {
  it.each([
    ['/runtime/live', ['canvas']], ['/roles', ['roles']], ['/grants', ['access', 'graph']],
    ['/secrets', ['secrets']], ['/network', ['network']], ['/configuration', ['settings']],
  ])('hides only the entries refused by %s, in the rail, palette and keys', async (path, hidden) => {
    await mount('#/me', { ...ANSWERS, [path]: refused(403, 'NotAdmitted', 'this read is not admitted') });
    expect($$('#rail a[data-nav]').map((entry) => entry.dataset.nav)).toEqual(expect.arrayContaining(NAVS.filter((nav) => !hidden.includes(nav))));
    for (const nav of hidden) expect($('#rail a[data-nav="' + nav + '"]')).toBeNull();
    await press('k', { metaKey: true }, document.body);
    for (const nav of hidden) {
      const address = nav === 'settings' ? '/settings' : '/' + nav;
      expect($('#palList [data-to="#' + address + '"]')).toBeNull();
    }
    await press('Escape', {}, document.body);
    if (hidden.includes('roles')) {
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
});
