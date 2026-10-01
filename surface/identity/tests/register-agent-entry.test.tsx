import { act } from 'react';
import { createRoot } from 'react-dom/client';
import type { Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { App } from '../src/App';
import { ADA, COURIER, ME, SERVICE, ok } from './fixtures';
import { serve } from './harness';

let root: Root | null = null;

beforeEach(() => sessionStorage.clear());
afterEach(() => {
  const mounted = root;
  if (mounted) act(() => mounted.unmount());
  root = null;
});

async function home() {
  serve({ ...SERVICE, '/teams': ok({ teams: [{
    id: 'team-crew', owner: ADA, name: 'Crew', description: '',
    members: [COURIER], state: 'active', created_by: ME.signed_in,
    created_at: 1, retired_at: null,
  }] }) });
  history.replaceState(null, '', '/#/me');
  const container = document.createElement('div');
  document.body.appendChild(container);
  const mounted = createRoot(container);
  root = mounted;
  await act(async () => { mounted.render(<App />); });
  expect(document.querySelector('h1')?.textContent).toBe('Ada (test person)');
}

function link(scope: ParentNode, name: string): HTMLAnchorElement {
  const found = [...scope.querySelectorAll('a')].find((entry) => entry.textContent?.trim() === name);
  if (!found) throw new Error('Missing visible link: ' + name);
  expect(found.hidden).toBe(false);
  expect(found.closest('[hidden], [aria-hidden="true"]')).toBeNull();
  return found;
}

async function follow(anchor: HTMLAnchorElement) {
  let changed: (() => void) | null = null;
  const navigation = new Promise<void>((resolve) => { changed = resolve; });
  const onChange = () => { if (changed) changed(); };
  window.addEventListener('hashchange', onChange, { once: true });
  try {
    await act(async () => { anchor.click(); await navigation; });
  } finally {
    window.removeEventListener('hashchange', onChange);
  }
}

async function directory() {
  await home();
  expect(document.querySelector('nav[aria-label="Main"] a[href="#/agents/new"]')).toBeNull();
  expect(document.querySelector('.you-page a[href^="#/agents/new"]')).toBeNull();
  expect(document.querySelector('[data-act="commission"]')).toBeNull();
  const entry = document.querySelector<HTMLAnchorElement>('nav[aria-label="Main"] a[href="#/people"]');
  if (!entry) throw new Error('Missing People and agents link');
  await follow(entry);
  expect(document.querySelector('h1')?.textContent).toBe('People and agents');
}

describe('Registration entry points', () => {
  it.each([
    { label: 'Add agent', target: '#/agents/new', form: 'Add an agent' },
    { label: 'Add person', target: '#/people/new', form: 'Add a person' },
  ])('reaches $label through People and agents by visible links only', async ({ label, target, form }) => {
    await directory();
    const heading = document.querySelector('.page .head');
    if (!heading) throw new Error('Missing People and agents heading');
    const entry = link(heading, label);
    expect(entry.getAttribute('href')).toBe(target);
    await follow(entry);
    expect(document.querySelector('form')?.getAttribute('aria-label')).toBe(form);
    expect(location.hash).toBe(target);
  });

  it('carries the person from their row to registration', async () => {
    await directory();
    const row = document.querySelector('tr[data-href="#/file/' + ADA + '"]');
    if (!row) throw new Error('Missing person row');
    await follow(link(row, 'Add agent under them'));
    const query = new URLSearchParams(location.hash.split('?')[1]);
    expect(location.hash.split('?')[0]).toBe('#/agents/new');
    expect(query.get('answers_to')).toBe(ADA);
    expect(document.querySelector('form')?.getAttribute('aria-label')).toBe('Add an agent');
  });

  it('carries an agent and its team from its row to registration', async () => {
    await directory();
    const row = document.querySelector('tr[data-href="#/file/' + COURIER + '"]');
    if (!row) throw new Error('Missing agent row');
    await follow(link(row, 'Add agent under them'));
    const query = new URLSearchParams(location.hash.split('?')[1]);
    expect(location.hash.split('?')[0]).toBe('#/agents/new');
    expect(query.get('team')).toBe('team-crew');
    expect(query.get('answers_to')).toBe(COURIER);
    expect(document.querySelector('form')?.getAttribute('aria-label')).toBe('Add an agent');
  });
});
