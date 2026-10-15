/** Setup is explicit, accessible, administrator-only and keeps one operation across retries. */
import { act } from 'react';
import { beforeEach, describe, expect, it } from 'vitest';
import { $, mount, settle, text, unmountAll, unreachable } from './harness';
import { ADA, ME, RECEIPTS, SERVICE, ok, refused } from './fixtures';

const needsSetup = refused(403, 'SetupRequired', 'administrator has no person');
beforeEach(() => sessionStorage.clear());

function recorded(body: unknown) {
  if (!body || typeof body !== 'object' || !('operation' in body)) throw new Error('Missing operation');
  return ok({ person: ADA, receipt: { ...RECEIPTS[4].receipt, operation: body.operation, identity: ADA, change_kind: 7 } });
}
/** Submit the finish form, first typing `given` and `family` when given. */
async function submit(given?: string, family?: string) {
  const form = $('form[aria-label="Finish setup"]');
  if (!form) throw new Error('Setup form missing');
  for (const [field, value] of [['given_name', given], ['family_name', family]] as const) {
    if (value === undefined) continue;
    const input = form.querySelector<HTMLInputElement>(`input[name="${field}"]`);
    if (!input) throw new Error(`no ${field} field`);
    input.value = value;
  }
  await act(async () => { form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })); });
  await settle();
}

describe('First administrator setup', () => {
  it('asks only for a first and a last name, and writes nothing until submitted', async () => {
    let ready = false;
    const { posted } = await mount('#/access', { ...SERVICE,
      '/me': () => ready ? ok(ME) : needsSetup,
      'POST /setup': (body) => { ready = true; return recorded(body); },
    });
    expect(posted).toHaveLength(0);
    expect($('label[for="setup-given-name"]')?.textContent).toBe('First name');
    expect($('label[for="setup-family-name"]')?.textContent).toBe('Last name');
    expect($('#setup-given-name')?.getAttribute('autocomplete')).toBe('given-name');
    expect($('#setup-family-name')?.getAttribute('autocomplete')).toBe('family-name');
    expect($('#setup-given-name')?.hasAttribute('required')).toBe(true);
    expect($('#setup-family-name')?.hasAttribute('required')).toBe(true);
    expect(document.querySelectorAll('form[aria-label="Finish setup"] input')).toHaveLength(2);
    expect(unreachable()).toEqual([]);
    await submit(' Tom ', ' Whiting ');
    expect(posted.filter((entry) => entry.path === '/setup')).toEqual([{ path: '/setup', body: { display_name: 'Tom Whiting', operation: expect.stringMatching(/^op-[0-9a-f]{32}$/) } }]);
    expect(text()).not.toContain('Finish setting up your account');
    expect(sessionStorage.getItem('lys.pending.first-setup')).toBeNull();
  });

  it('retries an uncertain result with the exact operation and name after remount', async () => {
    const first = await mount('#/people', { ...SERVICE, '/me': needsSetup,
      'POST /setup': refused(503, 'StorageUncertain', 'The write outcome is not known.'),
    });
    await submit('Tom', 'Bearup');
    expect(text()).toContain('Retry setup');
    expect($('#setup-name')?.getAttribute('readonly')).not.toBeNull();
    const retained = first.posted[0];
    unmountAll();
    document.body.innerHTML = '';
    let ready = false;
    const second = await mount('#/people', { ...SERVICE,
      '/me': () => ready ? ok(ME) : needsSetup,
      'POST /setup': (body) => { ready = true; return recorded(body); },
    });
    expect(second.posted).toHaveLength(0);
    await submit();
    expect(second.posted.filter((entry) => entry.path === '/setup')).toEqual([retained]);
    expect(sessionStorage.getItem('lys.pending.first-setup')).toBeNull();
  });

  it('keeps an unrelated receipt pending and never says setup succeeded', async () => {
    await mount('#/me', { ...SERVICE, '/me': needsSetup, 'POST /setup': (body) => {
      const answer = recorded(body);
      return ok({ ...answer.body as object, person: 'someone-else' });
    } });
    await submit('Tom', 'Whiting');
    expect(text()).toContain('Setup could not be confirmed');
    expect(text()).toContain('Retry setup');
    expect(sessionStorage.getItem('lys.pending.first-setup')).toContain('Tom');
  });

  it('folds nothing away on the setup page and links to no separate directory controls', async () => {
    const { posted } = await mount('#/people', { ...SERVICE, '/me': needsSetup });
    expect($('details')).toBeNull();
    expect($('a[href*="directory/manage"]')).toBeNull();
    expect(posted).toHaveLength(0);
  });

  it('does not offer administrator setup to an unbound ordinary account', async () => {
    await mount('#/me', { ...SERVICE, '/me': refused(403, 'NoPerson', 'Your sign-in has not been connected.') });
    expect($('form[aria-label="Finish setup"]')).toBeNull();
    expect(text()).not.toContain('makes you the directory administrator');
  });

  it('lets someone correct an invalid name before creating an operation', async () => {
    const { posted } = await mount('#/me', { ...SERVICE, '/me': needsSetup });
    await submit('Ada\u0007', 'Lovelace');
    expect(posted).toHaveLength(0);
    expect(sessionStorage.getItem('lys.pending.first-setup')).toBeNull();
    expect($('#setup-name')).toBeNull();
    expect($('#setup-given-name')?.hasAttribute('readonly')).toBe(false);
    expect(text()).toContain('without control characters');
  });

  it('sends a long name whole: no length is refused here', async () => {
    const long = 'A'.repeat(10_000);
    const { posted } = await mount('#/me', { ...SERVICE, '/me': needsSetup });
    await submit(long, 'B');
    expect(posted).toHaveLength(1);
    expect(posted[0]).toMatchObject({ path: '/setup', body: { display_name: long + ' B' } });
  });

  it('asks for both names before creating an operation', async () => {
    for (const [given, family] of [['Tom', ''], ['', 'Whiting'], ['  ', '  ']]) {
      const { posted } = await mount('#/me', { ...SERVICE, '/me': needsSetup });
      await submit(given, family);
      expect(posted).toHaveLength(0);
      expect(sessionStorage.getItem('lys.pending.first-setup')).toBeNull();
      expect($('#setup-name-error')?.textContent).toBe('Enter your first name and your last name.');
      expect($('#setup-given-name')?.getAttribute('aria-invalid')).toBe('true');
      unmountAll();
      document.body.innerHTML = '';
    }
  });

  it('refuses a broken saved operation without silently replacing it', async () => {
    sessionStorage.setItem('lys.pending.first-setup', '{broken');
    const { posted } = await mount('#/me', { ...SERVICE, '/me': needsSetup });
    await submit('Tom', 'Whiting');
    expect(posted).toHaveLength(0);
    expect($('form button')?.hasAttribute('disabled')).toBe(true);
    expect(sessionStorage.getItem('lys.pending.first-setup')).toBe('{broken');
  });
});
