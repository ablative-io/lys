// Connect an app: the Apps page's known-app card, the collapsed developer section, and #/apps/connect.
// Not opened in a browser: these run under jsdom.
import { afterEach, expect, it, vi } from 'vitest';
import { click, mount, settle, text } from './harness';
import { SERVICE, ok } from './fixtures';

const button = (label: string) => [...document.querySelectorAll('button')].find((entry) => (entry.getAttribute('aria-label') ?? entry.textContent) === label) ?? null;
const ORIGIN = 'https://cambium.example.test';
const MANIFEST = ORIGIN + '/identity/lys/manifest';
const BACK = ORIGIN + '/identity/lys/connected';
const manifest = { id: 'cambium', name: 'Cambium', redirects: [ORIGIN + '/identity/oidc/callback'], post_logout_redirects: [ORIGIN + '/'], profile: true,
  schema: { kinds: { 'cambium.place': { actions: ['read'], relations: { member: ['read'] }, parents: [] } } } };

afterEach(() => { vi.unstubAllGlobals(); });

it('shows Cambium as a known app with its state and a Connect button that asks for its address once', async () => {
  await mount('#/apps', { ...SERVICE, '/apps': () => ok({ apps: [] }) });
  const card = document.querySelector('[aria-label="Known apps"]');
  expect(card?.textContent).toContain('Cambium');
  expect(card?.textContent).toContain('not connected');
  await click(button('Connect Cambium'));
  const field = document.querySelector<HTMLInputElement>('input[aria-label="Cambium web address"]');
  expect(field?.placeholder).toBe('https://cambium.example.com');
  expect(document.querySelectorAll('[aria-label="Known app cambium"] input')).toHaveLength(1);
});

it('keeps the free-text registration in a collapsed developer section, with single-line return addresses', async () => {
  await mount('#/apps', { ...SERVICE, '/apps': () => ok({ apps: [] }) });
  const section = document.querySelector<HTMLDetailsElement>('details[aria-label="Add your own app (for developers)"]');
  expect(section).not.toBeNull();
  expect(section?.open).toBe(false);
  expect(section?.querySelector('[aria-label="Register an app"]')).not.toBeNull();
  expect(section?.querySelector('textarea')).toBeNull();
  expect(section?.querySelectorAll('input[aria-label^="Return address"]')).toHaveLength(1);
  await click(button('Add another address'));
  expect(section?.querySelectorAll('input[aria-label^="Return address"]')).toHaveLength(2);
});

it('approves a fetched manifest in one act and sends the browser back with the code', async () => {
  const assign = vi.fn();
  const routes = { ...SERVICE, ['POST /apps/connect']: () => ok({ redirect: BACK + '?code=c0de&state=s1' }), [MANIFEST]: () => ok(manifest) };
  const hash = '#/apps/connect?manifest=' + encodeURIComponent(MANIFEST) + '&return=' + encodeURIComponent(BACK) + '&state=s1';
  const { posted } = await mount(hash, routes);
  vi.stubGlobal('location', { ...window.location, assign, origin: window.location.origin });
  await settle();
  expect(text()).toContain('Cambium');
  expect(text()).toContain(ORIGIN);
  expect(document.querySelector('[aria-label="Permissions of cambium"]')).not.toBeNull();
  await click(button('Approve Cambium'));
  expect(posted).toEqual([{ path: '/apps/connect', body: { operation: expect.stringMatching(/^op-/), manifest, return: BACK, state: 's1' } }]);
  expect(assign).toHaveBeenCalledWith(BACK + '?code=c0de&state=s1');
  expect(button('Decline Cambium')).not.toBeNull();
});
