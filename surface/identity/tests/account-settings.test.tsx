import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, describe, expect, it } from 'vitest';
import { App } from '../src/App';
import { $, $$, serve, text } from './harness';
import { ADA, SERVICE, ok, refused } from './fixtures';

let root: ReturnType<typeof createRoot> | undefined;
afterEach(() => {
  act(() => root?.unmount());
  root = undefined;
});

async function open(path: string, routes = SERVICE) {
  const posted: { path: string; body: unknown }[] = [];
  serve(routes, posted);
  history.replaceState(null, '', path);
  const container = document.createElement('div');
  document.body.appendChild(container);
  root = createRoot(container);
  await act(async () => {
    root?.render(<App />);
  });
  return posted;
}

async function press(name: string) {
  const button = $(`button[aria-label="${name}"]`);
  if (!button) throw new Error(`Missing button: ${name}`);
  await act(async () => {
    button.click();
  });
}

function fields() {
  return $$('form[aria-label^="Change your"] input, form[aria-label^="Set your"] input') as HTMLInputElement[];
}

function fill(values: Record<string, string>) {
  for (const [name, value] of Object.entries(values)) {
    const input = document.querySelector<HTMLInputElement>(`input[name="${name}"]`);
    if (!input) throw new Error(`Missing field: ${name}`);
    input.value = value;
  }
}

async function save(label: string) {
  const form = $(`form[aria-label="${label}"]`);
  if (!form) throw new Error(`Missing form: ${label}`);
  await act(async () => {
    form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
  });
}

describe('Account changes belong in Settings', () => {
  it('keeps every password and email input off You and links to Settings', async () => {
    await open('#/me');
    expect($$('input[type="password"]')).toHaveLength(0);
    expect($$('input[type="email"]')).toHaveLength(0);
    expect(text()).toContain('You sign in with ada@example.test.');
    expect($('#lys-account a')?.getAttribute('href')).toBe('#/settings/account');
    expect($('#lys-account a')?.textContent).toBe('Change in Settings');
  });

  it('renders Your account with no password inputs before either change is requested', async () => {
    await open('#/settings/account');
    expect($('section[aria-label="Your account"] h2')?.textContent).toBe('Your account');
    expect($$('input[type="password"]')).toHaveLength(0);
    expect($$('input[type="email"]')).toHaveLength(0);
    expect($('button[aria-label="Change email"]')).not.toBeNull();
    expect($('button[aria-label="Change password"]')).not.toBeNull();
    expect($('nav[aria-label="Settings views"] a[aria-current="page"]')?.getAttribute('href')).toBe('#/settings/account');
  });

  it.each(['Change email', 'Change password'])('opens %s empty, cancels without a write, and opens empty again', async (name) => {
    const posted = await open('#/settings/account');
    await press(name);
    const expected = name === 'Change email' ? ['email', 'current-password'] : ['current-password', 'new-password', 'new-password'];
    expect(fields().map((input) => input.autocomplete)).toEqual(expected);
    expect(fields().map((input) => input.value)).toEqual(expected.map(() => ''));
    expect(fields().every((input) => !input.hasAttribute('value'))).toBe(true);
    fields().forEach((input) => {
      input.value = 'unsaved';
    });
    await press('Cancel');
    expect($$('input[type="password"]')).toHaveLength(0);
    expect(posted).toEqual([]);
    await press(name);
    expect(fields().map((input) => input.value)).toEqual(expected.map(() => ''));
  });

  it('saves the email, closes the form, and reopens every field empty', async () => {
    const posted = await open('#/settings/account', { ...SERVICE, 'POST /me/account/email': ok({ pending: true }), 'POST /me/account/email/confirm': ok({ email: 'ada@lovelace.test', enabled: true }) });
    await press('Change email');
    fill({ 'account-email': ' ada@lovelace.test ', 'account-password': 'Confirmation-password' });
    await save('Change your email');
    expect(posted).toEqual([{ path: '/me/account/email', body: { email: 'ada@lovelace.test', password: 'Confirmation-password' } }]);
    expect($$('input[type="password"]')).toHaveLength(0);
    expect(text()).toContain('You sign in with ada@example.test.');
    expect(text()).not.toContain('You now sign in with ada@lovelace.test.');
    fill({ 'account-email-code': 'mailed-code', 'account-held-email-code': 'held-code' });
    await save('Confirm your email');
    expect(posted[1]).toEqual({ path: '/me/account/email/confirm', body: { code: 'mailed-code', held_code: 'held-code' } });
    expect(text()).toContain('You now sign in with ada@lovelace.test.');
    await press('Change email');
    expect(fields().map((input) => input.value)).toEqual(['', '']);
  });

  it('saves a new password, closes the form, and reopens every field empty', async () => {
    const posted = await open('#/settings/account', { ...SERVICE, 'POST /me/account/password': ok({ email: 'ada@example.test', enabled: true }) });
    await press('Change password');
    fill({ 'account-current-password': 'Current-password', 'account-new-password': 'New-password', 'account-new-password-again': 'New-password' });
    await save('Change your password');
    expect(posted).toEqual([{ path: '/me/account/password', body: { current: 'Current-password', password: 'New-password' } }]);
    expect($$('input[type="password"]')).toHaveLength(0);
    expect(text()).toContain('Your password is changed.');
    await press('Change password');
    expect(fields().map((input) => input.value)).toEqual(['', '', '']);
  });

  it('keeps a refused change open and names the reason', async () => {
    await open('#/settings/account', { ...SERVICE, 'POST /me/account/email': refused(400, 'AccountRefused', 'AccountRefused: that email is already in use') });
    await press('Change email');
    fill({ 'account-email': 'bea@example.test', 'account-password': 'Confirmation-password' });
    await save('Change your email');
    expect($('[role="alert"]')?.textContent).toBe('that email is already in use');
    expect($$('input[type="password"]')).toHaveLength(1);
  });

  it('lets a person change their own account when startup configuration is administrator-only', async () => {
    await open('#/settings/account', { ...SERVICE, '/configuration': refused(403, 'NotAdmitted', 'Administrator only') });
    await press('Change email');
    expect(fields()).toHaveLength(2);
  });

  it('redirects the old account address to Your account in Settings', async () => {
    await open('#/account/' + ADA);
    expect(location.hash).toBe('#/settings/account');
    expect($$('input[type="password"]')).toHaveLength(0);
    expect($('section[aria-label="Your account"]')).not.toBeNull();
  });
});

/** The three kinds of account `/me/account` says: a password only, Google only, and both. */
describe('Your account says how you sign in', () => {
  const as = (password: boolean, providers: string[]) => ({ ...SERVICE, '/me/account': ok({ email: 'ada@example.test', enabled: true, password, providers }) });
  const row = (name: string) => $$('section[aria-label="Your account"] .set-row').find((each) => each.firstElementChild?.firstElementChild?.textContent === name)?.textContent ?? '';

  it('with a password only: the email, and a change that asks for the current password', async () => {
    await open('#/settings/account', as(true, []));
    expect(row('Email')).toContain('You sign in with ada@example.test.');
    await press('Change password');
    expect(fields().map((input) => input.autocomplete)).toEqual(['current-password', 'new-password', 'new-password']);
  });

  it('with Google only: says so, sets a password with no current one, and changes the email with no password', async () => {
    const posted = await open('#/settings/account', { ...as(false, ['google']),
      'POST /me/account/password': ok({ email: 'ada@example.test', enabled: true, password: true, providers: ['google'] }),
      'POST /me/account/email': ok({ email: 'ada@lovelace.test', enabled: true, password: false, providers: ['google'] }) });
    expect(row('Email')).toContain('You sign in with Google.');
    expect(row('Password')).toContain('You have no Lys password yet.');
    expect($('button[aria-label="Change password"]')).toBeNull();
    await press('Change email');
    expect(fields().map((input) => input.autocomplete)).toEqual(['email']);
    await press('Cancel');
    await press('Set a password');
    expect(fields().map((input) => input.autocomplete)).toEqual(['new-password', 'new-password']);
    fill({ 'account-new-password': 'First-password', 'account-new-password-again': 'First-password' });
    await save('Set your password');
    expect(posted).toEqual([{ path: '/me/account/password', body: { password: 'First-password' } }]);
    expect(text()).toContain('Your password is set. You can sign in with it or with Google.');
    // Now it has a password, a change asks for it.
    await press('Change email');
    expect(fields().map((input) => input.autocomplete)).toEqual(['email', 'current-password']);
  });

  it('with both: names the email and Google, and asks for the current password', async () => {
    await open('#/settings/account', as(true, ['google']));
    expect(row('Email')).toContain('You sign in with ada@example.test or with Google.');
    await press('Change email');
    expect(fields().map((input) => input.autocomplete)).toEqual(['email', 'current-password']);
  });
});
