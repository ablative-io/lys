// DIRECTORY-079 R2 on the Apps screen: an approved app's sign-in settings are shown as last set, the
// administrator changes an address or turns the name on and off through POST /apps/{app}/sign_in, and a
// refusal is shown in Lys's own words. Not opened in a browser: these run under jsdom.
import { act } from 'react';
import { expect, it } from 'vitest';
import { click, mount, settle, text } from './harness';
import { ME, SERVICE, ok, refused } from './fixtures';
import { PAGES } from '../src/shell/help/pages';

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
const approved = { id: APP, name: 'Notes fixture', state: 'approved', redirects: [BACK], sign_in: signIn, schema: { kinds: {} }, version: 1, versions: [1], pending: null, client_id: APP, service_account: null, client_credentials: [], registered_by: { kind: 'start' }, registered_at: 1 };
const ada = { kind: 'person', login: { issuer: 'https://issuer.example.test', subject: 'ada' } };

const button = (label: string) => [...document.querySelectorAll('button')].find((entry) => (entry.getAttribute('aria-label') ?? entry.textContent) === label) ?? null;
const addresses = () => document.querySelector<HTMLTextAreaElement>('textarea[aria-label="Return addresses of ' + APP + '"]');
const nameTick = () => document.querySelector<HTMLInputElement>('[aria-label="Change the sign-in of ' + APP + '"] input[type="checkbox"]');
const settings = () => document.querySelector('[aria-label="Sign-in settings of ' + APP + '"]')?.textContent ?? '';
/** The refusal's plain words, never its code. */
const alert = () => document.querySelector('[role="alert"] > p')?.textContent ?? '';
/** The refusal as Lys answered it, kept inside the Error details fold. */
const details = () => document.querySelector('[role="alert"] details .refusal-name')?.textContent ?? '';

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
  // The sign-in that set it is the signed-in person's own, so it is named through the directory and linked to their file.
  expect(settings()).toContain('as last set by ' + ME.person.display_name);
  expect(document.querySelector('[aria-label="Sign-in settings of ' + APP + '"] a')?.getAttribute('href')).toBe('#/file/' + ME.person.id);
  expect(settings()).not.toContain('as last set by ada');
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
  expect(alert()).toBe('Lys refused: ' + plain + ' is not admitted: it is not an https address');
  expect(alert()).not.toContain('redirect_invalid');
  expect(details()).toBe('redirect_invalid: ' + plain + ' is not admitted: it is not an https address');
  expect(settings()).toContain(BACK);
  expect(settings()).not.toContain(plain);
  expect(text()).not.toContain('now sends people back to');
});

it('says a refusal’s name once when Lys’s reason already carries it', async () => {
  const reason = 'redirect_invalid: `not-an-address` is not an absolute address';
  await mount('#/apps', routes(() => refused(400, 'redirect_invalid', reason)));
  await type(addresses(), 'not-an-address');
  await click(button('Save the sign-in of Notes fixture'));
  expect(alert()).toBe('Lys refused: `not-an-address` is not an absolute address');
  expect(details()).toBe(reason);
});

it('shows a non-administrator’s refusal in Lys’s words and tries nothing again on its own', async () => {
  const { posted } = await mount('#/apps', routes(() => refused(403, 'NotAdmitted', 'only the administrator sets an app’s sign-in')));
  await type(addresses(), MOVED);
  await click(button('Save the sign-in of Notes fixture'));
  expect(posted).toHaveLength(1);
  expect(alert()).toBe('You do not have permission to do this. Ask the directory administrator to make the change.');
  expect(details()).toBe('NotAdmitted: only the administrator sets an app’s sign-in');
  expect(settings()).toContain(BACK);
  expect(settings()).not.toContain(MOVED);
  expect(button('Save the sign-in of Notes fixture')).not.toBeNull();
});

// DIRECTORY-097 R3: both forms say, in one sentence, that an https address may name every machine of a
// domain with `*.` as its first label and that http addresses are admitted only on this machine; and a
// refused wildcard line is shown with the line and Lys's reason.
const WILD_HELP = 'An https address may name every machine of a domain with *. as its first label, such as https://*.example.com/signin/return; http addresses are admitted only on this machine.';

it('explains the wildcard form and the device-local http rule on the sign-in settings and on the registration form', async () => {
  await mount('#/apps', routes());
  const hints = [...document.querySelectorAll('.hint')].map((entry) => entry.textContent ?? '');
  const signIn = document.querySelector('[aria-label="Change the sign-in of ' + APP + '"]')?.textContent ?? '';
  expect(signIn).toContain(WILD_HELP);
  const register = document.querySelector('[aria-label="Register an app"]')?.textContent ?? '';
  expect(register).toContain(WILD_HELP);
  expect(hints.filter((words) => words.includes(WILD_HELP))).toHaveLength(2);
});

it('shows a refused wildcard line with its reason', async () => {
  const wild = 'http://*.example.com/signin/return';
  const reason = 'redirect_invalid: `' + wild + '` is neither https nor http to this machine';
  await mount('#/apps', routes(() => refused(400, 'redirect_invalid', reason)));
  await type(addresses(), BACK + '\n' + wild);
  await click(button('Save the sign-in of Notes fixture'));
  expect(alert()).toBe('Lys refused: `' + wild + '` is neither https nor http to this machine');
  expect(details()).toBe(reason);
  expect(settings()).toContain(BACK);
  expect(settings()).not.toContain(wild);
});

// DIRECTORY-098 R2: an address the app declared with its credential, or the administrator declared for
// it, carries the line "declared by the app, <time>" or "declared by <login>, <time>" under it, read
// from the settings' `declared`; an address saved in the list carries none; saving the list without a
// declared address posts the list without it; and the help page says so.
const DECLARED = 'https://notes.example.test/signin/return';
const AT = 1760780000;
const WHEN = '2025-10-18 09:33 UTC';
const declaredSignIn = (by: unknown) => ({ ...signIn, redirects: [BACK, DECLARED], declared: [{ address: DECLARED, by, at: AT }] });
/** The settings' list items, each with its own words. */
const items = () => [...document.querySelectorAll('[aria-label="Sign-in settings of ' + APP + '"] li')].map((entry) => entry.textContent ?? '');

function declaredRoutes(by: unknown) {
  let current: unknown = { ...approved, sign_in: declaredSignIn(by) };
  return {
    ...SERVICE,
    '/apps': () => ok({ apps: [current] }),
    ['POST /apps/' + APP + '/sign_in']: (body: unknown) => {
      const asked = body as { redirects: string[]; profile: boolean };
      current = { ...approved, sign_in: { redirects: asked.redirects, profile: asked.profile, operation: 'op-change', by: ada, at: 3 } };
      return ok(current);
    },
  };
}

it('shows an address the app declared with who declared it and when, and a saved address with no such line', async () => {
  await mount('#/apps', declaredRoutes({ kind: 'service_account', id: APP }));
  expect(items()).toEqual([BACK, DECLARED + 'declared by the app, ' + WHEN]);
  expect(document.querySelectorAll('.declared-by')).toHaveLength(1);
});

it('names the administrator who declared an address by their login', async () => {
  await mount('#/apps', declaredRoutes(ada));
  expect(items()[1]).toBe(DECLARED + 'declared by ada, ' + WHEN);
});

it('saving the list without the declared address posts the list without it, and the line is gone', async () => {
  const { posted } = await mount('#/apps', declaredRoutes({ kind: 'service_account', id: APP }));
  expect(addresses()?.value).toBe(BACK + '\n' + DECLARED);
  await type(addresses(), BACK);
  await click(button('Save the sign-in of Notes fixture'));
  expect(posted).toEqual([{ path: '/apps/' + APP + '/sign_in', body: { operation: expect.stringMatching(/^op-/), redirects: [BACK], profile: true } }]);
  expect(items()).toEqual([BACK]);
  expect(settings()).not.toContain('declared by');
});

it('the help page says an app may declare its own return address and that the list saved there is the whole list', () => {
  const help = PAGES.apps.regions.find((region) => region.sel === '.app-signin')?.text ?? '';
  expect(help).toContain('An app may declare its own return address with its credential, and such an address is shown here with who declared it and when.');
  expect(help).toContain('The list saved here is the whole list: saving it without a declared address removes it.');
});
