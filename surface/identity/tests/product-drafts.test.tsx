/**
 * ACCESS-001 R4: the Drafts screen lists what each product holds for approval, from GET /product-drafts, with the
 * target, the holder, the responsible person, who approved and when, and how the product closed it; a waiting one
 * is approved or refused in its row, and a decision is kept, never sent twice.
 */
import { beforeEach, describe, expect, it } from 'vitest';
import { $, $$, click, mount, text, type } from './harness';
import { ADA, SCRIBE, SERVICE, ok, refused } from './fixtures';
import { clock } from '../src/features/file/time';

const WAITING = 'pdraft-' + '1'.repeat(32);
const APPROVED = 'pdraft-' + '2'.repeat(32);
const EXECUTED = 'pdraft-' + '3'.repeat(32);
const REFUSED = 'pdraft-' + '4'.repeat(32);
const DIGEST = 'ef'.repeat(32);

const held = (id: string, more: Record<string, unknown> = {}) => ({
  id, app: 'notes', grant: 'grant-' + '5'.repeat(32), target: { kind: 'notes.doc', id: 'roadmap', action: 'publish' },
  request_digest: DIGEST, words: 'publish roadmap', mode: 'by_two', holder: SCRIBE, responsible: ADA, approvals: [], ...more,
});

const PAGE = { drafts: [
  held(WAITING, { state: 'waiting', approvals: [{ by: ADA, at: 1790000100 }] }),
  held(APPROVED, { state: 'approved', mode: 'by_draft', approvals: [{ by: ADA, at: 1790000200 }] }),
  held(EXECUTED, { state: 'executed', execution: { state: 'executed', request_digest: DIGEST, receipt_digest: 'aa'.repeat(32) } }),
  held(REFUSED, { state: 'refused_on_execution', execution: { state: 'refused_on_execution', request_digest: DIGEST, refusal: 'roadmap_locked', reason: 'the roadmap is locked for the release' } }),
], next: null, total: 4 };

const routes = { ...SERVICE, '/product-drafts': ok(PAGE) };
const row = (id: string) => $(`tr[data-product-draft="${id}"]`);
const cells = (id: string) => [...(row(id)?.querySelectorAll('td') ?? [])].map((td) => td.textContent);
const decided = (body: unknown) => ok({ draft: WAITING, operation: (body as { operation: string }).operation });

describe('Product drafts on the Drafts screen', () => {
  beforeEach(() => { sessionStorage.clear(); });

  it('lists each with its product, target, mode, holder, responsible person and approvals, as answered', async () => {
    const { requests } = await mount('#/access/drafts', routes);
    expect(requests).toContain('/product-drafts');
    expect($$('table.product-drafts thead th').map((th) => th.textContent)).toEqual(['Product', 'Wants to', 'Holder', 'Approved by', 'State']);
    expect(cells(WAITING).slice(0, 4)).toEqual([
      'notes', 'publish · notes.doc roadmap · by two approvals', 'Scribe · answered for by Ada (test person)', 'Ada (test person), ' + clock(1790000100),
    ]);
    expect(cells(APPROVED)[4]).toBe('Approved, not yet executed');
    expect(cells(EXECUTED)[3]).toBe('No approval yet');
    expect(cells(EXECUTED)[4]).toBe('Executed');
  });

  it("shows a refusal on execution in the product's own name and words", async () => {
    await mount('#/access/drafts', routes);
    expect(row(REFUSED)?.querySelector('.refusal-name')?.textContent).toBe('roadmap_locked');
    expect(cells(REFUSED)[4]).toContain('the roadmap is locked for the release');
  });

  it('approves a waiting one with the exact act it decides, and offers no decision on a closed one', async () => {
    const { posted } = await mount('#/access/drafts', { ...routes, ['POST /product-drafts/' + WAITING + '/approve']: decided });
    for (const closed of [APPROVED, EXECUTED, REFUSED]) expect(row(closed)?.querySelector('[data-act="product-approve"]')).toBeNull();
    await click(row(WAITING)?.querySelector('[data-act="product-approve"]') ?? null);
    expect(posted.at(-1)?.path).toBe('/product-drafts/' + WAITING + '/approve');
    expect(posted.at(-1)?.body).toEqual({ operation: expect.stringMatching(/^op-[0-9a-f]{32}$/), request_digest: DIGEST });
    expect(text()).toContain('Change recorded.');
  });

  it('refuses a waiting one with a reason', async () => {
    const { posted } = await mount('#/access/drafts', { ...routes, ['POST /product-drafts/' + WAITING + '/refuse']: decided });
    await click(row(WAITING)?.querySelector('[data-act="product-refuse"]') ?? null);
    await type(row(WAITING)?.querySelector('input[aria-label="Why you refuse it"]') ?? null, 'not before the release');
    await click(row(WAITING)?.querySelector('[data-act="product-refuse-confirm"]') ?? null);
    expect(posted.at(-1)?.path).toBe('/product-drafts/' + WAITING + '/refuse');
    expect(posted.at(-1)?.body).toEqual({ operation: expect.stringMatching(/^op-[0-9a-f]{32}$/), request_digest: DIGEST, reason: 'not before the release' });
  });

  it('names a refused approval by its name and keeps the operator drafts readable', async () => {
    await mount('#/access/drafts', { ...routes, ['POST /product-drafts/' + WAITING + '/approve']: refused(403, 'draft_self_approval', 'draft_self_approval: the holder may not approve their own draft') });
    await click(row(WAITING)?.querySelector('[data-act="product-approve"]') ?? null);
    expect(text()).toContain('draft_self_approval');
    expect($('table.drafts')).not.toBeNull();
  });

  it('says so by name when the product drafts cannot be read, and still lists the agents\' drafts', async () => {
    await mount('#/access/drafts', { ...routes, '/product-drafts': refused(503, 'DraftsUnavailable', 'the drafts store cannot be read') });
    expect($('section[aria-label="Held for products"]')?.textContent).toContain('DraftsUnavailable');
    expect($('table.drafts')).not.toBeNull();
  });
});
