/** The graph draws permission answers, preserves visibility and never exercises grants. */
import { describe, expect, it } from 'vitest';
import { $, $$, mount, text } from './harness';
import { SCRIBE, SERVICE, ok, refused } from './fixtures';

describe('Identity graph', () => {
  it('shows responsibility and permission connections from service answers', async () => {
    const app = await mount('#/graph');
    expect($$('.graph-identity')).toHaveLength(7);
    expect(text()).toContain('Answers to');
    expect($$('.graph-connection')).toHaveLength(3);
    expect($('a[href="#/access/who/project%3Aidentity"]')).not.toBeNull();
    expect(app.posted.every((call) => call.path === '/grants/who')).toBe(true);
  });
  it('focuses a selected agent and its responsible person', async () => {
    await mount('#/graph/' + SCRIBE);
    expect($$('.graph-identity')).toHaveLength(2);
    expect($$('.graph-connection')).toHaveLength(1);
    expect($('.graph-connection')?.textContent).toContain('Scribe');
  });
  it('does not turn recorded grants into permission when the evaluator returns none', async () => {
    await mount('#/graph', { ...SERVICE, 'POST /grants/who': ok({ holders: [], revision: 7, complete: true, next: null }) });
    expect($$('.graph-connection')).toHaveLength(0);
    expect(text()).toContain('No permitted connections');
  });
  it('names an unavailable evaluator instead of drawing partial permissions', async () => {
    await mount('#/graph', { ...SERVICE, 'POST /grants/who': refused(503, 'PermissionEngineUnavailable', 'engine unavailable') });
    expect(text()).toContain('PermissionEngineUnavailable');
    expect($('.identity-graph')).toBeNull();
  });
  it('names an identity outside the visible directory', async () => {
    await mount('#/graph/person-missing');
    expect(text()).toContain('Identity person-missing is not in the directory records you may see');
    expect($('.identity-graph')).toBeNull();
  });
});

describe('Directory reach', () => {
  it('shares service answers across the table and preview without exercising a grant', async () => {
    const app = await mount('#/people');
    expect($('tbody tr td:last-child')?.textContent).toBe('2 resources');
    expect($('.preview')?.textContent).toContain('project:identity');
    expect($('.preview')?.textContent).not.toContain('reach comes from grants');
    expect(app.posted).toHaveLength(4);
    expect(app.posted.every((call) => call.path === '/grants/who')).toBe(true);
  });
  it('shows the named refusal instead of zero reach on an unreadable grant store', async () => {
    await mount('#/people', { ...SERVICE, '/grants': refused(503, 'GrantStoreUnavailable', 'store offline') });
    expect($('tbody tr td:last-child')?.textContent).toContain('GrantStoreUnavailable');
    expect($('.preview')?.textContent).toContain('store offline');
  });
});

describe('Complete permission reads', () => {
  it('refuses a repeated paging cursor instead of displaying a partial graph', async () => {
    await mount('#/graph', { ...SERVICE, 'POST /grants/who': ok({ holders: [], revision: 7, complete: false, next: 'stuck' }) });
    expect(text()).toContain('PermissionPageIncomplete');
    expect($('.identity-graph')).toBeNull();
  });
  it('refuses permissions assembled across changing revisions', async () => {
    let revision = 7;
    await mount('#/graph', { ...SERVICE, 'POST /grants/who': () => ok({ holders: [], revision: revision++, complete: false, next: 'page' }) });
    expect(text()).toContain('GrantRevisionChanged');
    expect($('.identity-graph')).toBeNull();
  });
});
