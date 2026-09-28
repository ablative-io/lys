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
    expect(posted).toEqual([]); await click(button('I have reviewed version 1'));
    expect(posted).toEqual([{ path: review, body: { operation: expect.stringMatching(/^op-/) } }]); expect(text()).toContain('Your review of version 1 was recorded');
  });
  it('does not claim another person’s prior review as this operation', async () => {
    await mount('#/file/' + SCRIBE + '/provisioning', { ...routes, ['POST ' + review]: ok({ ...answer, recorded: { operation: 'op-' + 'b'.repeat(32), version: 1 } }) });
    await click(button('I have reviewed version 1')); expect(text()).toContain('was already reviewed'); expect(text()).not.toContain('Your review of version 1 was recorded'); expect(sessionStorage.length).toBe(0);
  });
  it('recovers the original version after a newer profile is published', async () => {
    const first = await mount('#/file/' + SCRIBE + '/provisioning', { ...routes, ['POST ' + review]: refused(503, 'ProvisioningUnavailable', 'Unknown outcome') });
    await click(button('I have reviewed version 1')); unmountAll(); document.body.innerHTML = '';
    const later = { ...answer, profile: { ...profile, version: 2, instructions: 'Newer profile' } };
    const next = await mount('#/file/' + SCRIBE + '/provisioning', { ...routes, [path]: ok(later), ['POST ' + review]: (body) => ok({ ...later, recorded: { operation: (body as Record<string, unknown>).operation, version: 1 } }) });
    await click(button('Check original change')); expect(next.posted).toEqual(first.posted); expect(text()).toContain('Newer profile'); expect(text()).toContain('Your review of version 1 was recorded');
  });
  it('keeps an answer for another version unresolved', async () => {
    await mount('#/file/' + SCRIBE + '/provisioning', { ...routes, ['POST ' + review]: (body) => ok({ ...answer, recorded: { operation: (body as Record<string, unknown>).operation, version: 2 } }) });
    await click(button('I have reviewed version 1')); expect(text()).toContain('original request is retained'); expect(sessionStorage.length).toBe(1);
  });
});
