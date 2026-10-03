/** A program Lys lists offers its own models and modes, its default among them, never a second default beside it. */
import { beforeEach, describe, expect, it } from 'vitest';
import { mount } from './harness';
import { ADA, SCRIBE, SERVICE, ok } from './fixtures';
import type { ProvisioningAnswer, ProvisioningProfile } from '../src/features/provisioning/Provisioning';

beforeEach(() => sessionStorage.clear());
const path = '/agents/' + SCRIBE + '/provisioning';
const description = { models: { minimum: 1, maximum: null, further_encoding: { kind: 'delimited', separator: ',' } }, permissions: { modes: ['default', 'acceptEdits'], rule_forms: ['tool_specifier'] }, mcp: { transports: ['stdio', 'http'], working_directory: false, handle_variables: true, channel_policies: ['off', 'wake'] }, rendering_contract: 'claude-code/template-v1' };
const profile: ProvisioningProfile = { version: 1, operation: 'op-' + 'a'.repeat(32), instructions: '', note: 'Initial profile', model_access: [], tools: [], skills: [], mcp_servers: [], set_by: ADA, set_at: 1790000000,
  harness: { name: 'Claude Code', description, program: '/opt/seat/bin/claude', package: 'claude-code-seat' }, permissions: null } as ProvisioningProfile;
const answer: ProvisioningAnswer = { agent: SCRIBE, profile, versions: [{ version: 1, set_by: ADA, set_at: 1790000000, note: 'Initial profile' }], enforced: false };
const programs = { programs: [{ name: 'Claude Code', line: 'An agent from Anthropic that helps read, change and test code.',
  models: [{ id: 'default', label: 'Default for this account' }, { id: 'opus', label: 'Opus' }],
  modes: [{ id: 'default', meaning: 'Reads freely and asks before most changes and commands.' }, { id: 'acceptEdits', meaning: 'Reads and edits files without asking.' }],
  description, builds: [{ name: 'Seat', program: '/opt/seat/bin/claude', package: 'claude-code-seat', from: 'profile' }] }] };
const routes = { ...SERVICE, [path]: ok(answer), '/harnesses': ok(programs), ['POST ' + path]: (body: unknown) => ok({ ...answer, profile: { ...profile, ...(body as Record<string, unknown>), version: 2 } }) };
const options = (select: Element | null | undefined) => [...(select?.querySelectorAll('option') ?? [])].map((option) => option.textContent);

describe('A listed program', () => {
  it('offers its own default model and its modes as plain choices, and no second default beside them', async () => {
    await mount('#/file/' + SCRIBE + '/provisioning', routes);
    const model = document.querySelector<HTMLSelectElement>('select[name="model"]');
    expect(options(model)).toEqual(['Default for this account', 'Opus']);
    expect(model?.value).toBe('default');
    const modes = [...document.querySelectorAll<HTMLInputElement>('input[name="permission-mode"]')];
    expect(modes.map((entry) => entry.value)).toEqual(['default', 'acceptEdits']);
    expect(modes.filter((entry) => entry.checked).map((entry) => entry.value)).toEqual(['default']);
    expect(document.querySelector('.permissions')?.textContent).toContain('Asks before it acts');
    expect(document.querySelector('.permissions')?.textContent).toContain('Reads freely and asks before most changes and commands.');
    expect(document.querySelector('.mode-words')?.textContent).toBe('Lys starts Claude Code without this computer’s own settings, plugins, hooks and connected tools.Instruction files (CLAUDE.md) and Claude Code’s own memory on this computer may still be read. Lys does not check each action.');
  });
  it('keeps the prompt textbox optional and behind a choice', async () => {
    await mount('#/file/' + SCRIBE + '/provisioning', routes);
    expect(document.querySelector('textarea[name="instructions"]')).toBeNull();
    expect(document.querySelector('input[required], textarea[required]')).toBeNull();
  });
});
