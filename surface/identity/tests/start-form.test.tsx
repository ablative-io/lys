import { act } from 'react';
import { beforeEach, describe, expect, it } from 'vitest';
import { choose, click, mount, text, unmountAll } from './harness';
import { ADA, SCRIBE, SERVICE, ok, refused } from './fixtures';
import type { Route } from './fixtures';

const path = '/agents/' + SCRIBE + '/provisioning';
const start = '/agents/' + SCRIBE + '/start-command';
const machine = { id: 'op-' + '1'.repeat(32), name: 'This computer', runtime: 'runner', state: 'in_use', may_run: [{ id: SCRIBE }] };
const description = { models: { minimum: 1, maximum: null, further_encoding: { kind: 'delimited', separator: ',' } }, permissions: { modes: ['default', 'acceptEdits'], rule_forms: ['tool_specifier'] }, mcp: { transports: ['stdio', 'http'], working_directory: false, handle_variables: true, channel_policies: ['off', 'wake'] }, rendering_contract: 'claude-code/template-v1' };
const program = { name: 'Claude Code', line: 'Read and change code', models: [{ id: 'default', label: 'Default for this account' }, { id: 'opus', label: 'Opus' }], modes: [{ id: 'default', meaning: 'Asks before changes' }, { id: 'acceptEdits', meaning: 'Allows edits' }], description, builds: [{ name: 'Installed copy', program: '/opt/bin/claude', package: 'claude', from: 'profile' }] };
const harness = { name: program.name, description, program: '/opt/bin/claude', package: 'claude' };
const profile = { version: 1, operation: 'op-' + 'a'.repeat(32), harness, model_access: ['default'], permissions: { default_mode: 'default', allow: ['Read'], ask: ['Edit'], deny: ['Bash(rm:*)'], additional_directories: ['/srv/kept'] }, instructions: '', tools: ['reader'], skills: ['review'], mcp_servers: [{ name: 'Kept service', url: 'http://localhost:6010' }], note: 'Kept', set_by: ADA, set_at: 1, reviewed_by: ADA, session: null };
const answer = (kept: unknown) => ({ agent: SCRIBE, profile: kept, versions: [], enforced: false });
const button = () => document.querySelector<HTMLButtonElement>('section[aria-label="Start this agent"] button[type="submit"]');
const field = (name: string) => document.querySelector<HTMLSelectElement>('select[name="' + name + '"]');
function routes(kept: unknown = null): Record<string, Route> {
  let latest = kept;
  return { ...SERVICE, [path]: ok(answer(kept)), '/harnesses': ok({ programs: [program] }), '/network': ok({ machines: [machine], reports_served: true }), '/roles': ok({ roles: [] }),
    ['POST ' + path]: (body) => { const draft = body as Record<string, unknown>; latest = { ...draft, version: Number(draft.from_version) + 1, set_by: ADA, set_at: 1, reviewed_by: null }; return ok({ ...answer(latest), recorded: { operation: draft.operation, version: Number(draft.from_version) + 1 } }); },
    ['POST ' + path + '/1/review']: (body) => { latest = { ...(latest as object), reviewed_by: ADA }; return ok({ ...answer(latest), recorded: { operation: (body as Record<string, unknown>).operation, version: 1 } }); },
    ['POST ' + start]: (body) => { const draft = body as Record<string, unknown>; return ok({ agent: SCRIBE, machine: draft.machine, runtime: 'runner', session: draft.operation, provisioning_version: 1, harness: 'Claude Code', executed: false, command: 'claude', left_out: [], runner: { session: draft.operation, state: 'running', pid: 1, started_at: 1 } }); },
  };
}
const open = (extra: Record<string, Route>) => mount('#/file/' + SCRIBE + '/provisioning', extra);
beforeEach(() => sessionStorage.clear());

describe('The route-backed start form', () => {
  it('offers known values with defaults, no required text and one submit button', async () => {
    const { posted } = await open(routes());
    expect(field('program')?.value).toBe('Claude Code');
    expect(field('model')?.value).toBe('default');
    expect(field('mode')?.value).toBe('default');
    expect(field('machine')?.value).toBe(machine.id);
    expect(document.querySelectorAll('section[aria-label="Start this agent"] input[required], section[aria-label="Start this agent"] textarea[required]')).toHaveLength(0);
    expect(document.querySelectorAll('section[aria-label="Start this agent"] button[type="submit"]')).toHaveLength(1);
    expect(button()?.textContent).toBe('Start this agent');
    expect(text()).toContain('Lys makes this agent’s own folder');
    expect(posted).toEqual([]);
  });
  it('saves, reviews and starts in order with one press and the displayed defaults', async () => {
    const { posted } = await open(routes());
    await click(button());
    expect(posted.map((entry) => entry.path)).toEqual([path, path + '/1/review', start]);
    expect(posted[0].body).toMatchObject({ from_version: 0, model_access: ['default'], permissions: { default_mode: 'default' }, harness, instructions: '', tools: [], skills: [], mcp_servers: [] });
    expect(posted[2].body).toMatchObject({ machine: machine.id });
    expect(text()).toContain('Its Lys runner has it running');
  });
  it('starts an unchanged reviewed profile without rewriting any of its kept members', async () => {
    const { posted } = await open(routes(profile));
    await click(button());
    expect(posted.map((entry) => entry.path)).toEqual([start]);
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
  it('only lists computers this agent may use, including its currently held roles', async () => {
    const { posted } = await open({ ...routes(profile), '/network': ok({ machines: [{ ...machine, may_run: [], may_run_roles: ['role-a'] }, { ...machine, id: 'other', name: 'Not admitted', may_run: [] }], reports_served: true }), '/roles': ok({ roles: [{ id: 'role-a', holders: [{ holder: SCRIBE, state: 'holding' }] }] }) });
    expect([...field('machine')!.options].map((option) => option.textContent)).toEqual(['This computer']);
    await click(button());
    expect(posted[0].body).toMatchObject({ machine: machine.id });
  });
  it('retains an uncertain save and repeats its exact body after remount before advancing', async () => {
    const first = await open({ ...routes(), ['POST ' + path]: refused(503, 'ProvisioningUnavailable', 'Unknown outcome') });
    await click(button());
    expect(first.posted).toHaveLength(1);
    unmountAll();
    document.body.innerHTML = '';
    const next = await open(routes());
    await click(button());
    expect(next.posted[0]).toEqual(first.posted[0]);
    expect(next.posted.map((entry) => entry.path)).toEqual([path, path + '/1/review', start]);
  });
  it('does not start after an inconsistent save receipt', async () => {
    const { posted } = await open({ ...routes(), ['POST ' + path]: ok({ ...answer(profile), recorded: { operation: 'wrong', version: 1 } }) });
    await click(button());
    expect(posted).toHaveLength(1);
    expect(text()).toContain('ProfileReceiptMismatch');
    expect(sessionStorage.length).toBeGreaterThan(0);
  });
  it('does not start after an unconfirmed review and retries the same review operation', async () => {
    const first = await open({ ...routes({ ...profile, reviewed_by: null }), ['POST ' + path + '/1/review']: refused(503, 'ProvisioningUnavailable', 'Unknown review') });
    await click(button());
    expect(first.posted.map((entry) => entry.path)).toEqual([path + '/1/review']);
    unmountAll();
    document.body.innerHTML = '';
    const next = await open(routes({ ...profile, reviewed_by: null }));
    await click(button());
    expect(next.posted[0]).toEqual(first.posted[0]);
    expect(next.posted[1].path).toBe(start);
  });
  it('does not claim a process started when the endpoint only returns a command', async () => {
    await open({ ...routes(profile), ['POST ' + start]: (body) => ok({ agent: SCRIBE, machine: machine.id, session: (body as Record<string, unknown>).operation, provisioning_version: 1, executed: false, command: 'claude', left_out: [] }) });
    await click(button());
    expect(text()).toContain('RunnerStartUnconfirmed');
    expect(text()).not.toContain('Its Lys runner has it running');
  });
  it('retains the start operation after an uncertain answer and remount', async () => {
    const first = await open({ ...routes(profile), ['POST ' + start]: refused(503, 'RuntimeUnavailable', 'Unknown start') });
    await click(button());
    unmountAll();
    document.body.innerHTML = '';
    const next = await open(routes(profile));
    await click(button());
    expect(next.posted).toEqual(first.posted);
  });
  it('blocks a second press while the first action is being submitted', async () => {
    const { posted } = await open(routes(profile)); const action = button(); if (!action) throw new Error('Start action missing');
    await act(async () => { action.click(); action.click(); });
    expect(posted).toHaveLength(1);
  });
  it('sends the supported replacement mode without inventing a program capability', async () => {
    const { posted } = await open({ ...routes(), '/harnesses': ok({ programs: [{ ...program, instructions_modes: ['keep', 'append', 'replace'] }] }) });
    await choose(field('prompt'), 'replace');
    await click(button());
    expect(posted[0].body).toMatchObject({ instructions_mode: 'replace', instructions: '' });
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
    await click(button());
    expect(posted).toEqual([]);
    expect(text()).toContain('ProfileReadIncomplete');
  });
  it('resumes a retained start from the old form with its exact operation', async () => {
    const body = { machine: machine.id, operation: 'op-' + 'c'.repeat(32) };
    sessionStorage.setItem('lys.pending.start.' + ADA + '.' + SCRIBE, JSON.stringify({ path: start, body }));
    const { posted } = await open(routes(profile));
    await click(button());
    expect(posted).toEqual([{ path: start, body }]);
    expect(sessionStorage.length).toBe(0);
  });
  it('names corrupt retained state and sends no replacement request', async () => {
    sessionStorage.setItem('lys.pending.agent-start.' + ADA + '.' + SCRIBE, 'null');
    const { posted } = await open(routes(profile));
    expect(button()?.disabled).toBe(true);
    expect(text()).toContain('PendingStartUnreadable');
    expect(posted).toEqual([]);
  });

  it('does not advance a save receipt that changes the chosen model', async () => {
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
    await click(button());
    expect(posted.map((entry) => entry.path)).toEqual([start]);
  });

  it('still confirms approval when the subsequent start is refused', async () => {
    await open({ ...routes({ ...profile, reviewed_by: null }), ['POST ' + start]: refused(409, 'AgentNotActive', 'Agent is suspended') });
    await click(button());
    expect(text()).toContain('Version 1 of these settings is approved');
    expect(text()).toContain('AgentNotActive');
  });

  it('does not claim the selected profile started when the receipt names another version', async () => {
    await open({ ...routes(profile), ['POST ' + start]: (body) => {
      const operation = (body as Record<string, unknown>).operation;
      return ok({ agent: SCRIBE, machine: machine.id, session: operation, provisioning_version: 2, executed: false, command: 'claude', left_out: [], runner: { session: operation, state: 'running' } });
    } });
    await click(button());
    expect(text()).toContain('StartVersionChanged');
    expect(text()).not.toContain('Its Lys runner has it running');
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
