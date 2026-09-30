/** Agent handles disclose metadata only; provisioning changes retain their operation and reviewed version. */
import { act } from 'react';
import { beforeEach, describe, expect, it } from 'vitest';
import { $, click, mount, settle, text, unmountAll } from './harness';
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
async function submitProfile() {
  const form = $('form[aria-label="This agent\'s settings"]');
  if (!form) throw new Error('Profile form missing');
  const note = form.querySelector<HTMLInputElement>('[name="note"]');
  if (!note) throw new Error('Change reason missing');
  note.value = 'Review profile';
  await act(async () => { form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })); });
  await settle();
}

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
  const saved = (body: unknown) => ok({ ...answer, profile: { ...profile, ...(body as Record<string, unknown>), version: 2 } });
  it('shows the settings once, as the form they are changed in, with no refresh button', async () => {
    const { posted } = await mount('#/file/' + SCRIBE + '/provisioning', routes);
    expect(document.querySelector<HTMLTextAreaElement>('[name="instructions"]')?.value).toBe('Check every receipt');
    expect(document.querySelector<HTMLTextAreaElement>('[name="allow"]')?.value).toBe('Read\nreader');
    expect(button('Refresh profile')).toBeNull(); expect(text()).toContain('added to the end of the program\'s own system prompt');
    expect(posted).toEqual([]);
  });
  it('records a new version with its from_version, the tools folded into the allow rules', async () => {
    const { posted } = await mount('#/file/' + SCRIBE + '/provisioning', { ...routes, ['POST ' + path]: saved });
    await submitProfile();
    expect(posted).toHaveLength(1);
    expect(posted[0]).toMatchObject({ path, body: { from_version: 1, operation: expect.stringMatching(/^op-/), model_access: ['model-one'], tools: [], skills: ['review'], instructions: 'Check every receipt', note: 'Review profile' } });
    expect((posted[0].body as { permissions: { allow: string[] } }).permissions.allow).toEqual(['Read', 'reader']);
    expect(text()).toContain('Settings saved as a new version');
  });
  it('records again every member the start carries, exactly as it was recorded', async () => {
    const { posted } = await mount('#/file/' + SCRIBE + '/provisioning', { ...routes, ['POST ' + path]: saved });
    await submitProfile();
    expect(posted[0]).toMatchObject({ path, body: { harness: profile.harness, mcp_servers: profile.mcp_servers, permissions: { ...profile.permissions, allow: ['Read', 'reader'] } } });
  });
  it('shows each connected service as it is started, with its secret by name', async () => {
    await mount('#/file/' + SCRIBE + '/provisioning', routes);
    const values = [...document.querySelectorAll<HTMLInputElement | HTMLTextAreaElement>('fieldset input, fieldset textarea')].map((entry) => entry.value);
    expect(values).toContain('/opt/mcp/excalidraw'); expect(values).toContain('--stdio\n  spaced  \n'); expect(values).toContain('http://localhost:6010');
    expect(text()).toContain('wake the agent when it is idle');
  });
  it('records no program and no permissions when none are given', async () => {
    const bare = { ...profile, harness: null, permissions: null, tools: [], mcp_servers: [] };
    const { posted } = await mount('#/file/' + SCRIBE + '/provisioning', { ...routes, [path]: ok({ ...answer, profile: bare }), ['POST ' + path]: (body) => ok({ ...answer, profile: { ...bare, ...(body as Record<string, unknown>), version: 2 } }) });
    await submitProfile();
    expect(posted[0]).toMatchObject({ body: { harness: null, permissions: null, mcp_servers: [] } });
  });
  it('retries the original operation and version after an uncertain answer and remount', async () => {
    const first = await mount('#/file/' + SCRIBE + '/provisioning', { ...routes, ['POST ' + path]: refused(503, 'ProvisioningUnavailable', 'Result uncertain') });
    await submitProfile();
    unmountAll(); document.body.innerHTML = '';
    const next = await mount('#/file/' + SCRIBE + '/provisioning', { ...routes, ['POST ' + path]: saved });
    expect(text()).toContain('original details are retained');
    await click(button('Check original change'));
    expect(next.posted).toEqual(first.posted);
  });
  it('accepts the original receipt after a later profile version, without replacing that later profile', async () => {
    const first = await mount('#/file/' + SCRIBE + '/provisioning', { ...routes, ['POST ' + path]: refused(503, 'ProvisioningUnavailable', 'Result uncertain') });
    await submitProfile(); unmountAll(); document.body.innerHTML = '';
    const later = { ...answer, profile: { ...profile, version: 3, operation: 'op-' + 'b'.repeat(32), instructions: 'Later profile instructions' } };
    const next = await mount('#/file/' + SCRIBE + '/provisioning', { ...routes, [path]: ok(later), ['POST ' + path]: (body) => ok({ ...later, recorded: { operation: (body as Record<string, unknown>).operation, version: 2 } }) });
    await click(button('Check original change'));
    expect(next.posted).toEqual(first.posted); expect(text()).toContain('Settings saved as a new version');
    expect(document.querySelector<HTMLTextAreaElement>('[name="instructions"]')?.value).toBe('Later profile instructions'); expect(sessionStorage.length).toBe(0);
  });
  it('holds an inconsistent receipt even when its latest profile matches the operation', async () => {
    await mount('#/file/' + SCRIBE + '/provisioning', { ...routes, ['POST ' + path]: (body) => ok({ ...answer, profile: { ...profile, ...(body as Record<string, unknown>), version: 2 }, recorded: { operation: 'op-' + 'f'.repeat(32), version: 2 } }) });
    await submitProfile(); expect(text()).not.toContain('Settings saved as a new version'); expect(sessionStorage.length).toBe(1);
  });
  it('shows the settings unchangeable to a non-administrator', async () => {
    await mount('#/file/' + SCRIBE + '/provisioning', { ...routes, '/directory/people': refused(403, 'NotAdmitted', 'Not an administrator'), '/people': ok(OWN) });
    expect(button('Save these settings')).toBeNull();
    expect(document.querySelector<HTMLTextAreaElement>('[name="instructions"]')?.value).toBe('Check every receipt');
  });
});
