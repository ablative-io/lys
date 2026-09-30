/** Agent handles disclose metadata only; provisioning changes retain their operation and reviewed version. */
import { beforeEach, describe, expect, it } from 'vitest';
import { mount, text } from './harness';
import { ADA, OWN, SCRIBE, SERVICE, ok, refused } from './fixtures';
import type { ProvisioningAnswer, ProvisioningProfile } from '../src/features/provisioning/Provisioning';

beforeEach(() => sessionStorage.clear());
const path = '/agents/' + SCRIBE + '/provisioning';
const profile: ProvisioningProfile = { version: 1, operation: 'op-' + 'a'.repeat(32), instructions: 'Check every receipt', note: 'Initial profile', model_access: ['model-one'], tools: ['reader'], skills: ['review'], mcp_servers: [{ name: 'Cambium', url: 'http://localhost:6010', channel: 'wake' }, { name: 'Excalidraw', command: { program: '/opt/mcp/excalidraw', args: ['--stdio', '  spaced  ', ''], env: { PORT: 3000, VERBOSE: false, TOKEN: { handle: 'excalidraw' } } }, channel: 'off' }], set_by: ADA, set_at: 1790000000,
  harness: { name: 'Claude Code', description: { models: { minimum: 1, maximum: null, further_encoding: { kind: 'delimited', separator: ',' } }, permissions: { modes: ['acceptEdits'], rule_forms: ['tool_specifier'] }, mcp: { transports: ['stdio', 'http'], working_directory: false, handle_variables: true, channel_policies: ['off', 'wake'] }, rendering_contract: 'claude-code/template-v1' }, program: '/opt/seat/bin/claude', package: 'claude-code-seat' },
  permissions: { allow: ['Read'], deny: ['Bash(rm:*)'], ask: ['Edit'], default_mode: 'acceptEdits', additional_directories: ['/srv/a', '/srv/b'] } };
const answer: ProvisioningAnswer = { agent: SCRIBE, profile, versions: [{ version: 1, set_by: ADA, set_at: 1790000000, note: 'Initial profile' }], enforced: false };
const routes = { ...SERVICE, [path]: ok(answer) };
const button = (label: string) => [...document.querySelectorAll('button')].find((entry) => entry.textContent === label) ?? null;
describe('Agent credential handles', () => {
  it('reads the named agent and renders metadata without any secret-shaped extra members', async () => {
    const { posted, requests } = await mount('#/file/' + SCRIBE + '/credentials', { ...SERVICE,
      ['/secrets/handles?holder=' + SCRIBE]: ok({ holder: SCRIBE, handles: [{ id: 'public-handle-id', secret: 'calendar', max_uses: 10, used: 3, not_after_ms: 1990000000000, dropped: false, spend_cap: null, settled: 0, parent: null, value: 'never-print-value', token: 'never-print-token' }] }),
    });
    expect(requests).toContain('/secrets/handles?holder=' + SCRIBE);
    expect(text()).toContain('public-handle-id'); expect(text()).toContain('3 of 10 uses'); expect(text()).toContain('Never shown');
    expect(text()).not.toContain('never-print'); expect(posted).toEqual([]);
  });
  it('names a broker refusal instead of showing an empty handle list', async () => {
    await mount('#/file/' + SCRIBE + '/credentials', { ...SERVICE, ['/secrets/handles?holder=' + SCRIBE]: refused(502, 'SecretsUnavailable', 'Broker not reached') });
    expect(text()).toContain('SecretsUnavailable'); expect(text()).not.toContain('No handles visible');
  });
  it('refuses a handles answer naming a different holder', async () => {
    await mount('#/file/' + SCRIBE + '/credentials', { ...SERVICE, ['/secrets/handles?holder=' + SCRIBE]: ok({ holder: ADA, handles: [] }) });
    expect(text()).toContain('did not answer handles'); expect(text()).not.toContain('No handles visible');
  });
});

describe('Agent provisioning', () => {
  it('reads a saved profile without exposing the removed settings fields', async () => {
    const { posted } = await mount('#/file/' + SCRIBE + '/provisioning', routes);
    for (const name of ['allow', 'ask', 'deny', 'writable', 'folders', 'note']) {
      expect(document.querySelector('[name="' + name + '"]')).toBeNull();
    }
    expect(button('Save these settings')).toBeNull();
    expect(button('Approve these settings')).toBeNull();
    expect(posted).toEqual([]);
  });
  it('does not rewrite an old profile just by reading its connected services', async () => {
    const { posted } = await mount('#/file/' + SCRIBE + '/provisioning', routes);
    expect(text()).not.toContain('/opt/mcp/excalidraw');
    expect(posted).toEqual([]);
  });
  it('does not offer the removed save action to a non-administrator', async () => {
    const { posted } = await mount('#/file/' + SCRIBE + '/provisioning', { ...routes, '/directory/people': refused(403, 'NotAdmitted', 'Not an administrator'), '/people': ok(OWN) });
    expect(button('Save these settings')).toBeNull();
    expect(posted).toEqual([]);
  });
});
