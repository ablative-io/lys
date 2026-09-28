/** Configuration shows server values and exposes no pretend writes or private configuration members. */
import { describe, expect, it } from 'vitest';
import { mount, text, unreachable } from './harness';
import { SERVICE, ok, refused } from './fixtures';
const settings = { source: 'startup_configuration', mutable_in_browser: false, sign_in: { provider_origin: 'https://login.test', session_seconds: 3600, secure_cookie: true }, directory: { roles_configured: true }, permissions: { model_version: 8, projection: 'spicedb' }, secrets: { configured: true }, runtimes: { machines_configured: true, provisioning_configured: true }, storage: { directory_format: 'signed_leaf_log', grant_format: 'signed_leaf_log', requests_configured: true }, client_secret: 'never-render-this-extra-value' };
describe('Effective configuration', () => {
  it.each([['signin', '3600 seconds'], ['directory', 'Role records'], ['permissions', 'SpiceDB'], ['secrets', 'Broker: Configured'], ['runtimes', 'Provisioning records'], ['storage', 'signed_leaf_log']])('reads %s from the configured service', async (section, expected) => {
    const { posted, requests } = await mount('#/settings/' + section, { ...SERVICE, '/configuration': ok(settings) });
    expect(requests).toContain('/configuration'); expect(text()).toContain(expected);
    expect(text()).not.toContain('not built yet'); expect(text()).not.toContain('never-render-this-extra-value');
    expect(posted).toEqual([]); expect(unreachable()).toEqual([]);
  });
  it('shows the refusal for a caller who cannot read startup settings', async () => {
    await mount('#/settings/storage', { ...SERVICE, '/configuration': refused(403, 'NotAdmitted', 'Administrator only') });
    expect(text()).toContain('NotAdmitted'); expect(text()).not.toContain('signed_leaf_log');
  });
});
