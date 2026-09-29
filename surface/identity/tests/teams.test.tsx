/** Team membership receipts survive subsequent changes and never imply inherited authority. */
import { act } from 'react';
import { beforeEach, describe, expect, it } from 'vitest';
import { $, choose, click, mount, settle, text, unmountAll } from './harness';
import type { Route } from './fixtures';
import { ADA, ME, SCRIBE, SERVICE, ok, refused } from './fixtures';
const team = { id: 'op-' + 'a'.repeat(32), name: 'Delivery', owner: ADA, description: 'Ship work', members: [SCRIBE], state: 'active', created_by: ME.signed_in, created_at: 1790000000, retired_at: null };
const routes = { ...SERVICE, '/teams': ok({ teams: [team] }) };
const button = (label: string) => [...document.querySelectorAll('button')].find((entry) => entry.textContent === label) ?? null;
const receipt = (body: unknown, act: string, member: string | null, current = team) => ok({ ...current, recorded: { operation: (body as Record<string, unknown>).operation, act, member, by: ME.signed_in, at: 1790000000 } });
async function open(extra: Record<string, Route> = routes) { const mounted = await mount('#/people', extra); await click($('[data-kind="teams"]')); return mounted; }
beforeEach(() => sessionStorage.clear());
describe('Teams', () => {
  it('creates an owned team without grants or credentials', async () => {
    const { posted } = await open({ ...routes, 'POST /teams': (body) => { const request = body as Record<string, string>; return receipt(body, 'created', null, { ...team, id: request.operation, name: request.name, description: request.description }); } });
    const input = $('form[aria-label="Create team"] input'); if (!(input instanceof HTMLInputElement)) throw new Error('Team name missing');
    await act(async () => { Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')?.set?.call(input, 'Reviewers'); input.dispatchEvent(new Event('input', { bubbles: true })); }); await settle();
    posted.length = 0; await click(button('Create team')); expect(posted).toEqual([{ path: '/teams', body: { operation: expect.stringMatching(/^op-/), name: 'Reviewers', description: '' } }]); expect(text()).toContain('Your team was recorded');
  });
  it('confirms membership changes and records only the selected person', async () => {
    const path = '/teams/' + team.id + '/members'; const { posted } = await open({ ...routes, ['POST ' + path]: (body) => receipt(body, 'added', ADA) });
    posted.length = 0; await choose($('select'), ADA); await click(button('Add member')); expect(posted).toEqual([]); await click(button('Confirm add member'));
    expect(posted).toEqual([{ path, body: { operation: expect.stringMatching(/^op-/), member: ADA } }]); expect(text()).toContain('Your team change was recorded');
  });
  it('recovers a removed member receipt after the member no longer appears', async () => {
    const path = '/teams/' + team.id + '/members/' + SCRIBE + '/remove'; const first = await open({ ...routes, ['POST ' + path]: refused(503, 'TeamsUnavailable', 'Unknown outcome') });
    first.posted.length = 0; await click(button('Remove Scribe')); await click(button('Confirm remove member')); unmountAll(); document.body.innerHTML = '';
    const changed = { ...team, members: [] }; const later = await open({ ...routes, '/teams': ok({ teams: [changed] }), ['POST ' + path]: (body) => receipt(body, 'removed', SCRIBE, changed) });
    later.posted.length = 0; await click(button('Check original change')); expect(later.posted).toEqual(first.posted); expect(text()).toContain('Your team change was recorded');
  });
  it('keeps a mismatched act unresolved', async () => {
    const path = '/teams/' + team.id + '/retire'; await open({ ...routes, ['POST ' + path]: (body) => receipt(body, 'removed', SCRIBE) }); await click(button('Retire team')); await click(button('Confirm retire team')); expect(text()).toContain('original request is retained'); expect(sessionStorage.length).toBe(1);
  });
  it('accepts an original add receipt even after the member was removed later', async () => {
    const path = '/teams/' + team.id + '/members'; await open({ ...routes, ['POST ' + path]: (body) => receipt(body, 'added', ADA, { ...team, members: [] }) }); await choose($('select'), ADA); await click(button('Add member')); await click(button('Confirm add member')); expect(text()).toContain('Your team change was recorded');
  });
  it('names an unavailable team store instead of claiming it is empty', async () => {
    await open({ ...routes, '/teams': refused(503, 'TeamsUnavailable', 'No team store') }); expect(text()).toContain('TeamsUnavailable'); expect(text()).not.toContain('No teams have been recorded');
  });
  it('names a held member and lets the administrator confirm with the original operation receipt', async () => {
    const path = '/teams/' + team.id + '/members/' + SCRIBE + '/confirm';
    const held = { ...team, held: [{ member: SCRIBE, reason: 'The earlier addition was not authorized.' }] };
    const { posted } = await open({ ...routes, '/teams': ok({ teams: [held] }), ['POST ' + path]: (body) => receipt(body, 'confirmed', SCRIBE) });
    expect(text()).toContain('Awaiting administrator confirmation');
    expect(text()).toContain('The earlier addition was not authorized.');
    posted.length = 0;
    await click(button('Allow Scribe to take part'));
    expect(posted).toEqual([]);
    await click(button('Confirm allow member to take part'));
    expect(posted).toEqual([{ path, body: { operation: expect.stringMatching(/^op-/) } }]);
    expect(text()).toContain('Your team change was recorded');
  });

});
