/** Broker metadata is rendered explicitly; extra credential-shaped fields never become page text. */
import { renderToStaticMarkup } from 'react-dom/server';
import { MemoryRouter } from 'react-router';
import { describe, expect, it } from 'vitest';
import { SecretRows } from '../src/features/secrets/Secrets';
import { $, mount, text } from './harness';
import { SERVICE } from './fixtures';

describe('Secrets listing renderer', () => {
  it('renders recorded metadata without extra values, handles, digests or upstream credentials', () => {
    const listing = { secrets: [{ name: 'calendar', class: 'oauth' as const, owner: 'person-owner', sequence: 12,
      upstream: 'https://user:upstream-password@example.test', header: 'authorization', value: 'private-value', handle: 'bearer-handle', digest: 'private-digest' }] };
    const html = renderToStaticMarkup(<MemoryRouter><SecretRows listing={listing} /></MemoryRouter>);
    expect(html).toContain('calendar');
    expect(html).toContain('person-owner');
    expect(html).toContain('12');
    for (const sensitive of ['private-value', 'bearer-handle', 'private-digest', 'upstream-password']) expect(html).not.toContain(sensitive);
  });
  it('names an empty discovery scope without claiming the whole store is empty, once, in the table body', () => {
    const html = renderToStaticMarkup(<MemoryRouter><SecretRows listing={{ secrets: [] }} /></MemoryRouter>);
    expect(html).toMatch(/<tr class="empty"><td[^>]*>No secret is visible to this account\.<\/td><\/tr>/);
    expect(html.split('No secret is visible to this account.')).toHaveLength(2);
    expect(html).not.toMatch(/Nothing here yet|No secrets were returned|0 secrets/);
  });

  it('explains the list once in the page subtitle above the table', async () => {
    await mount('#/secrets', SERVICE);
    expect($('.page-header .sub')?.textContent).toBe("A secret's value is never shown, and seeing it here does not give permission to use it.");
    expect(text().split('never shown')).toHaveLength(2);
    expect(text()).not.toContain('Find the secrets');
  });
});
