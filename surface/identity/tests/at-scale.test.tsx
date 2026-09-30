/** A business, not one person: 300 people, 1,200 agents, 40 nested teams. Every list is grouped by team, searched, and scoped. */
import { act } from 'react';
import { beforeEach, describe, expect, it } from 'vitest';
import { $, $$, choose, click, mount, text, unmountAll } from './harness';
import { ME, SERVICE, ok } from './fixtures';
import type { PeopleView } from '../src/generated';

const hex = (n: number, width = 32) => n.toString(16).padStart(width, '0');
const personId = (n: number) => n === 0 ? ME.person.id : 'person-' + hex(1000 + n);
const agentId = (n: number) => 'agent-' + hex(5000 + n);
const teamId = (n: number) => 'op-' + hex(900 + n);

/** Five departments, seven teams in each; person n works in team n mod 35. */
const departments = [0, 1, 2, 3, 4].map((d) => ({ id: teamId(d), name: 'Department ' + (d + 1), owner: personId(0), parent: null, lead: personId(d), members: [] as string[], held: [], description: '', state: 'active', created_by: ME.signed_in, created_at: 1, retired_at: null }));
const squads = Array.from({ length: 35 }, (_, s) => ({ ...departments[0], id: teamId(10 + s), name: 'Squad ' + String(s + 1).padStart(2, '0'), parent: teamId(Math.floor(s / 7)), lead: personId(s), members: [] as string[] }));
const people = Array.from({ length: 300 }, (_, n) => ({
  id: personId(n), display_name: 'Person ' + n, state: 'active' as const,
  agents: [0, 1, 2, 3].map((a) => ({ id: agentId(n * 4 + a), display_name: 'Agent ' + (n * 4 + a), state: 'active' as const })),
}));
for (const [n, person] of people.entries()) squads[n % 35].members.push(person.id);
const teams = [...departments, ...squads];
const directory: PeopleView = { scope: 'directory', people };
const routes = { ...SERVICE, '/directory/people': ok(directory), '/people': ok(directory), '/teams': ok({ teams }) };

async function search(value: string): Promise<void> {
  const input = $('input[type="search"]') as HTMLInputElement;
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')?.set?.call(input, value);
    input.dispatchEvent(new Event('input', { bubbles: true }));
  });
}

const rows = () => $$('#screen tbody tr[data-href]');
const groups = () => $$('#screen tr.group .group-name').map((name) => name.textContent);

beforeEach(() => { unmountAll(); document.body.innerHTML = ''; });

describe('Directory at the size of a business', () => {
  it('fills the window and scrolls only its list', async () => {
    await mount('#/people', routes);
    expect($('#screen .page.fill')).not.toBeNull();
    expect($('#screen .fill .pane table')).not.toBeNull();
  });

  it('groups everyone by team in tree order, says what each team holds, and never draws 1,500 rows', async () => {
    await mount('#/people', routes);
    expect(groups().slice(0, 3)).toEqual(['Department 1', 'Squad 01', 'Squad 02']);
    expect(groups()).toHaveLength(40);
    const squad = $$('#screen tr.group').find((row) => row.textContent?.includes('Squad 01'));
    expect(squad?.textContent).toMatch(/led by Person 0 · 9 people, 36 agents/);
    expect(rows().length).toBeLessThanOrEqual(40 * 25);
    expect($('.tools .count')?.textContent).toBe('1,500 people and agents in 40 groups');
  });

  it('folds each person\'s agents under them until opened', async () => {
    await mount('#/people', routes);
    expect(text()).not.toContain('Agent 5');
    const person = $(`tr[data-href="#/file/${personId(1)}"]`);
    const agents = person?.querySelector('button.under');
    expect(agents?.textContent).toBe('4 agents');
    await click(agents ?? null);
    expect(rows().map((row) => row.textContent)).toEqual(expect.arrayContaining([expect.stringContaining('Agent 5')]));
  });

  it('finds one agent among 1,500 by name', async () => {
    await mount('#/people', routes);
    await search('Agent 1199');
    expect(rows().filter((row) => row.textContent?.includes('Agent 1199'))).toHaveLength(1);
    expect($('.tools .count')?.textContent).toBe('1 of 1,500 match');
  });

  it('shows one department and the teams under it, and keeps that choice in the address', async () => {
    await mount('#/people', routes);
    const whose = $('select[aria-label="Whose"]');
    expect($$('select[aria-label="Whose"] option').map((option) => option.textContent?.trim()).slice(0, 4)).toEqual(['Mine', 'Everyone', 'Department 1', 'Squad 01']);
    await choose(whose, 'team:' + teamId(1));
    expect(location.hash).toContain('whose=team%3A' + teamId(1));
    expect(groups()).toEqual(['Department 2', ...Array.from({ length: 7 }, (_, s) => 'Squad ' + String(s + 8).padStart(2, '0'))]);
  });

  it('shows only my own under Mine', async () => {
    await mount('#/people?whose=mine', routes);
    expect(rows().map((row) => row.querySelector('td')?.textContent)).toEqual(['Person 0', 'Agent 0', 'Agent 1', 'Agent 2', 'Agent 3']);
    expect(groups()).toEqual(['Department 1', 'Squad 01']);
  });
});

describe('Teams at the size of a business', () => {
  it('draws the tree with each team\'s lead and size, and no identifiers', async () => {
    await mount('#/people', routes);
    await click($('[data-kind="teams"]'));
    const tree = $$('#screen .team-tree [data-team]');
    expect(tree).toHaveLength(40);
    expect(tree[1].getAttribute('data-depth')).toBe('1');
    expect(tree[1].textContent).toMatch(/Squad 01.*Person 0.*9 people/);
    expect(text()).not.toMatch(/op-0{10}/);
  });
});
