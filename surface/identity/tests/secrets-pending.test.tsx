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
    await type(container, 0, 'billing');
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
    const change = (secret: string, recipients: string, operation: string) => { asked.push(recipients); return Promise.resolve({ secret, recipients, operation, repeated: false }); };
    const container = await show(<RecipientsChange secret="calendar" change={change} store={store} />);
    await submit(container);
    await submit(container);
    expect(asked).toEqual(['people_only']);
    expect(container.textContent).toContain('calendar: confirmed change to people only, never agents.');
    expect(store.entries.size).toBe(0);
    await act(async () => {
      const select = container.querySelector('select');
      if (!select) throw new Error('no select');
      select.value = 'anyone';
      select.dispatchEvent(new Event('change', { bubbles: true }));
    });
    await submit(container);
    expect(asked).toEqual(['people_only', 'anyone']);
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

async function retryOriginal(container: HTMLElement): Promise<void> {
  const button = [...container.querySelectorAll('button')].find((entry) => entry.getAttribute('aria-label') === 'Retry original change');
  if (!button) throw new Error('No original-change retry offered');
  await act(async () => button.click());
  await settle();
}

describe('Owner operation receipts', () => {
  it('retains the operation before sending and retries the exact request after remount', async () => {
    const store = memoryStore();
    const calls: unknown[] = [];
    const first = await show(<RecipientsChange secret="calendar" store={store} change={(secret, recipients, operation) => {
      calls.push({ secret, recipients, operation });
      expect(store.getItem(pendingKey('recipients', secret))).toContain(operation);
      return unanswered();
    }} />);
    await submit(first);
    for (const root of roots.splice(0)) act(() => root.unmount());
    const next = await show(<RecipientsChange secret="calendar" store={store} change={(secret, recipients, operation) => {
      calls.push({ secret, recipients, operation });
      return Promise.resolve({ secret, recipients, operation, repeated: true });
    }} />);
    await retryOriginal(next);
    expect(calls).toHaveLength(2);
    expect(calls[1]).toEqual(calls[0]);
    expect(store.entries.size).toBe(0);
    expect(next.textContent).toContain('confirmed change');
  });

  it('does not accept matching values carrying a different operation id', async () => {
    const store = memoryStore();
    const container = await show(<RecipientsChange secret="calendar" store={store} change={(secret, recipients) =>
      Promise.resolve({ secret, recipients, operation: 'a-different-operation', repeated: false })} />);
    await submit(container);
    expect(container.textContent).toContain('UnconfirmedAnswer');
    expect(store.entries.size).toBe(1);
    expect(saveOf(container).disabled).toBe(true);
  });

  it('does not erase an uncertain original when a retry receives a 403', async () => {
    const store = memoryStore();
    let count = 0;
    const container = await show(<RecipientsChange secret="calendar" store={store} change={() => {
      count += 1;
      return count === 1 ? unanswered() : Promise.reject(new Refused(403, { refusal: 'NotOwner', reason: 'The current caller cannot check it' }));
    }} />);
    await submit(container);
    const original = store.getItem(pendingKey('recipients', 'calendar'));
    await retryOriginal(container);
    expect(store.getItem(pendingKey('recipients', 'calendar'))).toBe(original);
    expect(saveOf(container).disabled).toBe(true);
  });

  it('keeps legacy changes without operation ids held without inventing a retry', async () => {
    const store = memoryStore();
    store.setItem(pendingKey('recipients', 'calendar'), JSON.stringify({ asked: 'calendar handed to people only' }));
    let count = 0;
    const container = await show(<RecipientsChange secret="calendar" store={store} change={() => { count += 1; return unanswered(); }} />);
    await submit(container);
    expect(count).toBe(0);
    expect([...container.querySelectorAll('button')].some((button) => button.getAttribute('aria-label') === 'Retry original change')).toBe(false);
    expect(store.entries.size).toBe(1);
  });

  it('sends nothing when the request cannot be retained', async () => {
    const store = memoryStore();
    store.setItem = () => { throw new Error('storage full'); };
    let count = 0;
    const container = await show(<RecipientsChange secret="calendar" store={store} change={() => { count += 1; return unanswered(); }} />);
    await submit(container);
    expect(count).toBe(0);
    expect(container.textContent).toContain('PendingNotRecorded');
  });

  it('retries a scope with its original kind and name after remount', async () => {
    const store = memoryStore();
    const calls: unknown[] = [];
    const first = await show(<ScopeChange secret="calendar" store={store} change={(secret, kind, name, operation) => {
      calls.push({ secret, kind, name, operation }); return unanswered();
    }} />);
    await type(first, 0, 'person-owner');
    await submit(first);
    for (const root of roots.splice(0)) act(() => root.unmount());
    const next = await show(<ScopeChange secret="calendar" store={store} change={(secret, kind, name, operation) => {
      calls.push({ secret, kind, name, operation });
      return Promise.resolve({ secret, scope: 'person/' + name, operation, repeated: true });
    }} />);
    await retryOriginal(next);
    expect(calls).toHaveLength(2);
    expect(calls[1]).toEqual(calls[0]);
    expect(store.entries.size).toBe(0);
  });
});
