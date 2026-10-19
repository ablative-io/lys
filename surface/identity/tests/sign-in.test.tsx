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

/** Lys's configured policy as the service answers it, no default of anyone's. */
const POLICY = { length_min: 12, length_max: 48, words: 'At least 12 and at most 48 characters, with 2 of the kind: digit.' };

/** Every control on the page is one of the shared ones: fields in `.field`, buttons the act control. */
function unstyled(): string[] {
  const bad: string[] = [];
  for (const input of $$('input, select, textarea')) {
    if (!input.closest('.field')) bad.push(input.outerHTML.slice(0, 80));
  }
  for (const button of $$('button')) {
    if (!button.classList.contains('act')) bad.push(button.outerHTML.slice(0, 80));
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
    expect($$('form[aria-label="Sign in"] button.act.primary[data-symbol="enter"]')).toHaveLength(1);
    expect(unstyled()).toEqual([]);
    expect(document.querySelectorAll('link[rel="stylesheet"][href^="http"]')).toHaveLength(0);
  });

  it('posts the email and password to Lys and opens the signed-in screens', async () => {
    const { posted } = await mount('#/sign-in', { ...SERVICE, '/me': ok(ME), 'POST /sign-in': ok({ signed_in: { issuer: 'x', subject: 'y' }, authority: 'a' }) });
    await fill({ email: 'ada@example.test', password: 'Analytical-Engine-1843' });
    await submit('Sign in');
    expect(posted).toEqual([{ path: '/sign-in', body: { email: 'ada@example.test', password: 'Analytical-Engine-1843' } }]);
    expect(location.hash).toBe('#/');
  });

  it('says a wrong email or password in one sentence that names neither', async () => {
    await mount('#/sign-in', { ...SERVICE, 'POST /sign-in': refused(401, 'SignInRefused', 'SignInRefused: the email or password is not right') });
    await fill({ email: 'ada@example.test', password: 'wrong' });
    await submit('Sign in');
    expect($('[role="alert"]')?.textContent).toBe('That email or password is not right.');
    expect(location.hash).toBe('#/sign-in');
  });
});

describe('Lys sign-in page when Lys refused to start (DIRECTORY-107 R2)', () => {
  /** GET /health as a refused start answers it: the stores short of their receipt, the command and the computer. */
  const REFUSED = {
    status: 503,
    body: {
      service: 'lys-identity-server',
      version: '0.3.1',
      refusal: 'refused_to_start',
      reason: 'refused_to_start: Lys refused to start: grant-log is short of its receipt (the receipt holds head 9 of origin 3fa2c1d09e8b; found head 8 of origin 3fa2c1d09e8b). Run `lys identity accept-loss --store grant-log` on Ward Mac to start Lys at what was found.',
      stores: [{ store: 'grant-log', refusal: 'store_lost', held: 'head 9 of origin 3fa2c1d09e8b', found: 'head 8 of origin 3fa2c1d09e8b' }],
      command: 'lys identity accept-loss --store grant-log',
      computer: 'Ward Mac',
    },
  };

  it('shows the store, both heads, the command and the computer instead of the form', async () => {
    await mount('#/sign-in', { ...SERVICE, '/health': REFUSED });
    expect($('form[aria-label="Sign in"]')).toBeNull();
    expect($('.page.sign-in h1')?.textContent).toBe('Lys did not start');
    expect($('[role="alert"]')?.textContent).toBe(REFUSED.body.reason.replace('refused_to_start: ', ''));
    expect(text()).toContain('grant-log: its receipt holds head 9 of origin 3fa2c1d09e8b; Lys found head 8 of origin 3fa2c1d09e8b.');
    expect(text()).toContain('by running this on Ward Mac');
    expect($('pre code')?.textContent).toBe('lys identity accept-loss --store grant-log');
  });

  it('says the same on any page a person opens, since every route answers with the same words', async () => {
    await mount('#/', { ...SERVICE, '/health': REFUSED, '/me': REFUSED });
    expect($('h1')?.textContent).toBe('Lys did not start');
    expect($('pre code')?.textContent).toBe('lys identity accept-loss --store grant-log');
  });

  it('shows the form when Lys answers its health', async () => {
    await mount('#/sign-in', { ...SERVICE, '/health': ok({ service: 'lys-identity-server', version: '0.3.1' }) });
    expect($('form[aria-label="Sign in"]')).not.toBeNull();
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
    expect($$('form[aria-label="Set up Lys"] .field input')).toHaveLength(5);
    expect($('label[for="setup-given-name"]')?.textContent).toBe('First name');
    expect($('label[for="setup-family-name"]')?.textContent).toBe('Last name');
    expect($('#setup-given-name')?.getAttribute('autocomplete')).toBe('given-name');
    expect($('#setup-family-name')?.getAttribute('autocomplete')).toBe('family-name');
    expect($('#setup-given-name')?.hasAttribute('required')).toBe(true);
    expect($('#setup-family-name')?.hasAttribute('required')).toBe(true);
    expect(($('input[name="email"]') as HTMLInputElement | null)?.value).toBe('');
    expect(unstyled()).toEqual([]);
    await fill({ given_name: 'Ada', family_name: 'Lovelace', email: 'ada@example.test', password: 'Analytical-Engine-1843', confirm: 'Analytical-Engine-1844' });
    await submit('Set up Lys');
    expect($('[role="alert"]')?.textContent).toBe('The two passwords are not the same.');
    await fill({ confirm: 'Analytical-Engine-1843' });
    await submit('Set up Lys');
    const made = page.posted.filter((entry) => entry.path === '/setup/administrator');
    expect(made).toEqual([{ path: '/setup/administrator', body: {
      code: 'setup-code-1', operation: expect.stringMatching(/^op-[0-9a-f]{32}$/),
      display_name: 'Ada Lovelace', given_name: 'Ada', family_name: 'Lovelace', email: 'ada@example.test', password: 'Analytical-Engine-1843',
    } }]);
    expect(page.finished()).toBe(1);
    page.unmount();
  });

  it('asks for a first and a last name before anything is sent', async () => {
    const page = await render({
      'POST /setup/open': ok({ purpose: 'first-run', email: null, policy: POLICY }),
      'POST /setup/administrator': ok({ person: 'person-1', receipt: {} }),
    });
    for (const [given, family] of [['Ada', ''], ['', 'Lovelace'], [' ', ' ']]) {
      await fill({ given_name: given, family_name: family, email: 'ada@example.test', password: 'Analytical-Engine-1843', confirm: 'Analytical-Engine-1843' });
      await submit('Set up Lys');
      expect($('[role="alert"]')?.textContent).toBe('Enter your first name, your last name and your email.');
    }
    expect(page.posted.filter((entry) => entry.path === '/setup/administrator')).toEqual([]);
    page.unmount();
  });

  it('refuses a password outside Lys policy with its words before anything is sent', async () => {
    const page = await render({
      'POST /setup/open': ok({ purpose: 'first-run', email: null, policy: POLICY }),
      'POST /setup/administrator': ok({ person: 'person-1', receipt: {} }),
    });
    expect($('#setup-policy')?.textContent).toBe(POLICY.words);
    // The rule stands between the label and the field, read before anything is typed.
    const policy = $('#setup-policy');
    const field = $('#setup-password');
    expect(policy?.previousElementSibling?.textContent).toBe('Password');
    expect(policy?.nextElementSibling).toBe(field);
    expect(field?.getAttribute('placeholder')).toBe('At least 12 characters');
    expect(field?.getAttribute('aria-describedby')).toBe('setup-policy');
    await fill({ given_name: 'Ada', family_name: 'Lovelace', email: 'ada@example.test', password: 'Engine-1843', confirm: 'Engine-1843' });
    await submit('Set up Lys');
    expect($('[role="alert"]')?.textContent).toBe(POLICY.words);
    const long = 'Analytical-Engine-1843-' + 'x'.repeat(26);
    await fill({ password: long, confirm: long });
    await submit('Set up Lys');
    expect($('[role="alert"]')?.textContent).toBe(POLICY.words);
    expect(page.posted.filter((entry) => entry.path === '/setup/administrator')).toEqual([]);
    page.unmount();
  });

  it('shows no policy when the service names none', async () => {
    const page = await render({ 'POST /setup/open': ok({ purpose: 'first-run', email: null, policy: null }) });
    expect($('form[aria-label="Set up Lys"]')).not.toBeNull();
    expect($('#setup-policy')).toBeNull();
    expect($('#setup-password')?.hasAttribute('placeholder')).toBe(false);
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

  it('asks for the setup code when the address carries none, names where the install put it, and opens setup with it as the link does', async () => {
    const page = await render({
      'POST /setup/open': (body) => (body as { code: string }).code === 'Zq81mTn4Rw0pLk7H'
        ? ok({ purpose: 'first-run', email: null, policy: POLICY })
        : refused(401, 'SetupCodeRefused', 'SetupCodeRefused: x'),
      'POST /setup/administrator': ok({ person: 'person-1', receipt: {} }),
    }, '');
    expect(page.posted).toEqual([]);
    expect($('[role="alert"]')).toBeNull();
    expect($('label[for="setup-code"]')?.textContent).toBe('Setup code');
    expect($('#setup-code-where')?.textContent).toBe(
      'The setup code is in the file named setup-code in the folder the install printed: use the full path on the line “the setup code is in …”. Only the account that ran the install can read it.');
    expect(unstyled()).toEqual([]);

    await fill({ code: 'wrong' });
    await submit('Setup code');
    expect($('[role="alert"]')?.textContent).toBe('This setup code is not valid or was already used. Run lys identity setup-code on the machine Lys runs on for a fresh one.');
    expect($('#setup-code')).not.toBeNull();

    await fill({ code: ' Zq81mTn4Rw0pLk7H\n' });
    await submit('Setup code');
    expect(page.posted.filter((entry) => entry.path === '/setup/open')).toEqual([
      { path: '/setup/open', body: { code: 'wrong' } },
      { path: '/setup/open', body: { code: 'Zq81mTn4Rw0pLk7H' } },
    ]);
    expect($('#setup-code')).toBeNull();
    await fill({ given_name: 'Ada', family_name: 'Lovelace', email: 'ada@example.test', password: 'Analytical-Engine-1843', confirm: 'Analytical-Engine-1843' });
    await submit('Set up Lys');
    expect(page.posted.filter((entry) => entry.path === '/setup/administrator')).toEqual([{ path: '/setup/administrator', body: {
      code: 'Zq81mTn4Rw0pLk7H', operation: expect.stringMatching(/^op-[0-9a-f]{32}$/),
      display_name: 'Ada Lovelace', given_name: 'Ada', family_name: 'Lovelace', email: 'ada@example.test', password: 'Analytical-Engine-1843',
    } }]);
    expect(page.finished()).toBe(1);
    page.unmount();
  });

  it('says an entered code needs entering, and that setup attempts are limited, in setup words', async () => {
    const page = await render({ 'POST /setup/open': refused(429, 'SignInThrottled', 'SignInThrottled: too many sign-ins failed from this address; wait a little, then try again') }, '');
    await submit('Setup code');
    expect($('[role="alert"]')?.textContent).toBe('Enter the setup code.');
    expect(page.posted).toEqual([]);
    await fill({ code: 'Zq81mTn4Rw0pLk7H' });
    await submit('Setup code');
    expect($('[role="alert"]')?.textContent).toBe('Lys takes five setup attempts a minute, and that many have been made. Wait a minute, then try again.');
    expect($('#setup-code')).not.toBeNull();
    page.unmount();
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
    expect($('[role="alert"]')?.textContent).toBe('That sign-in could not be finished. It began at a different address from this one (' + location.host + '), or it expired or was already used. Sign in again on this page.');
  });
});

describe('A product signing in through Lys', () => {
  it('carries the pending authorize request on each provider link', async () => {
    for (const target of [
      '/oauth/authorize?client_id=cambium&state=a%26b&scope=openid%20profile',
      '/oauth/mcp/authorize?client_id=connector&state=original',
    ]) {
      const { container } = await mount('#/sign-in?continue=' + encodeURIComponent(target), {
        ...SERVICE,
        '/sign-in/providers': ok({ providers: [{ id: 'provider-1', name: 'Google', provider: 'google' }] }),
      });
      expect(container.querySelector('.sign-in-providers a')?.getAttribute('href'))
        .toBe('/api/sign-in/providers/provider-1?continue=' + encodeURIComponent(target));
    }
  });

  it('never forwards a foreign continue target to a provider', async () => {
    await mount('#/sign-in?continue=' + encodeURIComponent('https://elsewhere.example.test/'), {
      ...SERVICE,
      '/sign-in/providers': ok({ providers: [{ id: 'provider-1', name: 'Google', provider: 'google' }] }),
    });
    expect($('.sign-in-providers a')?.getAttribute('href')).toBe('/api/sign-in/providers/provider-1');
  });

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
