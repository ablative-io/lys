import { act } from 'react';
import { createRoot } from 'react-dom/client';
import type { Root } from 'react-dom/client';
import { vi } from 'vitest';
import { App } from '../src/App';
import { SERVICE } from './fixtures';
import type { Route } from './fixtures';

const roots: Root[] = [];

/** Unmount every app a test mounted, so no earlier shell still listens for keys. */
export function unmountAll(): void {
  for (const root of roots.splice(0)) act(() => root.unmount());
}

export interface Mounted {
  container: HTMLElement;
  requests: string[];
  posted: { path: string; body: unknown }[];
}

/**
 * Stub the service: each /api path answers from `routes`, a change under
 * its method, "POST /path" or "PUT /path"; anything else 404 with no refusal.
 */
export function serve(routes: Record<string, Route>, posted: { path: string; body: unknown }[] = []): string[] {
  const requests: string[] = [];
  vi.stubGlobal('fetch', async (input: string, init?: RequestInit) => {
    const path = String(input).replace(/^\/api/, '');
    const method = init?.method ?? 'GET';
    const post = method !== 'GET';
    const body: unknown = post ? JSON.parse(String(init?.body)) : undefined;
    const key = post ? method + ' ' + path : path;
    requests.push(key);
    if (post) posted.push({ path: method === 'POST' ? path : key, body });
    const route = routes[key];
    const answer = typeof route === 'function' ? route(body) : route;
    if (!answer) return new Response('', { status: 404 });
    const text = typeof answer.body === 'string' ? answer.body : JSON.stringify(answer.body);
    return new Response(text, { status: answer.status, headers: { 'content-type': 'application/json' } });
  });
  return requests;
}

export async function settle(): Promise<void> {
  for (let i = 0; i < 6; i += 1) {
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 0));
    });
  }
}

export async function mount(hash: string, routes: Record<string, Route> = SERVICE): Promise<Mounted> {
  const posted: { path: string; body: unknown }[] = [];
  const requests = serve(routes, posted);
  location.hash = hash;
  const container = document.createElement('div');
  document.body.appendChild(container);
  const root = createRoot(container);
  roots.push(root);
  await act(async () => {
    root.render(<App />);
  });
  await settle();
  return { container, requests, posted };
}

export async function press(key: string, init: KeyboardEventInit = {}, target?: Element | null): Promise<void> {
  const on = target ?? document.activeElement ?? document.body;
  await act(async () => {
    on.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true, ...init }));
  });
  await settle();
}

/** Choose `value` in a select, as a person would. */
export async function choose(select: Element | null, value: string): Promise<void> {
  if (!(select instanceof HTMLSelectElement)) throw new Error('no select');
  await act(async () => {
    const set = Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, 'value')?.set;
    set?.call(select, value);
    select.dispatchEvent(new Event('change', { bubbles: true }));
  });
  await settle();
}

export async function click(el: Element | null): Promise<void> {
  if (!(el instanceof HTMLElement)) throw new Error('nothing to click');
  await act(async () => {
    el.click();
  });
  await settle();
}

export const $ = (selector: string): HTMLElement | null => document.querySelector<HTMLElement>(selector);
export const $$ = (selector: string): HTMLElement[] => [...document.querySelectorAll<HTMLElement>(selector)];
export const text = (): string => document.body.textContent ?? '';

/**
 * Conformance 9.3: every clickable element on screen is reachable by
 * keyboard. Controls are links or buttons; anything else that acts is
 * focusable with a button role.
 */
export function unreachable(): string[] {
  const acting = '[data-href], [data-help], .concept, [data-pick], [data-share], [data-gnode], [data-pickrel]';
  const controls = '[data-act], [data-kind], [data-dock], [data-labels], [data-dockbtn], [data-dockbtn-close], [data-nav], [data-xm], [data-xoff]';
  const bad: string[] = [];
  for (const el of $$(acting)) {
    if (el.tagName === 'A' || el.tagName === 'BUTTON') continue;
    if (el.tabIndex < 0 || el.getAttribute('role') !== 'button') bad.push(el.outerHTML.slice(0, 80));
  }
  for (const el of $$(controls)) {
    if (!['A', 'BUTTON'].includes(el.tagName)) bad.push(el.outerHTML.slice(0, 80));
    if (el.tagName === 'A' && !el.getAttribute('href')) bad.push(el.outerHTML.slice(0, 80));
  }
  return bad;
}
