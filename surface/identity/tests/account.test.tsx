/** A Lys account's email, password and sign-in are changed on Lys screens, through the service (DIRECTORY-047 R5). */
import { act } from 'react';
import { describe, expect, it } from 'vitest';
import { $, $$, mount, settle, text, click } from './harness';
import { BEA, SERVICE, ok, refused } from './fixtures';

async function fill(form: string, values: Record<string, string>) {
  await act(async () => {
    for (const [name, value] of Object.entries(values)) {
      const input = document.querySelector<HTMLInputElement>(`form[aria-label="${form}"] input[name="${name}"]`);
      if (!input) throw new Error(`no ${name} field in ${form}`);
      input.value = value;
    }
  });
}

async function submit(form: string) {
  const element = $(`form[aria-label="${form}"]`);
  if (!element) throw new Error(`no ${form} form`);
  await act(async () => { element.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })); });
  await settle();
}

const ACCOUNT = '/directory/people/' + BEA + '/account';
const BEA_LOGINS = { ['/identities/' + BEA]: ok({ id: BEA, logins: [] }) };

describe('Your Lys account', () => {
  it('proves both mailboxes after confirming your password', async () => {
    const { posted } = await mount('#/settings/account', { ...SERVICE, 'POST /me/account/email': ok({ pending: true }), 'POST /me/account/email/confirm': ok({ email: 'ada@lovelace.test', enabled: true }) });
    expect(text()).toContain('You sign in with ada@example.test.');
    await click($('button[aria-label="Change email"]'));
    await fill('Change your email', { 'account-email': ' ada@lovelace.test ', 'account-password': 'Analytical-Engine-1843' });
    await submit('Change your email');
    expect(posted).toEqual([{ path: '/me/account/email', body: { email: 'ada@lovelace.test', password: 'Analytical-Engine-1843' } }]);
    expect(text()).toContain('You sign in with ada@example.test.');
    expect(text()).not.toContain('You now sign in with ada@lovelace.test.');
    expect(text()).toContain('Check both your new email and the email you are leaving');
    await fill('Confirm your email', { 'account-email-code': 'mailed-code', 'account-held-email-code': 'held-code' });
    await submit('Confirm your email');
    expect(posted[1]).toEqual({ path: '/me/account/email/confirm', body: { code: 'mailed-code', held_code: 'held-code' } });
    expect(text()).toContain('You now sign in with ada@lovelace.test.');
    expect($('input[name="account-password"]')).toBeNull();
  });

  it('refuses two different new passwords before sending anything', async () => {
    const { posted } = await mount('#/settings/account', SERVICE);
    await click($('button[aria-label="Change password"]'));
    await fill('Change your password', { 'account-current-password': 'Old-Password-12345', 'account-new-password': 'New-Password-12345', 'account-new-password-again': 'New-Password-12346' });
    await submit('Change your password');
    expect(posted).toEqual([]);
    expect(text()).toContain('The two new passwords are not the same.');
  });
});

describe("The administrator's account screen", () => {
  it('resets a password and disables sign-in, each through the service', async () => {
    let enabled = true;
    const { posted } = await mount('#/file/' + BEA + '/credentials', {
      ...SERVICE,
      ...BEA_LOGINS,
      [ACCOUNT]: ok({ email: 'ada@example.test', enabled: true }),
      ['POST ' + ACCOUNT + '/password']: ok({ email: 'ada@example.test', enabled: true }),
      ['POST ' + ACCOUNT + '/enabled']: (body) => {
        enabled = (body as { enabled: boolean }).enabled;
        return ok({ email: 'ada@example.test', enabled });
      },
    });
    expect(text()).toContain('Signs in with ada@example.test. Sign-in is enabled.');
    expect($$('section[aria-label="Lys account"] form input').every((input) => input.closest('.field') !== null)).toBe(true);
    await fill("Reset this person's password", { 'account-new-password': 'Reset-Password-777', 'account-new-password-again': 'Reset-Password-777' });
    await submit("Reset this person's password");
    await submit('Disable sign-in');
    expect(posted).toEqual([
      { path: ACCOUNT + '/password', body: { password: 'Reset-Password-777' } },
      { path: ACCOUNT + '/enabled', body: { enabled: false } },
    ]);
    expect(text()).toContain('They can no longer sign in.');
    expect($('form[aria-label="Enable sign-in"]')).not.toBeNull();
  });

  it("names the service's refusal in Lys words", async () => {
    await mount('#/file/' + BEA + '/credentials', {
      ...SERVICE,
      ...BEA_LOGINS,
      [ACCOUNT]: ok({ email: 'ada@example.test', enabled: true }),
      ['POST ' + ACCOUNT + '/email']: refused(400, 'AccountRefused', 'AccountRefused: an account with that email already exists'),
    });
    await fill("Change this person's email", { 'account-email': 'bea@example.test' });
    await submit("Change this person's email");
    expect($('[role="alert"]')?.textContent).toBe('an account with that email already exists');
  });
});

describe('Email proof (DIRECTORY-122)', () => {
  it('tells a password-less person to ask an administrator when mail is absent', async () => {
    await mount('#/settings/account', { ...SERVICE,
      '/me/account': ok({ email: 'ada@example.test', enabled: true, password: false }),
      'POST /me/account/email': refused(409, 'EmailChangeNeedsMail', "EmailChangeNeedsMail: Lys has no mail settings. Ask an administrator to change the email from your Account page."),
    });
    await click($('button[aria-label="Change email"]'));
    await fill('Change your email', { 'account-email': 'annabel@example.test' });
    await submit('Change your email');
    expect($('[role="alert"]')?.textContent).toContain('Ask an administrator');
    expect(text()).toContain('You sign in with ada@example.test.');
    expect($('form[aria-label="Confirm your email"]')).toBeNull();
  });
});
