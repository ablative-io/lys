/** Drafts under Access: what an agent prepared for its person, one table; a row opens the prepared request whole; a waiting draft is approved or refused in its own row with the exact bodies the service takes, and a decision with no answer is kept and asked about, never sent twice. */
import { beforeEach, describe, expect, it } from 'vitest';
import { $, $$, click, mount, text, type, unmountAll, unreachable } from './harness';
import { ADA, SCRIBE, SERVICE, ok, refused } from './fixtures';
import { clock } from '../src/features/file/time';

const DRAFT = 'op-' + '7'.repeat(32);
const OTHER = 'op-' + '8'.repeat(32);
const ADA_NAMED = { id: ADA, display_name: 'Ada (test person)' };
const decision = (more: Record<string, unknown> = {}) => ({ operation: 'op-' + '9'.repeat(32), by: ADA_NAMED, login: { provider: 'https://issuer.test', subject: 'ada' }, at: 1790000100, application: null, replacement: null, ...more });
const HASH = 'ab'.repeat(32);
const draft = (more: Record<string, unknown> = {}) => ({
  id: DRAFT, agent: { id: SCRIBE, display_name: 'Scribe' }, responsible: { ...ADA_NAMED, state: 'active' },
  target: { kind: 'project', id: 'identity', action: 'edit' }, method: 'POST', path: '/grants', body: '{"relation":"editor","resource":"project:identity"}',
  note: 'To fix the typo on the front page', created_at: 1790000000, creation_hash: HASH, state: 'waiting', ...more,
});
const recorded = (body: unknown) => { const sent = body as { operation: string; creation_hash: string }; return ok({ draft: DRAFT, operation: sent.operation, creation_hash: sent.creation_hash, index: 4, tree_size: 5, leaf_hash: 'cd'.repeat(32), replacement: null }); };
const routes = { ...SERVICE, '/drafts?state=waiting': ok({ drafts: [draft()] }) };
const row = () => $(`tr[data-draft="${DRAFT}"]`);
const button = (words: string) => $$('button').find((el) => el.textContent === words) ?? null;

describe('Drafts under Access', () => {
  beforeEach(() => { sessionStorage.clear(); });

  it('is a tab of Access after Requests, at its own address', async () => {
    await mount('#/access/drafts', routes);
    const tabs = $$('nav[aria-label="Access views"] a').map((tab) => tab.textContent);
    expect(tabs.indexOf('Drafts')).toBe(tabs.indexOf('Requests') + 1);
    expect($('nav[aria-label="Access views"] a.on')?.getAttribute('href')).toBe('#/access/drafts');
    expect($('#rail a.on')?.dataset.nav).toBe('access');
    expect(unreachable()).toEqual([]);
  });

  it('lists a draft by the agent, what it wants in plain words, its note and when it was asked, with no identifier shown', async () => {
    await mount('#/access/drafts', routes);
    expect($$('table.drafts thead th').map((th) => th.textContent)).toEqual(['Agent', 'Wants to', 'Note', 'Asked', 'Decision']);
    expect([...(row()?.querySelectorAll('td') ?? [])].map((td) => td.textContent)).toEqual([
      'Scribe', 'Edit this resource · project identity', 'To fix the typo on the front page', clock(1790000000), 'ApproveRefuse',
    ]);
    for (const id of [DRAFT, SCRIBE, ADA, HASH]) expect(text()).not.toContain(id);
  });

  it('opens a row to show the prepared request whole, and closes it again', async () => {
    await mount('#/access/drafts', routes);
    expect($('.draft-request')).toBeNull();
    await click(row());
    expect(row()?.getAttribute('aria-expanded')).toBe('true');
    expect($('.draft-request')?.textContent).toBe('POST /grants\n\n{"relation":"editor","resource":"project:identity"}');
    await click(row());
    expect($('.draft-request')).toBeNull();
  });

  it('approves with the draft\'s creation hash, a new operation and a separate application operation, and reads the list again', async () => {
    let decided = false;
    const { posted } = await mount('#/access/drafts', {
      ...routes,
      '/drafts?state=waiting': () => ok({ drafts: decided ? [] : [draft()] }),
      '/drafts?state=decided': () => ok({ drafts: decided ? [draft({ state: 'approved', decided: decision({ application: 'op-' + 'a'.repeat(32) }) })] : [] }),
      ['POST /drafts/' + DRAFT + '/approve']: (body) => { decided = true; return recorded(body); },
    });
    await click(button('Approve'));
    expect(posted.map((entry) => entry.path)).toEqual(['/drafts/' + DRAFT + '/approve']);
    const body = posted[0].body as Record<string, string>;
    expect(Object.keys(body).sort()).toEqual(['application', 'creation_hash', 'operation']);
    expect(body.creation_hash).toBe(HASH);
    expect(body.operation).toMatch(/^op-[0-9a-f]{32}$/);
    expect(body.application).toMatch(/^op-[0-9a-f]{32}$/);
    expect(body.application).not.toBe(body.operation);
    expect(row()).toBeNull();
    expect(text().split('Nothing is waiting.')).toHaveLength(2);
    await click(button('Decided'));
    expect(row()?.querySelectorAll('td')[4].textContent).toBe('Approved by Ada (test person), ' + clock(1790000100));
    expect(sessionStorage.length).toBe(0);
  });

  it('refuses with a reason asked for in the row itself', async () => {
    const { posted } = await mount('#/access/drafts', { ...routes, ['POST /drafts/' + DRAFT + '/refuse']: recorded });
    await click(button('Refuse'));
    const input = row()?.querySelector('input[aria-label="Why you refuse it"]') ?? null;
    expect(input).not.toBeNull();
    expect(posted).toEqual([]);
    await type(input, 'Not this week');
    await click(row()?.querySelector('button[data-act="refuse-confirm"]') ?? null);
    expect(posted).toEqual([{ path: '/drafts/' + DRAFT + '/refuse', body: { operation: expect.stringMatching(/^op-[0-9a-f]{32}$/), creation_hash: HASH, reason: 'Not this week' } }]);
  });

  it('keeps a decision whose answer was lost, offers no second decision, and asks about the same one after a remount', async () => {
    const service = { ...routes, ['POST /drafts/' + DRAFT + '/approve']: refused(503, 'AppendUncertain', 'not known') };
    const first = await mount('#/access/drafts', service);
    await click(button('Approve'));
    expect(row()?.textContent).toContain('This change has no confirmed answer.');
    expect(button('Approve')).toBeNull();
    expect(button('Refuse')).toBeNull();
    unmountAll(); document.body.innerHTML = '';
    const second = await mount('#/access/drafts', { ...service, ['POST /drafts/' + DRAFT + '/approve']: recorded });
    expect(button('Approve')).toBeNull();
    expect(button('Refuse')).toBeNull();
    await click(button('Check whether Lys saved it'));
    expect(second.posted).toEqual(first.posted);
    expect(second.posted).toHaveLength(1);
    expect(sessionStorage.length).toBe(0);
  });

  it('shows decided drafts under Decided with who decided and why, and says an empty list once', async () => {
    await mount('#/access/drafts', { ...routes, '/drafts?state=waiting': ok({ drafts: [] }), '/drafts?state=decided': ok({ drafts: [draft({ state: 'refused', decided: decision({ at: 1790000200, reason: 'Not this week' }) })] }) });
    expect($('table.drafts tbody tr.empty td')?.textContent).toBe('Nothing is waiting.');
    expect(text().split('Nothing is waiting.')).toHaveLength(2);
    await click(button('Decided'));
    expect(row()?.querySelectorAll('td')[4].textContent).toBe('Refused by Ada (test person), ' + clock(1790000200) + 'Not this week');
    expect(row()?.querySelector('button')).toBeNull();
    unmountAll(); document.body.innerHTML = '';
    await mount('#/access/drafts', SERVICE);
    await click(button('Decided'));
    expect(text().split('Nothing has been decided yet.')).toHaveLength(2);
  });

  it('names the service\'s refusal when the drafts cannot be read, never an empty list', async () => {
    await mount('#/access/drafts', { ...routes, '/drafts?state=waiting': refused(503, 'DraftsUnavailable', 'the drafts store is closed') });
    expect(text()).toContain('DraftsUnavailable');
    expect(text()).not.toContain('Nothing is waiting.');
  });

  it('names an agent and a decider the directory no longer holds without an identifier, and says a corrected draft was replaced', async () => {
    await mount('#/access/drafts', { ...routes, '/drafts?state=waiting': ok({ drafts: [draft({ agent: null })] }), '/drafts?state=decided': ok({ drafts: [
      draft({ id: OTHER, state: 'refused', agent: { id: SCRIBE, display_name: null }, decided: decision({ by: null, login: { provider: 'https://issuer.test', subject: 'person-' + '3'.repeat(32) }, reason: 'No' }) }),
      draft({ state: 'corrected', decided: decision({ by: null, login: { provider: 'https://issuer.test', subject: 'ada@example.test' }, reason: 'Wrong project', replacement: 'op-' + 'c'.repeat(32) }) }),
    ] }) });
    expect(row()?.querySelector('td')?.textContent).toBe('An agent outside your view');
    await click(button('Decided'));
    expect($(`tr[data-draft="${OTHER}"] td`)?.textContent).toBe('An agent outside your view');
    expect($(`tr[data-draft="${OTHER}"]`)?.querySelectorAll('td')[4].textContent).toBe('Refused by someone outside your view, ' + clock(1790000100) + 'No');
    expect(row()?.querySelectorAll('td')[4].textContent).toBe('Corrected by ada@example.test, ' + clock(1790000100) + 'Wrong projectReplaced by a corrected draft.');
    for (const id of [SCRIBE, 'person-' + '3'.repeat(32), 'op-' + 'c'.repeat(32)]) expect(text()).not.toContain(id);
    expect(text()).not.toContain('administrator');
  });

  it('links a corrected draft to its replacement when the list holds it, and opens that row', async () => {
    await mount('#/access/drafts', { ...routes, '/drafts?state=decided': ok({ drafts: [
      draft({ state: 'corrected', decided: decision({ reason: 'Wrong project', replacement: OTHER }) }),
      draft({ id: OTHER, state: 'refused', created_at: 1790000300, decided: decision({ reason: 'No' }) }),
    ] }) });
    await click(button('Decided'));
    const link = row()?.querySelector('a[data-act="replacement"]');
    expect(link?.getAttribute('href')).toBe('#/access/drafts?draft=' + OTHER);
    await click(link ?? null);
    expect($(`tr[data-draft="${OTHER}"]`)?.getAttribute('aria-expanded')).toBe('true');
  });

  it('reads every page of the list, one after another, until there is no next page', async () => {
    const { requests } = await mount('#/access/drafts', { ...routes,
      '/drafts?state=waiting': ok({ drafts: [draft()], total: 2, next: 'page-2' }),
      '/drafts?state=waiting&after=page-2': ok({ drafts: [draft({ id: OTHER, note: 'The second page', created_at: 1790000300 })], total: 2, next: null }),
    });
    expect($$('tr[data-draft]').map((tr) => tr.dataset.draft)).toEqual([DRAFT, OTHER]);
    expect(requests.filter((path) => path.startsWith('/drafts'))).toEqual(['/drafts?state=waiting', '/drafts?state=waiting&after=page-2']);
  });
});
