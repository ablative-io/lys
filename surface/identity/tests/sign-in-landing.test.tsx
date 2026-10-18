/**
 * One landing for every sign-in: Home from the sign-in page, or back on the page that asked for sign-in, whichever
 * way the person signed in. The service returns a provider sign-in to /me; that return is a hop taken once.
 */
import { beforeEach, expect, it } from 'vitest';
import { $, click, mount, unmountAll } from './harness';
import { SERVICE, ok, refused } from './fixtures';
import type { Route } from './fixtures';

const providers = { '/sign-in/providers': ok({ providers: [{ id: 'google-1', name: 'Google', provider: 'google' }] }) };
const signedOut: Record<string, Route> = { ...SERVICE, ...providers, '/me': refused(401, 'NotSignedIn', 'Sign in first'), '/directory/people': refused(401, 'NotSignedIn', 'Sign in first') };
const signedIn: Record<string, Route> = { ...SERVICE, ...providers };

beforeEach(() => sessionStorage.clear());

/** Press Sign in with Google as a person would, without leaving the test's page. */
async function pressGoogle(): Promise<void> {
  const google = $('a[href^="/api/sign-in/providers/google-1"]');
  expect(google, 'Sign in with Google is offered').not.toBeNull();
  google?.addEventListener('click', (event) => event.preventDefault());
  await click(google);
  unmountAll();
}

it('lands a Google sign-in begun on the sign-in page on Home, not on You', async () => {
  await mount('#/sign-in', signedOut);
  await pressGoogle();
  await mount('#/me', signedIn);
  expect(location.hash).toBe('#/');
  expect($('.you-page'), 'You is not what is drawn').toBeNull();
  expect($('h1')?.textContent).toBe('Dashboard');
});

it('lands a Google sign-in begun on a page back on that page', async () => {
  await mount('#/people', signedOut);
  await pressGoogle();
  await mount('#/me', signedIn);
  expect(location.hash).toBe('#/people');
  expect($('.you-page'), 'You is not what is drawn').toBeNull();
  expect($('h1')?.textContent).toBe('People and agents');
});

it('takes the landing once: a later visit to You stays on You', async () => {
  await mount('#/sign-in', signedOut);
  await pressGoogle();
  await mount('#/me', signedIn);
  unmountAll();
  await mount('#/me', signedIn);
  expect(location.hash).toBe('#/me');
  expect($('.you-page')).not.toBeNull();
});

it('drops a landing left by a Google sign-in that never came back, when a sign-in form opens again', async () => {
  await mount('#/people', signedOut);
  await pressGoogle();
  await mount('#/sign-in', signedOut);
  unmountAll();
  await mount('#/me', signedIn);
  expect(location.hash).toBe('#/me');
  expect($('.you-page')).not.toBeNull();
});
