import { act } from 'react';
import { createRoot } from 'react-dom/client';
import type { ReactNode } from 'react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { Act } from '../src/shell/Act';
import { SYMBOLS } from '../src/shell/symbols';
import type { SymbolName } from '../src/shell/symbols';

const roots: ReturnType<typeof createRoot>[] = [];
afterEach(() => { for (const root of roots.splice(0)) act(() => root.unmount()); document.body.replaceChildren(); });

function draw(node: ReactNode): HTMLElement {
  const container = document.createElement('div'); document.body.append(container);
  const root = createRoot(container); roots.push(root);
  act(() => root.render(node));
  return container;
}
const button = (container: HTMLElement) => container.querySelector('button') as HTMLButtonElement;

describe('The act control', () => {
  it('says the whole act to a screen reader and on hover, whatever is drawn', () => {
    const alone = button(draw(<Act symbol="retire" name="Retire Waffles the Terrible" />));
    expect(alone.getAttribute('aria-label')).toBe('Retire Waffles the Terrible');
    expect(alone.title).toBe('Retire Waffles the Terrible');
    expect(alone.textContent).toBe('');
    expect(alone.querySelector('svg')?.getAttribute('aria-hidden')).toBe('true');

    const worded = button(draw(<Act symbol="retire" name="Retire Waffles the Terrible" word="Retire" />));
    expect(worded.getAttribute('aria-label')).toBe('Retire Waffles the Terrible');
    expect(worded.textContent).toBe('Retire');
    expect(worded.classList.contains('worded')).toBe(true);
  });

  it('draws every symbol of the set, and each symbol is a drawing of its own', () => {
    const names = Object.keys(SYMBOLS) as SymbolName[];
    const drawn = names.map((name) => {
      const svg = button(draw(<Act symbol={name} name={name} />)).querySelector('svg');
      expect(svg?.getAttribute('viewBox'), name).toBe('0 0 24 24');
      expect(svg?.childElementCount, name).toBeGreaterThan(0);
      return svg?.innerHTML;
    });
    expect(new Set(drawn).size).toBe(names.length);
  });

  it('is a plain button unless it is a form\'s own act, so it never submits a form by accident', () => {
    const submitted = vi.fn((event: Event) => event.preventDefault());
    const container = draw(
      <form onSubmit={(event) => submitted(event.nativeEvent)}>
        <Act symbol="close" name="Cancel" />
        <Act symbol="save" name="Save these settings" word="Save" type="submit" tone="primary" />
      </form>,
    );
    const [cancel, save] = Array.from(container.querySelectorAll('button'));
    expect(cancel.type).toBe('button');
    expect(save.type).toBe('submit');
    act(() => cancel.click());
    expect(submitted).not.toHaveBeenCalled();
    act(() => save.click());
    expect(submitted).toHaveBeenCalledTimes(1);
  });

  it('takes its tone, a class of the screen\'s own, and what a button takes', () => {
    const pressed = vi.fn();
    const stop = button(draw(<Act symbol="stop" name="Stop everything now" tone="danger" className="left-of-badge" onClick={pressed} data-act="stop" />));
    expect(stop.className).toBe('act danger left-of-badge');
    expect(stop.dataset.act).toBe('stop');
    expect(stop.dataset.symbol).toBe('stop');
    act(() => stop.click());
    expect(pressed).toHaveBeenCalledTimes(1);

    const held = button(draw(<Act symbol="start" name="Start this agent" tone="primary" disabled onClick={pressed} />));
    expect(held.className).toBe('act primary');
    expect(held.disabled).toBe(true);
    act(() => held.click());
    expect(pressed).toHaveBeenCalledTimes(1);
  });
});
