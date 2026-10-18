// DIRECTORY-102 R1 and R4: connecting an app again leaves the running app's credentials live until it collects the new ones,
// so the approve page says so and approves plainly, with no warning; and Connect again lives on the connected app's own card,
// its address filled in from where Lys says it last connected (connect_origin), never from a name list in Lys.
// The second party is the service's own answer: the words follow what /apps returns, never a value the page substituted.
import { afterEach, expect, it, vi } from 'vitest';
import { click, mount, settle } from '../../../tests/harness';
import { SERVICE, ok, refused } from '../../../tests/fixtures';

const ORIGIN = 'https://cambium.example.test';
const MANIFEST = ORIGIN + '/identity/lys/manifest';
const BACK = ORIGIN + '/identity/lys/connected';
const HASH = '#/apps/connect?manifest=' + encodeURIComponent(MANIFEST) + '&return=' + encodeURIComponent(BACK) + '&state=s1';
const READ = '/apps/connect/manifest?address=' + encodeURIComponent(MANIFEST);
const manifest = { id: 'cambium', name: 'Cambium', redirects: [ORIGIN + '/identity/oidc/callback'], profile: true, schema: { kinds: {} } };
const cambium = (state: string, connect_origin: string | null) => ({ id: 'cambium', name: 'Cambium', state, redirects: [], sign_in: null, schema: { kinds: {} }, version: 1, versions: [1], pending: null,
  client_id: 'cambium', service_account: null, client_credentials: [], registered_by: { kind: 'start' }, registered_at: 1, connect_origin });
const PUBLIC = 'https://lys.example.test';
const listed = (apps: unknown[]) => ok({ apps, public_address: PUBLIC, public_address_reason: null });
const button = (label: string) => [...document.querySelectorAll('button')].find((entry) => entry.getAttribute('aria-label') === label) ?? null;
const KEEPS = 'The running app keeps its credentials until it collects the new ones.';

afterEach(() => { vi.unstubAllGlobals(); });

it('approves a connected app plainly, saying the running app keeps its credentials, with no warning', async () => {
  await mount(HASH, { ...SERVICE, [READ]: () => ok({ manifest, origin: ORIGIN }), '/apps': () => listed([cambium('approved', ORIGIN)]) });
  await settle();
  expect(button('Approve Cambium')?.textContent).toBe('Approve');
  expect(document.body.textContent).toContain(KEEPS);
  expect(document.querySelector('.app-replace')).toBeNull();
  expect(document.body.textContent).not.toContain('replace its credentials');
  expect(document.body.textContent).not.toContain('stops signing people in');
});

it('approves plainly, with nothing said of credentials, when Lys says the app is not yet connected', async () => {
  await mount(HASH, { ...SERVICE, [READ]: () => ok({ manifest, origin: ORIGIN }), '/apps': () => listed([]) });
  await settle();
  expect(button('Approve Cambium')?.textContent).toBe('Approve');
  expect(document.body.textContent).not.toContain('keeps its credentials');
});

it('says the running app keeps its credentials as an "if" when Lys could not say whether the app is connected', async () => {
  await mount(HASH, { ...SERVICE, [READ]: () => ok({ manifest, origin: ORIGIN }), '/apps': () => refused(503, 'ServiceUnavailable', 'apps not read') });
  await settle();
  expect(document.body.textContent).toContain('If Cambium is already connected, the running app keeps its credentials until it collects the new ones.');
  expect(button('Approve Cambium')?.textContent).toBe('Approve');
});

it('offers Connect again on a connected app’s own card, its address filled in from where it last connected', async () => {
  const assign = vi.fn();
  await mount('#/apps', { ...SERVICE, '/apps': () => listed([cambium('approved', ORIGIN)]) });
  vi.stubGlobal('location', { ...window.location, assign, origin: window.location.origin });
  const card = document.querySelector('[aria-label="App cambium"]');
  const again = button('Connect Cambium again');
  expect(card?.contains(again)).toBe(true);
  expect(again?.textContent).toBe('Connect again');
  expect(again?.classList.contains('primary')).toBe(false);
  await click(again);
  expect(card?.textContent).toContain('The running Cambium keeps its credentials until it collects the new ones.');
  expect(card?.querySelector('.app-replace')).toBeNull();
  const field = card?.querySelector<HTMLInputElement>('input[aria-label="Cambium web address"]');
  expect(field?.value).toBe(ORIGIN);
  await click(button('Continue to Cambium'));
  expect(assign).toHaveBeenCalledWith(ORIGIN + '/identity/lys/connect?lys=' + encodeURIComponent(PUBLIC));
});

it('offers no Connect again on an app registered by hand, which never connected', async () => {
  await mount('#/apps', { ...SERVICE, '/apps': () => listed([cambium('approved', null)]) });
  expect(button('Connect Cambium again')).toBeNull();
});

it('tells a person Lys says may not read apps so, in place of an Approve that could only fail', async () => {
  await mount(HASH, { ...SERVICE, [READ]: () => refused(403, 'NotAdmitted', 'only the administrator does this, on a Lys screen'), '/apps': () => refused(403, 'NotAdmitted', 'apps are approved by the administrator') });
  await settle();
  expect(button('Approve Cambium')).toBeNull();
  expect(document.body.textContent).toContain('only the administrator does this, on a Lys screen');
});
