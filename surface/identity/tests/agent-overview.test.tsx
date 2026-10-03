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
  '/network': ok({ machines: [{ id: 'computer-one', name: 'Ward computer', state: 'in_use', runtime: 'runner', may_run: [{ id: SCRIBE }], may_run_roles: [] }], reports_served: true }),
  [prefix + '/provisioning']: ok({ agent: SCRIBE, enforced: false, versions: [], profile: {
    version: 1, operation: 'op-' + 'b'.repeat(32), model_access: ['model-one'], runs_on: 'computer-one',
    tools: [], skills: [], mcp_servers: [], instructions: '', note: '', set_by: ADA, set_at: 1790000000,
  } }),
  [prefix + '/runtime/sessions']: ok({ sessions: [session] }),
  '/harnesses': ok({ programs: [{ name: 'Claude Code', line: 'claude', models: [{ id: 'model-one', label: 'Model One' }], modes: [], builds: [] }] }),
};

async function open(overrides: Record<string, Route> = {}, hash = '#/file/' + SCRIBE) {
  const posted: { path: string; body: unknown }[] = [];
  const requests = serve({ ...routes, ...overrides }, posted);
  history.replaceState(null, '', '/' + hash);
  const container = document.createElement('div');
  document.body.appendChild(container);
  root = createRoot(container);
  await act(async () => { root?.render(<App />); });
  return { posted, requests };
}

async function follow(link: HTMLElement | null) {
  if (!(link instanceof HTMLAnchorElement)) throw new Error('Navigation link is missing');
  await act(async () => {
    const changed = new Promise<void>((resolve) => window.addEventListener('hashchange', () => resolve(), { once: true }));
    link.click();
    await changed;
  });
}

describe('An agent page explains the agent before its controls', () => {
  it('shows its person, team, computer, saved model and reported running state before three explained next acts', async () => {
    const { posted, requests } = await open();
    const overview = $('[aria-label="About this agent"]');
    expect(overview?.textContent).toContain('Ada (test person)');
    expect(overview?.textContent).toContain('Care team');
    expect(overview?.textContent).toContain('Ward computer');
    expect(overview?.textContent).toContain('Model One');
    expect(overview?.textContent).not.toContain('model-one');
    expect(overview?.textContent).toContain('Running, as its runner last reported');
    const actions = $$('[aria-label="Next steps"] a');
    expect(actions.map((entry) => entry.querySelector('strong')?.textContent)).toEqual(['Start', 'Set limits', 'Give access']);
    expect(actions.map((entry) => entry.getAttribute('href'))).toEqual(['#/team/' + SCRIBE, '#/file/' + SCRIBE + '/budgets', '#/file/' + SCRIBE + '/access']);
    expect(actions.every((entry) => Boolean(entry.querySelector('span')?.textContent))).toBe(true);
    expect(actions[0]?.textContent).toContain('Start Scribe on Ward computer with Model One.');
    expect(actions[0]?.textContent).not.toContain('Choose its computer and model');
    expect($('.file .head')?.textContent).not.toContain('Edit name');
    expect($('.file .head [data-act="suspend"]')).toBeNull();
    expect(requests).not.toContain('/receipts/4');
    const details = $('.agent-details');
    expect(details?.querySelector('summary')?.textContent).toBe('Details');
    expect(details?.textContent).toContain(SCRIBE);
    expect(posted).toEqual([]);
  });

  it('says a saved model no program lists is unlisted instead of showing a bare id as its name', async () => {
    await open({ '/harnesses': ok({ programs: [{ name: 'Claude Code', line: 'claude', models: [{ id: 'model-two', label: 'Model Two' }], modes: [], builds: [] }] }) });
    expect($('[aria-label="About this agent"]')?.textContent).toContain('model-one (no program lists this model now)');
  });

  it('names an unreadable model list instead of showing the saved id', async () => {
    await open({ '/harnesses': refused(503, 'HarnessesUnavailable', 'The program list could not be read') });
    const overview = $('[aria-label="About this agent"]');
    expect(overview?.textContent).toContain('Lys could not read model names just now');
    expect(overview?.textContent).not.toContain('model-one');
    expect($('.agent-details')?.textContent).toContain('HarnessesUnavailable');
    expect($('[aria-label="Next steps"]')?.textContent).toContain('Choose its computer and model, then start this agent.');
  });

  it('asks for a computer and model when none is saved', async () => {
    await open({ [prefix + '/provisioning']: ok({ agent: SCRIBE, enforced: false, versions: [], profile: null }) });
    expect($('[aria-label="About this agent"]')?.textContent).toContain('Choose a model when you start');
    expect($('[aria-label="Next steps"]')?.textContent).toContain('Choose its computer and model, then start this agent.');
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
    expect(overview?.textContent).toContain('Lys could not read runner reports just now');
    expect($('.agent-details')?.textContent).toContain('RunnerUnavailable');
    expect($('.agent-details')?.textContent).toContain('The runner did not answer');
    expect(overview?.textContent).not.toContain('No runner has reported a session');
    expect(overview?.textContent).not.toContain('Stopped');
  });

  it('rejects another agent’s runtime report instead of claiming this agent is running', async () => {
    await open({ [prefix + '/runtime/sessions']: ok({ sessions: [{ ...session, agent: 'agent-' + 'f'.repeat(32) }] }) });
    expect($('.agent-details')?.textContent).toContain('RuntimeReportMismatch');
    expect($('[aria-label="About this agent"]')?.textContent).toContain('Lys could not read runner reports just now');
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

  it('names the missing computer permission without presenting activation as readiness', async () => {
    const { posted } = await open({ '/network': ok({ machines: [{ id: 'computer-one', name: 'Ward computer', state: 'in_use', runtime: 'runner', may_run: [], may_run_roles: [] }], reports_served: true }) });
    expect($('.file .head')?.textContent).toContain('Scribe is added. Choose Start to finish setting it up.');
    expect($('.file .head')?.textContent).not.toContain('Scribe is ready');
    expect($('[aria-label="Next steps"]')?.textContent).toContain('No computer lets Scribe run yet');
    expect($('[aria-label="Next steps"] a')?.getAttribute('href')).toBe('#/file/' + SCRIBE + '/provisioning');
    expect(posted).toEqual([]);
  });

  it.each([
    { name: 'no computers', machines: [] },
    { name: 'only a retired computer', machines: [{ id: 'computer-one', name: 'Ward computer', state: 'retired', runtime: 'runner', may_run: [{ id: SCRIBE }], may_run_roles: [] }] },
  ])('opens the existing add form when Lys has $name', async ({ machines }) => {
    const { posted } = await open({ '/network': ok({ machines, reports_served: true }), '/network/machines/computer-one/runner': ok({ runner: null }) });
    expect($('[aria-label="Next steps"]')?.textContent).toContain('Lys has no computer to run Scribe on yet.');
    const add = $$('[aria-label="Next steps"] a').find((link) => link.textContent === 'Add this computer') ?? null;
    expect(add?.getAttribute('href')).toBe('#/network?add=computer');
    expect($('[data-act="start"]')?.getAttribute('href')).toBe('#/team/' + SCRIBE);
    await follow(add);
    expect($('form[aria-label="Add a computer"]')).not.toBeNull();
    expect(posted).toEqual([]);
  });

  it('keeps an unreadable computer list unknown instead of offering to add a first computer', async () => {
    const { posted } = await open({ '/network': refused(503, 'NetworkUnavailable', 'The computer list could not be read') });
    expect($('.agent-details')?.textContent).toContain('NetworkUnavailable');
    expect($('[aria-label="Next steps"]')?.textContent).not.toContain('Lys has no computer');
    expect($('[aria-label="Next steps"] a[href="#/network?add=computer"]')).toBeNull();
    expect(posted).toEqual([]);
  });

  it('keeps the add-computer deep link administrator-only', async () => {
    const { posted } = await open({ '/network': ok({ machines: [], reports_served: true }), '/directory/people': refused(403, 'NotAdmitted', 'not administrator') }, '#/network?add=computer');
    expect($('.page h1')?.textContent).toBe('Computers');
    expect($('form[aria-label="Add a computer"]')).toBeNull();
    expect(posted).toEqual([]);
  });

  it.each([
    { tab: 'provisioning', control: '[aria-label="Settings of this agent"]' },
    { tab: 'budgets', control: 'section.usage' },
    { tab: 'access', control: '[data-act="grant"]' },
  ])('puts $tab before folded Details without reading receipts', async ({ tab, control }) => {
    const { posted, requests } = await open({}, '#/file/' + SCRIBE + '/' + tab);
    const form = $(control);
    const details = $('.agent-details');
    if (!form || !(details instanceof HTMLDetailsElement)) throw new Error('Agent settings or Details are missing');
    expect(form.closest('details')).toBeNull();
    expect(form.compareDocumentPosition(details) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(details.open).toBe(false);
    expect(requests).not.toContain('/receipts/4');
    expect(requests).not.toContain('/receipts/5');
    const stop = $('.file .head [data-act="stop"]');
    expect(stop).not.toBeNull();
    expect(stop?.closest('details')).toBeNull();
    expect(posted).toEqual([]);
  });
});
