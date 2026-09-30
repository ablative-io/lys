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
const profile = { version: 1, operation: 'op-' + 'a'.repeat(32), harness, model_access: ['default'], permissions: { default_mode: 'default', allow: ['Read'], ask: ['Edit'], deny: ['Bash(rm:*)'], additional_directories: ['/srv/kept'] }, instructions: '', tools: ['reader'], skills: ['review'], mcp_servers: [{ name: 'Kept service', url: 'http://localhost:6010' }], note: 'Kept', set_by: ADA, set_at: 1, reviewed_by: ADA };
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
    expect(field('program')?.value).toBe('Claude Code'); expect(field('model')?.value).toBe('default'); expect(field('mode')?.value).toBe('default'); expect(field('machine')?.value).toBe(machine.id);
    expect(document.querySelectorAll('section[aria-label="Start this agent"] input[required], section[aria-label="Start this agent"] textarea[required]')).toHaveLength(0);
    expect(document.querySelectorAll('section[aria-label="Start this agent"] button[type="submit"]')).toHaveLength(1);
    expect(button()?.textContent).toBe('Start this agent'); expect(text()).toContain('Lys makes this agent’s own folder'); expect(posted).toEqual([]);
  });
  it('saves, reviews and starts in order with one press and the displayed defaults', async () => {
    const { posted } = await open(routes()); await click(button());
    expect(posted.map((entry) => entry.path)).toEqual([path, path + '/1/review', start]);
    expect(posted[0].body).toMatchObject({ from_version: 0, model_access: ['default'], permissions: { default_mode: 'default' }, harness, instructions: '', tools: [], skills: [], mcp_servers: [] });
    expect(posted[2].body).toMatchObject({ machine: machine.id }); expect(text()).toContain('Its Lys runner has it running');
  });
  it('starts an unchanged reviewed profile without rewriting any of its kept members', async () => {
    const { posted } = await open(routes(profile)); await click(button()); expect(posted.map((entry) => entry.path)).toEqual([start]);
  });
  it('keeps hidden permissions, tools, skills and services when changing a model', async () => {
    const { posted } = await open(routes(profile)); await choose(field('model'), 'opus'); await click(button());
    expect(posted[0].body).toMatchObject({ from_version: 1, model_access: ['opus'], tools: profile.tools, skills: profile.skills, mcp_servers: profile.mcp_servers, permissions: profile.permissions });
  });
  it('shows prompt text only when a person chooses to add to the program prompt', async () => {
    await open(routes()); expect(document.querySelector('textarea[name="instructions"]')).toBeNull();
    await choose(field('prompt'), 'append'); expect(document.querySelector('textarea[name="instructions"]')).not.toBeNull();
  });
  it('names unsupported replacement without saving or starting', async () => {
    const { posted } = await open(routes()); await choose(field('prompt'), 'replace'); await click(button());
    expect(text()).toContain('PromptReplacementUnavailable'); expect(posted).toEqual([]);
  });
  it('names unavailable program choices rather than asking for a program string', async () => {
    const { posted } = await open({ ...routes(), '/harnesses': refused(404, 'RouteMissing', 'Update needed') });
    expect(text()).toContain('This Lys is too old to list programs'); expect(button()?.disabled).toBe(true); expect(posted).toEqual([]);
  });
  it('names a missing declared build rather than inventing an executable', async () => {
    const { posted } = await open({ ...routes(), '/harnesses': ok({ programs: [{ ...program, builds: [] }] }) });
    expect(text()).toContain('ProgramBuildUnavailable'); expect(button()?.disabled).toBe(true); expect(posted).toEqual([]);
  });
  it('only lists computers this agent may use, including its currently held roles', async () => {
    const { posted } = await open({ ...routes(profile), '/network': ok({ machines: [{ ...machine, may_run: [], may_run_roles: ['role-a'] }, { ...machine, id: 'other', name: 'Not admitted', may_run: [] }], reports_served: true }), '/roles': ok({ roles: [{ id: 'role-a', holders: [{ holder: SCRIBE, state: 'holding' }] }] }) });
    expect([...field('machine')!.options].map((option) => option.textContent)).toEqual(['This computer']); await click(button()); expect(posted[0].body).toMatchObject({ machine: machine.id });
  });
  it('retains an uncertain save and repeats its exact body after remount before advancing', async () => {
    const first = await open({ ...routes(), ['POST ' + path]: refused(503, 'ProvisioningUnavailable', 'Unknown outcome') }); await click(button());
    expect(first.posted).toHaveLength(1); unmountAll(); document.body.innerHTML = '';
    const next = await open(routes()); await click(button()); expect(next.posted[0]).toEqual(first.posted[0]); expect(next.posted.map((entry) => entry.path)).toEqual([path, path + '/1/review', start]);
  });
  it('does not start after an inconsistent save receipt', async () => {
    const { posted } = await open({ ...routes(), ['POST ' + path]: ok({ ...answer(profile), recorded: { operation: 'wrong', version: 1 } }) }); await click(button());
    expect(posted).toHaveLength(1); expect(text()).toContain('ProfileReceiptMismatch'); expect(sessionStorage.length).toBeGreaterThan(0);
  });
  it('does not start after an unconfirmed review and retries the same review operation', async () => {
    const first = await open({ ...routes({ ...profile, reviewed_by: null }), ['POST ' + path + '/1/review']: refused(503, 'ProvisioningUnavailable', 'Unknown review') }); await click(button());
    expect(first.posted.map((entry) => entry.path)).toEqual([path + '/1/review']); unmountAll(); document.body.innerHTML = '';
    const next = await open(routes({ ...profile, reviewed_by: null })); await click(button()); expect(next.posted[0]).toEqual(first.posted[0]); expect(next.posted[1].path).toBe(start);
  });
  it('does not claim a process started when the endpoint only returns a command', async () => {
    await open({ ...routes(profile), ['POST ' + start]: (body) => ok({ agent: SCRIBE, machine: machine.id, session: (body as Record<string, unknown>).operation, executed: false, command: 'claude', left_out: [] }) }); await click(button());
    expect(text()).toContain('RunnerStartUnconfirmed'); expect(text()).not.toContain('Its Lys runner has it running');
  });
  it('retains the start operation after an uncertain answer and remount', async () => {
    const first = await open({ ...routes(profile), ['POST ' + start]: refused(503, 'RuntimeUnavailable', 'Unknown start') }); await click(button()); unmountAll(); document.body.innerHTML = '';
    const next = await open(routes(profile)); await click(button()); expect(next.posted).toEqual(first.posted);
  });
  it('blocks a second press while the first action is being submitted', async () => {
    const { posted } = await open(routes(profile)); const action = button(); if (!action) throw new Error('Start action missing');
    await act(async () => { action.click(); action.click(); }); expect(posted).toHaveLength(1);
  });
});
