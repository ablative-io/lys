/** After Give access the list behind the form shows the new grant, and the form says in words what happened, with the next step as a link. */
import { act } from 'react';
import { beforeEach, describe, expect, it } from 'vitest';
import { $, $$, choose, click, mount, pick, settle } from './harness';
import { ADA, GRANTS, RECEIPTS, SERVICE, ok } from './fixtures';
import type { Grant } from '../src/generated/grants';

beforeEach(() => sessionStorage.clear());
const NEW = 'grant-' + 'f'.repeat(32);

describe('Give access, then the list', () => {
  it('re-reads the grants without closing the form, and says who can now do what', async () => {
    let grants: Grant[] = GRANTS;
    const { requests } = await mount('#/access/issue', { ...SERVICE, '/grants': () => ok({ revision: 7, grants }),
      'POST /grants/roots': (body: unknown) => {
        const operation = (body as { operation: string }).operation;
        grants = [...GRANTS, { ...GRANTS[1], id: NEW, operation, resource: { kind: 'project', id: 'atlas' }, relation: 'editor', actions: ['edit', 'view'] }];
        const receipt = { ...RECEIPTS[4].receipt, operation, identity: ADA };
        return ok({ operation, grant: NEW, index: receipt.log.index, receipt: { ...receipt, caller: ADA, revision: 1 } });
      } });
    const form = $('form[aria-label="Give access"]');
    if (!form) throw new Error('No Give access form');
    expect($$('[data-act="revoke"][data-g="' + NEW + '"]')).toHaveLength(0);
    const reads = requests.filter((path) => path === '/grants').length;
    await pick(form, 'Find a person', 'Ada', 'Ada (test person)');
    await choose(form.querySelector('select[name="kind"]'), 'project');
    await click(form.querySelector('.level-picker input[type="radio"][value="editor"]'));
    await choose(form.querySelector('select[name="resource"]'), '\u0000by-id');
    const id = form.querySelector<HTMLInputElement>('input[name="resource_id"]');
    if (!id) throw new Error('No id box');
    id.value = 'atlas';
    await act(async () => { form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })); });
    await settle();
    expect(requests.filter((path) => path === '/grants').length).toBeGreaterThan(reads);
    expect($('.act-panel.issue-root')).not.toBeNull();
    expect($('.next-step')?.textContent).toBe('Ada (test person) can now edit a project, atlas. See what Ada (test person) can do now, or give more access below.');
    // The form is fresh for the next give, so the one just made cannot be sent again by a second press.
    expect(($('form[aria-label="Give access"] select[name="kind"]') as HTMLSelectElement | null)?.value).toBe('');
    expect($('form[aria-label="Give access"] .level-picker input:checked')).toBeNull();
    expect($$('[data-act="revoke"][data-g="' + NEW + '"]')).toHaveLength(1);
  });
});
