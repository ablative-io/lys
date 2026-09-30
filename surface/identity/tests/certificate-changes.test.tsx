/** Certificate issue/withdrawal retain exact public requests and reasons across unknown outcomes. */
import { act } from 'react';
import { beforeEach, describe, expect, it } from 'vitest';
import { $, click, mount, settle, text, unmountAll } from './harness';
import { ADA, SCRIBE, SERVICE, ok, refused } from './fixtures';
const path = '/agents/' + SCRIBE + '/certificates';
const certificate = { serial: 'op-' + 'a'.repeat(32), person: ADA, claims: {}, der: 'AQID', issued_at: 1790000000, withdrawn: null, entry: { leaf: 0, leaf_bytes: '010203', tree_size: 1, root: 'aaaa', proof: 'bbbb' } };
const view = { agent: SCRIBE, recorded: null, certificates: [certificate], claims_are_live: false };
const routes = { ...SERVICE, [path]: ok(view) };
const button = (label: string) => [...document.querySelectorAll('button')].find((entry) => entry.textContent === label) ?? null;
async function input(selector: string, value: string) {
  const element = $(selector); if (!(element instanceof HTMLInputElement) && !(element instanceof HTMLTextAreaElement)) throw new Error('Input missing');
  const prototype = element instanceof HTMLInputElement ? HTMLInputElement.prototype : HTMLTextAreaElement.prototype;
  await act(async () => { Object.getOwnPropertyDescriptor(prototype, 'value')?.set?.call(element, value); element.dispatchEvent(new Event('input', { bubbles: true })); }); await settle();
}
beforeEach(() => sessionStorage.clear());
describe('Certificate mutations', () => {
  it('issues only an agent-made request, with no caller-supplied claims or private key', async () => {
    const { posted } = await mount('#/file/' + SCRIBE + '/certificate', { ...routes, ['POST ' + path]: (body) => { const asked = body as Record<string, unknown>; return ok({ ...view, recorded: asked.operation, certificates: [{ ...certificate, serial: asked.operation }] }); } });
    await input('form[aria-label="Issue capability certificate"] textarea', 'AQID'); await click(button('Issue certificate'));
    expect(posted).toEqual([{ path, body: { operation: expect.stringMatching(/^op-/), request: 'AQID' } }]); expect(sessionStorage.length).toBe(0);
  });
  it('retains the exact public request and operation after an uncertain issue', async () => {
    const first = await mount('#/file/' + SCRIBE + '/certificate', { ...routes, ['POST ' + path]: refused(503, 'CertificatesUnavailable', 'Unknown outcome') });
    await input('form[aria-label="Issue capability certificate"] textarea', 'AQID'); await click(button('Issue certificate')); unmountAll(); document.body.innerHTML = '';
    const next = await mount('#/file/' + SCRIBE + '/certificate', { ...routes, ['POST ' + path]: (body) => { const asked = body as Record<string, unknown>; return ok({ ...view, recorded: asked.operation, certificates: [{ ...certificate, serial: asked.operation }] }); } });
    await click(button('Check whether Lys saved it')); expect(next.posted).toEqual(first.posted);
  });
  it('requires a reason and confirms withdrawal separately from grants', async () => {
    const withdrawalPath = path + '/' + certificate.serial + '/withdrawal';
    const { posted } = await mount('#/file/' + SCRIBE + '/certificate', { ...routes, ['POST ' + withdrawalPath]: (body) => ok({ ...view, recorded: certificate.serial, certificates: [{ ...certificate, withdrawn: { serial: certificate.serial, by: ADA, reason: (body as Record<string, unknown>).reason, withdrawn_at: 1790000001 } }] }) });
    await click(button('Withdraw certificate')); expect(posted).toEqual([]); expect(text()).toContain('does not separately revoke grants');
    await input('form[aria-label="Withdraw capability certificate"] input', 'Key replaced'); await click(button('Confirm withdrawal'));
    expect(posted).toEqual([{ path: withdrawalPath, body: { reason: 'Key replaced' } }]); expect(sessionStorage.length).toBe(0);
  });
  it('refuses private-key text locally without sending it', async () => {
    const { posted } = await mount('#/file/' + SCRIBE + '/certificate', routes);
    await input('form[aria-label="Issue capability certificate"] textarea', '-----BEGIN PRIVATE KEY-----'); await click(button('Issue certificate'));
    expect(posted).toEqual([]); expect(text()).toContain('Never paste a private key');
  });
});
