/** Agent handles disclose metadata only; provisioning changes retain their operation and reviewed version. */
import { act } from 'react';
import { beforeEach, describe, expect, it } from 'vitest';
import { $, click, mount, settle, text, unmountAll } from './harness';
import { ADA, OWN, SCRIBE, SERVICE, ok, refused } from './fixtures';
import type { ProvisioningAnswer, ProvisioningProfile } from '../src/features/provisioning/Provisioning';

beforeEach(() => sessionStorage.clear());
const path = '/agents/' + SCRIBE + '/provisioning';
const profile: ProvisioningProfile = { version: 1, operation: 'op-' + 'a'.repeat(32), instructions: 'Check every receipt', note: 'Initial profile', model_access: ['model-one'], tools: ['reader'], skills: ['review'], mcp_servers: [{ name: 'Cambium', url: 'http://localhost:6010', channel: 'wake' }, { name: 'Excalidraw', command: { program: '/opt/mcp/excalidraw', args: ['--stdio'], env: { PORT: 3000, VERBOSE: false, TOKEN: { handle: 'excalidraw' } } }, channel: 'off' }], set_by: ADA, set_at: 1790000000,
  harness: { kind: 'claude_code', program: '/opt/seat/bin/claude', package: 'claude-code-seat' },
  permissions: { allow: ['Read'], deny: ['Bash(rm:*)'], ask: ['Edit'], default_mode: 'acceptEdits', additional_directories: ['/srv/a', '/srv/b'] } };
const answer: ProvisioningAnswer = { agent: SCRIBE, profile, versions: [{ version: 1, set_by: ADA, set_at: 1790000000, note: 'Initial profile' }], enforced: false };
const routes = { ...SERVICE, [path]: ok(answer) };
const button = (label: string) => [...document.querySelectorAll('button')].find((entry) => entry.textContent === label) ?? null;
async function submitProfile() {
  const form = $('form[aria-label="Record provisioning profile"]');
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
  it('shows actual profile words, history and recorded-only status', async () => {
    const { posted } = await mount('#/file/' + SCRIBE + '/provisioning', routes);
    expect(text()).toContain('Check every receipt'); expect(text()).toContain('Initial profile');
    expect(text()).toContain('no runtime is applying'); expect(posted).toEqual([]);
  });
  it('records a new version with the reviewed from_version and explicit declarations', async () => {
    const { posted } = await mount('#/file/' + SCRIBE + '/provisioning', { ...routes, ['POST ' + path]: (body) => ok({ ...answer, profile: { ...profile, ...(body as Record<string, unknown>), version: 2 } }) });
    await submitProfile();
    expect(posted).toHaveLength(1);
    expect(posted[0]).toMatchObject({ path, body: { from_version: 1, operation: expect.stringMatching(/^op-/), model_access: ['model-one'], tools: ['reader'], skills: ['review'], instructions: 'Check every receipt', note: 'Review profile' } });
    expect(text()).toContain('Provisioning profile recorded.');
  });
  it('records again every member the start carries, as it was recorded', async () => {
    const { posted } = await mount('#/file/' + SCRIBE + '/provisioning', { ...routes, ['POST ' + path]: (body) => ok({ ...answer, profile: { ...profile, ...(body as Record<string, unknown>), version: 2 } }) });
    await submitProfile();
    expect(posted[0]).toMatchObject({ path, body: { harness: profile.harness, mcp_servers: profile.mcp_servers, permissions: profile.permissions } });
  });
  it('shows the harness, each server as started and the permissions', async () => {
    await mount('#/file/' + SCRIBE + '/provisioning', routes);
    expect(text()).toContain('Claude Code · /opt/seat/bin/claude · package claude-code-seat');
    expect(text()).toContain('/opt/mcp/excalidraw --stdio · PORT = 3000 · VERBOSE = false · TOKEN = the handle on excalidraw');
    expect(text()).toContain('wakes the agent'); expect(text()).toContain('Bash(rm:*)'); expect(text()).toContain('Permission mode: acceptEdits.');
  });
  it('records no harness and no permissions when none are given, and refuses half a harness', async () => {
    const bare = { ...profile, harness: null, permissions: null, mcp_servers: [] };
    const { posted } = await mount('#/file/' + SCRIBE + '/provisioning', { ...routes, [path]: ok({ ...answer, profile: bare }), ['POST ' + path]: (body) => ok({ ...answer, profile: { ...bare, ...(body as Record<string, unknown>), version: 2 } }) });
    await submitProfile();
    expect(posted[0]).toMatchObject({ body: { harness: null, permissions: null, mcp_servers: [] } });
    unmountAll(); document.body.innerHTML = ''; sessionStorage.clear();
    const half = await mount('#/file/' + SCRIBE + '/provisioning', { ...routes, [path]: ok({ ...answer, profile: bare }) });
    const program = document.querySelector<HTMLInputElement>('[name="harness-program"]');
    if (!program) throw new Error('Harness program missing');
    program.value = '/opt/seat/bin/claude';
    await submitProfile();
    expect(half.posted).toEqual([]); expect(text()).toContain('both its program and its package');
  });
  it('retries the original operation and version after an uncertain answer and remount', async () => {
    const first = await mount('#/file/' + SCRIBE + '/provisioning', { ...routes, ['POST ' + path]: refused(503, 'ProvisioningUnavailable', 'Result uncertain') });
    await submitProfile();
    unmountAll(); document.body.innerHTML = '';
    const next = await mount('#/file/' + SCRIBE + '/provisioning', { ...routes, ['POST ' + path]: (body) => ok({ ...answer, profile: { ...profile, ...(body as Record<string, unknown>), version: 2 } }) });
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
    expect(next.posted).toEqual(first.posted); expect(text()).toContain('Provisioning profile recorded.');
    expect(text()).toContain('Later profile instructions'); expect(sessionStorage.length).toBe(0);
  });
  it('holds an inconsistent receipt even when its latest profile matches the operation', async () => {
    await mount('#/file/' + SCRIBE + '/provisioning', { ...routes, ['POST ' + path]: (body) => ok({ ...answer, profile: { ...profile, ...(body as Record<string, unknown>), version: 2 }, recorded: { operation: 'op-' + 'f'.repeat(32), version: 2 } }) });
    await submitProfile(); expect(text()).not.toContain('Provisioning profile recorded.'); expect(sessionStorage.length).toBe(1);
  });
  it('leaves the edit form absent for a non-administrator', async () => {
    await mount('#/file/' + SCRIBE + '/provisioning', { ...routes, '/directory/people': refused(403, 'NotAdmitted', 'Not an administrator'), '/people': ok(OWN) });
    expect($('form[aria-label="Record provisioning profile"]')).toBeNull();
    expect(text()).toContain('Check every receipt');
  });
});
