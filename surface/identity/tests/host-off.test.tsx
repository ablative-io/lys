/** A tab open while the Lys computer is off says which address is not answering and the act that helps, never "ask the administrator". */
import { expect, it } from 'vitest';
import { $, mount } from './harness';
import { SERVICE, refused } from './fixtures';
import type { Route } from './fixtures';

const unreachable = (() => { throw new TypeError('Failed to fetch'); }) as unknown as Route;
const page = () => $('#screen .page, .page');

it('names the address that is not answering, the likely cause and the act, on a read', async () => {
  await mount('#/me', { ...SERVICE, '/me': unreachable });
  expect(page()?.querySelector('h1')?.textContent).toBe('Lys at ' + location.host + ' is not answering');
  const words = page()?.textContent ?? '';
  expect(words).toContain('The computer Lys runs on may be off, asleep or off the network: check that it is switched on and awake. When it answers, this page reads again by itself.');
  expect(words).not.toContain('administrator');
  expect(words).not.toContain('keep its original request');
  expect(page()?.querySelector('details')?.textContent).toContain('ServiceUnreachable');
  expect(page()?.querySelector('a[href="#/"]')).toBeNull();
});

it('keeps a service failure in its own words, without a resend warning on a read', async () => {
  await mount('#/me', { ...SERVICE, '/me': refused(503, 'DirectoryUnavailable', 'directory unavailable') });
  expect(page()?.querySelector('h1')?.textContent).toBe('Lys could not complete this request');
  expect(page()?.textContent).not.toContain('keep its original request');
  expect(page()?.textContent).not.toContain('is not answering');
});
