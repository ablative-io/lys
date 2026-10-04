/** Broker metadata is rendered explicitly; extra credential-shaped fields never become page text. */
import { renderToStaticMarkup } from 'react-dom/server';
import { MemoryRouter } from 'react-router';
import { describe, expect, it } from 'vitest';
import { SecretRows } from '../src/features/secrets/Secrets';

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

  it('explains the list in one line above the table', () => {
    const html = renderToStaticMarkup(<MemoryRouter><SecretRows listing={{ secrets: [] }} /></MemoryRouter>);
    expect(html.match(/<p[ >]/g)).toHaveLength(1);
    expect(html).toContain('never shown');
    expect(html).not.toContain('Find the secrets');
  });
});
