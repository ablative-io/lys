/** Review decisions require explicit confirmation and recover the exact operation after a lost reply. */
import { beforeEach, describe, expect, it } from 'vitest';
import { click, mount, text, unmountAll } from './harness';
import { ADA, ME, SCRIBE, SERVICE, ok, refused } from './fixtures';
const view = { scope: 'personal', revision: 4, judged_at: 1790000000, decisions_recorded: true, unanswered: [], due: [{ agent: { id: SCRIBE, display_name: 'Builder', state: 'active' }, reviewer: ME.person, last_kept: null, grant: { id: 'grant-1', relation: 'editor', resource: { kind: 'project', id: 'Lys' }, actions: ['edit'] } }] };
const routes = { ...SERVICE, '/reviews': ok(view) };
const path = '/reviews/grant-1/keep';
const button = (label: string) => [...document.querySelectorAll('button')].find((entry) => entry.textContent === label) ?? null;
const kept = (body: unknown) => ok({ ...(body as Record<string, unknown>), grant: 'grant-1', kept_by: ADA, at: 1790000000, revision: 4 });
beforeEach(() => sessionStorage.clear());
describe('Keep access', () => {
  it('records an explicit review without extending or changing access', async () => {
    const { posted } = await mount('#/reviews', { ...routes, ['POST ' + path]: kept });
    await click(button('Keep access')); expect(posted).toEqual([]); await click(button('Confirm keep'));
    expect(posted).toEqual([{ path, body: { operation: expect.stringMatching(/^op-/), note: '' } }]); expect(text()).toContain('decision to keep this access was recorded'); expect(sessionStorage.length).toBe(0);
  });
  it('recovers the original decision after a lost reply and remount', async () => {
    const first = await mount('#/reviews', { ...routes, ['POST ' + path]: refused(503, 'ReviewsUnavailable', 'Unknown outcome') });
    await click(button('Keep access')); await click(button('Confirm keep')); unmountAll(); document.body.innerHTML = '';
    const later = await mount('#/reviews', { ...routes, ['POST ' + path]: kept }); await click(button('Check original change')); expect(later.posted).toEqual(first.posted);
  });
  it('does not accept a receipt for a different reviewer', async () => {
    await mount('#/reviews', { ...routes, ['POST ' + path]: (body) => ok({ ...kept(body).body as object, kept_by: 'another-person' }) });
    await click(button('Keep access')); await click(button('Confirm keep')); expect(text()).toContain('original request is retained'); expect(sessionStorage.length).toBe(1);
  });
  it('shows the named refusal when the grant no longer stands', async () => {
    await mount('#/reviews', { ...routes, ['POST ' + path]: refused(409, 'GrantNotDue', 'This grant has ended') });
    await click(button('Keep access')); await click(button('Confirm keep')); expect(text()).toContain('This grant has ended'); expect(text()).not.toContain('GrantNotDue'); expect(text()).not.toContain('decision to keep this access was recorded');
  });
  it('does not offer a recording action when the store is unavailable', async () => {
    await mount('#/reviews', { ...routes, '/reviews': ok({ ...view, decisions_recorded: false }) }); expect(button('Keep access')).toBeNull(); expect(text()).toContain('This service does not record keep decisions.');
  });
});
