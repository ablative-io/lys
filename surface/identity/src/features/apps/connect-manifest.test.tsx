// Domain 04 item 4: a manifest the page could not read is said as the two causes it could be, with the manifest offered to
// open as the test between them; an https Lys reading a plain http Cambium is said by name before any read is tried.
import { afterEach, expect, it, vi } from 'vitest';
import { mount, settle } from '../../../tests/harness';
import { SERVICE, ok } from '../../../tests/fixtures';

const hash = (manifest: string) => '#/apps/connect?manifest=' + encodeURIComponent(manifest) + '&return=' + encodeURIComponent(new URL(manifest).origin + '/back') + '&state=s1';
const alert = () => document.querySelector('[role="alert"]')?.textContent ?? '';
const openManifest = () => [...document.querySelectorAll('a')].find((entry) => entry.textContent === 'Open Cambium’s manifest') ?? null;

afterEach(() => { vi.unstubAllGlobals(); });

it('says a rejected read as either not answering or not letting this Lys read it, and offers the manifest to open', async () => {
  const manifest = 'https://cambium.example.test/identity/lys/manifest';
  await mount(hash(manifest), { ...SERVICE, '/apps': () => ok({ apps: [] }), [manifest]: () => { throw new TypeError('Failed to fetch'); } });
  await settle();
  expect(alert()).toBe('Lys could not read Cambium’s manifest at https://cambium.example.test from this page, so it cannot tell whether Cambium is not answering there or does not let this Lys read it.');
  const which = document.querySelector('[aria-label="Find out which"]')?.textContent ?? '';
  expect(which).toContain('does not let Lys at ' + window.location.origin + ' read it');
  expect(which).toContain('Cambium is not answering at that address');
  expect(alert()).not.toContain('Check that Cambium is running and the address is right');
  expect(openManifest()?.getAttribute('href')).toBe(manifest);
  expect(openManifest()?.getAttribute('target')).toBe('_blank');
});

it('says an https Lys cannot read a plain http Cambium before trying, and offers no manifest to open', async () => {
  const manifest = 'http://cambium.example.test/identity/lys/manifest';
  const read = vi.fn(() => ok({}));
  vi.stubGlobal('location', { ...window.location, protocol: 'https:', origin: 'https://lys.example.test' });
  await mount(hash(manifest), { ...SERVICE, '/apps': () => ok({ apps: [] }), [manifest]: read });
  await settle();
  expect(alert()).toContain('Lys is open over https, and Cambium at http://cambium.example.test is served over plain http');
  expect(read).not.toHaveBeenCalled();
  expect(openManifest()).toBeNull();
  expect(document.querySelector('[aria-label="Find out which"]')).toBeNull();
});

it('reads a plain http Cambium on this computer from an https Lys, which browsers allow', async () => {
  const manifest = 'http://localhost:4000/identity/lys/manifest';
  const read = vi.fn(() => ok({ id: 'cambium', name: 'Cambium', redirects: [], profile: false, schema: { kinds: {} } }));
  vi.stubGlobal('location', { ...window.location, protocol: 'https:', origin: 'https://lys.example.test' });
  await mount(hash(manifest), { ...SERVICE, '/apps': () => ok({ apps: [] }), [manifest]: read });
  await settle();
  expect(read).toHaveBeenCalledTimes(1);
  expect(alert()).toBe('');
});
