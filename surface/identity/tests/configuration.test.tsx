/** Configuration shows server values and exposes no pretend writes or private configuration members. */
import { describe, expect, it } from 'vitest';
import { $, $$, click, mount, settle, text, type, unreachable } from './harness';
import { SERVICE, ok, refused } from './fixtures';
const settings = { source: 'startup_configuration', mutable_in_browser: false, sign_in: { provider_origin: 'https://login.test', session_seconds: 3600, secure_cookie: true }, directory: { roles_configured: true }, permissions: { model_version: 8, projection: 'spicedb' }, secrets: { configured: true }, runtimes: { machines_configured: true, provisioning_configured: true }, storage: { directory_format: 'signed_leaf_log', grant_format: 'signed_leaf_log', requests_configured: true }, client_secret: 'never-render-this-extra-value' };
describe('Effective configuration', () => {
  it.each([['signin', '1 hour'], ['directory', 'Role records'], ['permissions', 'SpiceDB'], ['secrets', 'Broker: Configured'], ['runtimes', 'Provisioning records'], ['storage', 'signed_leaf_log']])('reads %s from the configured service', async (section, expected) => {
    const { posted, requests } = await mount('#/settings/' + section, { ...SERVICE, '/configuration': ok(settings) });
    expect(requests).toContain('/configuration'); expect(text()).toContain(expected);
    expect(text()).not.toContain('not built yet'); expect(text()).not.toContain('never-render-this-extra-value');
    expect(posted).toEqual([]); expect(unreachable()).toEqual([]);
  });
  it('shows the refusal for a caller who cannot read startup settings', async () => {
    await mount('#/settings/storage', { ...SERVICE, '/configuration': refused(403, 'NotAdmitted', 'Administrator only') });
    expect(text()).toContain('NotAdmitted'); expect(text()).not.toContain('signed_leaf_log');
  });
  it('declares each model\'s context window in a table, lists the models seen with none, and saves the whole table at once', async () => {
    const organisation = { zone: 'Australia/Sydney', version: 3, by: 'person', at: 1, model_windows: { 'claude-fable-5-1': 200000, 'claude-haiku-4-5': null } };
    const { posted } = await mount('#/settings', { ...SERVICE, '/configuration': ok({ ...settings, organisation, models_undeclared: ['gpt-6'] }), 'PUT /configuration': ok({ ...organisation, version: 4 }) });
    const cells = (model: string) => { const row = $('[data-model="' + model + '"]'); return [row?.querySelector<HTMLInputElement>('input[type=text]')?.value, row?.querySelector<HTMLInputElement>('input[type=checkbox]')?.checked]; };
    expect($$('[data-model]').map((each) => each.getAttribute('data-model'))).toEqual(['claude-fable-5-1', 'claude-haiku-4-5', 'gpt-6']);
    expect(cells('claude-fable-5-1')).toEqual(['200,000', false]);
    expect(cells('claude-haiku-4-5')).toEqual(['', true]);
    expect($('[data-model="gpt-6"]')?.textContent).toContain('Seen in calls; nothing declared.');
    // A window that is not a count of tokens is said beside it, and the table cannot be sent until it is one.
    await type($('[data-model="gpt-6"] input[type=text]'), 'lots');
    expect($('[data-model="gpt-6"] [role=alert]')?.textContent).toContain('Write a count of tokens');
    expect($('[data-act="save-model-windows"]')?.hasAttribute('disabled')).toBe(true);
    await type($('[data-model="gpt-6"] input[type=text]'), '400,000');
    // Another model is added by name; left with no window and no tick, it declares nothing and is not sent.
    await type($('[aria-label="Another model\'s name"]'), 'unfilled-model');
    await click($('[data-act="add-model"]'));
    expect($$('[data-model]').length).toBe(4);
    await click($('[data-act="save-model-windows"]'));
    await settle();
    expect(posted).toEqual([{ path: 'PUT /configuration', body: { zone: 'Australia/Sydney', version: 3, model_windows: { 'claude-fable-5-1': 200000, 'claude-haiku-4-5': null, 'gpt-6': 400000 } } }]);
  });
  it('says by name why the table was not saved', async () => {
    const organisation = { zone: 'Australia/Sydney', version: 3, by: 'person', at: 1 };
    await mount('#/settings', { ...SERVICE, '/configuration': ok({ ...settings, organisation, models_undeclared: [] }), 'PUT /configuration': refused(409, 'ConfigurationVersionConflict', 'the setting changed since it was read') });
    expect($$('[data-model]')).toEqual([]);
    await click($('[data-act="save-model-windows"]'));
    await settle();
    expect(text()).toContain('The table was not saved. ConfigurationVersionConflict the setting changed since it was read');
  });
});
