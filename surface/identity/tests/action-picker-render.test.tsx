import { act, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, describe, expect, it } from 'vitest';
import { ActionPicker } from '../src/features/grants/ActionPicker';
import { MODEL } from './fixtures';

const roots: ReturnType<typeof createRoot>[] = [];
afterEach(() => { for (const root of roots.splice(0)) act(() => root.unmount()); });
const model = { ...MODEL, withheld_from_agents: ['grant.revoke'], action_sentences: { 'agent.start': 'Start this agent', 'agent.stop': 'Stop this agent', 'grant.revoke': 'Withdraw this access' } };
const choices = ['agent.start', 'agent.stop', 'grant.revoke'].map((action) => ({ id: action, relation: 'only.' + action, resource: { kind: 'agent', id: 'one' }, actions: [action], reason: '' }));
function Harness({ metadata = model }: { metadata?: typeof MODEL }) {
  const [selected, change] = useState<string[]>([]);
  return <><ActionPicker model={metadata} groups={[{ id: 'one', title: 'One agent', choices }]} selected={selected} change={change} /><output>{JSON.stringify(selected)}</output></>;
}
function mount(metadata?: typeof MODEL) {
  const container = document.createElement('div'); document.body.append(container);
  const root = createRoot(container); roots.push(root);
  act(() => root.render(<Harness metadata={metadata} />));
  return container;
}
describe('Shared action picker', () => {
  it('renders served plain words and never offers a withheld action', () => {
    const container = mount();
    expect(container.textContent).toContain('Start this agent');
    expect(container.querySelector('input[value="grant.revoke"]')).toBeNull();
    const start = container.querySelector<HTMLInputElement>('input[value="agent.start"]');
    act(() => start?.click());
    expect(container.querySelector('output')?.textContent).toBe('["agent.start"]');
  });
  it('everything selects separate permitted acts and never an editor relation', () => {
    const container = mount();
    act(() => container.querySelector<HTMLInputElement>('input[name="all_actions"]')?.click());
    expect(container.querySelector('output')?.textContent).toBe('["agent.start","agent.stop"]');
  });
  it('offers nothing without the served withheld declaration', () => {
    const { withheld_from_agents: _withheld, ...silent } = MODEL;
    const container = mount(silent);
    expect(container.querySelector('input')).toBeNull();
    expect(container.textContent).toContain('until Lys declares');
  });
});
