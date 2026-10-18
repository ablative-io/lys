// Domain 04 item 5: an approved app's card says who can use it and links to giving people access. The holders come from
// the service's grants, and "nobody" is said outright only when the service answers the whole directory (the root
// authority sees every grant); anyone else is told the count covers what they can see.
import { expect, it } from 'vitest';
import { mount, settle } from '../../../tests/harness';
import { ADA, DIRECTORY, GRANTS, OWN, SERVICE, ok, refused } from '../../../tests/fixtures';
import type { Grant } from '../../generated/grants';
import { holdersOf } from './WhoCanUse';

const cambium = { id: 'cambium', name: 'Cambium', state: 'approved', redirects: [], sign_in: null, schema: { kinds: {} }, version: 1, versions: [1], pending: null,
  client_id: 'cambium', service_account: null, client_credentials: [], registered_by: { kind: 'start' }, registered_at: 1 };
const onCambium = (id: string, holder: string, stands = true): Grant => ({ ...GRANTS[0], id, holder, resource: { kind: 'cambium.place', id: 'general' },
  standing: stands ? { stands: true } : { stands: false, refusal: 'Revoked', grant: id, reason: 'revoked' } });
const section = () => document.querySelector('[aria-label="Who can use cambium"]');
const give = () => [...document.querySelectorAll('a')].find((entry) => entry.textContent === 'Give people access') ?? null;

it('counts each holder once, only for standing grants on the app’s own kinds', () => {
  const grants = [onCambium('g1', ADA), onCambium('g2', ADA), onCambium('g3', 'person-gone', false), GRANTS[0], { ...GRANTS[0], id: 'g4', holder: 'agent-x', resource: { kind: 'cambiumish.place', id: 'x' } }];
  expect(holdersOf('cambium', grants)).toEqual([ADA]);
});

async function draw(grants: Grant[], people = DIRECTORY) {
  // The service answers the whole directory only to the root authority, and refuses it to anyone else, who reads /people.
  const directory = people.scope === 'directory' ? ok(people) : refused(403, 'NotAdmitted', 'not administrator');
  await mount('#/apps', { ...SERVICE, '/apps': () => ok({ apps: [cambium] }), '/grants': () => ok({ grants, revision: 1 }), '/directory/people': () => directory, '/people': () => ok(people) });
  await settle();
}

it('tells the root authority nobody can use Cambium yet, with Give people access as the primary act', async () => {
  await draw(GRANTS);
  expect(section()?.textContent).toContain('Nobody yet. People can sign in to Cambium, but they can do nothing there until they are given access.');
  expect(give()?.getAttribute('href')).toBe('#/access/issue?app=cambium');
  expect(give()?.classList.contains('primary')).toBe(true);
});

it('names each holder, linked to their page, once some hold Cambium access', async () => {
  await draw([...GRANTS, onCambium('g1', ADA)]);
  expect(section()?.textContent).toContain('one person or agent holds Cambium access');
  const names = [...(section()?.querySelectorAll('li a') ?? [])].map((link) => [link.textContent, link.getAttribute('href')]);
  expect(names).toEqual([['Ada (test person)', '#/file/' + encodeURIComponent(ADA)]]);
  expect(give()?.classList.contains('primary')).toBe(false);
});

it('never says nobody outright to someone who sees only their own grants', async () => {
  await draw(GRANTS, OWN);
  expect(section()?.textContent).toContain('Nobody, of the access you can see.');
  expect(section()?.textContent).not.toContain('Nobody yet.');
});

it('keeps Give people access when the grants cannot be read, and says the list is what failed', async () => {
  await mount('#/apps', { ...SERVICE, '/apps': () => ok({ apps: [cambium] }), '/grants': () => refused(503, 'ServiceUnavailable', 'grants store closed') });
  await settle();
  expect(section()?.textContent).toContain('Lys could not read who holds Cambium access');
  expect(give()).not.toBeNull();
});

it('says an answer that is not a list of grants as unreadable, and keeps the page and Give people access', async () => {
  await mount('#/apps', { ...SERVICE, '/apps': () => ok({ apps: [cambium] }), '/grants': () => ok({ something: 'else' }) });
  await settle();
  expect(section()?.textContent).toContain('Lys could not read who holds Cambium access');
  expect(document.querySelector('[aria-label="Connect an app"]')).not.toBeNull();
  expect(give()).not.toBeNull();
});

it('says the next step in the approval notice', async () => {
  const pending = { ...cambium, state: 'pending' };
  const approved = { app: cambium, credentials: null };
  await mount('#/apps', { ...SERVICE, '/apps': () => ok({ apps: [pending] }), 'POST /apps/cambium/approve': () => ok(approved) });
  await settle();
  const approve = [...document.querySelectorAll('button')].find((entry) => entry.getAttribute('aria-label') === 'Approve Cambium');
  approve?.click();
  await settle();
  expect(document.querySelector('[role="status"]')?.textContent).toBe('Cambium is approved. People can sign in to it now, and can do things there once you give them access, under Who can use Cambium below.');
  expect(section()).not.toBeNull();
});
