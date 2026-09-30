/** Screens use the confirmed write response and explain failures without internal codes. */
import { act } from 'react';
import type { ReactNode } from 'react';
import { createRoot } from 'react-dom/client';
import type { Root } from 'react-dom/client';
import { afterEach, expect, it, vi } from 'vitest';
import { Refused } from '../src/api';
import { SignInProviders } from '../src/features/connections/SignInProviders';
import { Secrets } from '../src/features/secrets/Secrets';
import { SecretRows } from '../src/features/secrets/Secrets';

const roots: Root[] = [];
afterEach(() => { for (const root of roots.splice(0)) act(() => root.unmount()); });
async function show(node: ReactNode) {
  const container = document.createElement('div');
  document.body.appendChild(container);
  const root = createRoot(container);
  roots.push(root);
  await act(async () => { root.render(node); });
  return container;
}
async function enter(input: HTMLInputElement, value: string) {
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')?.set?.call(input, value);
    input.dispatchEvent(new Event('input', { bubbles: true }));
  });
}

it('shows the provider write answer even when a later read would be stale', async () => {
  const empty = { providers: [], offered: ['google'], redirect_address: 'https://example.test/callback' };
  const saved = { ...empty, providers: [{ id: 'provider-1', provider: 'google', name: 'Google', enabled: true, client_id: 'client-confirmed' }] };
  let reads = 0;
  const sent: unknown[] = [];
  vi.stubGlobal('fetch', async (_url: string, init?: RequestInit) => {
    if (init?.method === 'POST') {
      sent.push(JSON.parse(String(init.body)));
      return Response.json(saved);
    }
    reads += 1;
    return Response.json(empty);
  });
  const view = await show(<SignInProviders />);
  const inputs = view.querySelectorAll<HTMLInputElement>('form input');
  if (inputs.length !== 2) throw new Error('The provider form must have two inputs');
  await enter(inputs[0], 'client-confirmed');
  await enter(inputs[1], 'never-display-this-secret');
  await act(async () => { view.querySelector<HTMLFormElement>('form')?.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })); });
  expect(sent).toEqual([{ provider: 'google', client_id: 'client-confirmed', client_secret: 'never-display-this-secret' }]);
  expect(reads).toBe(1);
  expect(view.textContent).toContain('Enabled');
  expect(view.textContent).not.toContain('No sign-in provider is set yet');
  expect(view.innerHTML).not.toContain('never-display-this-secret');
});

it('explains a secrets read failure and the next step without its refusal code', async () => {
  const view = await show(<Secrets read={async () => { throw new Refused(503, { refusal: 'SecretsUnavailable', reason: 'no secrets broker is configured for this service' }); }} />);
  expect(view.textContent).toContain('Ask your administrator');
  expect(view.textContent).not.toContain('SecretsUnavailable');
  expect(view.textContent).not.toContain('scope');
});

it('puts a secret owner identifier behind a details toggle', async () => {
  const owner = 'person-' + 'a'.repeat(32);
  const view = await show(<SecretRows listing={{ secrets: [{ name: 'Calendar', class: 'credential', owner, sequence: 2, upstream: null, header: null }] }} />);
  const details = view.querySelectorAll('details');
  expect(details).toHaveLength(1);
  expect(details[0].textContent).toContain(owner);
  expect(details[0].open).toBe(false);
  for (const detail of details) detail.remove();
  expect(view.textContent).not.toContain(owner);
});
