/** The secrets detail screens render chosen metadata only, and name every refusal. */
import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';
import { Refused } from '../src/api';
import {
  AuditRows,
  ChangeView,
  GrantRows,
  RecipientsChange,
  RevocationLookup,
  RevocationView,
  ScopeChange,
  scopeWords,
} from '../src/features/secrets/SecretsDetail';
import type { RevocationAnswer, SecretAuditLog, SecretGrantListing } from '../src/features/secrets/secretsApi';

const SENSITIVE = ['private-value', 'bearer-handle-token', 'upstream-password', 'x-api-key', 'private-key-bytes', 'audit-public-key'];

const extras = {
  value: 'private-value',
  bearer: 'bearer-handle-token',
  upstream: 'https://user:upstream-password@example.test',
  header: 'x-api-key',
  key: 'private-key-bytes',
};

/** The same extras, with the upstream address under a name the revocation answer does not use for its own state. */
const { upstream: upstreamAddress, ...rest } = extras;
const revocationExtras = { ...rest, upstream_url: upstreamAddress };

describe('Secret grants', () => {
  it('renders each grant with its secret, who, what and granted-by', () => {
    const listing: SecretGrantListing = { grants: [
      { identity: 'agent-reader', secret: 'calendar', relation: 'use', granted_by: 'person-owner' },
      { identity: 'person-other', secret: 'mailbox', relation: 'lend', granted_by: null },
    ] };
    const html = renderToStaticMarkup(<GrantRows listing={listing} />);
    for (const text of ['calendar', 'agent-reader', 'person-owner', 'May use it', 'mailbox', 'person-other', 'May lend it on', 'Not recorded']) expect(html).toContain(text);
  });
  it('never renders extra credential-shaped fields', () => {
    const listing = { grants: [{ identity: 'agent-reader', secret: 'calendar', relation: 'use', granted_by: null, ...extras }] };
    const html = renderToStaticMarkup(<GrantRows listing={listing} />);
    for (const sensitive of SENSITIVE) expect(html).not.toContain(sensitive);
  });
  it('names an empty answer', () => {
    expect(renderToStaticMarkup(<GrantRows listing={{ grants: [] }} />)).toContain('No grants were returned');
  });
});

describe('Secret audit', () => {
  const log: SecretAuditLog = { verified_by: 'audit-public-key', lines: [
    { index: 3, kind: 'issue', at_ms: 1_790_000_000_000, handle: 'a1b2c3', identity: 'agent-reader', secret: 'calendar', operation: null, uses: null, outcome: 'issued' },
    { index: 7, kind: 'use', at_ms: 1_790_000_060_000, handle: 'a1b2c3', identity: 'agent-reader', secret: 'calendar', operation: 'feedface', uses: 1, outcome: 'LeaseExhausted' },
  ] };
  it('renders each line with its fields, newest (highest index) first, and not the verifying key', () => {
    const html = renderToStaticMarkup(<AuditRows log={log} />);
    for (const text of ['Handle issued', 'Used', 'calendar', 'agent-reader', 'a1b2c3', 'issued', 'LeaseExhausted']) expect(html).toContain(text);
    expect(html.indexOf('LeaseExhausted')).toBeLessThan(html.indexOf('>issued<'));
    expect(html).not.toContain('audit-public-key');
  });
  it('never renders extra credential-shaped fields', () => {
    const withExtras = { verified_by: 'audit-public-key', lines: log.lines.map((line) => ({ ...line, ...extras })) };
    const html = renderToStaticMarkup(<AuditRows log={withExtras} />);
    for (const sensitive of SENSITIVE) expect(html).not.toContain(sensitive);
  });
});

describe('Revocation lookup', () => {
  const answer = (upstream: RevocationAnswer['upstream'], upstream_reason: string | null, stopped_here = true): RevocationAnswer =>
    ({ handle: 'a1b2c3', stopped_here, upstream, upstream_reason });

  it('renders the handle id field and the Check button', () => {
    const html = renderToStaticMarkup(<RevocationLookup check={() => Promise.resolve(answer('not_asked', null))} />);
    expect(html).toContain('Handle id');
    expect(html).toContain('Check');
  });
  it('shows a provider that was not asked', () => {
    const html = renderToStaticMarkup(<RevocationView outcome={{ at: 'answered', answer: answer('not_asked', null, false) }} />);
    expect(html).toContain('Use stopped here: no');
    expect(html).toContain('not asked');
  });
  it('shows a provider asked and not yet confirmed, with its reason', () => {
    const html = renderToStaticMarkup(<RevocationView outcome={{ at: 'answered', answer: answer('unconfirmed', 'provider timed out') }} />);
    expect(html).toContain('Use stopped here: yes');
    expect(html).toContain('asked, not yet confirmed: provider timed out');
  });
  it('shows a provider that confirmed', () => {
    const html = renderToStaticMarkup(<RevocationView outcome={{ at: 'answered', answer: answer('confirmed', null) }} />);
    expect(html).toContain('confirmed by the provider');
  });
  it('says an unknown handle is not visible, as one never issued', () => {
    const refused = new Refused(404, { refusal: 'HandleUnknown', reason: 'HandleUnknown: no such handle' });
    const html = renderToStaticMarkup(<RevocationView outcome={{ at: 'refused', refused }} />);
    expect(html).toContain('No handle with that id is visible to you.');
    expect(html).not.toContain('no such handle');
  });
  it('never renders extra credential-shaped fields of an answer', () => {
    const html = renderToStaticMarkup(<RevocationView outcome={{ at: 'answered', answer: { ...answer('confirmed', null), ...revocationExtras } }} />);
    for (const sensitive of SENSITIVE) expect(html).not.toContain(sensitive);
  });
});

describe('Owner changes', () => {
  it('render their forms with the owner-only note', () => {
    const scope = renderToStaticMarkup(<ScopeChange secret="calendar" change={() => Promise.reject(new Error('not called'))} />);
    expect(scope).toContain('Only the secret&#x27;s owner can change this.');
    expect(scope).toContain('calendar');
    const recipients = renderToStaticMarkup(<RecipientsChange secret="calendar" change={() => Promise.reject(new Error('not called'))} />);
    expect(recipients).toContain('People only');
    expect(recipients).toContain('Anyone who is permitted');
  });
  it('show a refusal by its name and reason', () => {
    const refused = new Refused(403, { refusal: 'LendingNotPermitted', reason: 'LendingNotPermitted: person-other does not own calendar' });
    const html = renderToStaticMarkup(<ChangeView outcome={{ at: 'refused', refused }} done={() => 'done'} />);
    expect(html).toContain('LendingNotPermitted');
    expect(html).toContain('person-other does not own calendar');
  });
  it('say a change of unknown outcome is not repeated', () => {
    const refused = new Refused(502, { refusal: 'SecretsUnavailable', reason: 'the secrets broker could not be reached' });
    const html = renderToStaticMarkup(<ChangeView outcome={{ at: 'unknown', refused }} done={() => 'done'} />);
    expect(html).toContain('SecretsUnavailable');
    expect(html).toContain('It is not known whether this change was made.');
  });
  it('word a new scope plainly', () => {
    expect(scopeWords('team/billing')).toBe('the team billing');
    expect(scopeWords('person/person-owner')).toBe('the person person-owner');
    expect(scopeWords('organisation/acme')).toBe('the organisation acme');
  });
});
