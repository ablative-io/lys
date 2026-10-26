/** People and agents has four views; service accounts are a page of Settings, and an old link to them lands there. */
import { act } from 'react';
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

describe('Sign-in mix-ups (DIRECTORY-122)', () => {
  it('shows both addresses and the Account link only for the differing person', async () => {
    await mount('#/people', { ...SERVICE, '/directory/people': ok({ scope: 'directory', people: [
      { id: ADA, display_name: 'Dean Whiting', state: 'active', agents: [], sign_in_email: 'annabel@example.test', admitted_email: 'dean@example.test' },
      { id: BEA, display_name: 'Bea', state: 'active', agents: [], sign_in_email: 'bea@example.test', admitted_email: 'bea@example.test' },
    ] }) });
    expect(text()).toContain('Signs in as annabel@example.test, added as dean@example.test');
    expect(text()).not.toContain('Signs in as bea@example.test, added as');
    expect($('a[href="#/file/' + ADA + '/credentials"]')).not.toBeNull();
  });
  it('posts the existing or new target beside the chosen sign-in', async () => {
    for (const existing of [true, false]) {
      const path = '/people/' + BEA + '/logins/move';
      const { posted } = await mount('#/file/' + BEA + '/credentials', { ...accountRoutes,
        ['/identities/' + BEA]: ok({ id: BEA, logins: [{ issuer: 'https://issuer.test', subject: 'mixed' }] }),
        ['POST ' + path]: ok({ target: ADA, source_grants: ['grant-dean'], target_grants: [] }),
      });
      await click($('button[aria-label="This sign-in belongs to someone else"]'));
      const form = $('form[aria-label="Move sign-in"]');
      await act(async () => {
        const field = form?.querySelector<HTMLInputElement | HTMLSelectElement>(existing ? '[name="person"]' : '[name="display_name"]');
        if (!field) throw new Error('the target choice is shown');
        field.value = existing ? ADA : 'Annabel';
        form?.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
      });
      await settle();
      expect(posted).toContainEqual({ path, body: expect.objectContaining({ issuer: 'https://issuer.test', subject: 'mixed', target: existing ? { person: ADA } : { display_name: 'Annabel' } }) });
      expect(text()).toContain('grant-dean');
      await unmountAll();
    }
  });
  it('offers an existing target with a sign-in and explains that their sign-ins stay', async () => {
    const path = '/people/' + BEA + '/logins/move';
    const { posted } = await mount('#/file/' + BEA + '/credentials', { ...accountRoutes,
      '/directory/people': ok({ scope: 'directory', people: [
        { id: ADA, display_name: 'Annabel', state: 'active', agents: [], sign_in_email: 'annabel@example.test' },
        { id: BEA, display_name: 'Dean', state: 'active', agents: [] },
      ] }),
      ['/identities/' + BEA]: ok({ id: BEA, logins: [{ issuer: 'https://issuer.test', subject: 'mixed' }] }),
      ['POST ' + path]: ok({ target: ADA, source_grants: ['grant-dean'], target_grants: ['grant-annabel'] }),
    });
    await click($('button[aria-label="This sign-in belongs to someone else"]'));
    expect(text()).toContain('The target keeps their existing sign-ins.');
    const select = $('select[name="person"]') as HTMLSelectElement;
    expect(Array.from(select.options).some((option) => option.value === ADA)).toBe(true);
    expect(Array.from(select.options).some((option) => option.value === BEA)).toBe(false);
    await act(async () => {
      select.value = ADA;
      $('form[aria-label="Move sign-in"]')?.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
    });
    await settle();
    expect(posted).toContainEqual({ path, body: expect.objectContaining({ issuer: 'https://issuer.test', subject: 'mixed', target: { person: ADA } }) });
    expect(text()).toContain('grant-dean');
    expect(text()).toContain('grant-annabel');
  });
  it('shows a SameTarget refusal without reporting a completed move', async () => {
    const path = '/people/' + BEA + '/logins/move';
    await mount('#/file/' + BEA + '/credentials', { ...accountRoutes,
      ['/identities/' + BEA]: ok({ id: BEA, logins: [{ issuer: 'https://issuer.test', subject: 'mixed' }] }),
      ['POST ' + path]: refused(409, 'SameTarget', 'SameTarget: This sign-in is already bound to that person.'),
    });
    await click($('button[aria-label="This sign-in belongs to someone else"]'));
    await act(async () => {
      const select = $('select[name="person"]') as HTMLSelectElement;
      select.value = ADA;
      $('form[aria-label="Move sign-in"]')?.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
    });
    await settle();
    expect($('[role="alert"]')?.textContent).toContain('This sign-in is already bound to that person.');
    expect(text()).not.toContain('Sign-in moved.');
    expect($('form[aria-label="Move sign-in"]')).not.toBeNull();
  });
  it('offers no move to a non-administrator', async () => {
    await mount('#/file/' + BEA + '/credentials', { ...accountRoutes, ...BEA_SERVICE,
      '/directory/people': refused(403, 'NotAdmitted', 'Only administrators'),
    });
    expect($('button[aria-label="This sign-in belongs to someone else"]')).toBeNull();
  });
});


it('labels an address first recorded at provider sign-in', async () => {
  await mount('#/people', { ...SERVICE,
    '/directory/people': ok({ scope: 'directory', people: [{ id: BEA, display_name: 'Bea', state: 'active', agents: [], sign_in_email: 'bea@example.test', admitted_email: 'bea@example.test', email_recorded_at_sign_in: true }] }),
  });
  expect(text()).toContain('recorded at sign-in');
});
