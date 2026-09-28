/** The administrator sets Google, Microsoft or GitHub sign-in from Connections; the secret is sent once and never shown. */
import { act } from 'react';
import { describe, expect, it } from 'vitest';
import { $, $$, click, mount, settle, text } from './harness';
import { SERVICE, ok, refused } from './fixtures';
import type { ProvidersView } from '../src/features/connections/SignInProviders';

const connections = {
  connections: [
    { id: 'sign_in', name: 'Sign-in provider', purpose: 'Authenticates people.', state: 'configured', endpoint: 'http://localhost:18080' },
    { id: 'permissions', name: 'Permission engine', purpose: 'Checks grants.', state: 'local', endpoint: null },
    { id: 'secrets', name: 'Secrets broker', purpose: 'Provides secret access.', state: 'unconfigured', endpoint: null },
  ], health_checked: false,
};
const REDIRECT = 'http://localhost:8490/auth/v1/providers/callback';
const none: ProvidersView = { providers: [], offered: ['google', 'microsoft', 'github'], redirect_address: REDIRECT };
const google: ProvidersView['providers'][number] = { id: 'provider-1', provider: 'google', name: 'Google', enabled: true, client_id: '123.apps.googleusercontent.com' };
const button = (label: string) => [...document.querySelectorAll('button')].find((entry) => entry.textContent === label) ?? null;

async function input(selector: string, value: string) {
  const element = $(selector);
  if (!(element instanceof HTMLInputElement)) throw new Error('Input missing: ' + selector);
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')?.set?.call(element, value);
    element.dispatchEvent(new Event('input', { bubbles: true }));
  });
  await settle();
}

describe('Sign-in providers', () => {
  it('shows the callback address, sends the credentials once and lists the provider set', async () => {
    let held = none;
    const { posted } = await mount('#/connections', {
      ...SERVICE,
      '/connections': ok(connections),
      '/sign-in-providers': () => ok(held),
      'POST /sign-in-providers': (body) => {
        const asked = body as Record<string, unknown>;
        held = { ...none, providers: [{ ...google, client_id: String(asked.client_id) }] };
        return ok(held);
      },
    });
    expect(text()).toContain('No sign-in provider is set yet');
    expect(($('#sign-in-redirect') as HTMLInputElement | null)?.value).toBe(REDIRECT);
    expect(text()).toContain('Google Cloud credentials');
    expect(text()).toContain('A provider already registered with the old address needs the new one added.');
    expect($$('form[aria-label="Set a sign-in provider"] input[type="password"]')).toHaveLength(1);
    await input('form[aria-label="Set a sign-in provider"] input:not([type="password"])', '123.apps.googleusercontent.com');
    await input('form[aria-label="Set a sign-in provider"] input[type="password"]', 'GOCSPX-secret');
    await click(button('Set Google'));
    expect(posted).toEqual([{ path: '/sign-in-providers', body: { provider: 'google', client_id: '123.apps.googleusercontent.com', client_secret: 'GOCSPX-secret' } }]);
    expect(text()).toContain('Google is set');
    expect($$('table.table tbody tr')).toHaveLength(1);
    expect(text()).toContain('123.apps.googleusercontent.com');
    expect(text()).not.toContain('GOCSPX-secret');
    const secret = $('form[aria-label="Set a sign-in provider"] input[type="password"]');
    expect(secret instanceof HTMLInputElement && secret.value).toBe('');
  });
  it('asks Microsoft for its tenant and names a refusal without repeating the change', async () => {
    const { posted } = await mount('#/connections', {
      ...SERVICE,
      '/connections': ok(connections),
      '/sign-in-providers': ok(none),
      'POST /sign-in-providers': refused(502, 'SignInProvidersRefused', 'the issuer answered 400: name is invalid'),
    });
    await click($('[role="radiogroup"] [data-provider="microsoft"]'));
    expect(text()).toContain('Microsoft Entra app registrations');
    expect($$('form[aria-label="Set a sign-in provider"] input')).toHaveLength(3);
    await input('form[aria-label="Set a sign-in provider"] input:not([type="password"])', 'app-id');
    await input('form[aria-label="Set a sign-in provider"] input[type="password"]', 'secret');
    expect(button('Set Microsoft')?.hasAttribute('disabled')).toBe(true);
    await input('form[aria-label="Set a sign-in provider"] input[placeholder]', 'contoso.onmicrosoft.com');
    await click(button('Set Microsoft'));
    expect(posted).toEqual([{ path: '/sign-in-providers', body: { provider: 'microsoft', client_id: 'app-id', client_secret: 'secret', tenant: 'contoso.onmicrosoft.com' } }]);
    expect(text()).toContain('SignInProvidersRefused');
    expect(posted).toHaveLength(1);
  });
  it('says when the service has no issuer API to set providers through', async () => {
    await mount('#/connections', {
      ...SERVICE,
      '/connections': ok(connections),
      '/sign-in-providers': refused(503, 'SignInProvidersUnavailable', 'the configuration names no sign_in_providers'),
    });
    expect(text()).toContain('SignInProvidersUnavailable');
    expect($$('form[aria-label="Set a sign-in provider"]')).toHaveLength(0);
  });
});
