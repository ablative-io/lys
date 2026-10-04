/** Certificate tabs preserve issuance scope, withdrawal evidence and named unreadable outcomes. */
import { describe, expect, it } from 'vitest';
import { $, mount, text } from './harness';
import { ADA, SCRIBE, SERVICE, ok, refused } from './fixtures';
import type { AgentCertificate } from '../src/features/file/AgentCertificates';
const path = '/agents/' + SCRIBE + '/certificates';
const certificate: AgentCertificate = { serial: 'serial-one', person: ADA, claims: { role: 'reader', version: 1 }, der: 'AQID', issued_at: 1790000000, withdrawn: null, entry: { leaf: 0, leaf_bytes: '010203', tree_size: 1, root: 'aaaa', proof: 'bbbb' } };
describe('Agent certificates', () => {
  it('shows issuance claims and exports the exact evidence without claiming verification', async () => {
    const { posted } = await mount('#/file/' + SCRIBE + '/certificate', { ...SERVICE, [path]: ok({ agent: SCRIBE, certificates: [certificate], claims_are_live: false }) });
    expect(text()).toContain('Claims at issuance'); expect(text()).toContain('not independently verified'); expect(posted).toEqual([]);
    const href = $('a[download="certificate-evidence.json"]')?.getAttribute('href');
    if (!href) throw new Error('No evidence download');
    expect(JSON.parse(decodeURIComponent(href.slice(href.indexOf(',') + 1)))).toEqual({ agent: SCRIBE, claims_are_live: false, certificate });
    expect($('a[download="certificate.der"]')?.getAttribute('href')).toBe('data:application/pkix-cert;base64,AQID');
  });
  it('titles a certificate by when and for whom, keeps the serial small, and says its claims as facts, never as raw JSON or raw ids', async () => {
    const claims = { agent: SCRIBE, person: ADA, roles: [{ role: 'op-' + 'c'.repeat(32), version: 1 }], profile_version: 3, grants: [{}, {}], held_at: 1790000000 };
    await mount('#/file/' + SCRIBE + '/credentials', { ...SERVICE, [path]: ok({ agent: SCRIBE, certificates: [{ ...certificate, serial: 'op-' + 'd'.repeat(32), claims }], claims_are_live: false }) });
    const heading = [...document.querySelectorAll('.certificates h3')][0];
    expect(heading?.textContent).toMatch(/^Issued .+ for Ada \(test person\)/);
    expect(heading?.querySelector('.refusal-name')?.textContent).toBe('op-' + 'd'.repeat(32));
    const facts = [...document.querySelectorAll('.certificates .certificate dl')][1]?.textContent ?? '';
    expect(facts).toContain('Holds 2 grants'); expect(facts).toContain('Holds 1 role'); expect(facts).toContain('Version 3'); expect(facts).toContain('Held at');
    expect(facts).not.toContain(SCRIBE); expect(facts).not.toContain(ADA); expect(facts).not.toContain('op-');
    expect(document.querySelector('.certificates pre')).toBeNull();
    // Both credential parts are cards with the same heading.
    expect([...document.querySelectorAll('section.card > h2')].map((h) => h.textContent)).toEqual(expect.arrayContaining(['Credential handles', 'Capability certificates']));
  });
  it('shows a logged withdrawal and its reason', async () => {
    await mount('#/file/' + SCRIBE + '/certificate', { ...SERVICE, [path]: ok({ agent: SCRIBE, claims_are_live: false, certificates: [{ ...certificate, withdrawn: { serial: 'serial-one', by: ADA, reason: 'Key replaced', withdrawn_at: 1790000001 } }] }) });
    expect(text()).toContain('Withdrawn at'); expect(text()).toContain('Key replaced'); expect(text()).not.toContain('No withdrawal recorded');
  });
  it('shows not issued only when the log answered an empty list', async () => {
    await mount('#/file/' + SCRIBE + '/certificate', { ...SERVICE, [path]: ok({ agent: SCRIBE, claims_are_live: false, certificates: [] }) });
    expect(text()).toContain('No certificate has been entered');
  });
  it('preserves a named log refusal', async () => {
    await mount('#/file/' + SCRIBE + '/certificate', { ...SERVICE, [path]: refused(503, 'CertificatesUnavailable', 'Log unreadable') });
    expect(text()).toContain('CertificatesUnavailable'); expect(text()).not.toContain('No certificate has been entered');
  });
  it('rejects a response presenting issuance claims as live grants', async () => {
    await mount('#/file/' + SCRIBE + '/certificate', { ...SERVICE, [path]: ok({ agent: SCRIBE, claims_are_live: true, certificates: [certificate] }) });
    expect(text()).toContain('issuance-only claims'); expect(text()).not.toContain('serial-one');
  });
});
