import { afterEach, describe, expect, it } from 'vitest';
import { $, $$, mount, settle, text, unmountAll } from '../../../tests/harness';
import { BEA_SERVICE, refused } from '../../../tests/fixtures';

// Bea is not the administrator: the service refuses her the whole directory and answers her own scope.
const BEA = { ...BEA_SERVICE, '/directory/people': refused(403, 'NotAdmitted', 'only the administrator reads the whole directory') };

afterEach(() => unmountAll());

const heads = () => $$('.page .head a.btn').map((link) => link.textContent);

describe('Add person on People and agents', () => {
  it('is offered to the administrator, beside Add agent, on every view of the page', async () => {
    await mount('#/people');
    expect(heads()).toEqual(['Add agent', 'Add person']);
    unmountAll();
    await mount('#/people/view/teams');
    await settle();
    expect(heads()).toEqual(['Add agent', 'Add person']);
  });

  it('is not offered to a person who may not add people, and its address says why and what they can do instead', async () => {
    await mount('#/people', BEA);
    expect(heads()).toEqual(['Add agent']);
    unmountAll();
    await mount('#/people/new', BEA);
    await settle();
    expect(text()).toContain('Only the Lys administrator adds people, so this form is not open to you.');
    expect($('a[href="#/agents/new"]')?.textContent).toBe('Add an agent');
  });
});
