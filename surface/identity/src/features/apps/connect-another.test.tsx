// Domain 04 item 9 and DIRECTORY-102 R4: any app that serves Lys's connect address is connected by its web address, with no
// change to Lys (ADR-116); its name comes from its own manifest, read by Lys's server, never from a list in Lys.
import { afterEach, expect, it, vi } from 'vitest';
import { click, mount, settle, type } from '../../../tests/harness';
import { SERVICE, ok } from '../../../tests/fixtures';
import { CONNECT_PATH } from './ConnectApp';

const button = (label: string) => [...document.querySelectorAll('button')].find((entry) => entry.getAttribute('aria-label') === label) ?? null;
const PUBLIC = 'https://lys.example.test';

afterEach(() => { vi.unstubAllGlobals(); });

it('connects another app by its web address alone, sending the browser to its connect address with Lys’s', async () => {
  const assign = vi.fn();
  await mount('#/apps', { ...SERVICE, '/apps': () => ok({ apps: [], public_address: PUBLIC, public_address_reason: null }) });
  vi.stubGlobal('location', { ...window.location, assign, origin: window.location.origin });
  expect(document.querySelector('[aria-label="Connect an app"] h3')?.textContent).toBe('Connect an app');
  expect(button('Connect another app')?.classList.contains('primary')).toBe(false);
  await click(button('Connect another app'));
  const row = document.querySelector('[aria-label="Another app"]');
  const field = row?.querySelector<HTMLInputElement>('input[aria-label="The app web address"]') ?? null;
  expect(field?.placeholder).toBe('https://app.example.com');
  expect(row?.textContent).toContain('The app’s web address');
  await type(field, 'http://localhost:7300/somewhere');
  expect(row?.textContent).toContain('The app will call Lys back at ' + PUBLIC + '.');
  await click(button('Continue to the app'));
  expect(assign).toHaveBeenCalledWith('http://localhost:7300' + CONNECT_PATH + '?lys=' + encodeURIComponent(PUBLIC));
});

it('offers only Another app on the Connect card, naming no app', async () => {
  await mount('#/apps', { ...SERVICE, '/apps': () => ok({ apps: [], public_address: PUBLIC, public_address_reason: null }) });
  const card = document.querySelector('[aria-label="Connect an app"]');
  expect(card?.querySelectorAll('[aria-label^="Known app"]')).toHaveLength(0);
  expect(card?.textContent).not.toContain('Cambium');
  expect([...(card?.querySelectorAll('button') ?? [])].map((entry) => entry.getAttribute('aria-label'))).toEqual(['Connect another app']);
});

it('names an app it has no list entry for by the name its manifest gives', async () => {
  const origin = 'https://liminal.example.test';
  const manifest = origin + '/identity/lys/manifest';
  const hash = '#/apps/connect?manifest=' + encodeURIComponent(manifest) + '&return=' + encodeURIComponent(origin + '/back') + '&state=s1';
  const read = '/apps/connect/manifest?address=' + encodeURIComponent(manifest);
  await mount(hash, { ...SERVICE, '/apps': () => ok({ apps: [] }), [read]: () => ok({ manifest: { id: 'liminal', name: 'Liminal', redirects: [], profile: false, schema: { kinds: {} } }, origin }) });
  await settle();
  expect(document.querySelector('[aria-label="Connect an app"] h3')?.textContent).toBe('Liminal');
  expect(button('Approve Liminal')).not.toBeNull();
});
