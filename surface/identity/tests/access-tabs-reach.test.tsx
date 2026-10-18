/** The access map and roles have no place on the rail, so Access's tabs are their way in and out. */
import { afterEach, describe, expect, it } from 'vitest';
import { $$, mount, unmountAll } from './harness';
import { SERVICE, ok } from './fixtures';

afterEach(() => unmountAll());
const tabs = () => $$('nav[aria-label="Access views"] a').map((a) => [a.textContent, a.getAttribute('href'), a.getAttribute('aria-current')]);

describe('Access tabs reach the access map and roles', () => {
  it('lists both on every Access view and marks the map when it is open', async () => {
    await mount('#/graph', { ...SERVICE, '/apps': ok({ apps: [] }) });
    expect(document.querySelector('h1')?.textContent).toBe('Access');
    expect(tabs()).toContainEqual(['Access map', '#/graph', 'page']);
    expect(tabs()).toContainEqual(['Jobs', '#/roles', null]);
  });

  it('shows the same tabs on Jobs, so a person can go back to Access', async () => {
    await mount('#/roles', { ...SERVICE, '/roles': ok({ roles: [] }) });
    expect(tabs()).toContainEqual(['Jobs', '#/roles', 'page']);
    expect(tabs()).toContainEqual(['Grants', '#/access', null]);
  });
});
