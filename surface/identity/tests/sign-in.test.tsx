/// <reference types="vite/client" />
/**
 * Lys's own sign-in and setup pages (DIRECTORY-047 R1, R2, R6): built from the
 * surface's shared page, field and button styling, posting only to Lys, and
 * never naming the sign-in service behind it.
 */
import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { describe, expect, it } from 'vitest';
import { FirstRunSetup, forgetSetupCode, setupCode } from '../src/features/setup/Setup';
import { $, $$, mount, serve, settle, text } from './harness';
import { ME, SERVICE, ok, refused } from './fixtures';

const POLICY = { length_min: 14, length_max: 128, words: 'At least 14 characters, with a lower-case letter, an upper-case letter and a digit.' };

/** Every control on the page is one of the shared ones: fields in `.field`, buttons `.btn`. */
function unstyled(): string[] {
  const bad: string[] = [];
  for (const input of $$('input, select, textarea')) {
    if (!input.closest('.field')) bad.push(input.outerHTML.slice(0, 80));
  }
  for (const button of $$('button')) {
    if (!button.classList.contains('btn')) bad.push(button.outerHTML.slice(0, 80));
  }
  return bad;
}

async function fill(values: Record<string, string>) {
  await act(async () => {
    for (const [name, value] of Object.entries(values)) {
      const input = document.querySelector<HTMLInputElement>(`input[name="${name}"]`);
      if (!input) throw new Error(`no ${name} field`);
      input.value = value;
    }
  });
}

async function submit(label: string) {
  const form = $(`form[aria-label="${label}"]`);
  if (!form) throw new Error(`no ${label} form`);
  await act(async () => { form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })); });
  await settle();
}

describe('Lys sign-in page', () => {
  it('is built from the shared page, field and button styling', async () => {
    await mount('#/sign-in', SERVICE);
    expect($('.page.sign-in h1')?.textContent).toBe('Sign in');
    expect($$('form[aria-label="Sign in"] .field input')).toHaveLength(2);
    expect($$('form[aria-label="Sign in"] button.btn.primary')).toHaveLength(1);
    expect(unstyled()).toEqual([]);
    expect(document.querySelectorAll('link[rel="stylesheet"][href^="http"]')).toHaveLength(0);
  });

  it('posts the email and password to Lys and opens the signed-in screens', async () => {
    const { posted } = await mount('#/sign-in', { ...SERVICE, '/me': ok(ME), 'POST /sign-in': ok({ signed_in: { issuer: 'x', subject: 'y' }, authority: 'a' }) });
    await fill({ email: 'ada@example.test', password: 'Analytical-Engine-1843' });
    await submit('Sign in');
    expect(posted).toEqual([{ path: '/sign-in', body: { email: 'ada@example.test', password: 'Analytical-Engine-1843' } }]);
    expect(location.hash).toBe('#/me');
  });

  it('says a wrong email or password in one sentence that names neither', async () => {
    await mount('#/sign-in', { ...SERVICE, 'POST /sign-in': refused(401, 'SignInRefused', 'SignInRefused: the email or password is not right') });
    await fill({ email: 'ada@example.test', password: 'wrong' });
    await submit('Sign in');
    expect($('[role="alert"]')?.textContent).toBe('That email or password is not right.');
    expect(location.hash).toBe('#/sign-in');
  });
});

describe('Lys setup page', () => {
  it('takes the code from the address once and leaves it out of the address', () => {
    forgetSetupCode();
    history.replaceState(null, '', '/setup#code=Zq81mTn4Rw0pLk7H');
    expect(setupCode()).toBe('Zq81mTn4Rw0pLk7H');
    expect(location.href).not.toContain('Zq81mTn4Rw0pLk7H');
    expect(location.pathname).toBe('/setup');
    expect(setupCode()).toBe('Zq81mTn4Rw0pLk7H');
    forgetSetupCode();
  });

  async function render(routes: Parameters<typeof serve>[0], code = 'setup-code-1') {
    const posted: { path: string; body: unknown }[] = [];
    serve(routes, posted);
    let finished = 0;
    const container = document.createElement('div');
    document.body.appendChild(container);
    const root = createRoot(container);
    await act(async () => { root.render(<FirstRunSetup code={code} done={() => { finished += 1; }} />); });
    await settle();
    return { posted, finished: () => finished, unmount: () => act(() => root.unmount()) };
  }

  it('asks the name, email and password twice, shows the policy first, and makes the administrator', async () => {
    const page = await render({
      'POST /setup/open': ok({ purpose: 'first-run', email: null, policy: POLICY }),
      'POST /setup/administrator': ok({ person: 'person-1', receipt: {} }),
    });
    expect(text()).toContain(POLICY.words);
    expect($$('form[aria-label="Set up Lys"] .field input')).toHaveLength(4);
    expect(($('input[name="email"]') as HTMLInputElement | null)?.value).toBe('');
    expect(unstyled()).toEqual([]);
    await fill({ display_name: 'Ada Lovelace', email: 'ada@example.test', password: 'Analytical-Engine-1843', confirm: 'Analytical-Engine-1844' });
    await submit('Set up Lys');
    expect($('[role="alert"]')?.textContent).toBe('The two passwords are not the same.');
    await fill({ confirm: 'Analytical-Engine-1843' });
    await submit('Set up Lys');
    const made = page.posted.filter((entry) => entry.path === '/setup/administrator');
    expect(made).toEqual([{ path: '/setup/administrator', body: {
      code: 'setup-code-1', operation: expect.stringMatching(/^op-[0-9a-f]{32}$/),
      display_name: 'Ada Lovelace', email: 'ada@example.test', password: 'Analytical-Engine-1843',
    } }]);
    expect(page.finished()).toBe(1);
    page.unmount();
  });

  it('names a closed setup and a used code in Lys words', async () => {
    const closed = await render({ 'POST /setup/open': refused(409, 'SetupClosed', 'SetupClosed: x') });
    expect(text()).toContain('Lys is already set up. Sign in instead.');
    closed.unmount();
    document.body.innerHTML = '';
    const used = await render({ 'POST /setup/open': refused(401, 'SetupCodeRefused', 'SetupCodeRefused: x') });
    expect(text()).toContain('lys identity setup-code');
    expect($('form')).toBeNull();
    used.unmount();
  });

  it('sets the administrator a new password with a password code', async () => {
    const page = await render({
      'POST /setup/open': ok({ purpose: 'password', email: 'ada@example.test', policy: POLICY }),
      'POST /setup/password': ok({ signed_in: { issuer: 'x', subject: 'y' }, authority: 'a' }),
    });
    expect(text()).toContain('ada@example.test');
    expect($$('form[aria-label="New password"] .field input')).toHaveLength(2);
    await fill({ password: 'Difference-Engine-1822', confirm: 'Difference-Engine-1822' });
    await submit('New password');
    expect(page.posted.filter((entry) => entry.path === '/setup/password')).toEqual([
      { path: '/setup/password', body: { code: 'setup-code-1', password: 'Difference-Engine-1822' } },
    ]);
    expect(page.finished()).toBe(1);
    page.unmount();
  });
});

describe('What a person reads', () => {
  it('never names the sign-in service behind Lys in any screen source', () => {
    const sources = import.meta.glob('../src/**/*.{ts,tsx,css,html}', { query: '?raw', import: 'default', eager: true });
    const page = import.meta.glob('../index.html', { query: '?raw', import: 'default', eager: true });
    const all = { ...sources, ...page } as Record<string, string>;
    expect(Object.keys(all).length).toBeGreaterThan(50);
    const naming = Object.entries(all).filter(([, source]) => /rauthy/i.test(source)).map(([path]) => path);
    expect(naming).toEqual([]);
  });
});

describe('Sign-in providers on the sign-in page', () => {
  it('offers one button per provider, each starting at Lys', async () => {
    await mount('#/sign-in', { ...SERVICE, '/sign-in/providers': ok({ providers: [
      { id: 'provider-1', name: 'Google', provider: 'google' },
      { id: 'provider-2', name: 'GitHub', provider: 'github' },
    ] }) });
    const buttons = $$('.sign-in-providers a.btn');
    expect(buttons.map((entry) => entry.textContent?.trim())).toEqual(['Sign in with Google', 'Sign in with GitHub']);
    expect(buttons.map((entry) => entry.getAttribute('href'))).toEqual(['/api/sign-in/providers/provider-1', '/api/sign-in/providers/provider-2']);
  });

  it('names a provider sign-in that came back refused', async () => {
    await mount('#/sign-in?refused=SignInStateUnknown', SERVICE);
    expect($('[role="alert"]')?.textContent).toBe('That sign-in expired or was already used. Try again.');
  });
});

describe('A product signing in through Lys', () => {
  it('continues only to Lys own authorize address', async () => {
    const { continuation } = await import('../src/features/sign-in/SignIn');
    history.replaceState(null, '', '/#/sign-in?continue=' + encodeURIComponent('/oauth/authorize?client_id=p&state=s'));
    expect(continuation()).toBe('/oauth/authorize?client_id=p&state=s');
    history.replaceState(null, '', '/#/sign-in?continue=' + encodeURIComponent('https://elsewhere.example.test/'));
    expect(continuation()).toBeNull();
    history.replaceState(null, '', '/#/sign-in?continue=' + encodeURIComponent('//elsewhere.example.test/oauth/authorize?x'));
    expect(continuation()).toBeNull();
    history.replaceState(null, '', '/#/sign-in');
    expect(continuation()).toBeNull();
  });
});
