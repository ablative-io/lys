import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { expect, it } from 'vitest';
import { AppNext } from './AppNext';
import { ok, refused, SCRIBE } from '../../../tests/fixtures';
import type { Route } from '../../../tests/fixtures';
import { serve, settle } from '../../../tests/harness';

const ACCESS = '#/file/' + SCRIBE + '/access';
const NONE = ok({ revision: 1, grants: [] });

async function shown(apps: Route, administrator: boolean, grants: Route = NONE) {
  serve({ '/apps': apps, '/grants': grants });
  const host = document.createElement('div');
  document.body.append(host);
  const root = createRoot(host);
  await act(async () => { root.render(<AppNext agent={SCRIBE} name="Scribe" administrator={administrator} />); });
  await settle();
  const seen = { text: host.textContent ?? '', sections: host.querySelectorAll('section').length, links: [...host.querySelectorAll('a')].map((link) => [link.textContent, link.getAttribute('href')]) };
  act(() => root.unmount()); host.remove();
  return seen;
}

const held = (kind: string) => ok({ revision: 1, grants: [{ holder: SCRIBE, resource: { kind, id: 'c1' }, actions: ['read'], standing: { stands: true } }] });

it('gives the administrator the app’s own address, from its sign-in return address, and the agent’s Access tab once it is approved', async () => {
  const seen = await shown(ok({ apps: [{ id: 'fixture_product', name: 'Fixture Product', state: 'approved', redirects: ['https://fixture.example.test/auth/callback'], sign_in: null }] }), true);
  expect(seen.text).toContain('Use Scribe in Fixture Product');
  expect(seen.links).toEqual([['Open Fixture Product', 'https://fixture.example.test'], ['Give Scribe what it may use in Fixture Product', ACCESS]]);
});

it('gives a person who is not the administrator Open as a link, from the address their view of the app carries', async () => {
  const seen = await shown(ok({ apps: [{ id: 'fixture_product', name: 'Fixture Product', state: 'approved', address: 'https://fixture.example.test' }] }), false);
  expect(seen.links).toEqual([['Open Fixture Product', 'https://fixture.example.test'], ['Give Scribe what it may use in Fixture Product', ACCESS]]);
});

it('names the app without a link for a person whose view of it carries no address, as an older Lys answers', async () => {
  const seen = await shown(ok({ apps: [{ id: 'fixture_product', name: 'Fixture Product', state: 'approved' }] }), false);
  expect(seen.text).toContain('Open Fixture Product');
  expect(seen.links).toEqual([['Give Scribe what it may use in Fixture Product', ACCESS]]);
});

it('names an app by its id when the apps list carries no name for it', async () => {
  const seen = await shown(ok({ apps: [{ id: 'fixture_product', state: 'approved' }] }), false);
  expect(seen.text).toContain('Use Scribe in fixture_product');
});

it('sends the administrator to Apps before any app is connected, and tells a person the cause and who acts', async () => {
  const administrator = await shown(ok({ apps: [] }), true);
  expect(administrator.text).toContain('Use Scribe in an app');
  expect(administrator.text).toContain('No app is connected to this Lys yet.');
  expect(administrator.links).toEqual([['Connect one on Apps', '#/apps']]);
  const person = await shown(ok({ apps: [{ id: 'fixture_product', name: 'Fixture Product', state: 'pending' }] }), false);
  expect(person.text).toContain('No app is connected to this Lys yet, so Scribe cannot be used in one.');
  expect(person.links).toEqual([]);
});

it('folds to one line once the agent holds access in the app, still saying where to change it', async () => {
  const seen = await shown(ok({ apps: [{ id: 'fixture_product', name: 'Fixture Product', state: 'approved', address: 'https://fixture.example.test' }] }), false, held('fixture_product.space'));
  expect(seen.text).toContain('Scribe has access in Fixture Product.');
  expect(seen.sections).toBe(0);
  expect(seen.links).toEqual([['Open Fixture Product', 'https://fixture.example.test'], ['change what it may use', ACCESS]]);
});

it('says one thing per approved app, in the apps list’s order: a line where access is held, the steps where it is not', async () => {
  const seen = await shown(ok({ apps: [
    { id: 'fixture_product', name: 'Fixture Product', state: 'approved' },
    { id: 'other_fixture', name: 'Other Fixture', state: 'approved' },
    { id: 'pending_fixture', name: 'Pending Fixture', state: 'pending' },
  ] }), false, held('fixture_product.space'));
  expect(seen.text).toContain('Scribe has access in Fixture Product.');
  expect(seen.text).toContain('Use Scribe in Other Fixture');
  expect(seen.text).not.toContain('Pending Fixture');
  expect(seen.sections).toBe(1);
  expect(seen.text.indexOf('Fixture Product')).toBeLessThan(seen.text.indexOf('Other Fixture'));
});

it('shows a refused apps list in its words, under a heading that names no app', async () => {
  const seen = await shown(refused(403, 'NotAdmitted', 'Only the administrator.'), false);
  expect(seen.text).toContain('Use Scribe in an app');
  expect(seen.text).toContain('Only the administrator.');
});
