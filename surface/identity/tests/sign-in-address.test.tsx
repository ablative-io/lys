/**
 * A provider returns only to Lys's own address. A page opened elsewhere says so before the provider is tried,
 * when the service names its address; a return refused for want of the browser binding names that cause.
 */
import { expect, it } from 'vitest';
import { $, $$, mount, unmountAll } from './harness';
import { SERVICE, ok } from './fixtures';

const google = { id: 'provider-1', name: 'Google', provider: 'google' };
const answered = (origin?: string) => ({ ...SERVICE, '/sign-in/providers': ok({ providers: [google], ...(origin === undefined ? {} : { origin }) }) });

it('warns before Google is tried on a page opened at another address, and links to the same page there', async () => {
  await mount('#/sign-in', answered('https://lys.example.test'));
  expect($('.sign-in-providers [role="status"]')?.textContent).toBe('Sign in with Google works only at lys.example.test. You opened Lys at ' + location.host + '.');
  expect($$('.sign-in-providers a').map((link) => [link.textContent, link.getAttribute('href')])).toEqual([['Open Lys at lys.example.test', 'https://lys.example.test/#/sign-in']]);
});

it('offers Google as before at Lys’s own address, or when the service does not name one', async () => {
  for (const origin of [location.origin, undefined]) {
    await mount('#/sign-in', answered(origin));
    expect($$('.sign-in-providers a').map((link) => link.textContent?.trim())).toEqual(['Sign in with Google']);
    expect($('.sign-in-providers [role="status"]')).toBeNull();
    unmountAll();
    document.body.innerHTML = '';
  }
});

it('never links to an address that is not a web origin', async () => {
  await mount('#/sign-in', answered('javascript:alert(1)'));
  expect($$('.sign-in-providers a').map((link) => link.textContent?.trim())).toEqual(['Sign in with Google']);
});
