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

describe('Registration entry points', () => {
  it.each([
    { label: '+ Add agent', action: 'agent', form: 'Register an agent' },
    { label: 'Add a person', action: 'person', form: 'Register a person' },
  ])('reaches $label from the main menu by its text link', async ({ label, action, form }) => {
    await home();
    const main = document.querySelector('nav[aria-label="Main"]');
    if (!main) throw new Error('Missing main menu');
    const entry = link(main, label);
    expect(entry.getAttribute('href')).toBe('#/directory/manage?action=' + action);
    await follow(entry);
    expect(document.querySelector('form')?.getAttribute('aria-label')).toBe(form);
    expect(location.hash).toBe('#/directory/manage?action=' + action);
  });

  it('carries the responsible person from the top of the team tree to registration', async () => {
    await home();
    const heading = document.querySelector('.you-panel .section-h');
    if (!heading) throw new Error('Missing agent tree heading');
    const entry = link(heading, '+ Add agent');
    await follow(entry);
    const query = new URLSearchParams(location.hash.split('?')[1]);
    expect(query.get('action')).toBe('agent');
    expect(query.get('answers_to')).toBe(ADA);
    expect(document.querySelector('form')?.getAttribute('aria-label')).toBe('Register an agent');
  });

  it('carries the team and its owner from the team link to registration', async () => {
    await home();
    const team = [...document.querySelectorAll('.you-team')].find((entry) => entry.textContent?.includes('Crew'));
    if (!team) throw new Error('Missing Crew team');
    await follow(link(team, '+ Add agent'));
    const query = new URLSearchParams(location.hash.split('?')[1]);
    expect(query.get('action')).toBe('agent');
    expect(query.get('team')).toBe('team-crew');
    expect(query.get('answers_to')).toBe(ADA);
    expect(document.querySelector('form')?.getAttribute('aria-label')).toBe('Register an agent');
  });
});
