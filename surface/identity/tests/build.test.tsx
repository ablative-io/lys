/** The running build is named under Configuration, as the service's /authority answer says it; the sign-in page shows only its title and one button, with no rail beside it. */
import { describe, expect, it } from 'vitest';
import { $, $$, mount, text } from './harness';
import { BUILD, SERVICE, ok, refused } from './fixtures';

const settings = { source: 'startup_configuration', mutable_in_browser: false, sign_in: { provider_origin: 'https://login.test', session_seconds: 3600, secure_cookie: true }, directory: { roles_configured: true }, permissions: { model_version: 8, projection: 'local' }, secrets: { configured: true }, runtimes: { machines_configured: true, provisioning_configured: true }, storage: { directory_format: 'signed_leaf_log', grant_format: 'signed_leaf_log', requests_configured: true } };
const configured = { ...SERVICE, '/configuration': ok(settings) };

describe('Running build', () => {
  it('shows the commit from the /authority answer under Configuration, once', async () => {
    const { requests } = await mount('#/settings', configured);
    expect(requests).toContain('/authority');
    expect($$('dl.facts dd code').map((code) => code.textContent)).toEqual([BUILD]);
    expect(text().split(BUILD)).toHaveLength(2);
  });

  it('shows the words of a build with no commit exactly as the service says them', async () => {
    const words = 'not built from a git commit';
    await mount('#/settings', { ...configured, '/authority': ok({ authority: 'Step 1 of the directory has one administrator.', build: words }) });
    expect(text()).toContain(words);
    expect(text()).not.toContain(BUILD);
  });

  // A signed-out caller's change feed is refused as the service refuses it, so nothing reads on.
  const signedOut = { ...SERVICE, '/changes': refused(401, 'NotSignedIn', 'Sign in first'), '/directory/people': refused(401, 'NotSignedIn', 'Sign in first') };

  it('signs in with the title and one button, and no service prose or build', async () => {
    const { requests } = await mount('#/people', signedOut);
    expect($('#screen h1, .page h1')?.textContent).toBe('Sign in');
    const page = $('.page');
    expect([...(page?.querySelectorAll('a, button') ?? [])].map((each) => each.textContent)).toEqual(['Sign in']);
    expect(page?.querySelectorAll('p, section, code')).toHaveLength(0);
    expect(text()).not.toContain('Step 1 of the directory');
    expect(text()).not.toContain(BUILD);
    expect(requests).not.toContain('/authority');
  });

  it('shows no rail, help or palette beside the sign-in card while nobody is signed in', async () => {
    await mount('#/people', signedOut);
    expect($('.page.signed-out .card h1')?.textContent).toBe('Sign in');
    expect($('#rail')).toBeNull();
    expect($('[data-nav]')).toBeNull();
    expect($('#dock')).toBeNull();
    expect($('#palette')).toBeNull();
  });
});
