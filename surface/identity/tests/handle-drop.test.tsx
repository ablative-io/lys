/** Handle withdrawal requires confirmation and retains the exact operation across an uncertain response. */
import { beforeEach, describe, expect, it } from 'vitest';
import { click, mount, text, unmountAll } from './harness';
import { SCRIBE, SERVICE, ok, refused } from './fixtures';
const handle = 'public-handle-one';
const routes = { ...SERVICE, ['/secrets/handles?holder=' + SCRIBE]: ok({ holder: SCRIBE, handles: [{ id: handle, secret: 'calendar', used: 0, max_uses: 4, not_after_ms: 1990000000000, dropped: false, spend_cap: null, settled: 0, parent: null }] }) };
const button = (label: string) => [...document.querySelectorAll('button')].find((entry) => entry.textContent === label) ?? null;
const receipt = (body: unknown) => ok({ ...(body as Record<string, unknown>), outcome: 'ended', ended: [handle], stopped_here: true, upstream: 'unconfirmed', upstream_reason: 'Provider has not answered' });
beforeEach(() => sessionStorage.clear());
describe('Handle withdrawal', () => {
  it('waits for explicit confirmation and distinguishes provider state', async () => {
    const { posted } = await mount('#/file/' + SCRIBE + '/credentials', { ...routes, 'POST /secrets/drop': receipt });
    await click(button('End handle')); expect(posted).toEqual([]);
    await click(button('Confirm end handle'));
    expect(posted).toMatchObject([{ path: '/secrets/drop', body: { handle, operation: expect.stringMatching(/^op-/) } }]);
    expect(text()).toContain('Use stopped at this broker'); expect(text()).toContain('asked, not yet confirmed'); expect(text()).not.toContain('confirmed by the provider');
  });
  it('retries the same operation after a lost reply and remount', async () => {
    const first = await mount('#/file/' + SCRIBE + '/credentials', { ...routes, 'POST /secrets/drop': refused(503, 'SecretsUnavailable', 'Unknown outcome') });
    await click(button('End handle')); await click(button('Confirm end handle'));
    unmountAll(); document.body.innerHTML = '';
    const next = await mount('#/file/' + SCRIBE + '/credentials', { ...routes, ['/secrets/handles?holder=' + SCRIBE]: ok({ holder: SCRIBE, handles: [{ id: handle, secret: 'calendar', used: 0, max_uses: 4, not_after_ms: 1990000000000, dropped: true, spend_cap: null, settled: 0, parent: null }] }), 'POST /secrets/drop': receipt });
    await click(button('Check original change')); expect(next.posted).toEqual(first.posted); expect(text()).toContain('Use stopped at this broker');
  });
  it('does not accept a receipt for another handle', async () => {
    await mount('#/file/' + SCRIBE + '/credentials', { ...routes, 'POST /secrets/drop': (body) => ok({ ...(body as Record<string, unknown>), handle: 'wrong', outcome: 'ended', ended: [], stopped_here: true, upstream: 'not_asked' }) });
    await click(button('End handle')); await click(button('Confirm end handle'));
    expect(text()).toContain('original request is retained'); expect(text()).not.toContain('Use stopped at this broker');
  });
  it('shows the named authority refusal with no success', async () => {
    await mount('#/file/' + SCRIBE + '/credentials', { ...routes, 'POST /secrets/drop': refused(403, 'NotPersonActedFor', 'Only the person acted for may end this handle') });
    await click(button('End handle')); await click(button('Confirm end handle'));
    expect(text()).toContain('NotPersonActedFor'); expect(text()).not.toContain('Use stopped at this broker'); expect(sessionStorage.length).toBe(0);
  });
});
