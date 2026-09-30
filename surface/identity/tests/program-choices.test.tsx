/** A program Lys lists offers its own models and modes, its default among them, never a second default beside it. */
import { act } from 'react';
import { beforeEach, describe, expect, it } from 'vitest';
import { $, mount, settle } from './harness';
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
  it('offers its own default model and mode, and no second default beside them', async () => {
    await mount('#/file/' + SCRIBE + '/provisioning', routes);
    const model = [...document.querySelectorAll('label.field')].find((label) => label.textContent?.startsWith('Model'))?.querySelector('select');
    expect(options(model)).toEqual(['Default for this account', 'Opus']);
    expect(model?.value).toBe('default');
    expect(options($('select[name="mode"]'))).toEqual(['default — Reads freely and asks before most changes and commands.', 'acceptEdits — Reads and edits files without asking.']);
  });
  it('records the model the screen shows', async () => {
    const { posted } = await mount('#/file/' + SCRIBE + '/provisioning', routes);
    const form = $('form[aria-label="This agent\'s settings"]');
    const note = form?.querySelector<HTMLInputElement>('[name="note"]');
    if (!form || !note) throw new Error('Profile form missing');
    note.value = 'Choose the model';
    await act(async () => { form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })); });
    await settle();
    expect(posted[0]).toMatchObject({ path, body: { model_access: ['default'], permissions: { default_mode: 'default' } } });
  });
});
