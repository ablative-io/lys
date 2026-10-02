import { act } from 'react';
import { createRoot } from 'react-dom/client';
import type { Root } from 'react-dom/client';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { App } from '../src/App';
import { ADA, BEA, GRANTS, ME, ROOT_G, SCRIBE, SCRIBE_G, SERVICE, ok, refused } from './fixtures';
import type { Route } from './fixtures';
import { serve } from './harness';

let root: Root | null = null;
afterEach(() => { if (root) act(() => root?.unmount()); root = null; });

async function open(routes: Record<string, Route> = SERVICE, holder = SCRIBE) {
  const posted: { path: string; body: unknown }[] = [];
  serve(routes, posted);
  history.replaceState(null, '', '/#/file/' + holder + '/access');
  const container = document.createElement('div');
  document.body.appendChild(container);
  const mounted = createRoot(container);
  root = mounted;
  await act(async () => { mounted.render(<App />); });
  return { posted, container };
}

const issue = () => document.querySelector<HTMLButtonElement>('[data-act="issue-credential"]');
const credential = () => document.querySelector<HTMLInputElement>('input[aria-label="Credential"]');
async function click(selector: string) {
  const button = document.querySelector<HTMLButtonElement>(selector);
  if (!button) throw new Error('Missing control: ' + selector);
  await act(async () => { button.click(); });
}
const tokenRoutes = () => ({ ...SERVICE, ['POST /grants/' + SCRIBE_G + '/tokens']: (body: unknown) => ok({ id: 'token-' + 'a'.repeat(64), token: 'test-credential', expires_at: (body as { expires_at: number }).expires_at }) });

describe('Agent grant credentials', () => {
  it('offers issuance for an agent-held grant only to its responsible signed-in person', async () => {
    await open();
    expect(issue()?.textContent).toBe('Issue credential');
    expect(issue()?.getAttribute('data-g')).toBe(SCRIBE_G);
    expect(document.querySelectorAll('[data-act="issue-credential"]')).toHaveLength(1);
  });

  it('offers no issuance on a person-held grant', async () => {
    await open(SERVICE, ADA);
    expect(document.body.textContent).toContain('owner');
    expect(document.body.textContent).toContain(ROOT_G.slice(6, 14));
    expect(issue()).toBeNull();
  });

  it('does not give an administrator issuance for another responsible person', async () => {
    await open({ ...SERVICE, '/grants': ok({ grants: GRANTS.map((grant) => grant.id === SCRIBE_G ? { ...grant, responsible: BEA } : grant), revision: 7 }) });
    expect(document.body.textContent).toContain('viewer');
    expect(issue()).toBeNull();
  });

  it('posts a 24-hour expiry, displays it and copies the returned credential without retaining it', async () => {
    const now = 1790888400;
    vi.spyOn(Date, 'now').mockReturnValue(now * 1000);
    const writes = vi.spyOn(Storage.prototype, 'setItem');
    const copy = vi.fn().mockResolvedValue(undefined);
    vi.spyOn(navigator, 'clipboard', 'get').mockReturnValue({ writeText: copy } as unknown as Clipboard);
    const { posted } = await open(tokenRoutes());
    const url = location.href;
    await click('[data-act="issue-credential"]');
    expect(posted).toEqual([{ path: '/grants/' + SCRIBE_G + '/tokens', body: { expires_at: now + 86_400 } }]);
    expect(credential()?.value).toBe('test-credential');
    expect(credential()?.readOnly).toBe(true);
    expect(document.querySelector('time[data-credential-expiry]')?.getAttribute('datetime')).toBe(new Date((now + 86_400) * 1000).toISOString());
    expect(issue()?.disabled).toBe(true);
    await click('[data-act="copy-credential"]');
    expect(copy).toHaveBeenCalledExactlyOnceWith('test-credential');
    expect(document.body.textContent).toContain('Copied.');
    expect(writes.mock.calls.some((call) => call.some((value) => String(value).includes('test-credential')))).toBe(false);
    expect(location.href).toBe(url);
    await click('[data-act="hide-credential"]');
    expect(credential()).toBeNull();
    expect(posted).toHaveLength(1);
  });

  it('cannot redisplay the credential after leaving and reopening the screen', async () => {
    await open(tokenRoutes());
    await click('[data-act="issue-credential"]');
    expect(credential()?.value).toBe('test-credential');
    const mounted = root;
    await act(async () => { mounted?.unmount(); });
    root = null;
    document.body.innerHTML = '';
    const { posted } = await open(tokenRoutes());
    expect(credential()).toBeNull();
    expect(posted).toEqual([]);
    expect(issue()?.disabled).toBe(false);
  });

  it('shows the refusal name and reason without a credential or automatic retry', async () => {
    const { posted } = await open({ ...SERVICE, ['POST /grants/' + SCRIBE_G + '/tokens']: refused(503, 'GrantTokenUnavailable', 'The credential store cannot be read.') });
    await click('[data-act="issue-credential"]');
    expect(document.querySelector('[role="alert"]')?.textContent).toContain('GrantTokenUnavailable');
    expect(document.querySelector('[role="alert"]')?.textContent).toContain('The credential store cannot be read.');
    expect(credential()).toBeNull();
    expect(posted).toHaveLength(1);
  });

  it('refuses an unreadable success without reflecting secret response details', async () => {
    await open({ ...SERVICE, ['POST /grants/' + SCRIBE_G + '/tokens']: ok({ token: 'test-credential', expires_at: 'bad' }) });
    await click('[data-act="issue-credential"]');
    expect(document.querySelector('[role="alert"]')?.textContent).toContain('GrantTokenResponseUnreadable');
    expect(document.body.textContent).not.toContain('test-credential');
    expect(credential()).toBeNull();
  });

  it('does not carry a credential into a changed signed-in person', async () => {
    await open(tokenRoutes());
    await click('[data-act="issue-credential"]');
    expect(credential()?.value).toBe('test-credential');
    const mounted = root;
    await act(async () => { mounted?.unmount(); });
    root = null;
    document.body.innerHTML = '';
    await open({ ...tokenRoutes(), '/me': ok({ ...ME, person: { ...ME.person, id: BEA } }) });
    expect(credential()).toBeNull();
    expect(issue()).toBeNull();
  });
});
