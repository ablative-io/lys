import { act } from 'react';
import { createRoot } from 'react-dom/client';
import type { Root } from 'react-dom/client';
import { afterEach, expect, it } from 'vitest';
import { App } from '../src/App';
import { SERVICE, refused } from './fixtures';
import type { Route } from './fixtures';
import { serve } from './harness';

const roots: Root[] = [];
afterEach(() => { for (const root of roots.splice(0)) act(() => root.unmount()); });

async function render(hash: string, routes: Record<string, Route>) {
  serve(routes);
  location.hash = hash;
  const container = document.createElement('div');
  document.body.appendChild(container);
  const root = createRoot(container);
  roots.push(root);
  await act(async () => { root.render(<App />); });
  return container;
}

it('a signed-out visitor sees only the sign-in card, without Dashboard or Reconnect', async () => {
  const page = await render('#/', { ...SERVICE, '/me': refused(401, 'NotSignedIn', 'Sign in first') });
  expect([...page.querySelectorAll('h1')].map((heading) => heading.textContent)).toEqual(['Sign in']);
  expect(page.querySelectorAll('.card')).toHaveLength(1);
  expect(page.querySelector('.dash-page')).toBeNull();
  expect(page.querySelector('#rail')).toBeNull();
  expect(page.textContent).not.toContain('Reconnect');
});

it('both sign-in entries give one account admission and help line', async () => {
  for (const hash of ['#/', '#/sign-in']) {
    const page = await render(hash, { ...SERVICE, '/me': refused(401, 'NotSignedIn', 'Sign in first') });
    const lines = page.querySelectorAll('.account-help');
    expect(lines).toHaveLength(1);
    expect(lines[0].textContent).toBe('No account? Ask your administrator to admit you. Contact your administrator for sign-in help.');
    act(() => roots.pop()?.unmount());
  }
});

it('Reconnect appears only after a signed-in session loses its feed and recovers the page', async () => {
  let failFeed: (answer: { status: number; body: unknown }) => void = () => { throw new Error('feed not constructed'); };
  const feed = new Promise<{ status: number; body: unknown }>((resolve) => { failFeed = resolve; });
  const next = '/changes?after=op-' + '0'.repeat(32);
  const page = await render('#/', { ...SERVICE, [next]: (() => feed) as unknown as Route });
  expect(page.querySelector('h1')?.textContent).toBe('Dashboard');
  expect(page.textContent).not.toContain('Reconnect');
  await act(async () => { failFeed({ status: 503, body: { refusal: 'ChangeFeedUnavailable', reason: 'feed unavailable' } }); });
  const reconnect = [...page.querySelectorAll('button')].find((button) => button.textContent === 'Reconnect');
  expect(reconnect).toBeDefined();
  serve(SERVICE);
  await act(async () => { reconnect?.click(); });
  expect(page.querySelector('.dash-widget')).not.toBeNull();
  expect(page.textContent).not.toContain('Reconnect');
});

it('an application read refusal does not offer a feed reconnect', async () => {
  const page = await render('#/', { ...SERVICE, '/me': refused(503, 'DirectoryUnavailable', 'directory unavailable') });
  expect(page.textContent).toContain('DirectoryUnavailable');
  expect(page.textContent).not.toContain('Reconnect');
});
