import { act } from 'react';
import { createRoot } from 'react-dom/client';
import type { Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { App } from '../src/App';
import { ADA, ME, SERVICE, ok } from './fixtures';
import { serve, type } from './harness';

const computer = 'op-' + 'a'.repeat(32);
const team = 'op-' + 'b'.repeat(32);
const description = {
  models: { minimum: 1, maximum: 1, further_encoding: { kind: 'array' } },
  permissions: { modes: ['default'], rule_forms: [] },
  mcp: { transports: [], working_directory: false, handle_variables: false, channel_policies: [] },
  rendering_contract: 'test-program/v1',
};
const routes = {
  ...SERVICE,
  '/teams': ok({ teams: [{ id: team, owner: ADA, name: 'Care team', description: '', members: [], state: 'active', created_by: ME.signed_in, created_at: 1, retired_at: null }] }),
  '/network': ok({ machines: [{ id: computer, name: 'Care computer', kind: 'laptop', runtime: 'lys-runner', slots: 4, may_run: [], may_reach: [], named_by: ADA, named_at: 1, state: 'in_use', retired_at: null, last_report_at: 1 }], reports_served: true }),
  ['/network/machines/' + computer + '/runner']: ok({ runner: { kind: 'lys' } }),
  '/harnesses': ok({ programs: [{ name: 'Care program', line: 'Agent program', command: 'care', commands: ['care'], description,
    models: [{ id: 'care-model', label: 'Care model' }], modes: [{ id: 'default', meaning: 'Ask before making changes' }], instructions_modes: ['keep', 'append'],
    builds: [{ name: 'Installed care program', program: '/usr/local/bin/care', package: 'care', from: 'installed' }],
  }] }),
};
let root: Root | null = null;
beforeEach(() => sessionStorage.clear());
afterEach(() => { const mounted = root; if (mounted) act(() => mounted.unmount()); root = null; });

async function registration() {
  const posted: { path: string; body: unknown }[] = [];
  serve(routes, posted);
  history.replaceState(null, '', '/#/agents/new?team=' + team + '&answers_to=' + ADA);
  const container = document.createElement('div');
  document.body.appendChild(container);
  const mounted = createRoot(container);
  root = mounted;
  await act(async () => { mounted.render(<App />); });
  const form = document.querySelector<HTMLFormElement>('form[aria-label="Register an agent"]');
  if (!form) throw new Error('Registration form is missing');
  return { form, posted };
}

function choice(form: HTMLFormElement, name: string, label: string): HTMLSelectElement {
  const select = form.querySelector<HTMLSelectElement>('select[name="' + name + '"]');
  if (!select) throw new Error('Missing choice: ' + label);
  expect(select.closest('label')?.textContent).toContain(label);
  return select;
}

describe('Agent registration choices', () => {
  it('offers real people, teams, computers and models with the clicked defaults', async () => {
    const { form, posted } = await registration();
    expect(choice(form, 'answers_to', 'Who this agent answers to').value).toBe(ADA);
    expect(choice(form, 'team', 'Team this agent joins').value).toBe(team);
    expect(choice(form, 'machine', 'Where this agent runs').value).toBe(computer);
    expect(choice(form, 'model', 'Model this agent uses').textContent).toContain('Care model');
    expect(form.textContent).not.toContain(computer);
    expect(form.textContent).not.toContain(ADA);
    expect(form.querySelector('textarea')).toBeNull();
    expect(posted).toEqual([]);
  });

  it('names an existing agent under the chosen person inline and prevents another registration', async () => {
    const { form, posted } = await registration();
    await type(form.querySelector('input[name="display_name"]'), 'Scribe');
    expect(form.textContent).toContain('already has an agent named Scribe');
    const submit = form.querySelector<HTMLButtonElement>('button[type="submit"]');
    expect(submit?.disabled).toBe(true);
    await act(async () => { form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })); });
    expect(posted).toEqual([]);
    expect(document.querySelector('[role="alert"]')?.textContent).toContain('Scribe');
  });
});
