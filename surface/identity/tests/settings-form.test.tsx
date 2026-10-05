import { act } from 'react';
import { beforeEach, describe, expect, it } from 'vitest';
import { choose, click, mount, text, type, unmountAll } from './harness';
import { ADA, SCRIBE, SERVICE, ok, refused } from './fixtures';
import type { Route } from './fixtures';

const path = '/agents/' + SCRIBE + '/provisioning';
const machine = { id: 'op-' + '1'.repeat(32), name: 'This computer', runtime: 'runner', state: 'in_use', may_run: [{ id: SCRIBE }] };
const description = { models: { minimum: 1, maximum: null, further_encoding: { kind: 'delimited', separator: ',' } }, permissions: { modes: ['default', 'acceptEdits'], rule_forms: ['tool_specifier'] }, mcp: { transports: ['stdio', 'http'], working_directory: false, handle_variables: true, channel_policies: ['off', 'wake'] }, rendering_contract: 'claude-code/template-v1' };
const program = { name: 'Claude Code', line: 'Read and change code', models: [{ id: 'default', label: 'Default for this account' }, { id: 'opus', label: 'Opus' }], modes: [{ id: 'default', meaning: 'Asks before changes' }, { id: 'acceptEdits', meaning: 'Allows edits' }], description, builds: [{ name: 'Installed copy', program: '/opt/bin/claude', package: 'claude', from: 'profile' }] };
const harness = { name: program.name, description, program: '/opt/bin/claude', package: 'claude' };
const profile = { version: 1, operation: 'op-' + 'a'.repeat(32), harness, model_access: ['default'], permissions: { default_mode: 'default', allow: ['Read'], ask: ['Edit'], deny: ['Bash(rm:*)'], additional_directories: ['/srv/kept'] }, instructions: '', tools: ['reader'], skills: ['review'], mcp_servers: [{ name: 'Kept service', url: 'http://localhost:6010' }], note: 'Kept', set_by: ADA, set_at: 1, reviewed_by: ADA, session: null };
const answer = (kept: unknown) => ({ agent: SCRIBE, profile: kept, versions: [], enforced: false });
const button = () => document.querySelector<HTMLButtonElement>('section[aria-label="Settings of this agent"] button[type="submit"]');
const field = (name: string) => document.querySelector<HTMLSelectElement>('select[name="' + name + '"]');
function routes(kept: unknown = null): Record<string, Route> {
  return { ...SERVICE, [path]: ok(answer(kept)), '/harnesses': ok({ programs: [program] }), '/network': ok({ machines: [machine], reports_served: true }), '/roles': ok({ roles: [] }),
    ['POST ' + path]: (body) => { const draft = body as Record<string, unknown>; const latest = { ...draft, version: Number(draft.from_version) + 1, set_by: ADA, set_at: 1, reviewed_by: null, session: null }; return ok({ ...answer(latest), recorded: { operation: draft.operation, version: Number(draft.from_version) + 1 } }); },
  };
}
const open = (extra: Record<string, Route>) => mount('#/file/' + SCRIBE + '/provisioning', extra);
beforeEach(() => sessionStorage.clear());

/** The settings form saves what it shows as one request and never approves or starts; starting is the front page's. */
describe('The settings form', () => {
  it('offers a typed path last, saves it and reads the chosen path back', async () => {
    const service = routes();
    const { posted } = await open(service);
    const copies = field('build');
    expect([...copies?.options ?? []].at(-1)?.textContent).toBe('Another program…');
    await choose(copies, 'another');
    const input = document.querySelector('input[name="program-path"]');
    expect(input).not.toBeNull();
    await type(input, '/opt/forks/claude-fork');
    await click(button());
    const saved = posted.find((entry) => entry.path === path)?.body as Record<string, unknown>;
    expect(saved).toMatchObject({ harness: { name: program.name, program: '/opt/forks/claude-fork', description }, model_access: ['default'], permissions: { default_mode: 'default' } });
    expect(posted.map((entry) => entry.path)).toEqual([path]);
    unmountAll(); document.body.innerHTML = '';
    await open(routes({ ...saved, version: 1, reviewed_by: ADA, session: null }));
    expect(document.querySelector<HTMLInputElement>('input[name="program-path"]')?.value).toBe('/opt/forks/claude-fork');
  });
  it('offers the typed path when no installed copy was found', async () => {
    const { posted } = await open({ ...routes(), '/harnesses': ok({ programs: [{ ...program, builds: [] }] }) });
    await choose(field('build'), 'another');
    expect(button()?.disabled).toBe(true);
    await type(document.querySelector('input[name="program-path"]'), '/opt/forks/claude');
    await click(button());
    expect(posted[0].body).toMatchObject({ harness: { program: '/opt/forks/claude' } });
  });
  it('chooses an installed fork without changing model, prompt or mode', async () => {
    const fork = { ...program.builds[0], name: 'Installed fork', program: '/opt/forks/claude', package: 'fork-version' };
    const { posted } = await open({ ...routes(), '/harnesses': ok({ programs: [{ ...program, builds: [...program.builds, fork] }] }) });
    await choose(field('build'), fork.program + '\n' + fork.package);
    await click(button());
    expect(posted[0].body).toMatchObject({ harness: { program: fork.program, package: fork.package, description }, model_access: ['default'], instructions: '', permissions: { default_mode: 'default' } });
    expect(document.querySelector('input[name="program-path"]')).toBeNull();
  });
  it('offers known values with defaults, no required text and one submit button', async () => {
    const { posted } = await open(routes());
    expect(field('program')?.value).toBe('Claude Code');
    expect(field('model')?.value).toBe('default');
    expect(document.querySelector<HTMLInputElement>('input[name="permission-mode"]:checked')?.value).toBe('default');
    expect(document.querySelector('section[aria-label="Settings of this agent"] select[name="mode"]')).toBeNull();
    expect(field('machine')).toBeNull();
    expect(document.querySelectorAll('section[aria-label="Settings of this agent"] input[required], section[aria-label="Settings of this agent"] textarea[required]')).toHaveLength(0);
    expect(document.querySelectorAll('section[aria-label="Settings of this agent"] button[type="submit"]')).toHaveLength(1);
    expect([button()?.getAttribute('aria-label'), button()?.textContent]).toEqual(['Save these settings', 'Save']);
    expect(text()).toContain('No folder chosen yet.');
    expect(posted).toEqual([]);
  });
  it('saves the displayed defaults with one press, and neither approves nor starts', async () => {
    const { posted } = await open(routes());
    await click(button());
    expect(posted.map((entry) => entry.path)).toEqual([path]);
    expect(posted[0].body).toMatchObject({ from_version: 0, model_access: ['default'], permissions: { default_mode: 'default' }, harness, instructions: '', tools: [], skills: [], mcp_servers: [] });
    expect(text()).toContain('Saved.');
    expect(document.querySelector('section[aria-label="Settings of this agent"] a[href="#/file/' + SCRIBE + '"]')).not.toBeNull();
  });
  it('saves a second change from the version it just saved', async () => {
    const { posted } = await open(routes());
    await click(button());
    await choose(field('model'), 'opus');
    await click(button());
    expect(posted.map((entry) => entry.path)).toEqual([path, path]);
    expect(posted[1].body).toMatchObject({ from_version: 1, model_access: ['opus'] });
  });
  it('greys Save on an unchanged profile with the reason beside it, and sends nothing', async () => {
    const { posted } = await open(routes(profile));
    expect(button()?.disabled).toBe(true);
    expect(text()).toContain('Nothing has changed.');
    await click(button());
    expect(posted).toEqual([]);
  });
  it('says an unanswered start must be finished on the front page before a save', async () => {
    sessionStorage.setItem('lys.pending.agent-start.' + ADA + '.' + SCRIBE, JSON.stringify({ stage: 'start' }));
    const { posted } = await open(routes());
    expect(button()?.disabled).toBe(true);
    expect(text()).toContain('has no confirmed answer yet');
    expect(posted).toEqual([]);
  });
  it('keeps hidden permissions, tools, skills and services when changing a model', async () => {
    const { posted } = await open(routes(profile));
    await choose(field('model'), 'opus');
    await click(button());
    expect(posted[0].body).toMatchObject({ from_version: 1, model_access: ['opus'], tools: profile.tools, skills: profile.skills, mcp_servers: profile.mcp_servers, permissions: profile.permissions });
  });
  it('shows prompt text only when a person chooses to add to the program prompt', async () => {
    await open(routes());
    expect(document.querySelector('textarea[name="instructions"]')).toBeNull();
    await choose(field('prompt'), 'append');
    expect(document.querySelector('textarea[name="instructions"]')).not.toBeNull();
  });
  it('offers only Keep and Add when the catalogue has no prompt capability list', async () => {
    const { posted } = await open(routes());
    expect([...field('prompt')!.options].map((option) => option.value)).toEqual(['keep', 'append']);
    expect(posted).toEqual([]);
  });
  it('names unavailable program choices rather than asking for a program string', async () => {
    const { posted } = await open({ ...routes(), '/harnesses': refused(404, 'RouteMissing', 'Update needed') });
    expect(text()).toContain('This Lys is too old to list programs');
    expect(button()?.disabled).toBe(true);
    expect(posted).toEqual([]);
  });
  it('names a missing declared build rather than inventing an executable', async () => {
    const { posted } = await open({ ...routes(), '/harnesses': ok({ programs: [{ ...program, builds: [] }] }) });
    expect(text()).toContain('Claude Code is not installed on this computer');
    expect(button()?.disabled).toBe(true);
    expect(posted).toEqual([]);
  });
  it('retains an uncertain save and repeats its exact body after remount before advancing', async () => {
    const first = await open({ ...routes(), ['POST ' + path]: refused(503, 'ProvisioningUnavailable', 'Unknown outcome') });
    await click(button());
    expect(first.posted).toHaveLength(1);
    unmountAll();
    document.body.innerHTML = '';
    const next = await open(routes());
    expect(text()).toContain('The last save has no confirmed answer');
    await click(button());
    expect(next.posted).toEqual(first.posted);
    expect(text()).toContain('Saved.');
    expect(sessionStorage.length).toBe(0);
  });
  it('does not call an inconsistent save receipt saved, and keeps the request', async () => {
    const { posted } = await open({ ...routes(), ['POST ' + path]: ok({ ...answer(profile), recorded: { operation: 'wrong', version: 1 } }) });
    await click(button());
    expect(posted).toHaveLength(1);
    expect(text()).toContain('ProfileReceiptMismatch');
    expect(text()).not.toContain('Saved.');
    expect(sessionStorage.length).toBeGreaterThan(0);
  });
  it('blocks a second press while the first action is being submitted', async () => {
    const { posted } = await open(routes()); const action = button(); if (!action) throw new Error('Save action missing');
    await act(async () => { action.click(); action.click(); });
    expect(posted).toHaveLength(1);
  });
  it('sends the supported replacement mode without inventing a program capability', async () => {
    const { posted } = await open({ ...routes(), '/harnesses': ok({ programs: [{ ...program, instructions_modes: ['keep', 'append', 'replace'] }] }) });
    await choose(field('prompt'), 'replace');
    // A replaced prompt with no words is never saved: Codex refuses to start on one, and no agent should run on an empty prompt.
    expect(text()).toContain('Type the prompt this agent uses instead');
    await click(button());
    expect(posted).toHaveLength(0);
    const box = document.querySelector<HTMLTextAreaElement>('textarea[name="instructions"]');
    await act(async () => { Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, 'value')?.set?.call(box, 'Check every receipt'); box?.dispatchEvent(new Event('input', { bubbles: true })); });
    expect(text()).toContain('used instead of the program’s own prompt');
    await click(button());
    expect(posted[0].body).toMatchObject({ instructions_mode: 'replace', instructions: 'Check every receipt' });
  });
  it('shows the connected tools, skills and named tools the agent has, and saves one removed', async () => {
    const { posted } = await open({ ...routes(profile), '/skills': ok({ skills: [{ name: 'review' }, { name: 'triage' }] }) });
    expect(text()).toContain('Kept service'); expect(text()).toContain('Reached at http://localhost:6010');
    expect(text()).toContain('1 skill: review.');
    expect(text()).toContain('This agent uses the program’s own prompt.');
    await click([...document.querySelectorAll('button')].find((each) => (each.getAttribute('aria-label') ?? each.textContent) === 'Remove reader') ?? null);
    await click(button());
    expect(posted[0].body).toMatchObject({ tools: [], skills: ['review'], mcp_servers: [{ name: 'Kept service', url: 'http://localhost:6010' }] });
  });
  it('will not save a skill Lys does not keep, and says which', async () => {
    const { posted } = await open({ ...routes(profile), '/skills': ok({ skills: [{ name: 'triage' }] }) });
    expect(text()).toContain('Lys keeps no skill named review');
    await click(button());
    expect(posted).toHaveLength(0);
    await click([...document.querySelectorAll('button')].find((each) => (each.getAttribute('aria-label') ?? each.textContent) === 'Remove review') ?? null);
    await click(button());
    expect(posted[0].body).toMatchObject({ skills: [] });
  });
  it('preserves saved session rotation options during a model change', async () => {
    const session = { accounts: { kind: 'single', account: 'kept-account' } };
    const { posted } = await open(routes({ ...profile, session }));
    await choose(field('model'), 'opus');
    await click(button());
    expect(posted[0].body).toMatchObject({ session });
  });
  it('blocks rewriting a profile when the service omits its saved session field', async () => {
    const incomplete = { ...profile } as Record<string, unknown>;
    delete incomplete.session;
    const { posted } = await open(routes(incomplete));
    await choose(field('model'), 'opus');
    expect(button()?.disabled).toBe(true);
    await click(button());
    expect(posted).toEqual([]);
    expect(text()).toContain('ProfileReadIncomplete');
  });
  it('names corrupt retained state and sends no replacement request', async () => {
    sessionStorage.setItem('lys.pending.settings-save.' + ADA + '.' + SCRIBE, 'null');
    const { posted } = await open(routes());
    expect(button()?.disabled).toBe(true);
    expect(text()).toContain('PendingSaveUnreadable');
    expect(posted).toEqual([]);
  });

  it('does not call saved a receipt that changes the chosen model', async () => {
    const { posted } = await open({ ...routes(), ['POST ' + path]: (body) => {
      const draft = body as Record<string, unknown>;
      return ok({ ...answer({ ...draft, version: 1, model_access: ['different-model'] }), recorded: { operation: draft.operation, version: 1 } });
    } });
    await click(button());
    expect(posted).toHaveLength(1);
    expect(text()).toContain('ProfileReceiptMismatch');
  });
  it('does not rewrite an unchanged profile after capabilities begin listing prompt modes', async () => {
    const { posted } = await open({ ...routes({ ...profile, instructions_mode: 'append' }), '/harnesses': ok({ programs: [{ ...program, instructions_modes: ['keep', 'append', 'replace'] }] }) });
    expect(button()?.disabled).toBe(true);
    await click(button());
    expect(posted).toEqual([]);
  });

  it('shows the served reason when an installed copy cannot be offered', async () => {
    const reason = 'VersionCommandFailed: exit status: 7';
    const { posted } = await open({ ...routes(), '/harnesses': ok({ programs: [{ ...program, builds: [], not_found: reason }] }) });
    expect(text()).toContain('Claude Code is not installed on this computer');
    expect(text()).toContain(reason);
    expect(button()?.disabled).toBe(true);
    expect(posted).toEqual([]);
  });
  it('names the selected program when its installed copy is missing', async () => {
    await open({ ...routes(), '/harnesses': ok({ programs: [{ ...program, name: 'Codex', builds: [] }] }) });
    expect(text()).toContain('Codex is not installed on this computer');
    expect(text()).not.toContain('Claude Code is not installed');
    expect(button()?.disabled).toBe(true);
  });

});
