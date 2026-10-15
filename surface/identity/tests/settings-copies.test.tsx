import { beforeEach, describe, expect, it } from 'vitest';
import type { Machine } from '../src/features/network/contract';
import type { Program } from '../src/features/provisioning/choices';
import type { ProvisioningProfile } from '../src/features/provisioning/Provisioning';
import { choose, click, mount, text } from './harness';
import { ADA, SCRIBE, SERVICE, ok, refused } from './fixtures';
import type { Route } from './fixtures';

const path = '/agents/' + SCRIBE + '/provisioning';
const computer = (digit: string, name: string, state: Machine['state'] = 'in_use'): Machine => ({
  id: 'op-' + digit.repeat(32), name, state, runtime: 'lys-runner', kind: 'laptop', slots: 4,
  may_run: [{ id: SCRIBE, display_name: 'Scribe', state: 'active' }], may_reach: [],
  named_by: ADA, named_at: 1, retired_at: state === 'retired' ? 2 : null, last_report_at: 1,
});
const desk = computer('1', 'Writing desk');
const laptop = computer('2', 'Travel laptop');
const retired = computer('3', 'Retired computer', 'retired');
const description: Program['description'] = {
  models: { minimum: 1, maximum: 1, further_encoding: { kind: 'array' } },
  permissions: { modes: ['default'], rule_forms: [] },
  mcp: { transports: [], working_directory: false, handle_variables: false, channel_policies: [] },
  rendering_contract: 'claude-code/template-v1',
};
const copy = (program: string, version: string, name = 'Claude Code ' + version): Program['builds'][number] => ({
  name, program, package: 'claude ' + version, from: 'runner',
});
const program = (builds: Program['builds'], not_found?: string): Program => ({
  name: 'Claude Code', line: 'Read and change code', description, builds,
  models: [{ id: 'default', label: 'Default' }], modes: [{ id: 'default', meaning: 'Ask before changes' }],
  ...(not_found ? { not_found } : {}),
});
const profile: ProvisioningProfile = {
  version: 1, operation: 'op-' + 'a'.repeat(32), set_by: ADA, set_at: 1, reviewed_by: ADA,
  harness: { name: 'Claude Code', program: 'claude', package: 'claude 1', description },
  model_access: ['default'], permissions: { default_mode: 'default' }, working_folder: '/srv',
  tools: [], skills: [], mcp_servers: [], instructions: '', note: 'Saved settings', session: null,
};
const answer = (kept: unknown) => ({ agent: SCRIBE, profile: kept, versions: [], enforced: false });
const scoped = (machine: Machine, programs: Program[]) => ok({ machine: machine.id, programs });
const picker = () => document.querySelector<HTMLSelectElement>('select[name="build"]');
const field = (name: string) => document.querySelector<HTMLSelectElement>('select[name="' + name + '"]');
const button = () => document.querySelector<HTMLButtonElement>('section[aria-label="Settings of this agent"] button[type="submit"]');
const groups = () => [...document.querySelectorAll<HTMLOptGroupElement>('select[name="build"] optgroup')];

function routes(kept: ProvisioningProfile = profile): Record<string, Route> {
  return {
    ...SERVICE, [path]: ok(answer(kept)), '/roles': ok({ roles: [] }),
    '/network': ok({ machines: [desk, laptop, retired], reports_served: true }),
    '/harnesses': ok({ programs: [program([copy('/aggregate-only/claude', 'aggregate'), copy('/retired-only/claude', 'retired')])] }),
    ['/harnesses?machine=' + desk.id]: scoped(desk, [program([
      copy('claude', '1'), copy('claude', '1', 'Reviewed desk profile'), copy('/opt/forks/claude', 'fork'),
    ])]),
    ['/harnesses?machine=' + laptop.id]: scoped(laptop, [program([copy('claude', '2')])]),
    ['/harnesses?machine=' + retired.id]: scoped(retired, [program([copy('/retired-only/claude', 'retired')])]),
    ['POST ' + path]: (body) => {
      const draft = body as Record<string, unknown>;
      const version = Number(draft.from_version) + 1;
      return ok({ ...answer({ ...draft, version, set_by: ADA, set_at: 2, reviewed_by: null }), recorded: { operation: draft.operation, version } });
    },
  };
}

beforeEach(() => sessionStorage.clear());

describe('Installed copies on an agent’s Settings page', () => {
  it('groups actual installed commands by computer name, omits aggregate profiles and retired computers, and keeps the same command on each computer', async () => {
    const result = await mount('#/file/' + SCRIBE + '/provisioning', routes());
    expect(groups().map((group) => group.label)).toEqual(['Writing desk', 'Travel laptop']);
    expect(groups().map((group) => group.querySelectorAll('option').length)).toEqual([2, 1]);
    expect(groups()[0].textContent).toContain('/opt/forks/claude');
    expect(groups()[0].textContent).not.toContain('Reviewed desk profile');
    expect(groups()[1].textContent).toContain('Claude Code 2');
    expect(picker()?.textContent).not.toContain('/aggregate-only/claude');
    expect(picker()?.textContent).not.toContain('/retired-only/claude');
    expect(picker()?.textContent).not.toContain('Retired computer');
    expect(result.requests).toContain('/harnesses?machine=' + desk.id);
    expect(result.requests).toContain('/harnesses?machine=' + laptop.id);
    expect(result.requests).not.toContain('/harnesses?machine=' + retired.id);
  });

  it('shows only the chosen computer’s copies and never asks another computer for them', async () => {
    const result = await mount('#/file/' + SCRIBE + '/provisioning', routes({ ...profile, runs_on: laptop.id }));
    expect(groups().map((group) => group.label)).toEqual(['Travel laptop']);
    expect(picker()?.textContent).toContain('Claude Code 2');
    expect(picker()?.textContent).not.toContain('/opt/forks/claude');
    expect(result.requests).toContain('/harnesses?machine=' + laptop.id);
    expect(result.requests).not.toContain('/harnesses?machine=' + desk.id);
    expect(picker()?.value).toBe('claude\nclaude 2');
  });

  it('reads a copy found through a login PATH folder named with .. at its plain path, keeping the computer’s programs and models', async () => {
    const found = { ...program([copy('/home/ward/.local/share/../bin/claude', '3')]), models: [{ id: 'default', label: 'Default' }, { id: 'opus', label: 'Opus' }] };
    await mount('#/file/' + SCRIBE + '/provisioning', { ...routes({ ...profile, runs_on: laptop.id }), ['/harnesses?machine=' + laptop.id]: scoped(laptop, [found]) });
    expect(text()).not.toContain('did not match that computer or the program-list format');
    expect(picker()?.textContent).toContain('Claude Code 3');
    expect([...field('model')!.options].map((option) => option.value)).toEqual(['default', 'opus']);
  });

  it('pins a copy’s computer when a person chooses it, and saves that computer with that command', async () => {
    const result = await mount('#/file/' + SCRIBE + '/provisioning', routes());
    const selected = groups().find((group) => group.label === 'Travel laptop')?.querySelector('option');
    expect(selected).toBeDefined();
    if (!(selected instanceof HTMLOptionElement)) throw new Error('Travel laptop’s installed copy is missing');
    await choose(picker(), selected.value);
    expect(field('runs_on')?.value).toBe(laptop.id);
    expect(groups().map((group) => group.label)).toEqual(['Travel laptop']);
    expect(picker()?.value).toBe('claude\nclaude 2');
    await click(button());
    expect(result.posted.map((entry) => entry.path)).toEqual([path]);
    expect(result.posted[0].body).toMatchObject({ runs_on: laptop.id, harness: { program: 'claude', package: 'claude 2' } });
  });

  it('deduplicates legacy updater paths with their launch command while keeping a distinct fork', async () => {
    const service = routes({ ...profile, runs_on: desk.id });
    service['/harnesses?machine=' + desk.id] = scoped(desk, [program([
      copy('/Users/test/.local/share/claude/versions/2.1.288/claude', '2.1.288'),
      copy('claude', '2.1.299'), copy('claude', '2.1.299', 'Reviewed Claude profile'), copy('/opt/forks/claude', 'fork'),
    ])]);
    await mount('#/file/' + SCRIBE + '/provisioning', service);
    expect(groups()[0].querySelectorAll('option')).toHaveLength(2);
    expect(picker()?.value).toBe('claude\nclaude 2.1.299');
    expect(picker()?.textContent).not.toContain('2.1.288');
    expect(picker()?.textContent).toContain('/opt/forks/claude');
  });

  it.each(['claude', 'codex', 'cdx'])('saves the reported %s command rather than its resolved executable', async (command) => {
    const service = routes({ ...profile, harness: null, runs_on: desk.id });
    service['/harnesses?machine=' + desk.id] = scoped(desk, [program([
      { ...copy('/Users/x/.local/share/claude/versions/2.1.300', '2.1.300'), command },
    ])]);
    const result = await mount('#/file/' + SCRIBE + '/provisioning', service);
    await choose(field('prompt'), 'keep');
    expect(button()?.disabled, document.querySelector('.save-settings .why-not')?.textContent ?? '').toBe(false);
    await click(button());
    expect(result.posted[0]?.body).toMatchObject({ harness: { program: command } });
    expect(picker()?.value).toBe(command + '\nclaude 2.1.300');
  });

  it('offers one choice per reported command across resolved paths and versions', async () => {
    const service = routes({ ...profile, harness: null, runs_on: desk.id });
    service['/harnesses?machine=' + desk.id] = scoped(desk, [program([
      { ...copy('/Users/x/.local/share/claude/versions/2.1.300', '2.1.300'), command: 'claude' },
      { ...copy('/Users/x/.local/bin/claude', '2.1.299'), command: 'claude' },
      { ...copy('/opt/bin/cdx', '1'), command: 'cdx' },
    ])]);
    await mount('#/file/' + SCRIBE + '/provisioning', service);
    expect([...groups()[0].querySelectorAll('option')].map((option) => option.value)).toEqual([
      'claude\nclaude 2.1.300', 'cdx\nclaude 1',
    ]);
  });

  it('clears an installed copy when the computer changes and requires a choice when the new computer reports several commands', async () => {
    const fork = { ...profile.harness!, program: '/opt/forks/claude', package: 'claude fork' };
    const service = routes({ ...profile, runs_on: desk.id, harness: fork });
    service['/harnesses?machine=' + laptop.id] = scoped(laptop, [program([copy('claude', '2'), copy('/opt/travel/claude', 'travel')])]);
    const result = await mount('#/file/' + SCRIBE + '/provisioning', service);
    expect(picker()?.value).toBe('/opt/forks/claude\nclaude fork');
    await choose(field('runs_on'), laptop.id);
    expect(groups().map((group) => group.label)).toEqual(['Travel laptop']);
    expect(picker()?.value).toBe('');
    expect(picker()?.textContent).not.toContain('/opt/forks/claude');
    expect(button()?.disabled).toBe(true);
    await click(button());
    expect(result.posted).toEqual([]);
  });

  it('does not present aggregate copies as installed when no computer with a runner exists', async () => {
    const service = routes({ ...profile, harness: null });
    service['/network'] = ok({ machines: [retired, { ...desk, runtime: null }], reports_served: true });
    const result = await mount('#/file/' + SCRIBE + '/provisioning', service);
    expect(groups()).toEqual([]);
    expect(picker()?.textContent).not.toContain('/aggregate-only/claude');
    expect(result.requests.some((request) => request.startsWith('/harnesses?machine='))).toBe(false);
    expect(text()).toContain('No computer with a connected runner is available to report installed copies of Claude Code.');
    expect(button()?.disabled).toBe(true);
    expect(result.posted).toEqual([]);
  });

  it('accepts the server’s installed source and the explicitly requested scope when older answers omit its envelope', async () => {
    const service = routes({ ...profile, runs_on: desk.id });
    service['/harnesses?machine=' + desk.id] = ok({ programs: [{ ...program([]), builds: [{ ...copy('claude', '1'), from: 'installed' }] }] });
    await mount('#/file/' + SCRIBE + '/provisioning', service);
    expect(groups().map((group) => group.label)).toEqual(['Writing desk']);
    expect(picker()?.value).toBe('claude\nclaude 1');
    expect(text()).not.toContain('Lys could not read this computer’s installed programs');
  });

  it.each([
    ['missing program list', { machine: desk.id }],
    ['unreadable program list', { machine: desk.id, programs: null }],
    ['another computer’s envelope', { machine: laptop.id, programs: [program([copy('claude', '1')])] }],
    ['another computer’s copy', { machine: desk.id, programs: [program([{ ...copy('claude', '1'), machine: laptop.id }])] }],
    ['malformed copies', { machine: desk.id, programs: [{ ...program([]), builds: 'not a list' }] }],
    ['unknown copy source', { machine: desk.id, programs: [{ ...program([]), builds: [{ ...copy('claude', '1'), from: 'guessed' }] }] }],
    ['unnamed copy', { machine: desk.id, programs: [program([{ ...copy('claude', '1'), name: '' }])] }],
    ['unnamed package', { machine: desk.id, programs: [program([{ ...copy('claude', '1'), package: '' }])] }],
  ])('names a %s without trusting the aggregate list or sending a save', async (description, body) => {
    const service = routes({ ...profile, runs_on: desk.id });
    service['/harnesses?machine=' + desk.id] = ok(body);
    const result = await mount('#/file/' + SCRIBE + '/provisioning', service);
    expect(text()).toContain('Installed programs on Writing desk');
    expect(text()).toContain('Lys could not read this computer’s installed programs');
    expect(document.querySelector('[title^="harness_programs_unreadable:"]')).not.toBeNull();
    expect(groups()).toEqual([]);
    if (!button()) throw new Error('Save button missing while naming ' + String(description));
    expect(button()?.disabled).toBe(true);
    await click(button());
    expect(result.posted).toEqual([]);
  });

  it('names a failed computer read and lets the person explicitly choose a computer that answered', async () => {
    const service = routes();
    service['/harnesses?machine=' + laptop.id] = refused(503, 'runner_unreachable', 'The laptop is disconnected.');
    const result = await mount('#/file/' + SCRIBE + '/provisioning', service);
    expect(text()).toContain('Installed programs on Travel laptop');
    expect(text()).toContain('Lys could not reach this computer’s runner');
    expect(button()?.disabled).toBe(true);
    expect(groups().map((group) => group.label)).toEqual(['Writing desk']);
    await choose(field('runs_on'), desk.id);
    expect(groups().map((group) => group.label)).toEqual(['Writing desk']);
    expect(text()).not.toContain('Installed programs on Travel laptop');
    expect(result.posted).toEqual([]);
  });

  it('keeps a discovery refusal in details and gives its sentence as the headline', async () => {
    const reason = 'VersionCommandFailed: claude exited unsuccessfully';
    const service = routes({ ...profile, harness: null, runs_on: desk.id });
    service['/harnesses?machine=' + desk.id] = scoped(desk, [program([], reason)]);
    await mount('#/file/' + SCRIBE + '/provisioning', service);
    const problem = document.querySelector<HTMLElement>('[title="' + reason + '"]');
    expect(problem).not.toBeNull();
    expect(problem?.querySelector(':scope > p')?.textContent).toBe('The program’s version command failed. Check its installed copy before choosing it.');
    expect(problem?.querySelector('details')?.textContent).toContain(reason);
    expect(button()?.disabled).toBe(true);
  });
});
