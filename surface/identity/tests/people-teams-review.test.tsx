/**
 * What a first-time person sees on People and agents, Add a person, Add an agent, Teams, an agent's run and the
 * estate approval: plain words in the open, every code under Error details, fields laid out for what they hold,
 * and the obvious controls (Cancel, Save, Undo) where a change is made.
 */
import { act } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { $, $$, click, mount, settle, text } from './harness';
import { ADA, ME, SCRIBE, SERVICE, ok, refused } from './fixtures';
import type { Route } from './fixtures';
import { budgetsView } from './budget-fixtures';
import { mockTerminal } from './terminal-double';
vi.mock('@gespenst/core', () => ({ createTerminal: mockTerminal }));

beforeEach(() => sessionStorage.clear());

const button = (label: string) => [...document.querySelectorAll('button')].find((entry) => (entry.getAttribute('aria-label') ?? entry.textContent) === label) ?? null;
/** What a person reads without opening any fold: every closed details element left out, and no hover titles. */
function openWords(scope: Element | null = document.body): string {
  if (!scope) return '';
  const copy = scope.cloneNode(true) as Element;
  for (const fold of [...copy.querySelectorAll('details')]) fold.remove();
  return copy.textContent ?? '';
}
async function typeInto(input: Element | null, value: string) {
  if (!(input instanceof HTMLInputElement) && !(input instanceof HTMLTextAreaElement)) throw new Error('No field to type into');
  const prototype = input instanceof HTMLInputElement ? HTMLInputElement.prototype : HTMLTextAreaElement.prototype;
  await act(async () => { Object.getOwnPropertyDescriptor(prototype, 'value')?.set?.call(input, value); input.dispatchEvent(new Event('input', { bubbles: true })); });
  await settle();
}
async function chooseIn(select: Element | null, value: string) {
  if (!(select instanceof HTMLSelectElement)) throw new Error('No choice to make');
  await act(async () => { select.value = value; select.dispatchEvent(new Event('change', { bubbles: true })); });
  await settle();
}

describe('People and agents', () => {
  it('says a refused read of access in words in each row, never the refusal’s name', async () => {
    await mount('#/people', { ...SERVICE, '/grants': refused(503, 'GrantsUnavailable', 'the grant store is down') });
    const rows = $$('tbody tr[data-pick]');
    expect(rows.length).toBeGreaterThan(0);
    for (const row of rows) expect(openWords(row)).not.toContain('GrantsUnavailable');
    expect(rows[0].textContent).toContain('Not known');
  });
});

describe('Add a person', () => {
  it('lays first and last name side by side in a bounded form, and says what the email is for', async () => {
    await mount('#/people/new');
    const form = $('form[aria-label="Add a person"]');
    expect(form?.classList.contains('add-person-form')).toBe(true);
    const grid = form?.querySelector('.form-grid');
    expect(grid?.querySelector('input[name="given_name"]')).not.toBeNull();
    expect(grid?.querySelector('input[name="family_name"]')).not.toBeNull();
    const email = grid?.querySelector('input[name="email"]')?.closest('label');
    expect(email?.classList.contains('wide')).toBe(true);
    expect(email?.textContent).toContain('Their sign-in name.');
    // Add is greyed with its reason beside it, in the same row.
    const row = form?.querySelector('.add-person-act');
    expect((row?.querySelector('button[type="submit"]') as HTMLButtonElement | null)?.disabled).toBe(true);
    expect(row?.querySelector('.why-not')?.textContent).toBe('Enter their first name, last name and email.');
  });
});

describe('Add an agent', () => {
  it('has the page header every other page has: where it sits, its title and what it is for', async () => {
    await mount('#/agents/new');
    const header = $('header.page-header');
    expect(header?.querySelector('h1')?.textContent).toBe('Add an agent');
    expect(header?.querySelector('.breadcrumb a')?.getAttribute('href')).toBe('#/people');
    expect(header?.querySelector('.sub')?.textContent).toContain('An agent is an AI that works for a person.');
  });
});

describe('An agent’s run', () => {
  const LIVE = 'op-' + '5'.repeat(32);
  const running = { session: LIVE, agent: SCRIBE, machine: 'op-' + 'b'.repeat(32), machine_name: 'Lab', runtime: 'lys-runner', shown: 'running', last_reported: 'running', first_report_at: 1790000000, last_report_at: 1790000001, what: 'started', stopped: null, reported_by: 'the runner' };
  it('says a refused stop in words, with its name only under Error details', async () => {
    await mount('#/file/' + SCRIBE, { ...SERVICE, '/runtime/live': ok({ sessions: [running], unanswered: [] }), ['POST /runtime/sessions/' + LIVE + '/stop']: refused(503, 'RuntimeUnavailable', 'no reports store') });
    await click($('section[aria-label="Run"] [data-act="stop"]'));
    await click(button('Stop Scribe'));
    const asking = $('[role="alertdialog"][aria-label="Stop Scribe"]');
    expect(openWords(asking)).toContain('Lys could not reach this agent’s sessions just now.');
    expect(openWords(asking)).not.toContain('RuntimeUnavailable');
    expect(asking?.querySelector('details .refusal-name')?.textContent).toContain('RuntimeUnavailable');
  });
  it('says an unread run in words and offers to read it again', async () => {
    await mount('#/file/' + SCRIBE, { ...SERVICE, '/runtime/live': refused(503, 'RuntimeUnavailable', 'no reports store') });
    const run = $('section[aria-label="Run"]');
    expect(openWords(run)).toContain('Lys could not read whether Scribe is running.');
    expect(openWords(run)).not.toContain('RuntimeUnavailable');
    expect(run?.querySelector('button[aria-label="Read it again"]')).not.toBeNull();
  });
});

describe('Teams', () => {
  const team = { id: 'op-' + 'a'.repeat(32), name: 'Delivery', owner: ADA, description: 'Ship work', members: [SCRIBE], state: 'active', created_by: ME.signed_in, created_at: 1790000000, retired_at: null, parent: null, lead: null };
  const routes: Record<string, Route> = { ...SERVICE, '/teams': ok({ teams: [team] }), ['/budgets/team/' + team.id]: ok(budgetsView({ kind: 'team', id: team.id })), ['/teams/' + team.id + '/goals']: ok({ goals: [] }) };
  const open = async (more: Record<string, Route> = {}) => { const mounted = await mount('#/people', { ...routes, ...more }); await click($('[data-kind="teams"]')); return mounted; };

  it('lists teams as words alone, so the table fits beside the chosen team, and changes a team’s place on the team itself', async () => {
    const { posted } = await open({ ['POST /teams/' + team.id + '/nesting']: (body) => ok({ ...team, lead: (body as { lead: string }).lead, recorded: { operation: (body as { operation: string }).operation, act: 'nested', member: null, by: ME.signed_in, at: 1790000001 } }) });
    expect($$('.teams-table thead th').map((cell) => cell.textContent)).toEqual(['Team', 'Part of', 'Lead', 'Members, with teams under it']);
    expect($('.teams-table select')).toBeNull();
    const place = $('form[aria-label="Where Delivery sits"]');
    expect(place?.textContent).toContain('Part of');
    expect(place?.textContent).toContain('Lead');
    // Save and Undo appear only once something is changed.
    expect(button('Save place')).toBeNull();
    await chooseIn(place?.querySelector('select[aria-label="Lead of Delivery"]') ?? null, SCRIBE);
    expect(button('Undo these changes')).not.toBeNull();
    posted.length = 0;
    await click(button('Save place'));
    expect(posted).toEqual([{ path: '/teams/' + team.id + '/nesting', body: { operation: expect.stringMatching(/^op-/), parent: null, lead: SCRIBE } }]);
    expect(text()).toContain('The team’s place was recorded.');
  });

  it('offers Cancel on New team, and says why Create is greyed', async () => {
    await open();
    await click(button('Create a team'));
    const form = $('form[aria-label="Create team"]');
    expect((form?.querySelector('button[type="submit"]') as HTMLButtonElement | null)?.disabled).toBe(true);
    expect(form?.querySelector('.why-not')?.textContent).toBe('Give the team a name.');
    await typeInto(form?.querySelector('input[name="team-name"]') ?? null, 'Reviewers');
    expect(form?.querySelector('.why-not')).toBeNull();
    await click(button('Cancel'));
    expect($('form[aria-label="Create team"]')).toBeNull();
    expect($('section[aria-label="About Delivery"]')).not.toBeNull();
  });

  it('says an unreadable kept team change in words, never the browser’s own error text in the open', async () => {
    sessionStorage.setItem('lys.pending.team.' + ADA + '.' + team.id, '{"path":"/elsewhere"}');
    await open();
    const members = $('section[aria-label="Members"]');
    expect(openWords(members)).toContain('This browser kept an earlier team change it cannot read back');
    expect(openWords(members)).not.toContain('Error:');
    expect(openWords(members)).not.toContain('TeamChangeUnreadable');
  });
});

describe('Approve estate grants', () => {
  it('says plainly when no estate plan is installed, with where to give access instead', async () => {
    await mount('#/access/import');
    expect(text()).toContain('This Lys was installed without an estate plan, so there is nothing to approve here.');
    expect(openWords()).not.toContain('Unanswered');
    expect(button('Approve and apply estate grants')).toBeNull();
  });
  it('says a plan it cannot read in words, with the reason folded under Error details', async () => {
    await mount('#/access/import', { ...SERVICE, '/identity/estate-plan': ok({ loader: 'op-' + 'c'.repeat(32), plan: { version: 2 } }) });
    expect(text()).toContain('The installed estate plan could not be read, so nothing can be approved here.');
    expect(button('Approve and apply estate grants')).toBeNull();
  });
  it('counts what it gives in words that agree with the number, and says a kind as words', async () => {
    const plan = { version: 1, agents: [{ display_name: 'Scribe' }], resources: [{ kind: 'cambium.channel', id: 'lys', relation: 'participant', actions: ['read', 'send'], seats: ['Scribe'], evidence: 'fixture evidence' }] };
    await mount('#/access/import', { ...SERVICE, '/identity/estate-plan': ok({ plan, loader: 'op-' + 'c'.repeat(32) }) });
    expect(text()).toContain('1 thing to give access to, for 1 agent.');
    expect(text()).toContain('cambium channel');
    expect(text()).not.toContain('1 resources');
  });
});

describe('Connected tools', () => {
  it('labels every field of a new connected tool in words a person sees, not only as a placeholder', async () => {
    const SCRIBE_PATH = '/agents/' + SCRIBE + '/provisioning';
    const description = { models: { minimum: 1, maximum: null, further_encoding: { kind: 'delimited', separator: ',' } }, permissions: { modes: ['acceptEdits'], rule_forms: ['tool_specifier'] }, mcp: { transports: ['stdio', 'http'], working_directory: false, handle_variables: true, channel_policies: ['off', 'wake'] }, rendering_contract: 'claude-code/template-v1' };
    const profile = { version: 1, operation: 'op-' + 'a'.repeat(32), instructions: '', note: '', model_access: ['model-one'], tools: [], skills: [], mcp_servers: [], set_by: ADA, set_at: 1790000000, session: null,
      harness: { name: 'Claude Code', description, program: '/opt/seat/bin/claude', package: 'claude-code-seat' }, permissions: { default_mode: 'acceptEdits' } };
    await mount('#/file/' + SCRIBE + '/provisioning', { ...SERVICE, [SCRIBE_PATH]: ok({ agent: SCRIBE, profile, versions: [], enforced: false }),
      '/harnesses': ok({ programs: [{ name: 'Claude Code', line: 'Installed', models: [{ id: 'model-one', label: 'Model one' }], modes: [{ id: 'acceptEdits', meaning: 'Accept edits' }], instructions_modes: ['keep', 'append'], description,
        builds: [{ name: 'Installed', program: '/opt/seat/bin/claude', package: 'claude-code-seat', from: 'profile' }] }] }) });
    const row = $('tr.add-rule[aria-label="Add connected tools"]');
    expect(row?.querySelector('input[name="server-name"]')?.closest('label')?.textContent).toContain('Name');
    expect(row?.querySelector('select[name="server-kind"]')?.closest('label')?.textContent).toContain('How they are reached');
    await chooseIn(row?.querySelector('select[name="server-kind"]') ?? null, 'program');
    expect(row?.querySelector('input[name="server-program"]')?.closest('label')?.textContent).toContain('The command that starts it');
    expect(row?.querySelector('textarea[name="server-args"]')?.closest('label')?.textContent).toContain('What it is started with');
    expect(row?.querySelector('textarea[name="server-args"]')?.getAttribute('placeholder')).toBeNull();
  });
});
