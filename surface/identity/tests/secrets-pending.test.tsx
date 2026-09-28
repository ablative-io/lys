/** An owner change whose outcome is unknown is held, and never sent again from the tab. */
import { act } from 'react';
import { createRoot } from 'react-dom/client';
import type { Root } from 'react-dom/client';
import { afterEach, describe, expect, it } from 'vitest';
import { Refused } from '../src/api';
import { RecipientsChange, ScopeChange } from '../src/features/secrets/SecretsDetail';
import { pendingKey } from '../src/features/secrets/pendingChange';
import type { PendingStore } from '../src/features/secrets/pendingChange';
import { settle } from './harness';

const roots: Root[] = [];

afterEach(() => {
  for (const root of roots.splice(0)) act(() => root.unmount());
  document.body.innerHTML = '';
});

function memoryStore(): PendingStore & { entries: Map<string, string> } {
  const entries = new Map<string, string>();
  return {
    entries,
    getItem: (key) => entries.get(key) ?? null,
    setItem: (key, value) => { entries.set(key, value); },
    removeItem: (key) => { entries.delete(key); },
  };
}

async function show(node: React.ReactNode): Promise<HTMLElement> {
  const container = document.createElement('div');
  document.body.appendChild(container);
  const root = createRoot(container);
  roots.push(root);
  await act(async () => root.render(node));
  return container;
}

function formOf(container: HTMLElement): HTMLFormElement {
  const form = container.querySelector('form');
  if (!form) throw new Error('no form rendered');
  return form;
}

function saveOf(container: HTMLElement): HTMLButtonElement {
  const button = container.querySelector<HTMLButtonElement>('button[type="submit"]');
  if (!button) throw new Error('no Save button rendered');
  return button;
}

async function submit(container: HTMLElement): Promise<void> {
  await act(async () => { formOf(container).dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })); });
  await settle();
}

async function type(container: HTMLElement, index: number, value: string): Promise<void> {
  const input = container.querySelectorAll('input')[index];
  if (!input) throw new Error(`no input ${index}`);
  const set = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')?.set;
  if (!set) throw new Error('no value setter');
  await act(async () => {
    set.call(input, value);
    input.dispatchEvent(new Event('input', { bubbles: true }));
  });
}

const unanswered = () => Promise.reject(new Refused(0, { refusal: 'Unanswered', reason: 'the connection closed' }));

describe('An owner change of unknown outcome', () => {
  it('is sent once, however often Save is pressed', async () => {
    const store = memoryStore();
    let sent = 0;
    const change = () => { sent += 1; return unanswered(); };
    const container = await show(<RecipientsChange secret="calendar" change={change} store={store} />);
    await submit(container);
    await submit(container);
    await submit(container);
    expect(sent).toBe(1);
    expect(saveOf(container).disabled).toBe(true);
    expect(container.textContent).toContain('It is not known whether this change was made.');
    expect(store.entries.has(pendingKey('recipients', 'calendar'))).toBe(true);
  });

  it('stays held when the screen is left and opened again', async () => {
    const store = memoryStore();
    const first = await show(<RecipientsChange secret="calendar" change={unanswered} store={store} />);
    await submit(first);
    for (const root of roots.splice(0)) act(() => root.unmount());
    let sent = 0;
    const again = await show(<RecipientsChange secret="calendar" change={() => { sent += 1; return unanswered(); }} store={store} />);
    expect(again.textContent).toContain('has no confirmed answer');
    expect(saveOf(again).disabled).toBe(true);
    await submit(again);
    expect(sent).toBe(0);
  });

  it('holds the form when the record of the pending change is damaged', async () => {
    const store = memoryStore();
    store.setItem(pendingKey('recipients', 'calendar'), '{not json');
    let sent = 0;
    const container = await show(<RecipientsChange secret="calendar" change={() => { sent += 1; return unanswered(); }} store={store} />);
    await submit(container);
    expect(sent).toBe(0);
    expect(container.textContent).toContain('could not be read');
  });

  it('is a 200 whose answer cannot be read', async () => {
    const store = memoryStore();
    let sent = 0;
    const change = () => { sent += 1; return Promise.reject(new Refused(200, { refusal: 'UnreadableResponse', reason: 'the result could not be read' })); };
    const container = await show(<RecipientsChange secret="calendar" change={change} store={store} />);
    await submit(container);
    await submit(container);
    expect(sent).toBe(1);
    expect(container.textContent).toContain('UnreadableResponse');
    expect(container.textContent).toContain('It is not known whether this change was made.');
  });

  it('is a 200 whose answer does not say what was asked', async () => {
    const store = memoryStore();
    let sent = 0;
    const change = () => { sent += 1; return Promise.resolve({ secret: 'calendar', scope: 'team/other' }); };
    const container = await show(<ScopeChange secret="calendar" change={change} store={store} />);
    await act(async () => {
      const select = container.querySelector('select');
      if (!select) throw new Error('no select');
      select.value = 'team';
      select.dispatchEvent(new Event('change', { bubbles: true }));
    });
    await type(container, 1, 'billing');
    await submit(container);
    await submit(container);
    expect(sent).toBe(1);
    expect(container.textContent).toContain('UnconfirmedAnswer');
    expect(store.entries.has(pendingKey('scope', 'calendar'))).toBe(true);
  });
});

describe('A settled owner change', () => {
  it('is not sent again until a field is edited', async () => {
    const store = memoryStore();
    const asked: string[] = [];
    const change = (secret: string, recipients: string) => { asked.push(recipients); return Promise.resolve({ secret, recipients }); };
    const container = await show(<RecipientsChange secret="calendar" change={change} store={store} />);
    await submit(container);
    await submit(container);
    expect(asked).toEqual(['people_only']);
    expect(container.textContent).toContain('calendar can now be handed to people only, never agents.');
    expect(store.entries.size).toBe(0);
    await type(container, 0, 'mailbox');
    await submit(container);
    expect(asked).toEqual(['people_only', 'people_only']);
  });

  it('may be asked again after a definite refusal, which clears the hold', async () => {
    const store = memoryStore();
    let sent = 0;
    const change = () => { sent += 1; return Promise.reject(new Refused(403, { refusal: 'NotOwner', reason: 'person-other does not own calendar' })); };
    const container = await show(<RecipientsChange secret="calendar" change={change} store={store} />);
    await submit(container);
    expect(store.entries.size).toBe(0);
    expect(saveOf(container).disabled).toBe(false);
    await submit(container);
    expect(sent).toBe(2);
  });
});
