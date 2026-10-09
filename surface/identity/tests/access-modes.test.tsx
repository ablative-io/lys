/**
 * ACCESS-001 R4: a grant's mode in words wherever a grant is shown, the issuer
 * choosing it, the kind of thing picked from the app schemas, and the root-grant
 * form offered only to the root authority.
 */
import { act } from 'react';
import { beforeEach, describe, expect, it } from 'vitest';
import { $, $$, choose, click, mount, pick, settle, text } from './harness';
import { ADA, GRANTS, RECEIPTS, SCRIBE, SCRIBE_G, SERVICE, ok, refused } from './fixtures';
import type { Route } from './fixtures';

beforeEach(() => sessionStorage.clear());

/** Ada is not the administrator: the wider directory is refused her, and her own scope answered. */
const PERSONAL: Record<string, Route> = { ...SERVICE, '/directory/people': refused(403, 'NotAdmitted', 'the wider directory is for its administrators') };

/** Scribe's grant held by draft. */
const HELD = GRANTS.map((grant) => (grant.id === SCRIBE_G ? { ...grant, mode: 'by_draft' as const } : grant));

/** The reach answer with Scribe's view held by draft and Ada's actions outright. */
const heldReach = (body: unknown) => {
  const b = body as { resources: { kind: string; id: string; actions: string[] }[] };
  return ok({ revision: 7, resources: b.resources.map(({ kind, id, actions }) => ({ kind, id, holders: id === 'identity'
    ? [{ holder: ADA, actions, modes: actions.map(() => 'outright') }, { holder: SCRIBE, actions: ['view'], modes: ['by_draft'] }]
    : [{ holder: ADA, actions: ['view'], modes: ['outright'] }] })) });
};

const issued = (body: unknown) => {
  const operation = (body as { operation: string }).operation;
  const receipt = { ...RECEIPTS[4].receipt, operation, identity: ADA };
  return ok({ operation, grant: 'grant-recorded', index: receipt.log.index, receipt: { ...receipt, caller: ADA, revision: 1 } });
};

function form() {
  const value = $('form[aria-label="Issue root grant"]');
  if (!value) throw new Error('No Issue root grant form');
  return value;
}

async function submit(page: Element) {
  await act(async () => { page.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })); });
  await settle();
}

async function fillIn(page: Element, kind: string, id: string, mode: string) {
  await pick(page, 'Find a person', 'Ada', 'Ada (test person)');
  await choose(page.querySelector('select[name="relation"]'), 'viewer');
  await choose(page.querySelector('select[name="kind"]'), kind);
  const input = page.querySelector<HTMLInputElement>('input[name="resource"]');
  if (!input) throw new Error('No resource input');
  input.value = id;
  await choose(page.querySelector('select[name="mode"]'), mode);
  const checks = page.querySelectorAll('input[type="checkbox"]');
  await click(checks[1]);
}

describe('A grant shows its mode in words', () => {
  it('on every grant row, as the service answered it', async () => {
    await mount('#/access', { ...SERVICE, '/grants': ok({ grants: HELD, revision: 7 }) });
    const modes = $$('td[data-col="Mode"]').map((cell) => cell.textContent);
    expect(modes).toHaveLength(HELD.length);
    expect(modes).toContain('by draft');
    expect(modes.filter((mode) => mode === 'outright')).toHaveLength(HELD.length - 1);
  });

  it('on the who view, beside each holder', async () => {
    await mount('#/access/who/project:identity', { ...SERVICE, 'POST /grants/reach': heldReach });
    expect($$('#whoCan .row .mode').map((cell) => cell.textContent)).toEqual(['outright', 'by draft']);
  });

  it('on the reach view, beside each resource', async () => {
    await mount('#/access/reach/' + SCRIBE, { ...SERVICE, 'POST /grants/reach': heldReach });
    expect($$('.check .card tr .mode').map((cell) => cell.textContent)).toEqual(['by draft']);
  });

  it('refuses a reach answer that leaves out the modes, rather than calling them outright', async () => {
    const bare = (body: unknown) => ok({ revision: 7, resources: (body as { resources: { kind: string; id: string }[] }).resources
      .map(({ kind, id }) => ({ kind, id, holders: [{ holder: ADA, actions: ['view'] }] })) });
    await mount('#/access/reach/' + ADA, { ...SERVICE, 'POST /grants/reach': bare });
    expect(text()).toContain('PermissionAnswerIncomplete');
    expect($$('.check .card tr .mode')).toHaveLength(0);
  });
});

describe('The issuer chooses the mode', () => {
  it('sends the chosen mode on POST /grants/roots', async () => {
    const { posted } = await mount('#/access/issue', { ...SERVICE, 'POST /grants/roots': issued });
    const page = form();
    expect($$('select[name="mode"] option').map((option) => option.textContent)).toEqual(['outright', 'by draft', 'by two approvals']);
    await fillIn(page, 'project', 'integration-check', 'by_draft');
    await submit(page);
    expect(posted.at(-1)).toMatchObject({ path: '/grants/roots', body: { resource: { kind: 'project', id: 'integration-check' }, mode: 'by_draft' } });
  });

  it('shows the refusal of a held mode on a hot action by name and reason', async () => {
    const reason = 'grant_mode_on_hot_action: notes.doc.write is hot in app notes, so it cannot be held by_two';
    await mount('#/access/issue', { ...SERVICE, 'POST /grants/roots': refused(403, 'grant_mode_on_hot_action', reason) });
    const page = form();
    await fillIn(page, 'project', 'integration-check', 'by_two');
    await submit(page);
    expect(text()).toContain('grant_mode_on_hot_action');
    expect(text()).toContain('is hot in app notes');
  });
});

describe('The kind of thing is picked from the app schemas', () => {
  const apps: Record<string, Route> = {
    ...SERVICE,
    '/apps': ok({ apps: [{ id: 'notes', name: 'Notes', state: 'approved' }, { id: 'drafty', name: 'Drafty', state: 'pending' }] }),
    '/apps/notes/schema': ok({ app: 'notes', version: 3, schema: { kinds: { 'notes.doc': {}, 'notes.folder': {} } } }),
  };

  it('offers each approved app\'s kinds under its name, then the kinds Lys already holds, and no free-text kind', async () => {
    const { requests } = await mount('#/access/issue', apps);
    expect(requests).toContain('/apps/notes/schema');
    expect(requests).not.toContain('/apps/drafty/schema');
    expect($('input[name="kind"]')).toBeNull();
    expect($$('select[name="kind"] optgroup').map((group) => group.getAttribute('label'))).toEqual(['Notes', 'Already granted in Lys']);
    expect($$('select[name="kind"] optgroup[label="Notes"] option').map((option) => option.textContent)).toEqual(['notes.doc', 'notes.folder']);
    expect($$('select[name="kind"] optgroup[label="Already granted in Lys"] option').map((option) => option.textContent)).toEqual(['project']);
  });

  it('offers the ids Lys already holds for the chosen kind', async () => {
    await mount('#/access/issue', apps);
    await choose($('select[name="kind"]'), 'project');
    expect($$('#root-ids option').map((option) => option.getAttribute('value')).sort()).toEqual(['identity', 'ledger']);
    await choose($('select[name="kind"]'), 'notes.doc');
    expect($$('#root-ids option')).toHaveLength(0);
  });
});

describe('Only the root authority is offered a root grant', () => {
  it('the root authority sees the Issue entry and its form', async () => {
    await mount('#/access');
    expect($('a[href="#/access/issue"]')?.textContent).toBe('Issue root grant');
    location.hash = '#/access/issue';
    await settle();
    expect($('form[aria-label="Issue root grant"]')).not.toBeNull();
  });

  it('a person who is not the root authority sees no Issue entry', async () => {
    await mount('#/access', PERSONAL);
    expect($('a[href="#/access/issue"]')).toBeNull();
    expect(text()).not.toContain('Issue root grant');
  });

  it('a direct entry shows RootAuthorityRefused with its reason, and no form controls', async () => {
    const { posted } = await mount('#/access/issue', PERSONAL);
    expect($('form[aria-label="Issue root grant"]')).toBeNull();
    const panel = $('.act-panel.issue-root');
    expect(panel?.querySelectorAll('input, select, textarea, button[type="submit"]')).toHaveLength(0);
    expect(panel?.querySelector('.refusal-name')?.textContent).toContain('RootAuthorityRefused');
    expect(panel?.textContent).toContain('only it issues a root grant');
    expect(posted).toHaveLength(0);
  });
});
