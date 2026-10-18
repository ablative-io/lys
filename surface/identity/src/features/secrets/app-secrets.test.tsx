// Domain 04 item 10: a secret Lys keeps for an approved app is the app's, shown with its settings and changed only with the
// app on Apps; every secret's heading is its friendly name; "who it can be handed to" starts from what Lys holds, and while
// that is unknown Save is off, so a save never narrows it unseen. Each case keys on what the service answered.
import { expect, it } from 'vitest';
import { $, mount, settle } from '../../../tests/harness';
import { ADA, SERVICE, ok, refused } from '../../../tests/fixtures';

const APP_SECRET = 'lys-app-' + ADA + '-cambium-client';
const listing = (name: string) => ({ secrets: [{ name, class: 'credential', owner: ADA, sequence: 1, upstream: null, header: null }] });
const settings = (name: string, recipients: string) => ({ secret: name, scope: null, recipients });
const controls = (name: string) => document.querySelector('section[aria-label="Controls of ' + name + '"]');
const recipients = (name: string) => [...(controls(name)?.querySelectorAll('form') ?? [])]
  .find((form) => form.querySelector('h2')?.textContent === 'Who it can be handed to')?.querySelector('select') ?? null;
const save = () => [...document.querySelectorAll('button')].find((entry) => entry.getAttribute('aria-label') === 'Save who it can be handed to') ?? null;

it('shows an app’s own secret by its friendly name, says it changes with the app on Apps, and offers no change here', async () => {
  await mount('#/secrets/entries?secret=' + encodeURIComponent(APP_SECRET), { ...SERVICE, '/secrets': ok(listing(APP_SECRET)),
    ['/secrets/settings?secret=' + encodeURIComponent(APP_SECRET)]: ok(settings(APP_SECRET, 'anyone')) });
  await settle();
  const section = controls(APP_SECRET);
  expect(section?.querySelector('h2')?.textContent).toBe('Cambium · Ada (test person) · Sign-in credential');
  expect(section?.querySelector('.app-owned')?.textContent).toContain('This secret is Cambium’s own');
  expect(section?.textContent).not.toContain('stays held below');
  expect(section?.querySelector('.app-owned a')?.getAttribute('href')).toBe('#/apps');
  expect(section?.querySelectorAll('form')).toHaveLength(0);
  expect(section?.textContent).toContain('May be handed to: anyone who is permitted.');
});

it('keeps all four changes for a secret a person put in, and starts who it can be handed to from what Lys holds', async () => {
  await mount('#/secrets/entries?secret=Calendar', { ...SERVICE, '/secrets': ok(listing('Calendar')), '/secrets/settings?secret=Calendar': ok(settings('Calendar', 'anyone')) });
  await settle();
  expect(controls('Calendar')?.querySelectorAll('form')).toHaveLength(4);
  expect(controls('Calendar')?.querySelector('.app-owned')).toBeNull();
  expect(recipients('Calendar')?.value).toBe('anyone');
  expect(save()?.hasAttribute('disabled')).toBe(false);
});

it('starts on Choose with Save off when Lys could not say who it can be handed to now', async () => {
  await mount('#/secrets/entries?secret=Calendar', { ...SERVICE, '/secrets': ok(listing('Calendar')), '/secrets/settings?secret=Calendar': refused(503, 'BrokerUnavailable', 'broker closed') });
  await settle();
  expect(recipients('Calendar')?.value).toBe('');
  expect(recipients('Calendar')?.options[0]?.textContent).toBe('Choose: Lys could not say what it is now');
  expect(save()?.hasAttribute('disabled')).toBe(true);
  expect($('[aria-label="Controls of Calendar"]')).not.toBeNull();
});
