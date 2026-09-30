/** Profile review receipts distinguish the current request from an earlier review and retain the original target version. */
import { beforeEach, describe, expect, it } from 'vitest';
import { click, mount, text, unmountAll } from './harness';
import { ADA, SCRIBE, SERVICE, ok, refused } from './fixtures';
const path = '/agents/' + SCRIBE + '/provisioning';
const profile = { version: 1, operation: 'op-' + 'a'.repeat(32), instructions: 'Review these instructions', note: '', model_access: [], tools: [], skills: [], mcp_servers: [], set_by: ADA, set_at: 1790000000, reviewed_by: null, reviewed_at: null, self_reviewed: false };
const answer = { agent: SCRIBE, recorded: null, profile, versions: [], enforced: false };
const routes = { ...SERVICE, [path]: ok(answer) };
const review = path + '/1/review';
const button = (label: string) => [...document.querySelectorAll('button')].find((entry) => entry.textContent === label) ?? null;
beforeEach(() => sessionStorage.clear());
describe('Profile reviews', () => {
  it('reviews the displayed version only through an explicit action', async () => {
    const { posted } = await mount('#/file/' + SCRIBE + '/provisioning', { ...routes, ['POST ' + review]: (body) => ok({ ...answer, recorded: { operation: (body as Record<string, unknown>).operation, version: 1 } }) });
    expect(posted).toEqual([]); await click(button('Approve these settings'));
    expect(posted).toEqual([{ path: review, body: { operation: expect.stringMatching(/^op-/) } }]); expect(text()).toContain('Version 1 of these settings is approved');
  });
  it('does not claim another person’s prior review as this operation', async () => {
    await mount('#/file/' + SCRIBE + '/provisioning', { ...routes, ['POST ' + review]: ok({ ...answer, recorded: { operation: 'op-' + 'b'.repeat(32), version: 1 } }) });
    await click(button('Approve these settings')); expect(text()).toContain('was already approved'); expect(text()).not.toContain('Version 1 of these settings is approved'); expect(sessionStorage.length).toBe(0);
  });
  it('shows an unconfirmed approval of a replaced version as replaced, never sends it again, and offers only the newest (#119)', async () => {
    await mount('#/file/' + SCRIBE + '/provisioning', { ...routes, ['POST ' + review]: refused(503, 'ProvisioningUnavailable', 'Unknown outcome') });
    await click(button('Approve these settings')); unmountAll(); document.body.innerHTML = '';
    const later = { ...answer, profile: { ...profile, version: 2, instructions: 'Newer profile' } };
    const next = await mount('#/file/' + SCRIBE + '/provisioning', { ...routes, [path]: ok(later), ['POST ' + path + '/2/review']: (body) => ok({ ...later, recorded: { operation: (body as Record<string, unknown>).operation, version: 2 } }) });
    expect(button('Check original change')).toBeNull(); expect(next.posted).toEqual([]);
    expect(text()).toContain('version 2 has replaced it'); expect(sessionStorage.length).toBe(0);
    await click(button('Approve these settings'));
    expect(next.posted).toEqual([{ path: path + '/2/review', body: { operation: expect.stringMatching(/^op-/) } }]); expect(text()).toContain('Version 2 of these settings is approved');
  });
  it('keeps an answer for another version unresolved', async () => {
    await mount('#/file/' + SCRIBE + '/provisioning', { ...routes, ['POST ' + review]: (body) => ok({ ...answer, recorded: { operation: (body as Record<string, unknown>).operation, version: 2 } }) });
    await click(button('Approve these settings')); expect(text()).toContain('original request is retained'); expect(sessionStorage.length).toBe(1);
  });
});
