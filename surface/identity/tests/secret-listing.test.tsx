/** Broker metadata is rendered explicitly; extra credential-shaped fields never become page text. */
import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';
import { SecretRows } from '../src/features/secrets/Secrets';

describe('Secrets listing renderer', () => {
  it('renders recorded metadata without extra values, handles, digests or upstream credentials', () => {
    const listing = { secrets: [{ name: 'calendar', class: 'oauth' as const, owner: 'person-owner', sequence: 12,
      upstream: 'https://user:upstream-password@example.test', header: 'authorization', value: 'private-value', handle: 'bearer-handle', digest: 'private-digest' }] };
    const html = renderToStaticMarkup(<SecretRows listing={listing} />);
    expect(html).toContain('calendar');
    expect(html).toContain('person-owner');
    expect(html).toContain('12');
    for (const sensitive of ['private-value', 'bearer-handle', 'private-digest', 'upstream-password']) expect(html).not.toContain(sensitive);
  });
  it('names an empty discovery scope without claiming the whole store is empty', () => {
    expect(renderToStaticMarkup(<SecretRows listing={{ secrets: [] }} />)).toContain('No secrets were returned for this account.');
  });
});
