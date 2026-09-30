import { act } from 'react';
import { createRoot } from 'react-dom/client';
import type { Root } from 'react-dom/client';
import { afterEach, describe, expect, it } from 'vitest';
import { App } from '../src/App';
import { $, $$, serve } from './harness';
import { ADA, SCRIBE, SERVICE, ok, refused } from './fixtures';
import type { Route } from './fixtures';

let root: Root | null = null;
afterEach(() => { if (root) act(() => root?.unmount()); root = null; });

const prefix = '/agents/' + SCRIBE;
const session = {
  session: 'op-' + 'a'.repeat(32), agent: SCRIBE, machine: 'computer-one', machine_name: 'Ward computer',
  runtime: 'runner', shown: 'running', last_reported: 'running', first_report_at: 1790000000,
  last_report_at: 1790000200, what: 'Runner confirmed the process', stopped: null, reported_by: ADA,
};
const routes = {
  ...SERVICE,
  '/teams': ok({ teams: [{ id: 'team-one', name: 'Care team', state: 'active', members: [SCRIBE] }] }),
  '/network': ok({ machines: [{ id: 'computer-one', name: 'Ward computer' }], reports_served: true }),
  [prefix + '/provisioning']: ok({ agent: SCRIBE, enforced: false, versions: [], profile: {
    version: 1, operation: 'op-' + 'b'.repeat(32), model_access: ['model-one'], runs_on: 'computer-one',
    tools: [], skills: [], mcp_servers: [], instructions: '', note: '', set_by: ADA, set_at: 1790000000,
  } }),
  [prefix + '/runtime/sessions']: ok({ sessions: [session] }),
};

async function open(overrides: Record<string, Route> = {}) {
  const posted: { path: string; body: unknown }[] = [];
  const requests = serve({ ...routes, ...overrides }, posted);
  location.hash = '#/file/' + SCRIBE;
  const container = document.createElement('div');
  document.body.appendChild(container);
  root = createRoot(container);
  await act(async () => { root?.render(<App />); });
  return { posted, requests };
}

describe('An agent page explains the agent before its controls', () => {
  it('shows its person, team, computer, saved model and reported running state before three explained next acts', async () => {
    const { posted, requests } = await open();
    const overview = $('[aria-label="About this agent"]');
    expect(overview?.textContent).toContain('Ada (test person)');
    expect(overview?.textContent).toContain('Care team');
    expect(overview?.textContent).toContain('Ward computer');
    expect(overview?.textContent).toContain('model-one');
    expect(overview?.textContent).toContain('Running, as its runner last reported');
    const actions = $$('[aria-label="Next steps"] a');
    expect(actions.map((entry) => entry.querySelector('strong')?.textContent)).toEqual(['Start', 'Set limits', 'Give access']);
    expect(actions.map((entry) => entry.getAttribute('href'))).toEqual(['provisioning', 'budgets', 'access'].map((part) => '#/file/' + SCRIBE + '/' + part));
    expect(actions.every((entry) => Boolean(entry.querySelector('span')?.textContent))).toBe(true);
    expect($('.file .head')?.textContent).not.toContain('Edit name');
    expect($('.file .head [data-act="suspend"]')).toBeNull();
    expect(requests).not.toContain('/receipts/4');
    const details = $('.agent-details');
    expect(details?.querySelector('summary')?.textContent).toBe('Details');
    expect(details?.textContent).toContain(SCRIBE);
    expect(posted).toEqual([]);
  });

  it('keeps absence of a runtime report distinct from the active identity and never guesses stopped', async () => {
    await open({ [prefix + '/runtime/sessions']: ok({ sessions: [] }) });
    const overview = $('[aria-label="About this agent"]');
    expect(overview?.textContent).toContain('No runner has reported a session');
    expect(overview?.textContent).toContain('Running state is unknown');
    expect(overview?.textContent).not.toContain('Stopped');
    expect($('.agent-details')?.textContent).toContain('Active');
  });

  it('names a failed runtime read instead of inventing an empty or stopped state', async () => {
    await open({ [prefix + '/runtime/sessions']: refused(503, 'RunnerUnavailable', 'The runner did not answer') });
    const overview = $('[aria-label="About this agent"]');
    expect(overview?.textContent).toContain('RunnerUnavailable');
    expect(overview?.textContent).toContain('The runner did not answer');
    expect(overview?.textContent).not.toContain('No runner has reported a session');
    expect(overview?.textContent).not.toContain('Stopped');
  });

  it('rejects another agent’s runtime report instead of claiming this agent is running', async () => {
    await open({ [prefix + '/runtime/sessions']: ok({ sessions: [{ ...session, agent: 'agent-' + 'f'.repeat(32) }] }) });
    expect($('[aria-label="About this agent"]')?.textContent).toContain('RuntimeReportMismatch');
    expect($('[aria-label="About this agent"]')?.textContent).not.toContain('Running, as its runner last reported');
  });

  it('reads receipts only when Details opens and keeps lifecycle changes there', async () => {
    const { posted, requests } = await open();
    const details = $('.agent-details');
    if (!(details instanceof HTMLDetailsElement)) throw new Error('Agent details are missing');
    expect(details.open).toBe(false);
    expect(requests).not.toContain('/receipts/4');
    await act(async () => { details.open = true; details.dispatchEvent(new Event('toggle')); });
    expect(requests).toContain('/receipts/4');
    expect(requests).toContain('/receipts/5');
    expect(details.textContent).toContain('registered, under its person');
    expect(details.querySelector('[data-act="suspend"]')).not.toBeNull();
    expect(posted).toEqual([]);
  });
});
