/** Agent handles disclose metadata only; provisioning changes retain their operation and reviewed version. */
import { beforeEach, describe, expect, it } from 'vitest';
import { click, mount, text } from './harness';
import { ADA, OWN, SCRIBE, SERVICE, ok, refused } from './fixtures';
import type { ProvisioningAnswer, ProvisioningProfile } from '../src/features/provisioning/Provisioning';

beforeEach(() => sessionStorage.clear());
const path = '/agents/' + SCRIBE + '/provisioning';
const profile: ProvisioningProfile = { version: 1, operation: 'op-' + 'a'.repeat(32), instructions: 'Check every receipt', note: 'Initial profile', model_access: ['model-one'], tools: ['reader'], skills: ['review'], mcp_servers: [{ name: 'Cambium', url: 'http://localhost:6010', channel: 'wake' }, { name: 'Excalidraw', command: { program: '/opt/mcp/excalidraw', args: ['--stdio', '  spaced  ', ''], env: { PORT: 3000, VERBOSE: false, TOKEN: { handle: 'excalidraw' } } }, channel: 'off' }], set_by: ADA, set_at: 1790000000,
  harness: { name: 'Claude Code', description: { models: { minimum: 1, maximum: null, further_encoding: { kind: 'delimited', separator: ',' } }, permissions: { modes: ['acceptEdits'], rule_forms: ['tool_specifier'] }, mcp: { transports: ['stdio', 'http'], working_directory: false, handle_variables: true, channel_policies: ['off', 'wake'] }, rendering_contract: 'claude-code/template-v1' }, program: '/opt/seat/bin/claude', package: 'claude-code-seat' },
  permissions: { allow: ['Read'], deny: ['Bash(rm:*)'], ask: ['Edit'], default_mode: 'acceptEdits', additional_directories: ['/srv/a', '/srv/b'] } };
const answer: ProvisioningAnswer = { agent: SCRIBE, profile, versions: [{ version: 1, set_by: ADA, set_at: 1790000000, note: 'Initial profile' }], enforced: false };
const routes = { ...SERVICE, [path]: ok(answer) };
const button = (label: string) => [...document.querySelectorAll('button')].find((entry) => (entry.getAttribute('aria-label') ?? entry.textContent) === label) ?? null;
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
  it('reads a saved profile without exposing the removed settings fields, and sends nothing', async () => {
    const { posted } = await mount('#/file/' + SCRIBE + '/provisioning', routes);
    for (const name of ['allow', 'ask', 'deny', 'writable', 'folders', 'note']) {
      expect(document.querySelector('[name="' + name + '"]')).toBeNull();
    }
    // Save is the form's one button; on a profile nobody has changed it is greyed, with the reason beside it.
    expect(button('Save these settings')?.disabled).toBe(true);
    expect(document.querySelector('form.save-settings .why-not')?.textContent).toBeTruthy();
    expect(button('Approve these settings')).toBeNull();
    expect(posted).toEqual([]);
  });
  it('shows an old profile’s connected tools in words and does not rewrite it just by reading them', async () => {
    const { posted } = await mount('#/file/' + SCRIBE + '/provisioning', routes);
    expect(text()).toContain('CambiumReached at http://localhost:6010, and its messages wake the agent');
    expect(text()).toContain('ExcalidrawRuns /opt/mcp/excalidraw --stdio');
    expect(text()).not.toContain('"command"');
    expect(posted).toEqual([]);
  });
  it('greys Save for a non-administrator and says an administrator changes these settings', async () => {
    const { posted } = await mount('#/file/' + SCRIBE + '/provisioning', { ...routes, '/directory/people': refused(403, 'NotAdmitted', 'Not an administrator'), '/people': ok(OWN) });
    expect(button('Save these settings')?.disabled).toBe(true);
    expect(text()).toContain('An administrator changes these settings.');
    expect(posted).toEqual([]);
  });
});

describe('The folder an agent works in', () => {
  it('shows the saved folder in the settings, and never the old promise of a folder of its own', async () => {
    await mount('#/file/' + SCRIBE + '/provisioning', { ...routes, [path]: ok({ ...answer, profile: { ...profile, working_folder: '/Users/ada/Developer/receipts' } }) });
    expect(text()).toContain('Works in /Users/ada/Developer/receipts');
    expect(text()).not.toContain('own folder when it starts');
  });
  it('says no folder is chosen when the profile names none', async () => {
    await mount('#/file/' + SCRIBE + '/provisioning', routes);
    expect(text()).toContain('No folder chosen yet.');
    expect(text()).not.toContain('Works in /');
  });
});

describe('Managed controls in the saved setup', () => {
  it('leaves an old setup manual and sends only an explicit choice to require controls', async () => {
    const initial = { ...profile, session: null };
    const { posted } = await mount('#/file/' + SCRIBE + '/provisioning', {
      ...routes, [path]: ok({ ...answer, profile: initial }),
      '/harnesses': ok({ programs: [{ name: 'Claude Code', line: 'Installed',
        models: [{ id: 'model-one', label: 'Model one' }],
        modes: [{ id: 'acceptEdits', meaning: 'Accept edits' }], instructions_modes: ['keep', 'append'],
        description: profile.harness?.description,
        builds: [{ name: 'Installed', program: '/opt/seat/bin/claude', package: 'claude-code-seat', from: 'profile' }],
      }] }),
      '/skills': ok({ skills: [{ name: 'review' }] }),
      ['POST ' + path]: (value) => {
        const given = value as Record<string, unknown>;
        const saved = { ...initial, ...given, version: 2, set_by: ADA, set_at: 1790000001 };
        return ok({ ...answer, profile: saved, recorded: { operation: given.operation, version: 2 } });
      },
    });
    const field = document.querySelector<HTMLInputElement>('input[name="requires_controls"]');
    expect(field).not.toBeNull(); expect(field?.checked).toBe(false);
    expect(posted).toEqual([]);
    await click(field);
    expect((button('Save these settings') as HTMLButtonElement)?.disabled).toBe(false);
    await click(button('Save these settings'));
    expect(posted.filter((entry) => entry.path === path)).toHaveLength(1);
    expect(posted.find((entry) => entry.path === path)?.body).toMatchObject({ session: { requires_controls: true } });
    expect(text()).toContain('Saved.');
  });
  it('shows a saved managed choice without changing it', async () => {
    const { posted } = await mount('#/file/' + SCRIBE + '/provisioning', { ...routes,
      [path]: ok({ ...answer, profile: { ...profile, session: { message_prefix: '', sensitive: false, requires_controls: true } } }),
    });
    expect(document.querySelector<HTMLInputElement>('input[name="requires_controls"]')?.checked).toBe(true);
    expect(posted).toEqual([]);
  });
});
