/** Model and resource screens display served records and preserve their visibility boundary. */
import { describe, expect, it } from 'vitest';
import { $, $$, mount, text } from './harness';
import { SERVICE, ok, refused } from './fixtures';

describe('Access read views', () => {
  it('renders the served model version and relations instead of a fixed model', async () => {
    await mount('#/model', { ...SERVICE, '/grants/model': ok({ version: 19, relations: { reader: ['read'], publisher: ['read', 'publish'] } }) });
    expect(text()).toContain('version 19');
    expect($$('tbody tr').map((row) => row.textContent)).toEqual(['readerread', 'publisherread, publish']);
  });
  it('groups visible grants by resource and links to the served access question', async () => {
    await mount('#/resources');
    expect($$('tbody tr')).toHaveLength(2);
    expect($('a[href="#/access/who/project%3Aidentity"]')).not.toBeNull();
    expect(text()).toContain('This includes historical grants');
  });
  it('names a model read failure instead of substituting defaults', async () => {
    await mount('#/model', { ...SERVICE, '/grants/model': refused(503, 'PermissionEngineUnavailable', 'model cannot be read') });
    expect(text()).toContain('PermissionEngineUnavailable');
    expect($('tbody')).toBeNull();
  });
});
