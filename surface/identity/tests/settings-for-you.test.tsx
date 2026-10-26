/** Settings is split by whose each setting is: a person's own sign-in and layout for everyone, the service's for its administrators. */
import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { expect, it } from 'vitest';
import { EffectiveSettings } from '../src/features/settings/EffectiveSettings';
import { $, $$, mount, serve, text } from './harness';
import { SERVICE, ok, refused } from './fixtures';

const settings = { source: 'startup_configuration', mutable_in_browser: false, sign_in: { provider_origin: 'https://login.test', session_seconds: 3600, secure_cookie: true }, directory: { roles_configured: true }, permissions: { model_version: 8, projection: 'local' }, secrets: { configured: true }, runtimes: { machines_configured: true, provisioning_configured: true }, storage: { directory_format: 'signed_leaf_log', grant_format: 'signed_leaf_log', requests_configured: true } };
const tabs = () => $$('nav[aria-label="Settings views"] a').map((tab) => tab.textContent);

it('gives a person who is not an administrator their account and layout, and only their own tabs', async () => {
  await mount('#/settings/account', { ...SERVICE, '/configuration': refused(403, 'NotAdmitted', 'Administrator only') });
  expect($('section[aria-label="Your account"]')).not.toBeNull();
  expect($('section[aria-label="Layout"] [data-dock="right"]')).not.toBeNull();
  expect($('section[aria-label="Layout"] [data-labels="labels"]')).not.toBeNull();
  expect(tabs()).toEqual(['For you', 'Service accounts']);
  expect(document.body.textContent).not.toContain('NotAdmitted');
});

it('gives an administrator every Settings tab, the service settings apart from their own', async () => {
  await mount('#/settings/account', { ...SERVICE, '/configuration': ok(settings) });
  // Apps has its own rail entry, so it is not a Settings tab.
  expect(tabs()).toEqual(['For you', 'Service accounts', 'This service', 'Services and sign-in providers', 'Health']);
  expect($('.effective-settings')).toBeNull();
});

it('tells a person who reaches the service settings whose they are and where their own are', async () => {
  serve({ ...SERVICE, '/configuration': refused(403, 'NotAdmitted', 'Administrator only') });
  const container = document.createElement('div');
  document.body.appendChild(container);
  const root = createRoot(container);
  await act(async () => root.render(<EffectiveSettings />));
  expect(container.textContent).toBe('This service’s own settings are shown to its administrators. Yours are under For you.');
  expect(container.querySelector('a')?.getAttribute('href')).toBe('#/settings/account');
  act(() => root.unmount());
});

it('no longer guesses at a person with no Lys password: Your account says it from what Lys answers', async () => {
  await mount('#/settings/account', { ...SERVICE, '/configuration': refused(403, 'NotAdmitted', 'Administrator only') });
  expect($('.settings-no-password')).toBeNull();
  expect(text()).not.toContain('never set a Lys password');
});
