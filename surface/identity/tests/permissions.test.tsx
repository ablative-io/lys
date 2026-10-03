/** What an agent may do: a short column of plain choices in front, the rules behind one button, and a rule added by its kind and never typed as grammar. */
import { act, useState } from 'react';
import { createRoot } from 'react-dom/client';
import type { Root } from 'react-dom/client';
import { afterEach, describe, expect, it } from 'vitest';
import { click, serve, settle, type } from './harness';
import { SCRIBE, SERVICE, ok } from './fixtures';
import { Permissions } from '../src/features/provisioning/Permissions';
import type { Program } from '../src/features/provisioning/choices';
import { forcedBy, firstSentence } from '../src/features/provisioning/permission-modes';
import type { Permissions as Value } from '../src/features/provisioning/Provisioning';

const describes = (rules: string[]) => ({ models: { minimum: 1, maximum: null, further_encoding: { kind: 'array' as const } }, permissions: { modes: [], rule_forms: rules }, mcp: { transports: ['stdio'], working_directory: false, handle_variables: false, channel_policies: ['off' as const] }, rendering_contract: 'test' });
const mode = (id: string) => ({ id, meaning: 'First of ' + id + '. Second of ' + id + '.' });
const claude: Program = { name: 'Claude Code', line: '', models: [], builds: [], description: describes(['tool_specifier']),
  modes: ['default', 'acceptEdits', 'plan', 'auto', 'dontAsk', 'bypassPermissions', 'workspace-only'].map(mode) };
const codex: Program = { name: 'Codex', line: '', models: [], builds: [], description: describes([]), modes: ['read-only', 'workspace-write', 'danger-full-access'].map(mode) };
const computer = 'op-' + '1'.repeat(32);

let root: Root | null = null;
let host: HTMLElement | null = null;
let latest: Value = {};
afterEach(() => { if (root) act(() => root?.unmount()); root = null; host?.remove(); host = null; });

function Held({ program, first, agent }: { program: Program; first: Value; agent?: string }) {
  const [value, setValue] = useState(first);
  latest = value;
  return <Permissions agent={agent} program={program} value={value} change={setValue} computers={[{ id: computer, name: 'Ada’s laptop' }]} computer={computer} />;
}
async function shown(program: Program, first: Value, agent?: string, routes = {}) {
  const posted: { path: string; body: unknown }[] = [];
  const requests = serve({ ...SERVICE, ...routes }, posted);
  const element = document.createElement('div'); document.body.append(element); host = element; root = createRoot(element);
  await act(async () => { root?.render(<Held program={program} first={first} agent={agent} />); });
  return { element, posted, requests };
}
const button = (scope: HTMLElement, label: string) => [...scope.querySelectorAll('button')].find((entry) => entry.textContent?.trim() === label) ?? null;
const radios = (scope: HTMLElement, name: string) => [...scope.querySelectorAll<HTMLInputElement>('input[name="' + name + '"]')];
const radio = (scope: HTMLElement, name: string, value: string) => radios(scope, name).find((entry) => entry.value === value) ?? null;

describe('The quick picker', () => {
  it('shows three plain choices for Claude Code to begin with, and no drop-down anywhere', async () => {
    const { element, requests } = await shown(claude, { default_mode: 'default' });
    expect(radios(element, 'permission-mode').map((entry) => entry.value)).toEqual(['workspace-only', 'default', 'acceptEdits']);
    expect(element.textContent).toContain('Kept to its folder');
    expect(element.textContent).toContain('Asks before it acts');
    expect(element.textContent).toContain('Changes files in its folder without asking');
    expect(element.textContent).toContain('First of default.');
    expect(element.textContent).not.toContain('Second of default.');
    expect(element.querySelector('select')).toBeNull();
    expect(requests).toEqual([]);
  });
  it('opens the rest in place, and the one with no checks carries a warning', async () => {
    const { element } = await shown(claude, { default_mode: 'default' });
    await click(button(element, 'More ways it can work'));
    expect(radios(element, 'permission-mode').map((entry) => entry.value)).toEqual(['workspace-only', 'default', 'acceptEdits', 'plan', 'auto', 'dontAsk', 'bypassPermissions']);
    expect(radio(element, 'permission-mode', 'bypassPermissions')?.closest('label')?.querySelector('[aria-label="Warning"]')).not.toBeNull();
    expect(radio(element, 'permission-mode', 'default')?.closest('label')?.querySelector('[aria-label="Warning"]')).toBeNull();
  });
  it('shows a mode that is already chosen even when it is one of the rest', async () => {
    const { element } = await shown(claude, { default_mode: 'plan' });
    expect(radio(element, 'permission-mode', 'plan')?.checked).toBe(true);
  });
  it('sets the mode and nothing else when a choice is picked', async () => {
    const { element } = await shown(claude, { default_mode: 'default', deny: ['WebSearch'], additional_directories: ['/srv/a'] });
    await click(radio(element, 'permission-mode', 'acceptEdits'));
    expect(latest).toEqual({ default_mode: 'acceptEdits', deny: ['WebSearch'], additional_directories: ['/srv/a'] });
  });
  it('opens the rest of a mode’s words in place', async () => {
    const { element } = await shown(codex, { default_mode: 'read-only' });
    await click(button(element, 'more'));
    expect(element.textContent).toContain('Second of read-only.');
  });
  it('shows a mode nobody named by its own id', async () => {
    const { element } = await shown({ ...codex, modes: [mode('strange')] }, { default_mode: 'strange' });
    expect(radio(element, 'permission-mode', 'strange')?.closest('label')?.textContent).toContain('strange');
  });
});

describe('The rules', () => {
  const rules: Value = { default_mode: 'default', deny: ['WebSearch', 'Read(//srv/private/**)'], ask: ['Bash(git push *)'], allow: ['WebFetch(domain:example.org)'], additional_directories: ['/srv/a'] };
  it('are summed in one line and stay closed until asked for', async () => {
    const { element } = await shown(claude, rules);
    expect(element.textContent).toContain('4 rules: 2 refused, 1 asks first, 1 without asking. 1 extra folder.');
    expect(element.textContent).not.toContain('Read files under /srv/private');
    expect(element.querySelector('section[aria-label="Refused"]')).toBeNull();
  });
  it('open as three lists in plain words with the exact rule beside each, and a rule is removed by its button', async () => {
    const { element } = await shown(claude, rules);
    await click(button(element, 'Show every rule'));
    const refused = element.querySelector<HTMLElement>('section[aria-label="Refused"]');
    expect(refused?.textContent).toContain('Read files under /srv/private');
    expect(refused?.textContent).toContain('Read(//srv/private/**)');
    expect(element.querySelector('section[aria-label="Asks first"]')?.textContent).toContain('Bash(git push *)');
    expect(element.querySelector('section[aria-label="Without asking"]')?.textContent).toContain('WebFetch(domain:example.org)');
    const row = [...(refused?.querySelectorAll('li') ?? [])].find((entry) => entry.textContent?.includes('WebSearch'));
    await click(row?.querySelector('button') ?? null);
    expect(latest.deny).toEqual(['Read(//srv/private/**)']);
    expect(latest.ask).toEqual(rules.ask);
  });
  it('says an empty list has nothing in it, in one line', async () => {
    const { element } = await shown(claude, { default_mode: 'default' });
    expect(element.textContent).toContain('No extra rules.');
    await click(button(element, 'Show every rule'));
    expect(element.querySelector('section[aria-label="Asks first"]')?.textContent).toContain('Nothing.');
  });
  it('adds a rule step by step: the kind, the one thing it needs, then what happens', async () => {
    const { element } = await shown(claude, { default_mode: 'default' });
    await click(button(element, 'Show every rule'));
    await click(button(element, 'Add a rule'));
    const step = element.querySelector<HTMLElement>('[aria-label="Add a rule"]');
    if (!step) throw new Error('The add-a-rule step is missing');
    expect(radios(step, 'rule-kind')).toHaveLength(5);
    expect(step.querySelector('input:not([type="radio"])')).toBeNull();
    expect(button(step, 'Add this rule')?.disabled).toBe(true);
    expect(step.textContent).toContain('Choose what the rule is about.');
    await click(radio(step, 'rule-kind', 'command'));
    expect(radios(step, 'rule-list')).toHaveLength(0);
    await type(step.querySelector('input[name="rule-command"]'), 'git push');
    expect(step.textContent).toContain('Bash(git push *)');
    expect(button(step, 'Add this rule')?.disabled).toBe(true);
    await click(radio(step, 'rule-list', 'ask'));
    await click(button(step, 'Add this rule'));
    expect(latest).toEqual({ default_mode: 'default', ask: ['Bash(git push *)'] });
    expect(element.querySelector('[aria-label="Add a rule"]')).toBeNull();
  });
  it('says why a rule cannot be written and adds nothing', async () => {
    const { element } = await shown(claude, { default_mode: 'default' });
    await click(button(element, 'Show every rule'));
    await click(button(element, 'Add a rule'));
    const step = element.querySelector<HTMLElement>('[aria-label="Add a rule"]');
    if (!step) throw new Error('The add-a-rule step is missing');
    await click(radio(step, 'rule-kind', 'website'));
    await type(step.querySelector('input[name="rule-host"]'), 'https://example.org/page');
    expect(step.textContent).toContain('Type the website’s name alone');
    expect(radios(step, 'rule-list')).toHaveLength(0);
    expect(button(step, 'Add this rule')?.disabled).toBe(true);
    expect(latest).toEqual({ default_mode: 'default' });
  });
  it('shows the rules the agent’s policy forces, locked, read only when the lists are opened', async () => {
    const policy = { agent: SCRIBE, digest: null, applies: '', policy: { version: 1, agent: SCRIBE, rules: [
      { id: 'deny-Read', tool: 'Read', kind: 'tool', authority: 'hard' },
      { id: 'deny-top', tool: 'Read', kind: 'path_prefix', target: '/srv/x', authority: 'hard' },
      { id: 'liftable', tool: 'Bash', kind: 'tool', authority: { permission: { resource: { kind: 'agent', id: SCRIBE }, action: 'read' } } },
    ] } };
    const { element, requests } = await shown(claude, { default_mode: 'default' }, SCRIBE, { ['/agents/' + SCRIBE + '/policy']: ok(policy) });
    expect(requests).toEqual([]);
    await click(button(element, 'Show every rule'));
    await settle();
    expect(requests).toEqual(['/agents/' + SCRIBE + '/policy']);
    const locked = [...element.querySelectorAll('section[aria-label="Refused"] li.locked')];
    expect(locked.map((entry) => entry.querySelector('code')?.textContent)).toEqual(['Read', 'Read(//srv/x)', 'Read(//srv/x/**)']);
    expect(locked.every((entry) => entry.textContent?.includes('from this agent’s policy') && entry.querySelector('button') === null)).toBe(true);
  });
  it('says so when the policy cannot be read, and still shows the agent’s own rules', async () => {
    const { element } = await shown(claude, { default_mode: 'default', deny: ['WebSearch'] }, SCRIBE);
    await click(button(element, 'Show every rule'));
    await settle();
    const refused = element.querySelector('section[aria-label="Refused"]');
    expect(refused?.textContent).toContain('Lys could not read this agent’s policy');
    expect(refused?.textContent).toContain('WebSearch');
  });
  it('says Kept to its folder cannot carry rules that run without asking or extra folders', async () => {
    const { element } = await shown(claude, { default_mode: 'workspace-only' });
    await click(button(element, 'Show every rule'));
    expect(element.querySelector('section[aria-label="Without asking"]')?.textContent).toContain('Not available with Kept to its folder');
    expect(element.querySelector('section[aria-label="Extra folders"]')?.textContent).toContain('Not available with Kept to its folder');
    expect(button(element, 'Add an extra folder')).toBeNull();
    await click(button(element, 'Add a rule'));
    const step = element.querySelector<HTMLElement>('[aria-label="Add a rule"]');
    if (!step) throw new Error('The add-a-rule step is missing');
    await click(radio(step, 'rule-kind', 'tool'));
    await type(step.querySelector('input[name="rule-tool"]'), 'WebSearch');
    expect(radios(step, 'rule-list').map((entry) => entry.value)).toEqual(['ask', 'deny']);
  });
});

describe('Codex', () => {
  it('has the picker with all three shown, extra folders, and one sentence that it takes no rules', async () => {
    const { element } = await shown(codex, { default_mode: 'workspace-write', additional_directories: ['/srv/a'] });
    expect(radios(element, 'permission-mode').map((entry) => entry.value)).toEqual(['read-only', 'workspace-write', 'danger-full-access']);
    expect(button(element, 'More ways it can work')).toBeNull();
    expect(element.textContent).toContain('Codex takes no rules about single tools or files.');
    expect(button(element, 'Show every rule')).toBeNull();
    expect(element.querySelector('section[aria-label="Refused"]')).toBeNull();
    expect(radio(element, 'permission-mode', 'danger-full-access')?.closest('label')?.querySelector('[aria-label="Warning"]')).not.toBeNull();
    const folders = element.querySelector<HTMLElement>('section[aria-label="Extra folders"]');
    expect(folders?.textContent).toContain('/srv/a');
    expect(button(element, 'Add an extra folder')).not.toBeNull();
    await click(folders?.querySelector('li button') ?? null);
    expect(latest).toEqual({ default_mode: 'workspace-write', additional_directories: [] });
  });
  it('adds an extra folder from the computer’s own folders', async () => {
    const { element, posted } = await shown(codex, { default_mode: 'workspace-write' }, undefined, { ['POST /network/machines/' + computer + '/folders']: ok({ machine: computer, under: '/Users/ada', folders: ['notes'] }) });
    await click(button(element, 'Add an extra folder'));
    await settle();
    await click(button(element, 'Add ada'));
    expect(latest.additional_directories).toEqual(['/Users/ada']);
    expect(posted.map((entry) => entry.path)).toEqual(['/network/machines/' + computer + '/folders']);
  });
});

describe('What the policy forces', () => {
  it('is only its hard rules, written as the start writes them', () => {
    expect(forcedBy([
      { id: 'a', tool: 'WebFetch', kind: 'host', target: 'example.org', authority: 'hard' },
      { id: 'b', tool: 'Edit', kind: 'path_prefix', target: '/', authority: 'hard' },
      { id: 'c', tool: 'Glob', kind: 'path_prefix', target: '/srv', authority: 'hard' },
      { id: 'd', tool: 'Read', kind: 'path_prefix', target: '/srv/[x]', authority: 'hard' },
    ])).toEqual(['WebFetch(domain:example.org)', 'Edit(//)', 'Edit(///**)']);
  });
  it('takes the first sentence of a mode’s words and keeps the rest', () => {
    expect(firstSentence('One thing. Another thing.')).toEqual({ first: 'One thing.', rest: 'Another thing.' });
    expect(firstSentence('No full stop')).toEqual({ first: 'No full stop', rest: '' });
  });
});
