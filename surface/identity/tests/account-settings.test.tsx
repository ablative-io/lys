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
  return $$('form[aria-label^="Change your"] input') as HTMLInputElement[];
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
    expect($('nav[aria-label="Configuration views"] a[aria-current="page"]')?.getAttribute('href')).toBe('#/settings/account');
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
    const posted = await open('#/settings/account', { ...SERVICE, 'POST /me/account/email': ok({ email: 'ada@lovelace.test', enabled: true }) });
    await press('Change email');
    fill({ 'account-email': ' ada@lovelace.test ', 'account-password': 'Confirmation-password' });
    await save('Change your email');
    expect(posted).toEqual([{ path: '/me/account/email', body: { email: 'ada@lovelace.test', password: 'Confirmation-password' } }]);
    expect($$('input[type="password"]')).toHaveLength(0);
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
