/** People and agents has four views; service accounts are a page of Settings, and an old link to them lands there. */
import { afterEach, describe, expect, it } from 'vitest';
import { $, $$, mount, settle, unmountAll } from '../../../tests/harness';
import { ME } from '../../../tests/fixtures';

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
