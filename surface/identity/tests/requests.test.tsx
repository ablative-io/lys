/** Access requests keep their identity across failed transports and expose the server's decisions. */
import { act } from 'react';
import { beforeEach, describe, expect, it } from 'vitest';
import { $, choose, click, mount, settle, text, unmountAll, unreachable } from './harness';
import { ADA, ME, SERVICE, ok, refused } from './fixtures';
import type { Ask } from '../src/features/requests/contract';

beforeEach(() => sessionStorage.clear());
const model = { action_sentences: { read: 'Read this resource', write: 'Write to this resource' }, version: 1, relations: { reader: ['read'], editor: ['read', 'write'] } };
const routes = { ...SERVICE, '/requests': ok({ requests: [] }), '/grants/model': ok(model),
  '/grants': ok({ revision: 1, grants: [{ resource: { kind: 'project', id: 'Lys' } }] }) };
const kept = (asked: Ask) => ({ ...asked, id: asked.operation, asked_by: ADA, asked_by_name: ME.person.display_name, responsible: ME.person,
  actions: model.relations.reader, asked_at: 1790000000, state: 'waiting', approvers: [ME.person], sources: [], decision: null });
const button = (label: string) => [...document.querySelectorAll('button')].find((value) => value.textContent === label) ?? null;
const form = () => {
  const element = $('form[aria-label="Ask for access"]');
  if (!element) throw new Error('Ask form missing');
  return element;
};
async function prepare() {
  await choose(form().querySelector('select'), '0');
  await click(form().querySelector('input[name="action"][value="read"]'));
  const why = form().querySelector('textarea');
  if (!why) throw new Error('Why field missing');
  why.value = 'Review the release';
  await click(form().querySelector('input[name="no-expiry"]'));
}
async function submit() {
  await act(async () => { form().dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })); });
  await settle();
}

describe('Requests', () => {
  it('asks as the signed-in person using visible resources and the served permission model', async () => {
    const mounted = await mount('#/requests', { ...routes, 'POST /requests': (body) => ok(kept(body as Ask)) });
    await prepare(); await submit();
    expect(mounted.posted).toEqual([{ path: '/requests', body: { operation: expect.stringMatching(/^op-[0-9a-f]{32}$/),
      resource: { kind: 'project', id: 'Lys' }, relation: 'reader', ends_at: null, why: 'Review the release' } }]);
    expect(text()).toContain('Access is granted only after approval');
    expect(sessionStorage.getItem('lys.pending.request.' + ADA)).toBeNull();
    expect(unreachable()).toEqual([]);
  });

  it('asks for each ticked action on its own, one request per action, and names a refusal mid-run without re-sending', async () => {
    const wide = { ...model, action_sentences: { read: 'Read this resource', write: 'Write to this resource' }, relations: { ...model.relations, 'only.write': ['write'] } };
    const mounted = await mount('#/requests', { ...routes, '/grants/model': ok(wide), 'POST /requests': (body) => ok(kept(body as Ask)) });
    await choose(form().querySelector('select'), '0');
    await click(form().querySelector('input[name="action"][value="read"]'));
    await click(form().querySelector('input[name="action"][value="write"]'));
    const why = form().querySelector('textarea');
    if (!why) throw new Error('Why field missing');
    why.value = 'Review the release';
    await click(form().querySelector('input[name="no-expiry"]'));
    await submit();
    const asked = mounted.posted.filter((p) => p.path === '/requests').map((p) => p.body as Ask);
    expect(asked.map((a) => a.relation)).toEqual(['reader', 'only.write']);
    expect(new Set(asked.map((a) => a.operation)).size).toBe(2);
    expect(asked.every((a) => a.why === 'Review the release' && a.resource.id === 'Lys')).toBe(true);
    expect(form().querySelector('[role="status"]')?.textContent).toContain('2 requests recorded, one per action');
    expect(sessionStorage.getItem('lys.pending.request.' + ADA + '.rest')).toBeNull();

    unmountAll();
    sessionStorage.clear();
    let calls = 0;
    const second = await mount('#/requests', { ...routes, '/grants/model': ok(wide), 'POST /requests': (body) => (++calls === 1 ? ok(kept(body as Ask)) : refused(409, 'Expired', 'Expiry passed')) });
    await choose(form().querySelector('select'), '0');
    await click(form().querySelector('input[name="action"][value="read"]'));
    await click(form().querySelector('input[name="action"][value="write"]'));
    const why2 = form().querySelector('textarea');
    if (!why2) throw new Error('Why field missing');
    why2.value = 'Review the release';
    await click(form().querySelector('input[name="no-expiry"]'));
    await submit();
    expect(second.posted.filter((p) => p.path === '/requests')).toHaveLength(2);
    expect(form().querySelector('[role="status"]')?.textContent).toContain('1 recorded before the refusal; not sent: only.write');
    expect(sessionStorage.getItem('lys.pending.request.' + ADA + '.rest')).toBeNull();
    expect(sessionStorage.getItem('lys.pending.request.' + ADA)).toBeNull();
  });

  it('recovers a committed request by read-back after remount without sending it again', async () => {
    const first = await mount('#/requests', { ...routes, 'POST /requests': refused(503, 'Uncertain', 'No confirmed answer') });
    await prepare(); await submit(); await submit();
    expect(first.posted).toHaveLength(1);
    const asked = first.posted[0].body as Ask;
    unmountAll(); document.body.innerHTML = '';
    const second = await mount('#/requests', { ...routes, '/requests': ok({ requests: [kept(asked)] }) });
    await click(button('Check original request'));
    expect(second.posted).toHaveLength(0);
    expect(text()).toContain('Request recorded');
    expect(sessionStorage.getItem('lys.pending.request.' + ADA)).toBeNull();
  });

  it('retries only the retained operation and preserves it if that retry is refused', async () => {
    const first = await mount('#/requests', { ...routes, 'POST /requests': refused(503, 'Uncertain', 'No confirmed answer') });
    await prepare(); await submit();
    const asked = first.posted[0].body as Ask;
    unmountAll(); document.body.innerHTML = '';
    const second = await mount('#/requests', { ...routes, 'POST /requests': refused(400, 'Expired', 'Expiry passed') });
    await click(button('Check original request'));
    expect(second.posted).toEqual([{ path: '/requests', body: asked }]);
    expect(sessionStorage.getItem('lys.pending.request.' + ADA)).not.toBeNull();
    expect(form().querySelector('fieldset')?.disabled).toBe(true);
  });

  it('keeps unreadable or mismatched successful answers held', async () => {
    const mounted = await mount('#/requests', { ...routes, 'POST /requests': (body) => ok({ ...kept(body as Ask), asked_by: 'another-person' }) });
    await prepare(); await submit(); await submit();
    expect(mounted.posted).toHaveLength(1);
    expect(sessionStorage.getItem('lys.pending.request.' + ADA)).not.toBeNull();
    expect(text()).toContain('did not confirm');
  });

  it('refuses a damaged retained record without sending', async () => {
    sessionStorage.setItem('lys.pending.request.' + ADA, '{broken');
    const mounted = await mount('#/requests', routes);
    await submit();
    expect(mounted.posted).toHaveLength(0);
    expect(text()).toContain('retained request could not be read');
  });

  it('shows names, reason, status and expiry from the recorded request', async () => {
    const asked: Ask = { operation: 'op-' + '1'.repeat(32), resource: { kind: 'project', id: 'Lys' }, relation: 'reader', ends_at: null, why: 'Review the release' };
    await mount('#/requests', { ...routes, '/requests': ok({ requests: [kept(asked)] }) });
    expect(text()).toContain(ME.person.display_name + ' · reader on Lys');
    expect(text()).toContain('Awaiting a decision');
    expect(text()).toContain('Review the release');
    expect(text()).not.toContain('not built yet');
  });

  it('says an empty list once, in the list, and leaves the panel beside it empty', async () => {
    const asked: Ask = { operation: 'op-' + '7'.repeat(32), resource: { kind: 'project', id: 'Lys' }, relation: 'reader', ends_at: null, why: 'Review' };
    const decided = { ...kept(asked), state: 'approved', decision: { by: ADA, note: 'Needed', grant: null, decided_at: 1790000001 } };
    await mount('#/requests', { ...routes, '/requests': ok({ requests: [decided] }) });
    // Nothing is waiting, so the waiting list is empty: said once, in the list, never again beside it.
    expect(text().split('Nothing is waiting.').length - 1).toBe(1);
    expect(text()).not.toContain('Nothing here yet.');
  });

  it('names what a request is on by name when it is someone, never by a raw id', async () => {
    const asked: Ask = { operation: 'op-' + '8'.repeat(32), resource: { kind: 'person', id: ADA }, relation: 'reader', ends_at: null, why: 'Review' };
    await mount('#/requests', { ...routes, '/requests': ok({ requests: [kept(asked)] }) });
    expect(text()).toContain('reader on ' + ME.person.display_name);
    expect(text()).not.toContain(ADA);
  });

  it('requires an explicit decision and uses only the source grants returned by the service', async () => {
    const asked: Ask = { operation: 'op-' + '2'.repeat(32), resource: { kind: 'project', id: 'Lys' }, relation: 'reader', ends_at: null, why: 'Review the release' };
    const entry = { ...kept(asked), sources: ['grant-source'] };
    const approved = { ...entry, state: 'approved', decision: { by: ADA, note: 'Needed for review', grant: 'grant-new', decided_at: 1790000001 } };
    let decided = false;
    const mounted = await mount('#/requests', { ...routes, '/requests': () => ok({ requests: [decided ? approved : entry] }),
      ['POST /requests/' + entry.id + '/approve']: () => { decided = true; return ok(approved); },
    });
    await click(button('Approve access'));
    expect(mounted.posted).toHaveLength(0);
    expect(text()).not.toContain('Give access directly as the administrator');
    await choose($('select[name="source"]'), 'grant-source');
    const note = document.querySelector<HTMLTextAreaElement>('textarea[name="note"]');
    if (!note) throw new Error('Decision note missing');
    note.value = 'Needed for review';
    await click(button('Confirm approval'));
    expect(mounted.posted).toEqual([{ path: '/requests/' + entry.id + '/approve', body: {
      operation: expect.stringMatching(/^op-[0-9a-f]{32}$/), route: 'browser', source: 'grant-source', note: 'Needed for review',
    } }]);
    expect(text()).toContain('Approved');
    expect(button('Approve access')).toBeNull();
  });

  it('offers root issuance only when the service explicitly names that capability', async () => {
    const asked: Ask = { operation: 'op-' + '3'.repeat(32), resource: { kind: 'project', id: 'Lys' }, relation: 'reader', ends_at: null, why: 'Review' };
    await mount('#/requests', { ...routes, '/requests': ok({ requests: [{ ...kept(asked), can_issue_root: true }] }) });
    await click(button('Approve access'));
    expect(text()).toContain('Give access directly as the administrator');
  });

  it('honours an explicit decision capability without inferring grant authority', async () => {
    const asked: Ask = { operation: 'op-' + '6'.repeat(32), resource: { kind: 'project', id: 'Lys' }, relation: 'reader', ends_at: null, why: 'Review' };
    await mount('#/requests', { ...routes, '/requests': ok({ requests: [{ ...kept(asked), approvers: [], can_decide: true, can_issue_root: false }] }) });
    expect(button('Decline request')).not.toBeNull();
    expect(button('Approve access')?.hasAttribute('disabled')).toBe(true);
    unmountAll(); document.body.innerHTML = '';
    await mount('#/requests', { ...routes, '/requests': ok({ requests: [{ ...kept(asked), can_decide: false }] }) });
    expect(button('Decline request')).toBeNull();
    expect(button('Approve access')).toBeNull();
  });

  it('retains the same approval operation across remount and does not permit a conflicting decline', async () => {
    const asked: Ask = { operation: 'op-' + '4'.repeat(32), resource: { kind: 'project', id: 'Lys' }, relation: 'reader', ends_at: null, why: 'Review' };
    const entry = { ...kept(asked), sources: ['grant-source'] };
    const service = { ...routes, '/requests': ok({ requests: [entry] }), ['POST /requests/' + entry.id + '/approve']: refused(503, 'Uncertain', 'Outcome unknown') };
    const first = await mount('#/requests', service);
    await click(button('Approve access')); await choose($('select[name="source"]'), 'grant-source');
    const note = document.querySelector<HTMLTextAreaElement>('textarea[name="note"]');
    if (!note) throw new Error('Decision note missing');
    note.value = 'Approve review';
    await click(button('Confirm approval'));
    unmountAll(); document.body.innerHTML = '';
    const second = await mount('#/requests', service);
    expect(button('Decline request')).toBeNull();
    await click(button('Check original decision'));
    expect(second.posted).toEqual(first.posted);
    expect(second.posted).toHaveLength(1);
  });

  it('declines without issuing a grant and never offers decisions to a non-approver', async () => {
    const asked: Ask = { operation: 'op-' + '5'.repeat(32), resource: { kind: 'project', id: 'Lys' }, relation: 'reader', ends_at: null, why: 'Review' };
    const entry = kept(asked);
    const mounted = await mount('#/requests', { ...routes, '/requests': ok({ requests: [entry] }), ['POST /requests/' + entry.id + '/decline']: ok({ ...entry, state: 'declined', decision: { by: ADA, note: 'Not required', grant: null, decided_at: 1790000001 } }) });
    await click(button('Decline request'));
    const note = document.querySelector<HTMLTextAreaElement>('textarea[name="note"]');
    if (!note) throw new Error('Decision note missing');
    note.value = 'Not required'; await click(button('Confirm decline'));
    expect(mounted.posted).toEqual([{ path: '/requests/' + entry.id + '/decline', body: { note: 'Not required' } }]);
    unmountAll(); document.body.innerHTML = '';
    await mount('#/requests', { ...routes, '/requests': ok({ requests: [{ ...entry, approvers: [] }] }) });
    expect(button('Approve access')).toBeNull(); expect(button('Decline request')).toBeNull();
  });
});

describe('Held approval recovery', () => {
  it('settles the existing server intent without submitting a new decision', async () => {
    const id = 'op-' + 'e'.repeat(32);
    const entry = { ...kept({ operation: id, resource: { kind: 'project', id: 'Lys' }, relation: 'reader', ends_at: null, why: 'Review' }), held_by: ADA, can_decide: true };
    const { posted } = await mount('#/requests', { ...routes, '/requests': ok({ requests: [entry] }), ['POST /requests/' + id + '/reconcile']: ok({ ...entry, state: 'approved', held_by: null }) });
    expect(button('Approve access')).toBeNull(); expect(button('Decline request')).toBeNull();
    await click(button('Check pending approval'));
    expect(posted).toEqual([{ path: '/requests/' + id + '/reconcile', body: {} }]);
  });
  it('names an unresolved grant log without allowing a new decision', async () => {
    const id = 'op-' + 'f'.repeat(32);
    const entry = { ...kept({ operation: id, resource: { kind: 'project', id: 'Lys' }, relation: 'reader', ends_at: null, why: 'Review' }), held_by: ADA, can_decide: true };
    await mount('#/requests', { ...routes, '/requests': ok({ requests: [entry] }), ['POST /requests/' + id + '/reconcile']: refused(409, 'RequestHeld', 'grant outcome uncertain') });
    await click(button('Check pending approval'));
    expect(text()).toContain('An earlier approval still has no confirmed result');
    expect(text()).not.toContain('RequestHeld'); expect(button('Decline request')).toBeNull();
  });
});
