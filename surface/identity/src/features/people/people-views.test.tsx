/** People and agents has four views; service accounts are a page of Settings, and an old link to them lands there. */
import { afterEach, describe, expect, it } from 'vitest';
import { $, $$, mount, settle, unmountAll } from '../../../tests/harness';

afterEach(() => unmountAll());

describe('The views of People and agents', () => {
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
