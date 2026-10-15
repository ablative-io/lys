import { act } from 'react';
import { createRoot } from 'react-dom/client';
import type { Root } from 'react-dom/client';
import { afterEach, expect, it } from 'vitest';
import { SignIn } from '../src/features/sign-in/SignIn';
import { AccountHelp } from '../src/features/signin/AccountHelp';
import { ok } from './fixtures';
import { serve } from './harness';

let root: Root | null = null;
afterEach(() => { act(() => root?.unmount()); root = null; document.body.innerHTML = ''; });

async function open(refusal: string, email: string | null, helpOnly = false) {
  location.hash = '#/sign-in?refused=' + refusal + (email ? '&email=' + encodeURIComponent(email) : '');
  const requests = serve({ '/sign-in/help': ok({ administrator: { name: 'Ada', email: 'ada@example.test' } }),
    '/sign-in/providers': ok({ providers: [] }) });
  const container = document.createElement('div'); document.body.appendChild(container);
  root = createRoot(container);
  await act(async () => { root?.render(helpOnly ? <AccountHelp /> : <SignIn />); });
  return requests;
}

it('names the Google account and the administrator who can admit it', async () => {
  const requests = await open('NotAdmittedYet', 'member@example.test');
  expect(document.querySelector('[role="alert"]')?.textContent).toBe("This Google account (member@example.test) hasn't been added to Lys yet. Ask Ada (ada@example.test) to add it.");
  expect(document.querySelector('.account-help')?.textContent).toContain('Ask Ada (ada@example.test) to admit you.');
  expect(requests.filter((path) => path === '/sign-in/help')).toHaveLength(1);
});

it('gives a disabled account its own next step', async () => {
  await open('AccountDisabled', 'member@example.test');
  expect(document.querySelector('[role="alert"]')?.textContent).toContain('has been disabled in Lys. Ask Ada (ada@example.test) to enable it.');
});

it('does not invent an email when the issuer did not supply one', async () => {
  await open('NotAdmittedYet', null);
  expect(document.querySelector('[role="alert"]')?.textContent).toBe("This Google account hasn't been added to Lys yet. Ask Ada (ada@example.test) to add it.");
});

it('names the same contact without requiring a session on the other sign-in entry', async () => {
  const requests = await open('', null, true);
  expect(document.querySelector('.account-help')?.textContent).toContain('Contact Ada (ada@example.test) for sign-in help.');
  expect(requests).toEqual(['/sign-in/help']);
});
