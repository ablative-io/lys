/** Sign out is a worded act on You: it ends this browser's own session, and nothing else, then opens the sign-in page that says so. */
import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { signedOutPage } from '../src/features/me/SignOut';
import { SignIn } from '../src/features/sign-in/SignIn';
import { $, $$, click, mount } from './harness';
import { SERVICE, ok, refused } from './fixtures';
import type { Route } from './fixtures';
import { serve } from './harness';

const sessions = { person: '', sessions: [
  { id: 'session-other', login: { issuer: 'https://login.test', subject: 'x' }, started_at: 1792000000, ends_at: 1794600000, current: false },
  { id: 'session-current', login: { issuer: 'https://login.test', subject: 'x' }, started_at: 1792100000, ends_at: 1794700000, current: true },
] };
const signOut = () => $$('.page-header button').find((each) => each.textContent === 'Sign out') ?? null;
let opened: ReturnType<typeof vi.spyOn>;
beforeEach(() => { opened = vi.spyOn(signedOutPage, 'open').mockImplementation(() => undefined); });
afterEach(() => { opened.mockRestore(); });

it('shows Sign out in words beside the name and ends only the current session', async () => {
  const { posted } = await mount('#/me', { ...SERVICE, '/sessions': ok(sessions), 'POST /sessions/session-current/end': ok({ ended: 'session-current' }) });
  const button = signOut();
  expect(button?.querySelector('.act-word')?.textContent).toBe('Sign out');
  await click(button);
  expect(posted.map((each) => each.path)).toEqual(['/sessions/session-current/end']);
  expect(opened).toHaveBeenCalledOnce();
});

it('says the person is still signed in when the end could not be confirmed, and does not leave', async () => {
  const routes: Record<string, Route> = { ...SERVICE, '/sessions': ok(sessions), 'POST /sessions/session-current/end': refused(503, 'SessionsUnavailable', 'the session store is not answering') };
  await mount('#/me', routes);
  await click(signOut());
  expect($('.page-header [role="alert"]')?.textContent).toContain('You are still signed in. Press Sign out again.');
  expect(opened).not.toHaveBeenCalled();
});

it('treats a session that already ended as signed out', async () => {
  await mount('#/me', { ...SERVICE, '/sessions': ok(sessions) });
  serve({ ...SERVICE, '/sessions': refused(401, 'NotSignedIn', 'Sign in first') });
  await click(signOut());
  expect(opened).toHaveBeenCalledOnce();
});

it('the sign-in page opened by Sign out says why it is showing, and only then', async () => {
  serve({ '/sign-in/help': ok({ administrator: null }), '/sign-in/providers': ok({ providers: [] }) });
  for (const [hash, shown] of [['#/sign-in?signed-out', true], ['#/sign-in', false]] as const) {
    location.hash = hash;
    const container = document.createElement('div');
    document.body.appendChild(container);
    const root = createRoot(container);
    await act(async () => root.render(<SignIn />));
    expect(container.textContent?.includes('You have signed out of Lys in this browser.')).toBe(shown);
    act(() => root.unmount());
    container.remove();
  }
});
