/** Every Access view has one head and the tabs, drawn at once and kept when its read is refused, so a refusal never strands a person. */
import { afterEach, describe, expect, it } from 'vitest';
import { $, $$, mount, unmountAll } from './harness';
import { GRANTS, MODEL, SERVICE, ok, refused } from './fixtures';

afterEach(() => unmountAll());
const closed = refused(503, 'ServiceUnavailable', 'the service is restarting');
/** Answers the first read, the shell's own entry check, and refuses every read after it, which is the page's. */
const thenClosed = (first: unknown) => { let reads = 0; return () => (++reads === 1 ? ok(first) : closed); };

describe('Access views keep their head and tabs', () => {
  it.each([
    ['Grants', '#/access', { '/grants': thenClosed({ revision: 7, grants: GRANTS }) }],
    ['Ask', '#/access/can', { '/grants': thenClosed({ revision: 7, grants: GRANTS }) }],
    ['Requests', '#/requests', { '/requests': closed }],
    ['Resources', '#/resources', { '/resources': closed }],
    ['Model', '#/model', { '/grants/model': thenClosed(MODEL) }],
    ['Reviews', '#/reviews', { '/reviews': closed }],
    ['Drafts', '#/access/drafts', { '/drafts?state=waiting': closed }],
  ])('%s, when its read is refused', async (_name, hash, routes) => {
    await mount(hash, { ...SERVICE, ...routes });
    expect($$('.page h1').map((h1) => h1.textContent)).toEqual(['Access']);
    expect($('.page > header.page-header')).not.toBeNull();
    expect($$('nav[aria-label="Access views"] a').length).toBeGreaterThan(5);
    // The refusal is said under the tabs, never as a second page inside this one.
    expect($$('.page .page')).toHaveLength(0);
    // The read was refused, and said: a pass on a page that never refused would prove nothing.
    expect($('.page [role="alert"]')?.textContent).toMatch(/\S/);
  });
});
