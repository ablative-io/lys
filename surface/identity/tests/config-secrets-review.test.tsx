// The critical review of 15 October 2026 on Configuration, Apps, Secrets, You and the refusal page: every screen says
// what happened in plain words, its code only under Error details; a dangerous act asks first; a greyed act says why;
// a field is labelled and wide enough for what it holds. Each case is a defect a person saw before the fix.
import { act } from 'react';
import { createRoot } from 'react-dom/client';
import type { Root } from 'react-dom/client';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { Refused } from '../src/api';
import { AuditRows, outcomeWords, RevocationView } from '../src/features/secrets/SecretsDetail';
import { SecretProblem } from '../src/features/secrets/SecretProblem';
import { additionWords } from '../src/features/secrets/AddSecretRow';
import { replaceKey } from '../src/features/secrets/ValueChange';
import { $, $$, click, mount, settle, text, type } from './harness';
import { ADA, ME, SERVICE, ok, refused } from './fixtures';

const roots: Root[] = [];
afterEach(() => {
  for (const root of roots.splice(0)) act(() => root.unmount());
  document.body.innerHTML = '';
  sessionStorage.clear();
  vi.unstubAllGlobals();
});

async function show(node: React.ReactNode): Promise<HTMLElement> {
  const container = document.createElement('div');
  document.body.appendChild(container);
  const root = createRoot(container);
  roots.push(root);
  await act(async () => { root.render(node); });
  await settle();
  return container;
}

const named = (label: string) => $$('button').find((entry) => entry.getAttribute('aria-label') === label) ?? null;

describe('The refusal page', () => {
  it('says what happened in words and keeps the code under Error details, with a way back', async () => {
    await mount('#/settings/storage', { ...SERVICE, '/configuration': refused(403, 'NotAdmitted', 'Administrator only') });
    expect($('h1')?.textContent).toBe('You do not have access to this');
    expect($('h1')?.textContent).not.toBe('Refused');
    const fold = $('.why-not details .refusal-name');
    expect(fold?.textContent).toContain('NotAdmitted');
    // The code is said nowhere outside the fold.
    expect($('.why-not > p')?.textContent).not.toContain('NotAdmitted');
    expect($$('.page a.btn').some((link) => link.getAttribute('href') === '#/')).toBe(true);
  });
});

describe('Configuration', () => {
  const settings = { source: 'startup_configuration', mutable_in_browser: false, sign_in: { provider_origin: 'https://login.test', session_seconds: 3600, secure_cookie: true }, directory: { roles_configured: true }, permissions: { model_version: 8, projection: 'local' }, secrets: { configured: false }, runtimes: { machines_configured: true, provisioning_configured: true }, storage: { directory_format: 'signed_leaf_log', grant_format: 'signed_leaf_log', requests_configured: true } };
  it('names the running build’s refusal in words, never as its code', async () => {
    await mount('#/settings', { ...SERVICE, '/configuration': ok(settings), '/authority': refused(503, 'AuthorityUnavailable', 'the build could not be read') });
    const build = $$('dt').find((dt) => dt.textContent === 'Running build')?.nextElementSibling;
    expect(build?.querySelector('.why-not > p')?.textContent).not.toContain('AuthorityUnavailable');
    expect(build?.querySelector('details .refusal-name')?.textContent).toContain('AuthorityUnavailable');
    expect(text()).toContain('Secret storage: Not set up');
    expect(text()).toContain('Inside Lys');
    expect(text()).not.toContain('It stands on its own');
  });
  it('says why a model cannot be added twice', async () => {
    const organisation = { zone: 'Australia/Sydney', version: 3, model_windows: { 'gpt-6': 400000 } };
    await mount('#/settings', { ...SERVICE, '/configuration': ok({ ...settings, organisation, models_undeclared: [] }) });
    await type($('input[aria-label="Another model\'s name"]'), 'gpt-6');
    await settle();
    expect($('[data-act="add-model"]')?.hasAttribute('disabled')).toBe(true);
    expect($('tr.add')?.textContent).toContain('gpt-6 is in the table already.');
  });
});

describe('Apps', () => {
  const by = { kind: 'start' };
  const base = { redirects: ['https://notes.example.test/back'], sign_in: null, schema: { kinds: {} }, version: 1, versions: [1], pending: null, client_id: null, service_account: null, client_credentials: [], registered_by: by, registered_at: 1 };
  it('says an app’s state in words, not its code', async () => {
    await mount('#/apps', { ...SERVICE, '/apps': () => ok({ apps: [{ ...base, id: 'cambium', name: 'Cambium', state: 'approved' }] }) });
    const card = $('[aria-label="App cambium"] .app-state');
    expect(card?.textContent).toBe('Approved');
  });
  it('asks before retiring an app, with a reason and a Cancel that sends nothing', async () => {
    const app = { ...base, id: 'fixture_notes', name: 'Notes', state: 'approved' };
    const { posted } = await mount('#/apps', { ...SERVICE, '/apps': () => ok({ apps: [app] }), 'POST /apps/fixture_notes/retire': ok({ ...app, state: 'retired' }) });
    await click(named('Retire Notes'));
    expect(posted).toEqual([]);
    const ask = $('[role="group"][aria-label="Retire Notes?"]');
    expect(ask?.textContent).toContain('This cannot be undone.');
    await click(named('Keep Notes'));
    expect($('[role="group"][aria-label="Retire Notes?"]')).toBeNull();
    expect(posted).toEqual([]);
    await click(named('Retire Notes'));
    await type($('[aria-label="Retire Notes?"] input'), 'Replaced by Cambium notes');
    await click(named('Retire Notes for good'));
    await settle();
    expect(posted).toMatchObject([{ path: '/apps/fixture_notes/retire', body: { reason: 'Replaced by Cambium notes' } }]);
  });
  it('gives a waiting permission change on an approved app its own reason to decline', async () => {
    const app = { ...base, id: 'fixture_notes', name: 'Notes', state: 'approved', pending: { operation: 'op-x', replaces: 1, schema: { kinds: {} }, by, at: 2 } };
    const { posted } = await mount('#/apps', { ...SERVICE, '/apps': () => ok({ apps: [app] }), 'POST /apps/fixture_notes/schema/decline': ok({}) });
    const waiting = $('[aria-label="Change waiting for fixture_notes"]');
    const reason = waiting?.querySelector('input');
    expect(reason?.closest('label')?.textContent).toContain('Why, if you decline the change');
    await type(reason ?? null, 'Too wide');
    await click(named('Decline the change'));
    await settle();
    expect(posted).toMatchObject([{ path: '/apps/fixture_notes/schema/decline', body: { reason: 'Too wide' } }]);
  });
  it('labels the developer form’s fields and says why its template is not offered yet', async () => {
    await mount('#/apps', { ...SERVICE, '/apps': () => ok({ apps: [] }) });
    const form = $('[aria-label="Register an app"]');
    expect(form?.textContent).toContain('Give the app an id.');
    const id = [...(form?.querySelectorAll('label.field') ?? [])].find((label) => label.textContent?.startsWith('App id'))?.querySelector('input');
    expect(id?.placeholder).toBe('');
    await type(id ?? null, 'Bad Id');
    await settle();
    expect(form?.textContent).toContain('An id is 3 to 40 characters');
    await click(named('Add another address'));
    expect(form?.querySelectorAll('input[aria-label^="Return address"]')).toHaveLength(2);
    await click(named('Remove return address 2'));
    expect(form?.querySelectorAll('input[aria-label^="Return address"]')).toHaveLength(1);
  });
  it('gives a link back to Apps when a connect link cannot be read', async () => {
    await mount('#/apps/connect?manifest=nope', SERVICE);
    expect($('[role="alert"]')?.textContent).toContain('This link is incomplete');
    expect($$('a.btn').some((link) => link.getAttribute('href') === '#/apps')).toBe(true);
  });
});

describe('Secrets', () => {
  it('says each kind of secret in words and opens its controls by Manage', async () => {
    await mount('#/secrets', { ...SERVICE, '/secrets': ok({ secrets: [{ name: 'calendar', class: 'oauth', owner: ADA, sequence: 1, upstream: null, header: null }, { name: 'notes', class: 'memory', owner: ADA, sequence: 1, upstream: null, header: null }] }) });
    const kinds = $$('.listing [data-col="Kind"]').map((cell) => cell.textContent);
    expect(kinds).toEqual(expect.arrayContaining(['Sign-in token', 'Agent memory']));
    expect(kinds).not.toContain('oauth');
    expect($('.listing a[href*="?secret=calendar"]')?.getAttribute('aria-label')).toBe('Manage calendar');
  });
  it('labels every part of a new secret above its field, the address given a whole line', async () => {
    await mount('#/secrets', { ...SERVICE, '/secrets': ok({ secrets: [] }) });
    const form = $('form.add-secret');
    const labels = [...(form?.querySelectorAll('label') ?? [])].map((label) => label.textContent ?? '');
    for (const words of ['Name', 'Kind', 'Sent to', 'In the header', 'After the text', 'Value']) expect(labels.some((label) => label.startsWith(words))).toBe(true);
    expect(form?.querySelector('input[name="upstream"]')?.closest('.field')?.classList.contains('wide')).toBe(true);
    // No field is labelled by its placeholder alone.
    expect(form?.querySelector('input[name="name"]')?.getAttribute('placeholder')).toBeNull();
    expect(form?.closest('td')?.getAttribute('colspan')).toBe('5');
  });
  it('unlocks the add row from a kept addition it cannot read, rather than naming a storage key to remove', async () => {
    sessionStorage.setItem('lys.pending.secrets.add.' + ME.person.id, '{not json');
    await mount('#/secrets', { ...SERVICE, '/secrets': ok({ secrets: [] }) });
    const form = $('form.add-secret');
    expect(form?.textContent).not.toContain('lys.pending');
    expect(form?.querySelector<HTMLButtonElement>('button[type="submit"]')?.disabled).toBe(true);
    await click(named('Check and clear the saved request'));
    expect(sessionStorage.getItem('lys.pending.secrets.add.' + ME.person.id)).toBeNull();
    expect($('form.add-secret button[type="submit"]')?.hasAttribute('disabled')).toBe(false);
  });
  it('says an unknown refusal of an addition without its code', () => {
    const words = additionWords(new Refused(409, { refusal: 'SomethingOdd', reason: 'the broker said no' }), 'calendar');
    expect(words).not.toContain('SomethingOdd');
    expect(words).toContain('the broker said no');
    expect(additionWords(new Refused(409, { refusal: 'SecretExists', reason: '' }), 'calendar')).toContain('A secret named calendar exists already');
  });
  it('says an unconfirmed answer as what it means for the change, the code folded away', async () => {
    const view = await show(<SecretProblem problem={new Refused(200, { refusal: 'UnconfirmedAnswer', reason: 'The service answered, but not with calendar retired.' })} />);
    expect(view.querySelector('.why-not > p')?.textContent).toBe('Lys answered, but its answer did not confirm this change. Check whether it was made before trying again.');
    expect(view.querySelector('details .refusal-name')?.textContent).toContain('UnconfirmedAnswer');
  });
  it('folds a refused revocation check’s code', async () => {
    const view = await show(<RevocationView outcome={{ at: 'refused', refused: new Refused(503, { refusal: 'SecretsUnavailable', reason: 'down' }) }} />);
    expect(view.querySelector('.why-not > p')?.textContent).not.toContain('SecretsUnavailable');
    expect(view.querySelector('details .refusal-name')?.textContent).toContain('SecretsUnavailable');
  });
  it('says what came of each audit line in words', async () => {
    expect(outcomeWords('admitted')).toBe('Allowed');
    expect(outcomeWords('dropped')).toBe('Ended');
    expect(outcomeWords('SomethingUnnamed')).toBe('Refused');
    const view = await show(<AuditRows log={{ verified_by: 'k', lines: [{ index: 0, kind: 'use', at_ms: 1_760_000_000_000, handle: null, identity: null, secret: 'calendar', operation: null, uses: 1, outcome: 'admitted' }] }} />);
    expect(view.querySelector('tbody')?.textContent).toContain('Allowed');
    expect(view.querySelector('tbody')?.textContent).not.toContain('admitted');
  });
  it('names the person a secret is kept for, never their id', async () => {
    await mount('#/secrets/entries?secret=calendar', { ...SERVICE, '/secrets': ok({ secrets: [{ name: 'calendar', class: 'credential', owner: ADA, sequence: 1, upstream: null, header: null }] }), '/secrets/settings?secret=calendar': ok({ secret: 'calendar', scope: 'person/' + ADA, recipients: 'anyone', last_operation: null }) });
    const current = $('[aria-label="Current secret settings"]');
    expect(current?.textContent).toContain('Visible to: the person');
    expect(current?.textContent).not.toContain(ADA);
    expect(current?.querySelector('a[href="#/file/' + ADA + '"]')).not.toBeNull();
  });
  it('unlocks a value change kept in a shape it cannot read', async () => {
    sessionStorage.setItem(replaceKey('calendar'), '{not json');
    await mount('#/secrets/entries?secret=calendar', { ...SERVICE, '/secrets': ok({ secrets: [{ name: 'calendar', class: 'credential', owner: ADA, sequence: 1, upstream: null, header: null }] }), '/secrets/settings?secret=calendar': ok({ secret: 'calendar', scope: null, recipients: 'anyone', last_operation: null }) });
    const form = $('form[aria-label="Change the value of calendar"]');
    expect(form?.textContent).not.toContain('lys.pending');
    expect(form?.textContent).not.toContain('PendingSecretUnreadable');
    await click(form?.querySelector('button[aria-label="Check and clear the saved request"]') ?? null);
    expect(sessionStorage.getItem(replaceKey('calendar'))).toBeNull();
    expect($('form[aria-label="Change the value of calendar"] input[type="password"]')?.hasAttribute('disabled')).toBe(false);
    expect($('[aria-label="Current secret settings"]')?.textContent).toContain('Visible to: not set yet');
  });
});

describe('You', () => {
  it('says a service account’s state in words, never its code', async () => {
    await mount('#/me?tab=account', { ...SERVICE, '/me': ok({ ...ME, service_accounts: [{ id: 'service-test', owner: ADA, name: 'Invoices', description: '', state: 'suspended', created_by: ME.signed_in, created_at: 1790000000, retired_by: null, retired_at: null }] }) });
    const accounts = $('#service-accounts');
    expect(accounts?.textContent).toContain('Paused');
    expect(accounts?.textContent).not.toContain('suspended');
  });
});
