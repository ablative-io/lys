import { describe, expect, it } from 'vitest';
import { $, $$, mount, press, text, unreachable } from './harness';
import { ISSUER, SCRIBE, SERVICE, refused } from './fixtures';

describe('You', () => {
  it('shows the signed-in person from /me', async () => {
    const { requests } = await mount('#/me');
    expect(requests).toEqual(expect.arrayContaining(['/me', '/people']));
    expect($('.eyebrow')?.textContent).toBe('Signed in as');
    expect($('.page h1')?.textContent).toBe('Ada (test person)');
  });

  it('keeps sign-in identities and service accounts as separate lists (conformance 1.1, 1.3)', async () => {
    await mount('#/me');
    const signIn = $('#signin-identities');
    const service = $('#service-accounts');
    expect(signIn?.querySelector('h2')?.textContent).toBe('Sign-in identities');
    expect(signIn?.textContent).toContain(new URL(ISSUER).host);
    expect(signIn?.textContent).toContain('this session');
    expect(signIn?.textContent).toContain('Never lent to an agent.');
    expect(service?.querySelector('h2')?.textContent).toBe('Your service-account records');
    expect(service?.textContent).toContain('No service-account records were returned for you.');
    expect(signIn?.contains(service ?? null)).toBe(false);
  });

  it('lists your agents that still stand, with what each holds', async () => {
    await mount('#/me');
    const agents = $$('tr[data-href]').map((tr) => tr.querySelector('td')?.textContent);
    expect(agents).toEqual(["Scribe", "Courier", "Archivist"]);
    expect($$('tr[data-href]')[0].textContent).toContain('viewer of project:identity');
    expect($$('tr[data-href]')[1].textContent).toContain('no access');
    expect(text()).not.toContain('finance-readonly');
  });

  it('opens an agent from the keyboard, and every control is reachable (9.3)', async () => {
    await mount('#/me');
    expect(unreachable()).toEqual([]);
    $(`tr[data-href="#/file/${SCRIBE}"]`)?.focus();
    await press('Enter');
    expect(location.hash).toBe('#/file/' + SCRIBE);
  });

  it('names the refusal when the login is bound to no person', async () => {
    await mount('#/me', { ...SERVICE, '/me': refused(403, 'NoPerson', 'NoPerson: the signed-in login is bound to no person') });
    expect($('.why-not b')?.textContent).toBe('NoPerson');
  });
});
