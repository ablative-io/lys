/**
 * The Access, Roles and Service accounts screens as a first-time person meets them (walk of 15 Oct 2026): every
 * state in words, never the service's own names or identifiers; every greyed act with its reason beside it; every
 * open form with a way out; and nothing offered before the person has chosen what it is for.
 */
import { beforeEach, describe, expect, it } from 'vitest';
import { $, $$, choose, click, mount, text } from './harness';
import { ADA, DIRECTORY, ME, ROOT_G, SCRIBE, SERVICE, ok } from './fixtures';
import type { Role, RoleHolder, RoleVersion } from '../src/features/roles/contract';

beforeEach(() => sessionStorage.clear());
const button = (label: string) => $$('button').find((each) => each.getAttribute('aria-label') === label) ?? null;

describe('Requests', () => {
  const model = { action_sentences: { read: 'Read this resource', write: 'Write to this resource' }, version: 1, relations: { reader: ['read'], editor: ['read', 'write'] }, kinds: {} };
  const SOURCE = 'grant-' + '6'.repeat(32);
  const asked = (n: string, more: Record<string, unknown> = {}) => ({
    id: 'op-' + n.repeat(32), asked_by: SCRIBE, asked_by_name: 'Scribe', responsible: ME.person, resource: { kind: 'project', id: 'Lys' }, relation: 'reader', actions: ['read'],
    ends_at: null, why: 'Read the release notes', asked_at: 1790000000, state: 'waiting', approvers: [ME.person], sources: [SOURCE], can_decide: true, held_by: null, decision: null, ...more,
  });
  const routes = { ...SERVICE, '/grants/model': ok(model),
    '/grants': ok({ revision: 1, grants: [{ id: SOURCE, holder: ADA, relation: 'editor', resource: { kind: 'project', id: 'Lys' }, actions: ['read', 'write'] }] }),
    '/requests': ok({ requests: [asked('1'), asked('2', { state: 'approved', decision: { by: ADA, note: 'Fine for the release', grant: 'grant-' + '7'.repeat(32), decided_at: 1790050000 } })] }) };

  it('names the access an approval draws on by what it is on, never by its identifier', async () => {
    await mount('#/requests', routes);
    await click(button('Approve access'));
    const options = $$('select[name="source"] option').map((option) => option.textContent);
    expect(options).toContain('Your editor access to project Lys');
    expect(text()).not.toContain(SOURCE);
    expect(text()).not.toContain('Permission 1');
  });

  it('says a decided request in words and who decided it', async () => {
    await mount('#/requests', routes);
    await click($$('.seg button').find((each) => each.textContent === 'Decided') ?? null);
    expect($$('tbody tr').map((row) => row.textContent).join(' ')).toContain('Approved');
    expect($$('tbody tr').map((row) => row.textContent).join(' ')).not.toMatch(/\bapproved\b/);
    expect($('article[aria-label="Request"]')?.textContent).toContain('Approved by Ada (test person)');
    expect($('article[aria-label="Request"]')?.textContent).toContain('Fine for the release');
  });

  it('offers no actions until something is chosen, and Cancel closes the ask form', async () => {
    await mount('#/requests', routes);
    await click(button('Ask for access'));
    const form = $('form[aria-label="Ask for access"]');
    expect(form?.querySelector('input[name="action"]')).toBeNull();
    await choose(form?.querySelector('select') ?? null, '0');
    expect(form?.querySelector('input[name="action"][value="read"]')).not.toBeNull();
    expect(form?.querySelector('label.tick input[name="no-expiry"]')).not.toBeNull();
    await click(button('Cancel asking'));
    expect($('form[aria-label="Ask for access"]')).toBeNull();
    expect($('article[aria-label="Request"]')).not.toBeNull();
  });

  it('asks the type and name of something not in the list with labels and hints, never "kind and ID"', async () => {
    await mount('#/requests', routes);
    await click(button('Ask for access'));
    await click($('input[name="another"]'));
    expect(text()).toContain('What I need is not in the list');
    expect(text()).not.toContain('kind and ID');
    expect($('input[name="kind"]')?.closest('label')?.textContent).toContain('Type of thing');
    expect($('input[name="resource"]')?.closest('label')?.textContent).toContain('Its name');
  });
});

describe('Service accounts', () => {
  const account = (n: string, more: Record<string, unknown> = {}) => ({ id: 'op-' + n.repeat(32), owner: ADA, name: 'Invoice processing', description: 'Files invoices', state: 'active', created_by: ME.signed_in, created_at: 1790000000, retired_by: null, retired_at: null, ...more });
  const routes = { ...SERVICE, '/service-accounts': ok({ scope: 'personal', service_accounts: [account('a'), account('b', { name: 'Old mailer', state: 'retired', retired_at: 1790000100 })] }) };

  it('says each state in words and asks a retirement above the list, with Cancel', async () => {
    await mount('#/service-accounts', routes);
    const states = $$('tbody tr').map((row) => row.textContent ?? '');
    expect(states.join(' ')).toContain('Registered');
    expect(states.join(' ')).toContain('Retired');
    expect(states.join(' ')).not.toMatch(/\bretired\b/);
    await click(button('Retire this service account'));
    const card = $('section.card[aria-label="Confirm service account retirement"]');
    expect(card?.closest('td')).toBeNull();
    expect(card?.textContent).toContain('Retire Invoice processing?');
    await click(button('Cancel'));
    expect($('section[aria-label="Confirm service account retirement"]')).toBeNull();
  });

  it('describes an account in a text area, says why Register is greyed, and can be cancelled', async () => {
    await mount('#/service-accounts', routes);
    await click(button('Register an account'));
    const form = $('form[aria-label="Register service account"]');
    expect(form?.querySelector('textarea[name="description"]')).not.toBeNull();
    expect(form?.querySelector('input[name="name"]')?.getAttribute('placeholder')).toBeNull();
    expect(form?.textContent).toContain('Give the account a name to register it.');
    await click(button('Cancel registering'));
    expect($('form[aria-label="Register service account"]')).toBeNull();
  });
});

describe('Drafts', () => {
  const DRAFT = 'op-' + '7'.repeat(32);
  const draft = { id: DRAFT, agent: { id: SCRIBE, display_name: 'Scribe' }, responsible: { ...ME.person }, target: { kind: 'project', id: 'identity', action: 'edit' },
    method: 'POST', path: '/grants', body: '{}', note: 'Fix a typo', created_at: 1790000000, creation_hash: 'ab'.repeat(32), state: 'waiting' };

  it('labels the reason for a refusal above its box, not as a placeholder', async () => {
    await mount('#/access/drafts', { ...SERVICE, '/drafts?state=waiting': ok({ drafts: [draft] }) });
    await click($('[data-act="refuse"]'));
    const input = $('input[aria-label="Why you refuse it"]');
    expect(input?.getAttribute('placeholder')).toBeNull();
    expect(input?.closest('label')?.textContent).toContain('Why you refuse it');
  });
});

describe('Roles', () => {
  const version: RoleVersion = { responsibilities: 'Review changes', goals: 'Correct releases', practice: 'Check receipts', profile: '', grant_templates: [], note: 'First', number: 1, made_by: ADA, made_at: 1790000000 };
  const holder: RoleHolder = { assignment: 'op-' + 'b'.repeat(32), holder: SCRIBE, display_name: 'Scribe', version: 1, behind: true, assigned_by: ADA, assigned_at: 1790000001, ends_at: null, moves_at: null, state: 'ended', moves: [], ended_by: ADA, ended_at: 1790000099 };
  const role: Role = { id: 'op-' + 'a'.repeat(32), name: 'Reviewer', latest: 2, policy: 'stays_until_moved', versions: [version, { ...version, number: 2 }], holders: [holder] };
  const routes = { ...SERVICE, '/roles': ok({ roles: [role] }) };

  it('says an ended assignment in words, without offering a newer version to it', async () => {
    await mount('#/roles/' + role.id, routes);
    const row = $('tr[aria-label="Holder Scribe"]');
    expect(row?.textContent).toContain('Removed');
    expect(row?.textContent).not.toContain('A newer version is available');
    expect(row?.textContent).not.toContain('ended');
  });

  it('assigns through labelled fields under the table, and the editor has Cancel', async () => {
    await mount('#/roles/' + role.id, routes);
    const form = $('form[aria-label="Assign job"]');
    expect(form?.closest('table')).toBeNull();
    expect(form?.textContent).toContain('Person or agent');
    expect(form?.textContent).toContain('Until (your local time)');
    await click(button('New version'));
    expect($('form[aria-label="Publish job version"]')).not.toBeNull();
    await click(button('Cancel editing'));
    expect($('form[aria-label="Publish job version"]')).toBeNull();
  });
});

describe('Reviews', () => {
  it('tells the person what to do about an agent whose owner is not active, in words', async () => {
    await mount('#/reviews', { ...SERVICE, '/reviews': ok({ scope: 'directory', due: [], revision: 1, judged_at: 1790000000, decisions_recorded: true,
      unanswered: [{ agent: { id: SCRIBE, display_name: 'Scribe', state: 'active' }, person: { id: 'person-' + '2'.repeat(32), display_name: 'Bea', state: 'registered' } }] }) });
    const card = $('section[aria-label="Agents needing an active owner"]');
    expect(card?.textContent).toContain('Open each agent to change who it answers to');
    expect(card?.textContent).toContain('whose account is not yet activated');
    expect(card?.textContent).not.toContain('registered');
  });
});

describe('Give part of a grant', () => {
  it('says why Give is greyed, and says so when there is no one to give to', async () => {
    await mount('#/me?tab=account');
    await click($(`[data-act="delegate"][data-g="${ROOT_G}"]`));
    expect(($('[data-act="delegatedo"]') as HTMLButtonElement).disabled).toBe(true);
    expect($('[data-why-not]')?.textContent).toBe('Tick at least one thing they may do.');
    expect($$('#dTo optgroup').map((group) => group.getAttribute('label'))).toEqual(['Your agents']);
  });

  it('names no recipient as a next step rather than an empty choice', async () => {
    const alone = { scope: 'directory' as const, people: [{ ...DIRECTORY.people[0], agents: [] }] };
    // The Access page offers Give on every grant you may pass on, even with no agent of your own yet.
    await mount('#/access', { ...SERVICE, '/directory/people': ok(alone), '/people': ok(alone) });
    await click($(`[data-act="delegate"][data-g="${ROOT_G}"]`));
    expect($('#dTo')).toBeNull();
    expect($('[data-no-recipient]')?.textContent).toContain('Add an agent');
    expect($('[data-why-not]')?.textContent).toBe('Choose who to give it to.');
  });
});

