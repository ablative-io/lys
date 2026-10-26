/** People and agents has four views; service accounts are a page of Settings, and an old link to them lands there. */
import { afterEach, describe, expect, it } from 'vitest';
import { $, $$, click, mount, settle, text, unmountAll } from '../../../tests/harness';
import { ADA, BEA, BEA_SERVICE, ME, SERVICE, ok, refused } from '../../../tests/fixtures';

afterEach(() => unmountAll());

describe('The views of People and agents', () => {
  it('name the organisation the caller belongs to in the header, as text', async () => {
    await mount('#/people');
    await settle();
    const organisation = ME.organisation?.display_name ?? '';
    expect(organisation).not.toBe('');
    expect($('p.sub')?.textContent).toContain(organisation);
  });

  it('are All, People, Agents and Teams; service accounts are not among them', async () => {
    await mount('#/people');
    expect($$('nav[aria-label="People and agents views"] a').map((link) => link.textContent)).toEqual(['All', 'People', 'Agents', 'Teams']);
  });

  it('send the old service accounts address to Settings, keeping what it asked', async () => {
    await mount('#/people/view/accounts?account=acct-1');
    await settle();
    expect(location.hash).toBe('#/settings/service-accounts?account=acct-1');
    expect($('nav[aria-label="People and agents views"]')).toBeNull();
  });
});

const admissionPath = '/people/' + BEA + '/admission';
const accountRoutes = {
  ...SERVICE,
  ['/identities/' + BEA]: ok({ id: BEA, logins: [] }),
  ['/directory/people/' + BEA + '/account']: ok({ email: 'bea@example.test', enabled: true }),
  [admissionPath]: ok({ person: BEA, waiting: true }),
};

describe('An admission waiting on the Account page (DIRECTORY-121)', () => {
  it('shows an administrator the waiting link and withdraws it through the service', async () => {
    const { posted } = await mount('#/file/' + BEA + '/credentials', { ...accountRoutes,
      ['POST ' + admissionPath + '/withdraw']: ok({ person: BEA, digest: 'd1' }),
    });
    expect(text()).toContain('A sign-in link is waiting');
    await click($('button[aria-label="Withdraw sign-in link"]'));
    expect(posted).toContainEqual({ path: admissionPath + '/withdraw', body: {} });
    expect(text()).toContain('The sign-in link was withdrawn');
    expect(text()).not.toContain('A sign-in link is waiting');
    expect($('button[aria-label="Withdraw sign-in link"]')).toBeNull();
  });

  it('keeps the waiting link visible when withdrawal is refused', async () => {
    await mount('#/file/' + BEA + '/credentials', { ...accountRoutes,
      ['POST ' + admissionPath + '/withdraw']: refused(503, 'DirectoryUnavailable', 'The write was refused'),
    });
    await click($('button[aria-label="Withdraw sign-in link"]'));
    expect(text()).toContain('A sign-in link is waiting');
    expect($('[role="alert"]')?.textContent).toContain('The write was refused');
  });

  it('shows no waiting link or Withdraw to a non-administrator', async () => {
    const { requests } = await mount('#/file/' + BEA + '/credentials', { ...accountRoutes, ...BEA_SERVICE,
      '/directory/people': refused(403, 'NotAdmitted', 'Only administrators'),
    });
    expect(text()).not.toContain('A sign-in link is waiting');
    expect($('button[aria-label="Withdraw sign-in link"]')).toBeNull();
    expect(requests).not.toContain(admissionPath);
  });

  it('shows no waiting link once the service says there is none', async () => {
    await mount('#/file/' + BEA + '/credentials', { ...accountRoutes, [admissionPath]: ok({ person: BEA, waiting: false }) });
    expect(text()).not.toContain('A sign-in link is waiting');
    expect($('button[aria-label="Withdraw sign-in link"]')).toBeNull();
  });

  it('lets an administrator withdraw a link on their own record too', async () => {
    await mount('#/file/' + ADA + '/credentials', { ...SERVICE,
      ['/people/' + ADA + '/admission']: ok({ person: ADA, waiting: true }),
    });
    expect(text()).toContain('A sign-in link is waiting');
    expect($('button[aria-label="Withdraw sign-in link"]')).not.toBeNull();
  });
});
