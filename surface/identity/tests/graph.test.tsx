/** The graph draws permission answers, preserves visibility and never exercises grants. */
import { describe, expect, it } from 'vitest';
import { $, $$, mount, serve, text } from './harness';
import { reachMap } from '../src/features/grants/check';
import { OWN, SCRIBE, SERVICE, ok, refused } from './fixtures';

const drawing = () => $('svg[aria-label="Permission graph"]');
const titles = (kind: string) => $$('line.ge.' + kind).map((line) => line.textContent ?? '');
const card = () => $('.node-card')?.textContent ?? '';
/** A reach answer naming every resource asked about, with no holder on any. */
const noHolders = (body: unknown) => ok({ revision: 7, resources: (body as { resources: { kind: string; id: string }[] }).resources.map(({ kind, id }) => ({ kind, id, holders: [] })) });

describe('Identity graph', () => {
  it('draws every identity and resource, the recorded grants and who answers to whom', async () => {
    const app = await mount('#/graph');
    const labels = $$('g.gn').map((node) => node.getAttribute('aria-label'));
    for (const name of ['Ada (test person)', 'Scribe', 'Courier', 'Archivist', 'Bea (test person)', 'Reviewer', 'Lamplighter', 'identity', 'ledger']) expect(labels).toContain(name);
    expect(titles('grant')).toEqual(['Ada (test person): everything here on project identity', 'Ada (test person): View this resource on project ledger', 'Scribe: View this resource on project identity']);
    expect(titles('answers')).toHaveLength(5);
    expect(titles('answers')).toContain('Scribe answers to Ada (test person)');
    expect(app.posted.map((call) => call.path)).toEqual(['/grants/reach']);
  });
  it('lights up a selected agent, its responsible person and what the service says it reaches', async () => {
    await mount('#/graph/' + SCRIBE);
    const hot = titles('hot');
    expect(hot).toHaveLength(2);
    expect(hot).toContain('Scribe answers to Ada (test person)');
    expect(hot).toContain('Scribe: View this resource on project identity');
    const lit = $$('g.gn:not(.dimmed)').map((node) => node.getAttribute('aria-label'));
    expect(lit.sort()).toEqual(['Ada (test person)', 'Scribe', 'identity']);
    expect($('.node-card h2')?.textContent).toBe('Scribe');
    expect($('.node-card a[href="#/access/who/project%3Aidentity"]')?.textContent).toContain('View this resource');
    expect($('.node-card a[href="#/file/' + SCRIBE + '"]')).not.toBeNull();
  });
  it('shows a resource reached by the holders the service names, and opens it by its full name', async () => {
    await mount('#/graph/' + encodeURIComponent('resource:project:identity'));
    expect($('.node-card h2')?.textContent).toBe('identity');
    expect(card()).toContain('Ada (test person)');
    expect(card()).toContain('Scribe');
    expect(card()).not.toContain('Nobody.');
    expect($('a[href="#/access/who/project%3Aidentity"]')).not.toBeNull();
  });
  it('does not turn recorded grants into permission when the evaluator returns none', async () => {
    await mount('#/graph/' + SCRIBE, { ...SERVICE, 'POST /grants/reach': noHolders });
    expect(titles('grant')).toContain('Scribe: View this resource on project identity');
    expect(card()).toContain('Nothing.');
    expect($('.node-card a[href^="#/access/who/"]')).toBeNull();
  });
  it('draws the connections the install records when no grant is recorded', async () => {
    const app = { id: 'notes', name: 'Notes', state: 'approved', redirects: [], schema: null, version: 1, versions: [1], pending: null, client_id: 'c1', service_account: null, registered_by: { kind: 'lys' }, registered_at: 1 };
    await mount('#/graph', { ...SERVICE, '/grants': ok({ grants: [], revision: 7 }), '/apps': ok({ apps: [app] }) });
    expect(titles('grant')).toHaveLength(0);
    const installed = titles('installed');
    expect(installed).toHaveLength(2);
    expect(installed[0]).toContain('administers');
    expect(installed[1]).toContain('Notes');
    expect(installed[1]).toContain('signs in through Lys');
  });
  it('names no installed connection in a personal view', async () => {
    await mount('#/graph', { ...SERVICE, '/directory/people': refused(403, 'NotAdmitted', 'not admitted'), '/people': ok(OWN) });
    expect(drawing()).not.toBeNull();
    expect(titles('installed')).toHaveLength(0);
  });
  it('names an unavailable evaluator instead of drawing partial permissions', async () => {
    await mount('#/graph', { ...SERVICE, 'POST /grants/reach': refused(503, 'PermissionEngineUnavailable', 'engine unavailable') });
    expect(text()).toContain('PermissionEngineUnavailable');
    expect(drawing()).toBeNull();
  });
  it('names an identity outside the visible directory', async () => {
    await mount('#/graph/person-missing');
    expect(text()).toContain('Identity person-missing is not in the directory records you may see');
    expect(drawing()).toBeNull();
  });
});

describe('Directory reach', () => {
  it('shares service answers across the table and preview without exercising a grant', async () => {
    const app = await mount('#/people');
    expect($('tbody tr td:last-child')?.textContent).toBe('2 resources');
    expect(app.posted.map((call) => call.path)).toEqual(['/grants/reach']);
  });
  it('shows the named refusal instead of zero reach on an unreadable grant store', async () => {
    await mount('#/people', { ...SERVICE, '/grants': refused(503, 'GrantStoreUnavailable', 'store offline') });
    expect($('tbody tr td:last-child')?.textContent).toContain('GrantStoreUnavailable');
    expect(text()).toContain('store offline');
  });
});

describe('Complete permission reads', () => {
  it('refuses an answer that leaves out a resource asked about instead of displaying a partial graph', async () => {
    await mount('#/graph', { ...SERVICE, 'POST /grants/reach': ok({ revision: 7, resources: [] }) });
    expect(text()).toContain('PermissionAnswerIncomplete');
    expect(drawing()).toBeNull();
  });
  it('asks about every resource in one question, however many, so the answer is at one revision', async () => {
    const posted: { path: string; body: unknown }[] = [];
    serve({ 'POST /grants/reach': (body) => noHolders(body) }, posted);
    const many = Array.from({ length: 1001 }, (_, n) => ({ resource: { kind: 'project', id: String(n) }, actions: ['view'] }));
    const map = await reachMap(many);
    expect(map.size).toBe(1001);
    expect(posted.map((call) => (call.body as { resources: unknown[] }).resources.length)).toEqual([1001]);
  });
});
