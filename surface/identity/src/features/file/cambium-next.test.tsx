import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { expect, it } from 'vitest';
import { CambiumNext } from './CambiumNext';
import { ok, SCRIBE } from '../../../tests/fixtures';
import type { Route } from '../../../tests/fixtures';
import { serve, settle } from '../../../tests/harness';

async function shown(apps: Route, administrator: boolean) {
  serve({ '/apps': apps, '/grants': ok({ revision: 1, grants: [] }) });
  const host = document.createElement('div');
  document.body.append(host);
  const root = createRoot(host);
  await act(async () => { root.render(<CambiumNext agent={SCRIBE} name="Scribe" administrator={administrator} />); });
  await settle();
  const seen = { text: host.textContent ?? '', links: [...host.querySelectorAll('a')].map((link) => [link.textContent, link.getAttribute('href')]) };
  act(() => root.unmount()); host.remove();
  return seen;
}

it('gives the administrator Cambium’s own address and the agent’s Access tab once Cambium is approved', async () => {
  const seen = await shown(ok({ apps: [{ id: 'cambium', name: 'Cambium', state: 'approved', redirects: ['https://cambium.example/auth/callback'], sign_in: null }] }), true);
  expect(seen.text).toContain('choose Scribe');
  expect(seen.links).toEqual([['Open Cambium', 'https://cambium.example'], ['Give Scribe the Cambium channels it may use', '#/file/' + SCRIBE + '/access']]);
});

it('gives a person who is not the administrator Open Cambium as a link, from the address their view of the app carries', async () => {
  const seen = await shown(ok({ apps: [{ id: 'cambium', name: 'Cambium', state: 'approved', address: 'https://cambium.example' }] }), false);
  expect(seen.links).toEqual([['Open Cambium', 'https://cambium.example'], ['Give Scribe the Cambium channels it may use', '#/file/' + SCRIBE + '/access']]);
});

it('names Cambium without a link for a person whose view of the app carries no address, as an older Lys answers', async () => {
  const seen = await shown(ok({ apps: [{ id: 'cambium', name: 'Cambium', state: 'approved' }] }), false);
  expect(seen.text).toContain('Open Cambium, go to a channel and press Invite.');
  expect(seen.links).toEqual([['Give Scribe the Cambium channels it may use', '#/file/' + SCRIBE + '/access']]);
});

it('sends the administrator to Apps before Cambium is connected, and tells a person the cause and who acts', async () => {
  const administrator = await shown(ok({ apps: [] }), true);
  expect(administrator.links).toEqual([['Connect Cambium on Apps', '#/apps']]);
  const person = await shown(ok({ apps: [{ id: 'cambium', name: 'Cambium', state: 'pending' }] }), false);
  expect(person.text).toContain('Cambium is not connected to this Lys yet, so Scribe cannot be used there.');
  expect(person.links).toEqual([]);
});

it('folds to one line once the agent holds access in Cambium, still saying where to change it', async () => {
  serve({ '/apps': ok({ apps: [{ id: 'cambium', name: 'Cambium', state: 'approved' }] }), '/grants': ok({ revision: 1, grants: [{ holder: SCRIBE, resource: { kind: 'cambium.channel', id: 'c1' }, actions: ['read'], standing: { stands: true } }] }) });
  const host = document.createElement('div');
  document.body.append(host);
  const root = createRoot(host);
  await act(async () => { root.render(<CambiumNext agent={SCRIBE} name="Scribe" administrator={false} />); });
  await settle();
  expect(host.textContent).toContain('Scribe has access in Cambium.');
  expect(host.querySelector('ol')).toBeNull();
  act(() => root.unmount()); host.remove();
});
