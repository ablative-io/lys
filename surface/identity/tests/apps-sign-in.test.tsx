// DIRECTORY-079 R2 on the Apps screen: an approved app's sign-in settings are shown as last set, the
// administrator changes an address or turns the name on and off through POST /apps/{app}/sign_in, and a
// refusal is shown in Lys's own words. Not opened in a browser: these run under jsdom.
import { act } from 'react';
import { expect, it } from 'vitest';
import { click, mount, settle, text } from './harness';
import { SERVICE, ok, refused } from './fixtures';

/** Type into the addresses field, a textarea, as a person would. */
async function type(el: Element | null, value: string): Promise<void> {
  if (!(el instanceof HTMLTextAreaElement)) throw new Error('no textarea to type in');
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, 'value')?.set?.call(el, value);
    el.dispatchEvent(new Event('input', { bubbles: true }));
  });
  await settle();
}

const APP = 'fixture_notes';
const BACK = 'https://notes.example.test/signed-in';
const MOVED = 'https://notes.example.test/moved-here';
const signIn = { redirects: [BACK], profile: true, operation: 'op-approve', by: { kind: 'start' }, at: 2 };
const approved = { id: APP, name: 'Notes fixture', state: 'approved', redirects: [BACK], sign_in: signIn, schema: { kinds: {} }, version: 1, versions: [1], pending: null, client_id: APP, service_account: null, registered_by: { kind: 'start' }, registered_at: 1 };
const ada = { kind: 'person', login: { issuer: 'https://issuer.example.test', subject: 'ada' } };

const button = (label: string) => [...document.querySelectorAll('button')].find((entry) => (entry.getAttribute('aria-label') ?? entry.textContent) === label) ?? null;
const addresses = () => document.querySelector<HTMLTextAreaElement>('textarea[aria-label="Return addresses of ' + APP + '"]');
const nameTick = () => document.querySelector<HTMLInputElement>('[aria-label="Change the sign-in of ' + APP + '"] input[type="checkbox"]');
const settings = () => document.querySelector('[aria-label="Sign-in settings of ' + APP + '"]')?.textContent ?? '';
const alert = () => document.querySelector('[role="alert"]')?.textContent ?? '';

/** A service whose one app is approved, answering a sign-in change by keeping it, as Lys does. */
function routes(answer?: (body: { redirects: string[]; profile: boolean }) => ReturnType<typeof ok>) {
  let current = approved;
  return {
    ...SERVICE,
    '/apps': () => ok({ apps: [current] }),
    ['POST /apps/' + APP + '/sign_in']: (body: unknown) => {
      const asked = body as { redirects: string[]; profile: boolean };
      if (answer) return answer(asked);
      current = { ...approved, sign_in: { redirects: asked.redirects, profile: asked.profile, operation: 'op-change', by: ada, at: 3 } };
      return ok(current);
    },
  };
}

it('shows the sign-in settings as last set, and offers them to change', async () => {
  await mount('#/apps', routes());
  expect(settings()).toContain('Its sign-in, as last set by Lys at start');
  expect(settings()).toContain(BACK);
  expect(settings()).toContain('It is given the person’s name.');
  expect(addresses()?.value).toBe(BACK);
  expect(nameTick()?.checked).toBe(true);
  expect(button('Save the sign-in of Notes fixture')).not.toBeNull();
});

it('changes a return address through the sign-in route and shows the settings as kept', async () => {
  const { posted } = await mount('#/apps', routes());
  await type(addresses(), MOVED);
  await click(button('Save the sign-in of Notes fixture'));
  expect(posted).toEqual([{ path: '/apps/' + APP + '/sign_in', body: { operation: expect.stringMatching(/^op-/), redirects: [MOVED], profile: true } }]);
  expect(text()).toContain('Notes fixture’s sign-in now sends people back to ' + MOVED + ' and gives it their name.');
  expect(settings()).toContain('as last set by ada');
  expect(settings()).toContain(MOVED);
  expect(settings()).not.toContain(BACK);
});

it('turns the name off and on again, each kept by the sign-in route', async () => {
  const { posted } = await mount('#/apps', routes());
  await click(nameTick());
  await click(button('Save the sign-in of Notes fixture'));
  expect(posted[0]).toEqual({ path: '/apps/' + APP + '/sign_in', body: { operation: expect.stringMatching(/^op-/), redirects: [BACK], profile: false } });
  expect(text()).toContain('and withholds their name.');
  expect(settings()).toContain('It is not given the person’s name.');
  expect(nameTick()?.checked).toBe(false);
  await click(nameTick());
  await click(button('Save the sign-in of Notes fixture'));
  expect(posted).toHaveLength(2);
  expect(posted[1]).toEqual({ path: '/apps/' + APP + '/sign_in', body: { operation: expect.stringMatching(/^op-/), redirects: [BACK], profile: true } });
  expect(settings()).toContain('It is given the person’s name.');
  expect(nameTick()?.checked).toBe(true);
});

it('shows a refused address in Lys’s words and keeps the settings as they were', async () => {
  const plain = 'http://notes.example.test/plain';
  const { posted } = await mount('#/apps', routes(() => refused(400, 'redirect_invalid', plain + ' is not admitted: it is not an https address')));
  await type(addresses(), plain);
  await click(button('Save the sign-in of Notes fixture'));
  expect(posted).toHaveLength(1);
  expect(alert()).toBe('redirect_invalid: ' + plain + ' is not admitted: it is not an https address');
  expect(settings()).toContain(BACK);
  expect(settings()).not.toContain(plain);
  expect(text()).not.toContain('now sends people back to');
});

it('says a refusal’s name once when Lys’s reason already carries it', async () => {
  const reason = 'redirect_invalid: `not-an-address` is not an absolute address';
  await mount('#/apps', routes(() => refused(400, 'redirect_invalid', reason)));
  await type(addresses(), 'not-an-address');
  await click(button('Save the sign-in of Notes fixture'));
  expect(alert()).toBe(reason);
});

it('shows a non-administrator’s refusal in Lys’s words and tries nothing again on its own', async () => {
  const { posted } = await mount('#/apps', routes(() => refused(403, 'NotAdmitted', 'only the administrator sets an app’s sign-in')));
  await type(addresses(), MOVED);
  await click(button('Save the sign-in of Notes fixture'));
  expect(posted).toHaveLength(1);
  expect(alert()).toBe('NotAdmitted: only the administrator sets an app’s sign-in');
  expect(settings()).toContain(BACK);
  expect(settings()).not.toContain(MOVED);
  expect(button('Save the sign-in of Notes fixture')).not.toBeNull();
});
