/**
 * ACCESS-001 R4: a grant's mode in words wherever a grant is shown, the issuer
 * choosing it, the kind of thing picked from the app schemas, and the root-grant
 * form offered only to the root authority.
 */
import { act } from 'react';
import { beforeEach, describe, expect, it } from 'vitest';
import { $, $$, choose, mount, pick, settle, text, click } from './harness';
import { ADA, FOUNDING_G, GRANTS, RECEIPTS, SCRIBE, SCRIBE_G, SERVICE, ok, refused } from './fixtures';
import type { Route } from './fixtures';

beforeEach(() => sessionStorage.clear());

/** Ada is not the administrator and holds nothing on her organisation: the wider directory is refused her, her own scope answered, and her founding grant is not hers (ACCESS-009 R4). */
const PERSONAL: Record<string, Route> = {
  ...SERVICE,
  '/directory/people': refused(403, 'NotAdmitted', 'the wider directory is for its administrators'),
  '/grants': ok({ grants: GRANTS.filter((grant) => grant.id !== FOUNDING_G), revision: 7 }),
};

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
  const value = $('form[aria-label="Give access"]');
  if (!value) throw new Error('No Give access form');
  return value;
}

async function submit(page: Element) {
  await act(async () => { page.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })); });
  await settle();
}

async function fillIn(page: Element, kind: string, id: string, mode: string) {
  await pick(page, 'Find a person', 'Ada', 'Ada (test person)');
  await choose(page.querySelector('select[name="kind"]'), kind);
  await click(page.querySelector('.level-picker input[type="radio"][value="viewer"]'));
  await choose(page.querySelector('select[name="resource"]'), '\u0000by-id');
  const input = page.querySelector<HTMLInputElement>('input[name="resource_id"]');
  if (!input) throw new Error('No resource id input');
  input.value = id;
  await choose(page.querySelector('select[name="mode"]'), mode);
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
    expect($$('select[name="kind"] optgroup').map((group) => group.getAttribute('label'))).toEqual(['Notes', 'Lys', 'Already granted in Lys']);
    // Each kind is named in words; its value is the kind the grant is sent with.
    expect($$('select[name="kind"] optgroup[label="Notes"] option').map((option) => option.textContent)).toEqual(['Doc', 'Folder']);
    expect($$('select[name="kind"] optgroup[label="Notes"] option').map((option) => option.getAttribute('value'))).toEqual(['notes.doc', 'notes.folder']);
    expect($$('select[name="kind"] optgroup[label="Lys"] option').map((option) => option.textContent)).toEqual(['Organisation']);
    expect($$('select[name="kind"] optgroup[label="Already granted in Lys"] option').map((option) => option.textContent)).toEqual(['Project']);
  });

  it('offers the ids Lys already holds for the chosen kind', async () => {
    await mount('#/access/issue', apps);
    // Before a kind is chosen, nothing is chosen for which one, and no id box is drawn.
    expect(($('select[name="resource"]') as HTMLSelectElement | null)?.value).toBe('');
    expect(($('select[name="resource"]') as HTMLSelectElement | null)?.disabled).toBe(true);
    expect($('input[name="resource_id"]')).toBeNull();
    await choose($('select[name="kind"]'), 'project');
    expect($$('select[name="resource"] option').map((option) => option.getAttribute('value'))).toEqual(['', 'identity', 'ledger', '\u0000by-id']);
    expect($('input[name="resource_id"]')).toBeNull();
    await choose($('select[name="kind"]'), 'notes.doc');
    // None of this kind is known to Lys, so the only choice is naming one by its id.
    expect(($('select[name="resource"]') as HTMLSelectElement | null)?.value).toBe('\u0000by-id');
    expect($('input[name="resource_id"]')).not.toBeNull();
  });
});

describe('Which one names what the directory knows', () => {
  it('lists every agent of the directory by name for a directory kind, never a raw id, and still offers one by id', async () => {
    const onAgent = { ...GRANTS[0], id: 'grant-' + 'd'.repeat(32), resource: { kind: 'agent', id: SCRIBE } };
    await mount('#/access/issue', { ...SERVICE, '/grants': ok({ grants: [...GRANTS, onAgent], revision: 7 }) });
    await choose($('select[name="kind"]'), 'agent');
    const options = $$('select[name="resource"] option');
    expect(options.map((option) => option.textContent)).toContain('the agent Scribe');
    expect(options.some((option) => /agent-[0-9a-f]{8}/.test(option.textContent ?? ''))).toBe(false);
    expect(options.at(-1)?.getAttribute('value')).toBe('\u0000by-id');
    // Directory names are known, so no free id box is drawn until "by its id" is chosen.
    expect($('input[name="resource_id"]')).toBeNull();
  });
});

describe('Only the root authority is offered a root grant', () => {
  it('the root authority sees the Issue entry and its form', async () => {
    await mount('#/access');
    expect($('a[href="#/access/issue"]')?.textContent).toBe('Give access');
    location.hash = '#/access/issue';
    await settle();
    expect($('form[aria-label="Give access"]')).not.toBeNull();
  });

  it('a person who is not the root authority sees no Issue entry', async () => {
    await mount('#/access', PERSONAL);
    expect($('a[href="#/access/issue"]')).toBeNull();
    expect(text()).not.toContain('Issue root grant');
    expect($('.head a.btn.primary')).toBeNull();
  });

  it('a direct entry shows RootAuthorityRefused with its reason, and no form controls', async () => {
    const { posted } = await mount('#/access/issue', PERSONAL);
    expect($('form[aria-label="Give access"]')).toBeNull();
    const panel = $('.act-panel.issue-root');
    expect(panel?.querySelectorAll('input, select, textarea, button[type="submit"]')).toHaveLength(0);
    expect(panel?.querySelector('.refusal-name')?.textContent).toContain('RootAuthorityRefused');
    expect(panel?.textContent).toContain('you hold no organisation.grant on Example organisation');
    expect(posted).toHaveLength(0);
  });
});
