/** Agent handles disclose metadata only; provisioning changes retain their operation and reviewed version. */
import { act } from 'react';
import { beforeEach, describe, expect, it } from 'vitest';
import { $, click, mount, text } from './harness';
import { ADA, BEA_ME, OWN, SCRIBE, SERVICE, ok, refused } from './fixtures';
import type { ProvisioningAnswer, ProvisioningProfile } from '../src/features/provisioning/Provisioning';

beforeEach(() => sessionStorage.clear());
const path = '/agents/' + SCRIBE + '/provisioning';
const profile: ProvisioningProfile = { version: 1, operation: 'op-' + 'a'.repeat(32), instructions: 'Check every receipt', note: 'Initial profile', model_access: ['model-one'], tools: ['reader'], skills: ['review'], mcp_servers: [{ name: 'Cambium', url: 'http://localhost:6010', channel: 'wake' }, { name: 'Excalidraw', command: { program: '/opt/mcp/excalidraw', args: ['--stdio', '  spaced  ', ''], env: { PORT: 3000, VERBOSE: false, TOKEN: { handle: 'excalidraw' } } }, channel: 'off' }], set_by: ADA, set_at: 1790000000,
  harness: { name: 'Claude Code', description: { models: { minimum: 1, maximum: null, further_encoding: { kind: 'delimited', separator: ',' } }, permissions: { modes: ['acceptEdits'], rule_forms: ['tool_specifier'] }, mcp: { transports: ['stdio', 'http'], working_directory: false, handle_variables: true, channel_policies: ['off', 'wake'] }, rendering_contract: 'claude-code/template-v1' }, program: '/opt/seat/bin/claude', package: 'claude-code-seat' },
  permissions: { allow: ['Read'], deny: ['Bash(rm:*)'], ask: ['Edit'], default_mode: 'acceptEdits', additional_directories: ['/srv/a', '/srv/b'] } };
const answer: ProvisioningAnswer = { agent: SCRIBE, profile, versions: [{ version: 1, set_by: ADA, set_at: 1790000000, note: 'Initial profile' }], enforced: false };
const routes = { ...SERVICE, [path]: ok(answer) };
async function typeInto(selector: string, value: string) {
  const field = document.querySelector(selector);
  if (!(field instanceof HTMLInputElement)) throw new Error(selector + ' is missing');
  await act(async () => { Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')?.set?.call(field, value); field.dispatchEvent(new Event('input', { bubbles: true })); });
}
async function submitForm(selector: string) {
  const form = document.querySelector(selector);
  if (!(form instanceof HTMLFormElement)) throw new Error(selector + ' is missing');
  await act(async () => { form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })); });
}
const button = (label: string) => [...document.querySelectorAll('button')].find((entry) => (entry.getAttribute('aria-label') ?? entry.textContent) === label) ?? null;
describe('Agent credential handles', () => {
  it('reads the named agent and renders metadata without any secret-shaped extra members', async () => {
    const { posted, requests } = await mount('#/file/' + SCRIBE + '/credentials', { ...SERVICE,
      ['/secrets/handles?holder=' + SCRIBE]: ok({ holder: SCRIBE, handles: [{ id: 'public-handle-id', secret: 'calendar', max_uses: 10, used: 3, not_after_ms: 1990000000000, dropped: false, spend_cap: null, settled: 0, parent: null, value: 'never-print-value', token: 'never-print-token' }] }),
    });
    expect(requests).toContain('/secrets/handles?holder=' + SCRIBE);
    // The handle's identifier is a reference for whoever checks the record: kept on the row, never on its face.
    const row = document.querySelector('tr[title="Handle public-handle-id"]');
    expect(row?.textContent).toContain('3 of 10'); expect(row?.textContent).not.toContain('public-handle-id');
    expect(text()).toContain('never the secret itself');
    expect(text()).not.toContain('never-print'); expect(posted).toEqual([]);
  });
  it('names a broker refusal instead of showing an empty handle list', async () => {
    await mount('#/file/' + SCRIBE + '/credentials', { ...SERVICE, ['/secrets/handles?holder=' + SCRIBE]: refused(502, 'SecretsUnavailable', 'Broker not reached') });
    expect(text()).toContain('SecretsUnavailable'); expect(text()).not.toContain('No handles visible');
  });
  it('refuses a handles answer naming a different holder', async () => {
    await mount('#/file/' + SCRIBE + '/credentials', { ...SERVICE, ['/secrets/handles?holder=' + SCRIBE]: ok({ holder: ADA, handles: [] }) });
    expect(text()).toContain('did not answer this agent’s handles'); expect(text()).not.toContain('No handles visible');
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
  it('lets the person an agent answers to change its settings, as the service does, though they are not the administrator', async () => {
    await mount('#/file/' + SCRIBE + '/provisioning', { ...routes, '/directory/people': refused(403, 'NotAdmitted', 'Not an administrator'), '/people': ok(OWN) });
    expect(text()).not.toContain('changes these settings.');
    expect($('section[aria-label="Settings of this agent"] fieldset.settings-read-only')).toBeNull();
    expect(($('select[name="model"]') as HTMLSelectElement | null)?.disabled).toBe(false);
  });
  it('shows someone else’s agent’s settings without letting them be changed, and says who changes them', async () => {
    const { posted } = await mount('#/file/' + SCRIBE + '/provisioning', { ...routes, '/directory/people': refused(403, 'NotAdmitted', 'Not an administrator'), '/people': ok(OWN), '/me': ok(BEA_ME) });
    expect(button('Save these settings')?.disabled).toBe(true);
    expect(text()).toContain('Only the person Scribe answers to, or the administrator, changes these settings.');
    // Every field is shown as saved and none can be changed, so nothing invites a change that cannot be saved.
    expect(($('fieldset.settings-read-only') as HTMLFieldSetElement | null)?.disabled).toBe(true);
    expect(posted).toEqual([]);
  });
});

describe('The folder an agent works in', () => {
  const computer = { id: 'machine-seat', name: 'Ward desk', runtime: 'runner', state: 'in_use', may_run: [{ id: SCRIBE }] };
  const folders = 'POST /network/machines/' + computer.id + '/folders';
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
  it.each([
    ['folder_unreadable', 'The runner could not read this folder. Choose a readable folder or ask its owner to check access.'],
    ['folder_invalid', 'Lys refused: the saved folder cannot be opened'],
  ])('explains %s while retaining the named home-folder recovery', async (code, words) => {
    const saved = '/srv/removed';
    const { posted } = await mount('#/file/' + SCRIBE + '/provisioning', {
      ...routes, [path]: ok({ ...answer, profile: { ...profile, runs_on: computer.id, working_folder: saved } }),
      '/network': ok({ machines: [computer], reports_served: true }),
      [folders]: (value) => {
        const request = value as { under?: string };
        return request.under ? refused(409, code, 'the saved folder cannot be opened')
          : ok({ machine: computer.id, under: '/home/ward', folders: ['notes'] });
      },
    });
    await click(button('Choose another folder'));
    const recovery = document.querySelector('.folder-recovery');
    const warning = recovery?.querySelector('[role="alert"]');
    expect(recovery?.querySelector('p')?.textContent).toContain(saved + ', could not be read. Lys asked for the computer’s home folder instead.');
    expect(recovery?.textContent).toContain('Choose a readable folder or ask its owner to check access.');
    expect(warning?.querySelector('p')?.textContent).toBe(words);
    expect(warning?.querySelector('p')?.textContent).not.toContain(code);
    expect(warning?.querySelector('details')?.hasAttribute('open')).toBe(false);
    expect(warning?.querySelector('details')?.textContent).toContain(code + ': the saved folder cannot be opened');
    expect(document.querySelector('.folder-chooser[role="group"]')?.textContent).toContain('On Ward desk, in /home/ward');
    expect(posted.map((entry) => entry.body)).toEqual([{ under: saved }, {}]);
    await click(button('Work in ward'));
    expect(document.querySelector('.works-in code')?.textContent).toBe('/home/ward');
    expect(posted.every((entry) => entry.path.endsWith('/folders'))).toBe(true);
  });
  it('opens a typed path on the computer, goes back up by any part of it, and works in the folder chosen', async () => {
    const listing: Record<string, string[]> = { '/home/ward': ['Developer'], '/home/ward/Developer/receipts': ['src'], '/home/ward/Developer': ['receipts'] };
    const { posted } = await mount('#/file/' + SCRIBE + '/provisioning', {
      ...routes, [path]: ok({ ...answer, profile: { ...profile, runs_on: computer.id } }),
      '/network': ok({ machines: [computer], reports_served: true }),
      [folders]: (value) => { const under = (value as { under?: string }).under ?? '/home/ward'; return listing[under] ? ok({ machine: computer.id, under, folders: listing[under] }) : refused(409, 'folder_unreadable', 'no such folder'); },
    });
    await click(button('Choose a folder'));
    expect((document.querySelector('select[name="folder-computer"]') as HTMLSelectElement).selectedOptions[0]?.textContent).toBe('Ward desk');
    await typeInto('[name="folder-path"]', '/home/ward/Developer/receipts/');
    await submitForm('form.folder-path');
    expect(text()).toContain('On Ward desk, in /home/ward/Developer/receipts');
    expect(posted.at(-1)?.body).toEqual({ under: '/home/ward/Developer/receipts' });
    await click(button('Back to Developer'));
    expect(text()).toContain('On Ward desk, in /home/ward/Developer');
    await click(button('Work in Developer'));
    expect(document.querySelector('.works-in code')?.textContent).toBe('/home/ward/Developer');
  });
  it('says plainly when a typed path is not a folder there, and keeps the folder shown', async () => {
    const { posted } = await mount('#/file/' + SCRIBE + '/provisioning', {
      ...routes, [path]: ok({ ...answer, profile: { ...profile, runs_on: computer.id } }),
      '/network': ok({ machines: [computer], reports_served: true }),
      [folders]: (value) => (value as { under?: string }).under ? refused(409, 'folder_unreadable', 'no such folder') : ok({ machine: computer.id, under: '/home/ward', folders: ['notes'] }),
    });
    await click(button('Choose a folder'));
    await typeInto('[name="folder-path"]', '/nowhere/at/all');
    await submitForm('form.folder-path');
    expect(document.querySelector('.folder-path [role="alert"]')?.textContent).toBe('There is no folder Lys can open at /nowhere/at/all on this computer. Check the path, or choose from the folders below.');
    expect(text()).toContain('On Ward desk, in /home/ward');
    expect(posted.map((entry) => entry.body)).toEqual([{}, { under: '/nowhere/at/all' }]);
  });
  it('asks for a whole path before asking the computer anything', async () => {
    const { posted } = await mount('#/file/' + SCRIBE + '/provisioning', {
      ...routes, [path]: ok({ ...answer, profile: { ...profile, runs_on: computer.id } }),
      '/network': ok({ machines: [computer], reports_served: true }),
      [folders]: ok({ machine: computer.id, under: '/home/ward', folders: [] }),
    });
    await click(button('Choose a folder'));
    await typeInto('[name="folder-path"]', 'Developer/receipts');
    await submitForm('form.folder-path');
    expect(document.querySelector('.folder-path [role="alert"]')?.textContent).toContain('Type the whole path, starting with /');
    expect(posted).toHaveLength(1);
  });
  it('explains a folder read refusal with a next action and keeps the server code in details', async () => {
    const { posted } = await mount('#/file/' + SCRIBE + '/provisioning', {
      ...routes, [path]: ok({ ...answer, profile: { ...profile, runs_on: computer.id } }),
      '/network': ok({ machines: [computer], reports_served: true }),
      [folders]: refused(409, 'runner_absent', 'machine names no runner'),
    });
    await click(button('Choose a folder'));
    const failure = document.querySelector('.folder-problem');
    const warning = failure?.querySelector('[role="alert"]');
    expect(failure?.querySelector('p')?.textContent).toBe('Lys could not look at the folders there. Check the computer’s runner, then close and reopen this chooser to try again.');
    expect(warning?.querySelector('p')?.textContent).toBe('This computer has no connected Lys runner. Connect its runner before continuing.');
    expect(warning?.querySelector('details')?.hasAttribute('open')).toBe(false);
    expect(warning?.querySelector('details')?.textContent).toContain('runner_absent: machine names no runner');
    expect(posted.map((entry) => entry.path)).toEqual(['/network/machines/' + computer.id + '/folders']);
    expect(document.querySelector('.works-in')?.textContent).toBe('No folder chosen yet.');
  });
  it('names both failed reads when the saved folder and the home folder are unreadable', async () => {
    const { posted } = await mount('#/file/' + SCRIBE + '/provisioning', {
      ...routes, [path]: ok({ ...answer, profile: { ...profile, runs_on: computer.id, working_folder: '/srv/removed' } }),
      '/network': ok({ machines: [computer], reports_served: true }),
      [folders]: (value) => (value as { under?: string }).under
        ? refused(409, 'folder_unreadable', 'the saved folder is gone')
        : refused(409, 'home_unknown', 'the runner account has no home folder'),
    });
    await click(button('Choose another folder'));
    expect(document.querySelector('.folder-recovery [role="alert"] > p')?.textContent).toContain('Choose a readable folder');
    expect(document.querySelector('.folder-problem [role="alert"] > p')?.textContent).toBe('The runner could not find its home folder. Ask the administrator to check the account it runs under.');
    expect(document.querySelector('.folder-recovery details')?.textContent).toContain('folder_unreadable: the saved folder is gone');
    expect(document.querySelector('.folder-problem details')?.textContent).toContain('home_unknown: the runner account has no home folder');
    expect(button('Work in removed')).toBeNull();
    expect(posted.map((entry) => entry.body)).toEqual([{ under: '/srv/removed' }, {}]);
    expect(document.querySelector('.works-in code')?.textContent).toBe('/srv/removed');
  });
});

describe('Managed controls in the saved setup', () => {
  it('does not offer the controls requirement while no adapter is qualified, and leaves an old setup manual', async () => {
    const initial = { ...profile, working_folder: '/Users/ada/Developer/receipts', session: null };
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
    // No managed adapter is reported qualified, so the requirement is not offered: an old setup stays manual.
    expect(document.querySelector('input[name="requires_controls"]')).toBeNull();
    expect(posted).toEqual([]);
    await click(button('Save these settings'));
    expect(posted.filter((entry) => entry.path === path)).toHaveLength(1);
    const sent = posted.find((entry) => entry.path === path)?.body as { session?: { requires_controls?: boolean } | null; working_folder?: string };
    expect(sent.working_folder).toBe(initial.working_folder);
    expect(sent.session?.requires_controls).toBeUndefined();
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

describe('Plugins checked on the agent’s own computer', () => {
  const withPlugins = { ...profile, plugins: ['argus@ablative', 'manifold@manifold'], runs_on: 'machine-seat' };
  const asked = '/harnesses/plugins?machine=machine-seat';
  const computer = { id: 'machine-seat', name: 'Ward desk', runtime: 'runner', state: 'in_use', may_run: [{ id: SCRIBE }] };
  const pluginRoutes = { ...routes, '/network': ok({ machines: [computer], reports_served: true }) };
  it('names the missing plugin and its remedy while keeping the start refusal code in a title', async () => {
    const { requests } = await mount('#/file/' + SCRIBE + '/provisioning', {
      ...pluginRoutes, [path]: ok({ ...answer, profile: withPlugins }),
      [asked]: ok({ machine: 'machine-seat', config_dir: '/Users/seat/.claude', installed: ['argus@ablative'] }),
    });
    expect(requests).toContain(asked);
    const warning = document.querySelector('.given-plugins > .why-not');
    expect(warning?.textContent).toContain('Not installed in /Users/seat/.claude on Ward desk: manifold@manifold');
    expect(warning?.textContent).toContain('until each plugin is installed in that folder or removed from these settings');
    expect(warning?.textContent).not.toContain('plugin_not_installed');
    expect(warning?.getAttribute('title')).toBe('plugin_not_installed');
  });
  it.each([
    ['plugins_unreadable', 'The runner could not read the installed plugins. Check the program’s configuration folder.'],
    ['PluginProbeRefused', 'Lys refused: the installed plugins file is not JSON'],
  ])('explains %s and a next action while retaining the server details without claiming installation', async (code, words) => {
    await mount('#/file/' + SCRIBE + '/provisioning', {
      ...pluginRoutes, [path]: ok({ ...answer, profile: withPlugins }),
      [asked]: refused(409, code, 'the installed plugins file is not JSON'),
    });
    const plugins = document.querySelector('.given-plugins');
    const warning = plugins?.querySelector('[role="alert"]');
    expect(plugins?.textContent).toContain('Check that computer’s runner and the program’s configuration folder before starting the agent.');
    expect(warning?.querySelector('p')?.textContent).toBe(words);
    expect(warning?.querySelector('p')?.textContent).not.toContain(code);
    expect(warning?.querySelector('details')?.hasAttribute('open')).toBe(false);
    expect(warning?.querySelector('details')?.textContent).toContain(code + ': the installed plugins file is not JSON');
    expect(plugins?.textContent).not.toContain('Each is installed');
    expect(plugins?.textContent).not.toContain('plugin_not_installed');
  });
  it('asks for a computer before checking installation and explains that starts still check', async () => {
    const { requests } = await mount('#/file/' + SCRIBE + '/provisioning', {
      ...routes, [path]: ok({ ...answer, profile: { ...withPlugins, runs_on: '' } }),
    });
    const plugins = document.querySelector('.given-plugins');
    expect(plugins?.textContent).toContain('Choose the computer this agent runs on to check its plugins. A start checks again and refuses any plugin that is missing.');
    expect(plugins?.textContent).not.toContain('plugin_not_installed');
    expect(requests.some((request) => request.startsWith('/harnesses/plugins'))).toBe(false);
  });
  it.each([
    { description: 'another computer', installed: { machine: 'other-computer', config_dir: '/Users/seat/.claude', installed: ['argus@ablative', 'manifold@manifold'] } },
    { description: 'a missing configuration folder', installed: { machine: 'machine-seat', config_dir: '', installed: ['argus@ablative', 'manifold@manifold'] } },
    { description: 'an unreadable installed list', installed: { machine: 'machine-seat', config_dir: '/Users/seat/.claude', installed: 'argus@ablative' } },
    { description: 'an invalid installed name', installed: { machine: 'machine-seat', config_dir: '/Users/seat/.claude', installed: ['argus@ablative', null] } },
  ])('names a plugins answer for $description as unreadable instead of claiming installation', async ({ installed }) => {
    await mount('#/file/' + SCRIBE + '/provisioning', {
      ...pluginRoutes, [path]: ok({ ...answer, profile: withPlugins }), [asked]: ok(installed),
    });
    const plugins = document.querySelector('.given-plugins');
    const warning = plugins?.querySelector('[role="alert"]');
    expect(warning?.querySelector('p')?.textContent).toBe('The runner could not read the installed plugins. Check the program’s configuration folder.');
    expect(warning?.querySelector('details')?.textContent).toContain('plugins_unreadable: The computer did not answer which plugins are installed in its configuration folder.');
    expect(plugins?.textContent).not.toContain('Each is installed');
  });
  it('says each is installed when the computer’s folder holds them all', async () => {
    await mount('#/file/' + SCRIBE + '/provisioning', {
      ...pluginRoutes, [path]: ok({ ...answer, profile: withPlugins }),
      [asked]: ok({ machine: 'machine-seat', config_dir: '/Users/seat/.claude', installed: ['argus@ablative', 'manifold@manifold'] }),
    });
    expect(text()).toContain('Each is installed in /Users/seat/.claude on Ward desk');
  });
});
