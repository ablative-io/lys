/** "They may" in three words (CONSENSUS): view, edit, own, said from the model's actions; everything else folded under More, one sentence each. */
import { act, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, describe, expect, it } from 'vitest';
import { LevelPicker } from '../src/features/grants/LevelPicker';
import type { GrantModel } from '../src/generated/grants';

const roots: ReturnType<typeof createRoot>[] = [];
afterEach(() => { for (const root of roots.splice(0)) act(() => root.unmount()); document.body.innerHTML = ''; });
const shipped = (relations: Record<string, string[]>): GrantModel => ({ version: 1, relations, kinds: {}, withheld_from_agents: [],
  action_sentences: { view: 'View this resource', edit: 'Edit this resource', grant: 'Give access to this resource', 'agent.start': 'Start this agent', 'agent.stop': 'Stop this agent', 'budget.set': 'Set the budget' } } as GrantModel);
const LYS = { owner: ['view', 'edit', 'grant'], editor: ['view', 'edit'], viewer: ['view'], 'only.agent.start': ['agent.start'], 'only.agent.stop': ['agent.stop'], 'only.budget.set': ['budget.set'] };

function Harness({ model, relations, allowed }: { model: GrantModel; relations: Record<string, string[]>; allowed?: (actions: string[]) => boolean }) {
  const [value, set] = useState('');
  return <form><LevelPicker model={model} relations={relations} value={value} onChange={set} name="relation" allowed={allowed} /><output>{value}</output></form>;
}
function draw(relations: Record<string, string[]>, allowed?: (actions: string[]) => boolean) {
  const container = document.createElement('div'); document.body.append(container);
  const root = createRoot(container); roots.push(root);
  act(() => root.render(<Harness model={shipped(relations)} relations={relations} allowed={allowed} />));
  return container;
}
const rows = (page: HTMLElement) => [...page.querySelectorAll('.level')].map((row) => row.textContent);

describe('They may', () => {
  it('offers three levels with what each lets them do, and folds every other relation under More by area', () => {
    const page = draw(LYS);
    expect(rows(page)).toEqual(['View it See it.', 'Edit it See it and change it.', 'Own it See it, change it and give others access to it.']);
    expect(page.querySelectorAll('input[type="radio"]')).toHaveLength(3);
    expect([...page.querySelectorAll('.level-more optgroup')].map((group) => group.getAttribute('label'))).toEqual(['Agent', 'Budget']);
    expect([...page.querySelectorAll('.level-more option')].map((option) => option.textContent)).toEqual(['Choose one', 'Start this agent', 'Stop this agent', 'Set the budget']);
  });

  it('says a level from the actions the model gives it, never from its name', () => {
    const page = draw({ ...LYS, viewer: ['view', 'edit'] });
    expect(rows(page)[0]).toBe('View it See it and change it.');
  });

  it('counts the other actions a wide level carries, so it never reads narrower than it is', () => {
    const page = draw({ ...LYS, editor: ['view', 'edit', 'agent.start', 'agent.stop', 'budget.set'] });
    expect(rows(page)[1]).toBe('Edit it See it, change it and 3 other actions.');
  });

  it('sends the chosen level, or the one specific thing, as the relation', () => {
    const page = draw(LYS);
    act(() => page.querySelector<HTMLInputElement>('input[value="editor"]')?.click());
    expect(new FormData(page.querySelector('form') as HTMLFormElement).get('relation')).toBe('editor');
    const select = page.querySelector<HTMLSelectElement>('.level-more select');
    act(() => { if (select) { Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, 'value')?.set?.call(select, 'only.agent.stop'); select.dispatchEvent(new Event('change', { bubbles: true })); } });
    expect(new FormData(page.querySelector('form') as HTMLFormElement).get('relation')).toBe('only.agent.stop');
    expect(page.querySelector<HTMLInputElement>('input[value="editor"]')?.checked).toBe(false);
  });

  it('lists an app kind\'s own relations by name when the three levels are not all there', () => {
    const page = draw({ owner: ['read', 'post', 'admin'], member: ['read', 'post'] });
    expect(rows(page)).toEqual(['owner read, post, admin', 'member read, post']);
    expect(page.querySelector('.level-more')).toBeNull();
  });

  it('greys a level that may not be chosen here and says why in its place', () => {
    const page = draw(LYS, (actions) => !actions.includes('grant'));
    expect(page.querySelector<HTMLInputElement>('input[value="owner"]')?.disabled).toBe(true);
    expect(rows(page)[2]).toBe('Own it More than you hold');
  });
});
