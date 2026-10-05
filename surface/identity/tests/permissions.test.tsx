/** What an agent may do: the ways it can work as one table, the rules as one table with an add row, and a rule added by its kind and never typed as grammar. Nothing is folded away. */
import { act, useState } from 'react';
import { createRoot } from 'react-dom/client';
import type { Root } from 'react-dom/client';
import { afterEach, describe, expect, it } from 'vitest';
import { choose, click, serve, settle, type } from './harness';
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

function Held({ program, first, agent, tools }: { program: Program; first: Value; agent?: string; tools?: string[] }) {
  const [value, setValue] = useState(first);
  latest = value;
  return <Permissions agent={agent} tools={tools} program={program} value={value} change={setValue} computers={[{ id: computer, name: 'Ada’s laptop' }]} computer={computer} />;
}
async function shown(program: Program, first: Value, agent?: string, routes = {}, tools?: string[]) {
  const posted: { path: string; body: unknown }[] = [];
  const requests = serve({ ...SERVICE, ...routes }, posted);
  const element = document.createElement('div'); document.body.append(element); host = element; root = createRoot(element);
  await act(async () => { root?.render(<Held program={program} first={first} agent={agent} tools={tools} />); });
  return { element, posted, requests };
}
const button = (scope: HTMLElement, label: string) => [...scope.querySelectorAll('button')].find((entry) => (entry.getAttribute('aria-label') ?? entry.textContent?.trim()) === label) ?? null;
const radios = (scope: HTMLElement, name: string) => [...scope.querySelectorAll<HTMLInputElement>('input[name="' + name + '"]')];
const radio = (scope: HTMLElement, name: string, value: string) => radios(scope, name).find((entry) => entry.value === value) ?? null;

const table = (scope: HTMLElement, label: string) => scope.querySelector<HTMLTableElement>('table[aria-label="' + label + '"]');
const ruleRows = (scope: HTMLElement) => [...(table(scope, 'Rules')?.querySelectorAll<HTMLTableRowElement>('tbody tr.rule-row') ?? [])];
const rowOf = (scope: HTMLElement, rule: string) => ruleRows(scope).find((row) => row.querySelector('code')?.textContent === rule) ?? null;
const addRow = (scope: HTMLElement) => {
  const row = scope.querySelector<HTMLElement>('tr[aria-label="Add a rule"]');
  if (!row) throw new Error('The add-a-rule row is missing');
  return row;
};
const options = (select: Element | null) => [...(select?.querySelectorAll('option') ?? [])].map((entry) => entry.getAttribute('value')).filter(Boolean);

describe('The ways it can work', () => {
  it('shows every way Claude Code can work in one table, the usual three first, each with all of its words', async () => {
    const { element, requests } = await shown(claude, { default_mode: 'default' });
    expect(radios(element, 'permission-mode').map((entry) => entry.value)).toEqual(['workspace-only', 'default', 'acceptEdits', 'plan', 'auto', 'dontAsk', 'bypassPermissions']);
    expect(table(element, 'How it works')?.textContent).toContain('Kept to its folder');
    expect(element.textContent).toContain('Asks before it acts');
    expect(element.textContent).toContain('Changes files in its folder without asking');
    expect(element.textContent).toContain('First of default. Second of default.');
    expect(button(element, 'More ways it can work')).toBeNull();
    expect(button(element, 'more')).toBeNull();
    expect(element.querySelector('details')).toBeNull();
    expect(requests).toEqual([]);
  });
  it('carries a warning on the one with no checks', async () => {
    const { element } = await shown(claude, { default_mode: 'default' });
    expect(radio(element, 'permission-mode', 'bypassPermissions')?.closest('tr')?.querySelector('[aria-label="Warning"]')).not.toBeNull();
    expect(radio(element, 'permission-mode', 'default')?.closest('tr')?.querySelector('[aria-label="Warning"]')).toBeNull();
  });
  it('lays each way out in three fixed columns: the radio alone, the name with its own name whole beneath, and the meaning', async () => {
    const { element } = await shown(claude, { default_mode: 'default' });
    const row = radio(element, 'permission-mode', 'bypassPermissions')?.closest('tr');
    const cells = [...row?.children ?? []];
    expect(cells).toHaveLength(3);
    expect(cells[0].className).toBe('mode-pick');
    expect(cells[0].textContent).toBe('');
    expect(cells[1].querySelector('.mode-id')?.textContent).toBe('bypassPermissions');
    expect(cells[1].querySelector('label')?.htmlFor).toBe(radio(element, 'permission-mode', 'bypassPermissions')?.id);
    expect(cells[2].className).toBe('mode-meaning');
    expect(table(element, 'How it works')?.querySelectorAll('col')).toHaveLength(3);
  });
  it('shows a mode that is already chosen', async () => {
    const { element } = await shown(claude, { default_mode: 'plan' });
    expect(radio(element, 'permission-mode', 'plan')?.checked).toBe(true);
  });
  it('sets the mode and nothing else when a choice is picked', async () => {
    const { element } = await shown(claude, { default_mode: 'default', deny: ['WebSearch'], additional_directories: ['/srv/a'] });
    await click(radio(element, 'permission-mode', 'acceptEdits'));
    expect(latest).toEqual({ default_mode: 'acceptEdits', deny: ['WebSearch'], additional_directories: ['/srv/a'] });
  });
  it('shows a mode nobody named by its own id', async () => {
    const { element } = await shown({ ...codex, modes: [mode('strange')] }, { default_mode: 'strange' });
    expect(radio(element, 'permission-mode', 'strange')?.closest('tr')?.querySelector('.mode-name')?.textContent).toContain('strange');
  });
});

describe('The rules', () => {
  const rules: Value = { default_mode: 'default', deny: ['WebSearch', 'Read(//srv/private/**)'], ask: ['Bash(git push *)'], allow: ['WebFetch(domain:example.org)'], additional_directories: ['/srv/a'] };
  it('are summed in one line and all shown in one table, with nothing to open', async () => {
    const { element } = await shown(claude, rules);
    expect(element.textContent).toContain('4 rules: 2 refused, 1 asks first, 1 without asking. 1 extra folder.');
    expect(button(element, 'Show every rule')).toBeNull();
    expect(ruleRows(element)).toHaveLength(4);
    expect(element.querySelectorAll('table[aria-label="Rules"]')).toHaveLength(1);
  });
  it('shows each rule in plain words with the exact rule beside it and what happens in its row, and a rule is removed by its button', async () => {
    const { element } = await shown(claude, rules);
    const refused = rowOf(element, 'Read(//srv/private/**)');
    expect(refused?.children[0]?.textContent).toBe('Read files under /srv/private');
    expect(refused?.querySelector('select')?.value).toBe('deny');
    expect(rowOf(element, 'Bash(git push *)')?.querySelector('select')?.value).toBe('ask');
    expect(rowOf(element, 'WebFetch(domain:example.org)')?.querySelector('select')?.value).toBe('allow');
    expect(refused?.children[3]?.textContent).toBe('Claude Code');
    await click(rowOf(element, 'WebSearch')?.querySelector('button') ?? null);
    expect(latest.deny).toEqual(['Read(//srv/private/**)']);
    expect(latest.ask).toEqual(rules.ask);
  });
  it('changes what happens to a rule in its own row, moving it between the lists and losing nothing', async () => {
    const { element } = await shown(claude, rules);
    await choose(rowOf(element, 'Bash(git push *)')?.querySelector('select') ?? null, 'deny');
    expect(latest).toEqual({ ...rules, ask: [], deny: ['WebSearch', 'Read(//srv/private/**)', 'Bash(git push *)'] });
    expect(rowOf(element, 'Bash(git push *)')?.querySelector('select')?.value).toBe('deny');
  });
  it('says an empty table has nothing in it, in one line', async () => {
    const { element } = await shown(claude, { default_mode: 'default' });
    expect(element.textContent).toContain('No extra rules.');
    expect(table(element, 'Rules')?.querySelector('tbody')?.textContent).toBe('Nothing.');
  });
  it('adds a rule in the table’s last row: the kind, the one thing it needs, then what happens', async () => {
    const { element } = await shown(claude, { default_mode: 'default' });
    const step = addRow(element);
    expect(options(step.querySelector('select[name="rule-kind"]'))).toHaveLength(5);
    expect(step.querySelector('input')).toBeNull();
    expect(button(step, 'Add this rule')?.disabled).toBe(true);
    await choose(step.querySelector('select[name="rule-kind"]'), 'command');
    await type(step.querySelector('input[name="rule-command"]'), 'git push');
    expect(step.textContent).toContain('Bash(git push *)');
    expect(button(step, 'Add this rule')?.disabled).toBe(true);
    await choose(step.querySelector('select[name="rule-list"]'), 'ask');
    await click(button(step, 'Add this rule'));
    expect(latest).toEqual({ default_mode: 'default', ask: ['Bash(git push *)'] });
    expect(rowOf(element, 'Bash(git push *)')?.querySelector('select')?.value).toBe('ask');
    expect(addRow(element).querySelector<HTMLSelectElement>('select[name="rule-kind"]')?.value).toBe('');
    expect(addRow(element).querySelector('input')).toBeNull();
  });
  it('says why a rule cannot be written and adds nothing', async () => {
    const { element } = await shown(claude, { default_mode: 'default' });
    const step = addRow(element);
    await choose(step.querySelector('select[name="rule-kind"]'), 'website');
    await type(step.querySelector('input[name="rule-host"]'), 'https://example.org/page');
    expect(step.textContent).toContain('Type the website’s name alone');
    await choose(step.querySelector('select[name="rule-list"]'), 'deny');
    expect(button(step, 'Add this rule')?.disabled).toBe(true);
    expect(latest).toEqual({ default_mode: 'default' });
  });
  const hard = (id: string, tool: string, kind: string, target?: string) => ({ id, tool, kind, ...(target === undefined ? {} : { target }), authority: 'hard' });
  const policyOf = (rules: unknown[]) => ({ ['/agents/' + SCRIBE + '/policy']: ok({ agent: SCRIBE, digest: null, applies: '', policy: { version: 1, agent: SCRIBE, rules } }) });
  const seeded = [hard('deny-Read', 'Read', 'tool'), hard('deny-top', 'Read', 'path_prefix', '/srv/x'),
    { id: 'liftable', tool: 'Bash', kind: 'tool', authority: { permission: { resource: { kind: 'agent', id: SCRIBE }, action: 'read' } } }];
  it('never says there are no rules while the agent’s policy refuses things: the line above the table carries the count', async () => {
    const { element, requests } = await shown(claude, { default_mode: 'default' }, SCRIBE, policyOf(seeded));
    await settle();
    expect(requests).toEqual(['/agents/' + SCRIBE + '/policy']);
    expect(element.querySelector('.rules-summary')?.textContent).toContain('No extra rules. This agent’s policy refuses 3 things.');
    expect(table(element, 'Rules')?.querySelector('tbody')?.textContent).not.toContain('Nothing.');
  });
  it('shows the rules of the policy as rows of the same table, refused and said to be the policy’s, each written as the start writes it', async () => {
    const { element } = await shown(claude, { default_mode: 'default' }, SCRIBE, policyOf(seeded));
    await settle();
    const locked = ruleRows(element).filter((row) => row.classList.contains('locked'));
    expect(locked.map((entry) => entry.dataset.policy)).toEqual(['deny-Read', 'deny-top', 'liftable']);
    expect(locked.map((entry) => [...entry.querySelectorAll('code')].map((code) => code.textContent))).toEqual([['Read'], ['Read(//srv/x)', 'Read(//srv/x/**)'], []]);
    expect(locked[2]?.children[1]?.textContent).toBe('Judged by Lys at each call');
    expect(locked.every((entry) => entry.children[2]?.textContent === 'Refused' && entry.textContent?.includes('from this agent’s policy') && entry.querySelector('select') === null)).toBe(true);
    expect(locked[0]?.textContent).toContain('Nobody: no grant can lift it');
    expect(locked[2]?.textContent).toContain('A grant of read on agent ' + SCRIBE);
  });
  it('offers the policy as who enforces a new rule only for an agent whose policy was read', async () => {
    const without = await shown(claude, { default_mode: 'default' });
    expect(addRow(without.element).querySelector('select[name="rule-enforcer"]')).toBeNull();
    act(() => root?.unmount()); root = null; host?.remove(); host = null;
    const { element } = await shown(claude, { default_mode: 'default' }, SCRIBE, policyOf(seeded));
    await settle();
    expect(options(addRow(element).querySelector('select[name="rule-enforcer"]'))).toEqual(['program', 'policy']);
  });
  it('says when the policy cannot be read, above the table and in it, and still shows the agent’s own rules', async () => {
    const { element } = await shown(claude, { default_mode: 'default', deny: ['WebSearch'] }, SCRIBE);
    await settle();
    expect(element.querySelector('.rules-summary')?.textContent).toContain('Lys could not read this agent’s policy, so what it refuses is not shown.');
    const body = table(element, 'Rules')?.querySelector('tbody');
    expect(body?.textContent).toContain('Lys could not read this agent’s policy');
    expect(rowOf(element, 'WebSearch')?.querySelector('select')?.value).toBe('deny');
  });
  it('names a policy rule the start cannot write, and says the agent will not start until it is changed', async () => {
    const { element } = await shown(claude, { default_mode: 'default' }, SCRIBE, policyOf([hard('deny-glob-path', 'Glob', 'path_prefix', '/srv'), hard('deny-Read', 'Read', 'tool')]));
    await settle();
    expect(element.textContent).toContain('This policy rule cannot be written for Claude Code, so the agent will not start until it is changed: deny-glob-path');
    expect(element.querySelector('.rules-summary')?.textContent).toContain('This agent’s policy refuses 1 thing.');
  });
  it('says a Codex agent whose policy has rules will not start, and where to remove them', async () => {
    const { element } = await shown(codex, { default_mode: 'workspace-write' }, SCRIBE, policyOf(seeded));
    await settle();
    expect(element.textContent).toContain('This agent’s policy has 2 rules. Codex cannot carry them, so this agent will not start until they are removed from its policy, in the table above.');
    expect(ruleRows(element).map((row) => row.dataset.policy)).toEqual(['deny-Read', 'deny-top', 'liftable']);
    expect(element.querySelector('button[aria-label="Remove policy rule deny-Read"]')).not.toBeNull();
  });
  it('says nothing of a policy to a Codex agent whose policy has no hard rule', async () => {
    const { element } = await shown(codex, { default_mode: 'workspace-write' }, SCRIBE, policyOf([seeded[2]]));
    await settle();
    expect(element.textContent).not.toContain('will not start');
  });
  it('says what must be removed before an agent with rules, extra folders or tools can start as Kept to its folder', async () => {
    const { element } = await shown(claude, { default_mode: 'workspace-only', allow: ['WebSearch'], additional_directories: ['/srv/a'] }, undefined, {}, ['reader']);
    expect(element.textContent).toContain('Kept to its folder cannot carry rules that run without asking or extra folders. Remove these before it can start as Kept to its folder.');
    expect(element.textContent).toContain('These settings list tools, so it cannot start as Kept to its folder.');
    expect(rowOf(element, 'WebSearch')?.textContent).toContain('Not available with Kept to its folder');
    await click(radio(element, 'permission-mode', 'default'));
    expect(element.textContent).not.toContain('cannot start as Kept to its folder');
    expect(element.textContent).not.toContain('Remove these before');
    expect(element.textContent).not.toContain('Not available with Kept to its folder');
  });
  it('says Kept to its folder cannot carry rules that run without asking or extra folders', async () => {
    const { element } = await shown(claude, { default_mode: 'workspace-only' });
    expect(table(element, 'Extra folders')?.textContent).toContain('Not available with Kept to its folder');
    expect(button(element, 'Add an extra folder')).toBeNull();
    const step = addRow(element);
    await choose(step.querySelector('select[name="rule-kind"]'), 'tool');
    await type(step.querySelector('input[name="rule-tool"]'), 'WebSearch');
    expect(options(step.querySelector('select[name="rule-list"]'))).toEqual(['ask', 'deny']);
  });
});

describe('Codex', () => {
  it('has all three ways of working shown, extra folders as a table, and one sentence that it takes no rules', async () => {
    const { element } = await shown(codex, { default_mode: 'workspace-write', additional_directories: ['/srv/a'] });
    expect(radios(element, 'permission-mode').map((entry) => entry.value)).toEqual(['read-only', 'workspace-write', 'danger-full-access']);
    expect(element.textContent).toContain('Codex takes no rules about single tools or files.');
    expect(table(element, 'Rules')).toBeNull();
    expect(radio(element, 'permission-mode', 'danger-full-access')?.closest('label')?.querySelector('[aria-label="Warning"]')).not.toBeNull();
    const folders = table(element, 'Extra folders');
    expect(folders?.textContent).toContain('/srv/a');
    expect(button(element, 'Add an extra folder')).not.toBeNull();
    await click(folders?.querySelector('tbody tr button') ?? null);
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
  it('is only its hard rules, written as the start writes them, with the ones it cannot write named by id', () => {
    expect(forcedBy([
      { id: 'a', tool: 'WebFetch', kind: 'host', target: 'example.org', authority: 'hard' },
      { id: 'b', tool: 'Edit', kind: 'path_prefix', target: '/', authority: 'hard' },
      { id: 'c', tool: 'Glob', kind: 'path_prefix', target: '/srv', authority: 'hard' },
      { id: 'd', tool: 'Read', kind: 'path_prefix', target: '/srv/[x]', authority: 'hard' },
      { id: 'e', tool: 'Read', kind: 'host', target: 'example.org', authority: 'hard' },
      { id: 'f', tool: 'Read', kind: 'tool', target: '/x', authority: 'hard' },
      { id: 'g', tool: 'Bash', kind: 'tool', authority: { permission: { resource: { kind: 'agent', id: 'x' }, action: 'read' } } },
    ])).toEqual({ written: ['WebFetch(domain:example.org)', 'Edit(//)', 'Edit(///**)'], unwritable: ['c', 'd', 'e', 'f'], hard: 6 });
  });
  it('writes a tool with a hyphen in its name, as the start does, and not one that starts with an underscore', () => {
    expect(forcedBy([
      { id: 'h', tool: 'mcp__my-server__read', kind: 'tool', authority: 'hard' },
      { id: 'i', tool: '_hidden', kind: 'tool', authority: 'hard' },
      { id: 'j', tool: '9lives', kind: 'tool', authority: 'hard' },
    ])).toEqual({ written: ['mcp__my-server__read'], unwritable: ['i', 'j'], hard: 3 });
  });
  it('takes the first sentence of a mode’s words and keeps the rest', () => {
    expect(firstSentence('One thing. Another thing.')).toEqual({ first: 'One thing.', rest: 'Another thing.' });
    expect(firstSentence('No full stop')).toEqual({ first: 'No full stop', rest: '' });
  });
});
