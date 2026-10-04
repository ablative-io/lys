/** Model and resource screens display served records and preserve their visibility boundary. */
import { describe, expect, it } from 'vitest';
import { $, $$, mount, text, type } from './harness';
import { ADA, SERVICE, ok, refused } from './fixtures';

describe('Access read views', () => {
  it('renders the served model version and relations instead of a fixed model', async () => {
    await mount('#/model', { ...SERVICE, '/grants/model': ok({ action_sentences: { read: 'Read this resource', publish: 'Publish this resource' }, version: 19, relations: { reader: ['read'], publisher: ['read', 'publish'] } }) });
    expect(text()).toContain('version 19');
    // The model is the one permissions matrix: a row for each action in its sentence, a column for each relation.
    expect($$('table.schema-matrix thead th').map((cell) => cell.textContent)).toEqual(['What it may do', 'reader', 'publisher']);
    expect($$('table.schema-matrix tbody th[scope="row"]').map((cell) => cell.textContent)).toEqual(['Read this resource', 'Publish this resource']);
    expect($$('table.schema-matrix tbody td').map((cell) => cell.getAttribute('aria-label') + ':' + cell.textContent)).toEqual([
      'reader may read this resource:✓', 'publisher may read this resource:✓',
      'reader may not publish this resource:', 'publisher may publish this resource:✓',
    ]);
  });
  it('shows server-judged counts and filters only the returned resources', async () => {
    const { requests } = await mount('#/resources', { ...SERVICE, '/resources': ok({
      kinds: ['project', 'secret'], revision: 23, judged_at: 1790540000,
      resources: [{ kind: 'project', id: 'identity', standing: 7, ended: 2, holders: 3 }, { kind: 'secret', id: 'build', standing: 1, ended: 0, holders: 1 }],
    }) });
    expect($$('tbody tr[data-href]')).toHaveLength(2);
    expect($('a[href="#/access/who/project%3Aidentity"]')).not.toBeNull();
    expect($$('tbody tr[data-href]')[0].textContent).toContain('identity723');
    expect(text()).toContain('revision 23');
    expect(requests).toContain('/resources');
    await type($('input[aria-label="Search resources"]'), 'secret');
    expect($$('tbody tr[data-href]')).toHaveLength(1);
    expect($('tbody tr[data-href]')?.textContent).toContain('build');
  });
  it('reads a person or an agent among the resources by name, never by raw id', async () => {
    await mount('#/resources', { ...SERVICE, '/resources': ok({
      kinds: ['person'], revision: 23, judged_at: 1790540000,
      resources: [{ kind: 'person', id: ADA, standing: 1, ended: 0, holders: 1 }, { kind: 'person', id: 'person-' + 'f'.repeat(32), standing: 1, ended: 0, holders: 1 }],
    }) });
    const rows = $$('tbody tr[data-href]').map((row) => row.textContent);
    expect(rows[0]).toContain('Ada (test person)');
    expect(rows[1]).toContain('someone outside your view');
    expect(text()).not.toMatch(/person-[0-9a-f]{32}/);
    await type($('input[aria-label="Search resources"]'), 'Ada');
    expect($$('tbody tr[data-href]')).toHaveLength(1);
  });
  it('names a model read failure instead of substituting defaults', async () => {
    await mount('#/model', { ...SERVICE, '/grants/model': refused(503, 'PermissionEngineUnavailable', 'model cannot be read') });
    expect(text()).toContain('PermissionEngineUnavailable');
    expect($('tbody')).toBeNull();
  });
});
