/** A single-use OIDC answer is exchanged once, including under React StrictMode. */
import { StrictMode, act } from 'react';
import { createRoot } from 'react-dom/client';
import type { Root } from 'react-dom/client';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { api, Refused } from '../src/api';
import { App } from '../src/App';
import type { SignedIn } from '../src/generated';

let root: Root | undefined;

afterEach(() => {
  if (root) act(() => root?.unmount());
  root = undefined;
});

async function mountCallback(pathname = '/callback') {
  const replace = vi.fn();
  vi.stubGlobal('location', { pathname, search: '?code=once&state=bound', replace });
  const container = document.createElement('div');
  document.body.appendChild(container);
  root = createRoot(container);
  await act(async () => root?.render(<StrictMode><App /></StrictMode>));
  return { replace, container };
}

const signedIn: SignedIn = {
  signed_in: { issuer: 'https://issuer.example.test', subject: 'person' },
  authority: 'test authority',
};

function pendingAnswer() {
  let complete: ((value: SignedIn) => void) | undefined;
  let fail: ((reason: Refused) => void) | undefined;
  const promise = new Promise<SignedIn>((resolve, reject) => {
    complete = resolve;
    fail = reject;
  });
  return {
    promise,
    resolve(value: SignedIn) {
      if (!complete) throw new Error('The test exchange has not been created');
      complete(value);
    },
    reject(reason: Refused) {
      if (!fail) throw new Error('The test exchange has not been created');
      fail(reason);
    },
  };
}

describe('Sign-in callback', () => {
  it('names an unreadable exchange answer without retrying the single-use code', async () => {
    const fetch = vi.fn().mockResolvedValue(new Response('not JSON', { status: 200 }));
    vi.stubGlobal('fetch', fetch);
    const { replace, container } = await mountCallback('/auth/callback');
    expect(fetch).toHaveBeenCalledTimes(1);
    expect(container.textContent).toContain('UnreadableResponse');
    expect(container.textContent).not.toContain('code=once');
    expect(replace).not.toHaveBeenCalled();
  });

  it.each(['/callback', '/auth/callback'])('exchanges once at %s when StrictMode replays the effect', async (pathname) => {
    const answer = pendingAnswer();
    const callback = vi.spyOn(api, 'callback').mockReturnValue(answer.promise);
    const { replace } = await mountCallback(pathname);
    expect(callback.mock.calls).toEqual([['?code=once&state=bound']]);
    await act(async () => answer.resolve(signedIn));
    expect(replace.mock.calls).toEqual([['/#/']]);
  });

  it('shows the original exchange refusal without attempting the consumed code again', async () => {
    const answer = pendingAnswer();
    const callback = vi.spyOn(api, 'callback').mockReturnValue(answer.promise);
    const { replace, container } = await mountCallback();
    await act(async () => answer.reject(new Refused(400, {
      refusal: 'SignInFailed', reason: 'The issuer refused the code exchange',
    })));
    expect(callback).toHaveBeenCalledTimes(1);
    expect(container.textContent).toContain('The issuer refused the code exchange');
    expect(replace).not.toHaveBeenCalled();
  });

  it('does not navigate after the callback page has unmounted', async () => {
    const answer = pendingAnswer();
    vi.spyOn(api, 'callback').mockReturnValue(answer.promise);
    const { replace } = await mountCallback();
    act(() => root?.unmount());
    root = undefined;
    await act(async () => answer.resolve(signedIn));
    expect(replace).not.toHaveBeenCalled();
  });
});
