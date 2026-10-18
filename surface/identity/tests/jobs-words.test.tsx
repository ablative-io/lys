/** The Roles pages' object is a job (CONSENSUS): nothing a person reads or hears on them says "role", which means the grant's bundle elsewhere. */
import { beforeEach, describe, expect, it } from 'vitest';
import { $, click, mount, unmountAll } from './harness';
import { ADA, SCRIBE, SERVICE, ok } from './fixtures';
import type { Role, RoleHolder, RoleVersion, RoleWords } from '../src/features/roles/contract';

beforeEach(() => sessionStorage.clear());
const words: RoleWords = { responsibilities: 'Review changes', goals: 'Correct releases', practice: 'Check receipts', profile: 'Reviewer', grant_templates: [{ relation: 'viewer', resource: { kind: 'project', id: 'ledger' }, days: 7 }], note: 'Initial version' };
const version: RoleVersion = { ...words, number: 1, made_by: ADA, made_at: 1790000000 };
const holder: RoleHolder = { assignment: 'op-' + 'b'.repeat(32), holder: SCRIBE, display_name: 'Scribe', version: 1, behind: true, assigned_by: ADA, assigned_at: 1790000001, ends_at: null, moves_at: null, state: 'holding', moves: [], ended_by: null, ended_at: null };
const job: Role = { id: 'op-' + 'a'.repeat(32), name: 'Reviewer', latest: 2, policy: 'stays_until_moved', versions: [version, { ...version, number: 2, note: 'Include docs' }], holders: [holder] };
const routes = { ...SERVICE, '/roles': ok({ roles: [job] }) };

/** Every word on the page a person reads, and every label a screen reader says, in one string. */
const said = (): string => {
  const page = $('.page');
  if (!page) throw new Error('No page');
  const labels = [...page.querySelectorAll('[aria-label], [title]')].map((each) => (each.getAttribute('aria-label') ?? '') + ' ' + (each.getAttribute('title') ?? ''));
  return (page.textContent ?? '') + ' ' + labels.join(' ');
};
const button = (label: string) => [...document.querySelectorAll('button')].find((value) => (value.getAttribute('aria-label') ?? value.textContent) === label) ?? null;

describe('Jobs say job', () => {
  it('says job, never role, on the list, a job, its holders and both forms', async () => {
    await mount('#/roles', routes);
    expect($('.page h1')?.textContent).toBe('Jobs');
    expect(said()).not.toMatch(/\brole/i);
    await click(button('Create a job'));
    expect($('form[aria-label="Create job"]')).not.toBeNull();
    expect(said()).not.toMatch(/\brole/i);
    unmountAll();
    await mount('#/roles/' + job.id, routes);
    expect(said()).toContain('People and agents doing this job');
    expect(said()).toContain('Move to version 2');
    expect(said()).not.toMatch(/\brole/i);
  });
});
