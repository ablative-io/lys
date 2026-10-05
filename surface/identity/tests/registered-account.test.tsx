/** Registered people can read their own account and sign out without requesting acting authority. */
import { describe, expect, it } from 'vitest';
import { $, $$, click, mount, text } from './harness';
import { ADA, ISSUER, ME, SERVICE, ok, refused } from './fixtures';

const SESSION = 'registered-own-session';
const waiting = {
  ...SERVICE,
  '/me': ok({ ...ME, person: { ...ME.person, state: 'registered' } }),
  '/me/account': ok({ email: 'registered@example.test', enabled: true }),
  '/sessions': ok({ person: ADA, sessions: [{ id: SESSION, login: { issuer: ISSUER, subject: 'ada' }, current: true, started_at: 100, ends_at: 700 }] }),
  '/people': refused(403, 'inactive', 'registered and may not act'),
  '/directory/people': refused(403, 'inactive', 'registered and may not act'),
  '/grants': refused(403, 'inactive', 'registered and may not act'),
  '/grants/model': refused(403, 'inactive', 'registered and may not act'),
  ['POST /sessions/' + SESSION + '/end']: ok({ ended: SESSION }),
};

describe('Registered own account', () => {
  it('reads its own identity first and shows account and sessions without asking for grants or people', async () => {
    const { requests, posted } = await mount('#/me', waiting);
    expect(requests[0]).toBe('/me');
    expect(requests).toEqual(expect.arrayContaining(['/me', '/me/account', '/sessions']));
    for (const path of ['/people', '/directory/people', '/grants', '/grants/model']) expect(requests).not.toContain(path);
    expect($('.page h1')?.textContent).toBe(ME.person.display_name);
    expect(text()).toContain('waiting for activation by an administrator');
    expect(text()).toContain('registered@example.test');
    expect(text()).toContain('This session');
    expect($('form[aria-label="Change your email"]')).toBeNull();
    expect($('form[aria-label="Change your password"]')).toBeNull();
    expect(posted).toEqual([]);
  });

  it('can confirm ending its own session without requesting the directory or granting itself access', async () => {
    const { requests, posted } = await mount('#/me', waiting);
    await click($$('button').find((button) => (button.getAttribute('aria-label') ?? button.textContent) === 'Sign out') ?? null);
    await click($$('button').find((button) => (button.getAttribute('aria-label') ?? button.textContent) === 'Confirm end session') ?? null);
    expect(posted).toEqual([{ path: '/sessions/' + SESSION + '/end', body: {} }]);
    for (const path of ['/people', '/directory/people', '/grants', '/grants/model']) expect(requests).not.toContain(path);
  });
});
