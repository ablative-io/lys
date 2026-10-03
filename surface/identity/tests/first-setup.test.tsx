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
async function submit(name?: string) {
  const form = $('form[aria-label="Finish setup"]');
  const input = form?.querySelector<HTMLInputElement>('input[name="display_name"]');
  if (!form || !input) throw new Error('Setup form missing');
  if (name !== undefined) input.value = name;
  await act(async () => { form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })); });
  await settle();
}

describe('First administrator setup', () => {
  it('asks only for a full name, explains single names, and writes nothing until submitted', async () => {
    let ready = false;
    const { posted } = await mount('#/access', { ...SERVICE,
      '/me': () => ready ? ok(ME) : needsSetup,
      'POST /setup': (body) => { ready = true; return recorded(body); },
    });
    expect(posted).toHaveLength(0);
    expect(text()).toContain('Full name');
    expect(text()).toContain('first name and surname');
    expect(text()).toContain('single name');
    expect($('label[for="setup-name"]')).not.toBeNull();
    expect($('#setup-name')?.getAttribute('aria-describedby')).toBe('setup-name-help');
    expect(document.querySelectorAll('form[aria-label="Finish setup"] input')).toHaveLength(1);
    expect(unreachable()).toEqual([]);
    await submit('Tom');
    expect(posted.filter((entry) => entry.path === '/setup')).toEqual([{ path: '/setup', body: { display_name: 'Tom', operation: expect.stringMatching(/^op-[0-9a-f]{32}$/) } }]);
    expect(text()).not.toContain('Finish setting up your account');
    expect(sessionStorage.getItem('lys.pending.first-setup')).toBeNull();
  });

  it('retries an uncertain result with the exact operation and name after remount', async () => {
    const first = await mount('#/people', { ...SERVICE, '/me': needsSetup,
      'POST /setup': refused(503, 'StorageUncertain', 'The write outcome is not known.'),
    });
    await submit('Tom Bearup');
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
    await submit('Tom');
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
    await submit('Ada\u0007');
    expect(posted).toHaveLength(0);
    expect(sessionStorage.getItem('lys.pending.first-setup')).toBeNull();
    expect($('#setup-name')?.hasAttribute('readonly')).toBe(false);
    expect(text()).toContain('without control characters');
  });

  it('sends a long name whole: no length is refused here', async () => {
    const long = 'A'.repeat(10_000);
    const { posted } = await mount('#/me', { ...SERVICE, '/me': needsSetup });
    await submit(long);
    expect(posted).toHaveLength(1);
    expect(posted[0]).toMatchObject({ path: '/setup', body: { display_name: long } });
  });

  it('refuses a broken saved operation without silently replacing it', async () => {
    sessionStorage.setItem('lys.pending.first-setup', '{broken');
    const { posted } = await mount('#/me', { ...SERVICE, '/me': needsSetup });
    await submit('Tom');
    expect(posted).toHaveLength(0);
    expect($('form button')?.hasAttribute('disabled')).toBe(true);
    expect(sessionStorage.getItem('lys.pending.first-setup')).toBe('{broken');
  });
});
