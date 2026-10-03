/** Roles keep assignments on their version and ask for deliberate, fenced changes. */
import { act } from 'react';
import { beforeEach, describe, expect, it } from 'vitest';
import { $, click, mount, settle, text, unmountAll } from './harness';
import { ADA, OWN, SCRIBE, SERVICE, ok, refused } from './fixtures';
import type { Role, RoleHolder, RoleVersion, RoleWords } from '../src/features/roles/contract';

beforeEach(() => sessionStorage.clear());
const words: RoleWords = { responsibilities: 'Review changes', goals: 'Correct releases', practice: 'Check receipts', profile: 'Reviewer', grant_templates: [], note: 'Initial version' };
const version: RoleVersion = { ...words, number: 1, made_by: ADA, made_at: 1790000000 };
const holder: RoleHolder = { assignment: 'op-' + 'b'.repeat(32), holder: SCRIBE, display_name: 'Scribe', version: 1, behind: true, assigned_by: ADA, assigned_at: 1790000001, ends_at: 1990000000, moves_at: null, state: 'holding', moves: [], ended_by: null, ended_at: null };
const role: Role = { id: 'op-' + 'a'.repeat(32), name: 'Reviewer', latest: 2, policy: 'stays_until_moved', versions: [version, { ...version, number: 2, responsibilities: 'Review code and docs', note: 'Include docs' }], holders: [holder] };
const routes = { ...SERVICE, '/roles': ok({ roles: [role] }) };
const button = (label: string) => [...document.querySelectorAll('button')].find((value) => value.textContent === label) ?? null;
function input(name: string, value: string) {
  const element = $('[name="' + name + '"]');
  if (!(element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement)) throw new Error('Missing ' + name);
  element.value = value;
}
async function submit(label: string) {
  const form = $('form[aria-label="' + label + '"]'); if (!form) throw new Error('Missing ' + label);
  await act(async () => { form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })); }); await settle();
}
function fillRole() {
  input('name', 'Release reviewer'); input('responsibilities', words.responsibilities); input('goals', words.goals); input('practice', words.practice); input('profile', ''); input('note', 'Create reviewer');
}
const created = (body: unknown) => {
  const value = body as RoleWords & { operation: string; name: string };
  return { ...role, id: value.operation, name: value.name, latest: 1, versions: [{ ...value, number: 1, made_by: ADA, made_at: 1790000000 }], holders: [] };
};

describe('Roles', () => {
  it('shows actual versions, holder state and expiry without changing any holder', async () => {
    const { posted } = await mount('#/roles/' + role.id, routes);
    expect(text()).toContain('Review code and docs'); expect(text()).toContain('Scribe');
    expect([...document.querySelectorAll('tr[aria-label="Holder Scribe"] td')].slice(1, 3).map((cell) => cell.textContent)).toEqual(['Version 1A newer version is available.', 'holding']); expect(text()).toContain('never extends its expiry');
    expect(posted).toEqual([]); expect(text()).not.toContain('not built yet');
  });
  it('creates a role with complete explicit words and no implicit access grants', async () => {
    const { posted } = await mount('#/roles', { ...routes, 'POST /roles': (body) => ok(created(body)) });
    await click(button('Create a role')); fillRole(); await submit('Create role');
    expect(posted).toHaveLength(1);
    expect(posted[0]).toMatchObject({ path: '/roles', body: { name: 'Release reviewer', responsibilities: words.responsibilities, goals: words.goals, practice: words.practice, profile: '', grant_templates: [], note: 'Create reviewer' } });
    expect(text()).toContain('Lys saved the role change.');
  });
  it('keeps an unknown creation under the exact operation across remount', async () => {
    const first = await mount('#/roles', { ...routes, 'POST /roles': refused(503, 'RolesUnavailable', 'write outcome unknown') });
    await click(button('Create a role')); fillRole(); await submit('Create role');
    unmountAll(); document.body.innerHTML = '';
    const second = await mount('#/roles', { ...routes, 'POST /roles': (body) => ok(created(body)) });
    await click(button('Create a role')); await click(button('Check whether Lys saved it'));
    expect(second.posted).toEqual(first.posted); expect(text()).toContain('Lys saved the role change.');
  });
  it('publishes a new version without changing existing holders', async () => {
    const path = '/roles/' + role.id + '/versions';
    const { posted } = await mount('#/roles/' + role.id, { ...routes, ['POST ' + path]: (body) => ok({ ...role, latest: 3, versions: [...role.versions, { ...(body as RoleWords), number: 3, made_by: ADA, made_at: 1790000002 }] }) });
    await click(button('New version')); input('responsibilities', 'Review signed releases'); input('note', 'Add signatures'); await submit('Publish role version');
    expect(posted).toHaveLength(1); expect(posted[0].path).toBe(path);
    expect(text()).toContain('Lys saved the role change.');
  });
  it('assigns an identity with explicit expiry and checks the returned assignment operation', async () => {
    const path = '/roles/' + role.id + '/holders';
    const { posted } = await mount('#/roles/' + role.id, { ...routes, ['POST ' + path]: (body) => {
      const value = body as { operation: string; holder: string; ends_at: number | null };
      return ok({ ...role, holders: [{ ...holder, assignment: value.operation, holder: value.holder, ends_at: value.ends_at }] });
    } });
    const find = $('input[aria-label="Find a person or agent"]') as HTMLInputElement;
    await act(async () => { Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')?.set?.call(find, 'Scri'); find.dispatchEvent(new Event('input', { bubbles: true })); });
    await click(button('Scribe')); await click($('form[aria-label="Assign role"] input[type="checkbox"]')); await submit('Assign role');
    expect(posted[0]).toMatchObject({ path, body: { holder: SCRIBE, ends_at: null } }); expect(text()).toContain('Lys saved the role change.');
  });
  it('reviews the change and fences a move to the exact assignment and previous version', async () => {
    const path = '/roles/' + role.id + '/holders/' + SCRIBE + '/move';
    const { posted } = await mount('#/roles/' + role.id, { ...routes, ['POST ' + path]: ok({ role: role.id, holder: { ...holder, version: 2 }, from: version, to: role.versions[1] }) });
    await click(button('Review changes in version 2')); expect(posted).toEqual([]);
    expect(text()).toContain('Review changes'); expect(text()).toContain('Review code and docs'); expect(text()).toContain('Existing grants are not changed');
    await click(button('Apply version 2'));
    expect(posted).toEqual([{ path, body: { assignment: holder.assignment, from_version: 1, to_version: 2 } }]);
    expect(text()).toContain('Lys saved the role change.');
  });
  it('confirms ending the exact assignment without retiring the person or revoking grants', async () => {
    const path = '/roles/' + role.id + '/holders/' + SCRIBE + '/end';
    const { posted } = await mount('#/roles/' + role.id, { ...routes, ['POST ' + path]: ok({ ...role, holders: [{ ...holder, state: 'ended' }] }) });
    await click(button('Remove this role assignment')); expect(posted).toEqual([]); await click(button('Yes, remove this assignment'));
    expect(posted).toEqual([{ path, body: { assignment: holder.assignment } }]);
  });
  it('offers no write controls to a non-administrator', async () => {
    await mount('#/roles/' + role.id, { ...routes, '/directory/people': refused(403, 'NotAdmitted', 'not administrator'), '/people': ok(OWN) });
    expect(button('New version')).toBeNull(); expect(button('Assign role')).toBeNull(); expect(button('Remove this role assignment')).toBeNull();
  });
});


describe('Roles on identity screens', () => {
  it('renders the held version in the identity file, not the latest role words', async () => {
    const { posted } = await mount('#/file/' + SCRIBE, routes);
    expect(text()).toContain('Review changes');
    expect(text()).not.toContain('Review code and docs');
    expect(text()).toContain('Version 2 is available; this assignment has not moved.');
    expect(text()).toContain('Correct releases');
    expect(text()).toContain('Check receipts');
    expect(posted).toEqual([]);
  });
  it('shows current role versions in directory rows and their preview', async () => {
    const { posted } = await mount('#/people', routes);
    const row = document.querySelector('[data-href="#/file/' + SCRIBE + '"]');
    expect(row?.textContent).toContain('Reviewer · v1');
    expect(posted.map((call) => call.path)).toEqual(['/grants/reach']);
  });
  it('keeps ended assignments in history without representing them as current', async () => {
    await mount('#/file/' + SCRIBE, { ...routes, '/roles': ok({ roles: [{ ...role, holders: [{ ...holder, state: 'ended', ended_at: 1790000099 }] }] }) });
    expect(text()).toContain('No role is currently assigned');
    expect(text()).toContain('Past role assignments (1)');
    expect(text()).not.toContain('Review changes');
  });
  it('names unreadable role state instead of claiming there are no assignments', async () => {
    await mount('#/file/' + SCRIBE, { ...routes, '/roles': refused(503, 'RolesUnavailable', 'Could not read role records') });
    expect(text()).toContain('RolesUnavailable');
    expect(text()).not.toContain('No role is currently assigned');
  });
});
