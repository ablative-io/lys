/** Person security tabs use existing login/session authority without reading credentials or acting on agent runtimes. */
import { describe, expect, it } from 'vitest';
import { $, click, mount, text } from './harness';
import { ADA, BEA, ME, SERVICE, ok, refused } from './fixtures';

const session = { id: 'session-this-browser', current: true, login: { issuer: 'https://issuer.test', subject: 'test-account' }, started_at: 1790000000, ends_at: 1990000000 };

describe('Person security tabs', () => {
  it('reads own bound accounts without requesting the administrator record', async () => {
    const { requests, posted } = await mount('#/file/' + ADA + '/credentials');
    expect(text()).toContain('Sign-in accounts');
    expect(text()).toContain(ME.sign_in_identities[0].subject);
    expect(text()).toContain(ME.sign_in_identities[0].provider);
    expect(requests).not.toContain('/identities/' + ADA);
    expect(posted).toEqual([]);
    expect($('a[href="#/directory/manage?action=login&identity=' + ADA + '"]')).not.toBeNull();
  });
  it('reads another person’s bindings from the protected identity route', async () => {
    const { requests, posted } = await mount('#/file/' + BEA + '/credentials', { ...SERVICE,
      ['/identities/' + BEA]: ok({ id: BEA, logins: [{ issuer: 'https://other-issuer.test', subject: 'another-test-account' }] }),
    });
    expect(requests).toContain('/identities/' + BEA);
    expect(text()).toContain('another-test-account');
    expect(posted).toEqual([]);
  });
  it('shows a named refusal instead of inventing an empty account list', async () => {
    await mount('#/file/' + BEA + '/credentials', { ...SERVICE, ['/identities/' + BEA]: refused(403, 'NotAdmitted', 'Only an administrator can read this account') });
    expect(text()).toContain('NotAdmitted');
    expect(text()).not.toContain('No sign-in account is bound');
  });
  it('shows and confirms ending the signed-in person’s own browser session', async () => {
    const { requests, posted } = await mount('#/file/' + ADA + '/sessions', { ...SERVICE,
      '/sessions': ok({ person: ADA, sessions: [session] }),
      'POST /sessions/session-this-browser/end': ok({ ended: session.id }),
    });
    expect(requests).toContain('/sessions');
    expect(requests).not.toContain('/directory/people/' + ADA + '/sessions');
    expect(text()).toContain('does not stop agent processes');
    await click([...document.querySelectorAll('button')].find((entry) => entry.textContent === 'Sign out') ?? null);
    expect(posted).toEqual([]);
    await click([...document.querySelectorAll('button')].find((entry) => entry.textContent === 'Confirm end session') ?? null);
    expect(posted).toEqual([{ path: '/sessions/session-this-browser/end', body: {} }]);
  });
  it('uses the protected person route for somebody else’s browser sessions', async () => {
    const { requests, posted } = await mount('#/file/' + BEA + '/sessions', { ...SERVICE, ['/directory/people/' + BEA + '/sessions']: ok({ person: BEA, sessions: [] }) });
    expect(requests).toContain('/directory/people/' + BEA + '/sessions');
    expect(requests).not.toContain('/sessions');
    expect(text()).toContain('No live sessions');
    expect(posted).toEqual([]);
  });
});
