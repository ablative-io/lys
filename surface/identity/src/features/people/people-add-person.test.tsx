import { act } from 'react';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { $, $$, mount, settle, text, type, unmountAll } from '../../../tests/harness';
import { BEA, BEA_SERVICE, SERVICE, ok, refused } from '../../../tests/fixtures';

// Bea is not the administrator: the service refuses her the whole directory and answers her own scope.
const BEA_NOT_ADMITTED = { ...BEA_SERVICE, '/directory/people': refused(403, 'NotAdmitted', 'only the administrator reads the whole directory') };

beforeEach(() => sessionStorage.clear());
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
    await mount('#/people', BEA_NOT_ADMITTED);
    expect(heads()).toEqual(['Add agent']);
    unmountAll();
    await mount('#/people/new', BEA_NOT_ADMITTED);
    await settle();
    expect(text()).toContain('Adding a person to Example organisation needs organisation.admit there, which you do not hold, so this form is not open to you.');
    expect($('a[href="#/agents/new"]')?.textContent).toBe('Add an agent');
  });
});

async function askForAnnabel() {
  await type($('input[name="given_name"]'), 'Annabel');
  await type($('input[name="family_name"]'), 'Example');
  await type($('input[name="email"]'), 'a@example.test');
  const form = $('form[aria-label="Add a person"]');
  if (!form) throw new Error('the add person form');
  await act(async () => { form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })); });
}

describe('Admission names the record it opens (DIRECTORY-121)', () => {
  it('says the email already signs in as the held person and links to their file', async () => {
    await mount('#/people/new', { ...SERVICE, 'POST /people/admit': { status: 409, body: {
      refusal: 'EmailHeldByAnother', reason: 'The email is already held',
      person: BEA, display_name: 'Dean Whiting', email: 'a@example.test',
    } } });
    await askForAnnabel();
    expect(text()).toContain('a@example.test already signs in as Dean Whiting');
    expect($('[role="alert"] a')?.getAttribute('href')).toBe('#/file/' + BEA);
    expect(text()).not.toContain('Annabel Example is added');
  });

  it('says a reuse was already the stored name and names that record beside the link', async () => {
    await mount('#/people/new', { ...SERVICE, 'POST /people/admit': (body) => ok({
      person: BEA, display_name: 'Dean Whiting', subject: 'dean', grant: null,
      letter: { letter: 'added', to: 'a@example.test', sent: false, by: 'lys', handed: 'https://lys.test/#/added/proof' },
      receipt: { operation: (body as { operation: string }).operation, person: BEA,
        existing: { person: BEA, display_name: 'Dean Whiting' }, failed: null,
        completed: ['register', 'account', 'bind', 'activate', 'letter'], logged: [] },
    }) });
    await askForAnnabel();
    const receipt = $('[aria-label="Receipt"]');
    expect(receipt?.textContent).toContain('a@example.test was already Dean Whiting; Dean Whiting now holds');
    expect(receipt?.textContent).toContain('This link signs in as Dean Whiting');
    expect(receipt?.textContent).not.toContain('is added');
    expect(receipt?.textContent).not.toContain('They have no access yet');
    expect($('a[href="#/file/' + BEA + '"]')).not.toBeNull();
  });

  it('names a newly admitted person beside the handed link', async () => {
    await mount('#/people/new', { ...SERVICE, 'POST /people/admit': (body) => ok({
      person: BEA, display_name: 'Annabel Example', subject: 'annabel', grant: null,
      letter: { letter: 'added', sent: false, by: 'lys', handed: 'https://lys.test/#/added/proof' },
      receipt: { operation: (body as { operation: string }).operation, person: BEA, failed: null,
        completed: ['register', 'account', 'bind', 'activate', 'letter'], logged: [] },
    }) });
    await askForAnnabel();
    expect(text()).toContain('Annabel Example is added');
    expect(text()).toContain('This link signs in as Annabel Example');
  });
});
