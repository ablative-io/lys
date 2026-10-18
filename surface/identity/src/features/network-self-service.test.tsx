import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { MemoryRouter } from 'react-router';
import { afterEach, expect, it, vi } from 'vitest';
import { ADA, DIRECTORY, OWN, SCRIBE, SERVICE, ok, refused } from '../../tests/fixtures';
import type { Route } from '../../tests/fixtures';
import { serve } from '../../tests/harness';
import { Network } from './network/Network';
import type { NameMachine } from './network/contract';
import { ClientDownload } from './network/JoinCode';
import { AddAgent } from './people/AddAgent';
import { AgentCertificates } from './file/AgentCertificates';

let root: ReturnType<typeof createRoot> | null = null;
afterEach(() => { if (root) act(() => root?.unmount()); root = null; sessionStorage.clear(); });

async function draw(routes: Record<string, Route> = {}) {
  const posted: { path: string; body: unknown }[] = [];
  const answers = { ...SERVICE, '/people': ok(OWN), '/directory/people': refused(403, 'NotAdmitted', 'not administrator'),
    '/network': ok({ machines: [], reports_served: false }), ...routes };
  Object.assign(routes, answers);
  serve(routes, posted);
  const host = document.createElement('div'); document.body.append(host);
  root = createRoot(host);
  await act(async () => { root?.render(<MemoryRouter><Network /></MemoryRouter>); });
  return { host, posted };
}

it('shows the signed-in person the add row with Another computer chosen and the computer Lys runs on disabled', async () => {
  const { host } = await draw();
  expect(host.querySelector('form[aria-label="Add a computer"]')).not.toBeNull();
  const buttons = [...host.querySelectorAll('button')];
  expect(buttons.find((button) => button.textContent === 'The computer Lys runs on')?.disabled).toBe(true);
  expect(buttons.find((button) => button.textContent === 'Another computer')?.getAttribute('aria-pressed')).toBe('true');
});

it('adds an own computer and offers one installer download for it, asking for no code until a terminal is chosen', async () => {
  const answers: Record<string, Route> = {};
  answers['POST /network/machines'] = (value) => {
    const body = value as NameMachine;
    return ok({ ...body, id: body.operation, named_by: ADA, named_at: 1, retired: null, state: 'in_use',
      may_run: body.may_run.map((id) => ({ id, display_name: id, state: 'active' })) });
  };
  const { host, posted } = await draw(answers);
  const field = host.querySelector<HTMLInputElement>('input[name="name"]');
  const form = host.querySelector('form[aria-label="Add a computer"]');
  if (!field || !form) throw new Error('add row missing');
  field.value = 'My laptop';
  await act(async () => { form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })); });
  expect(posted[0]?.path).toBe('/network/machines');
  expect((posted[0]?.body as NameMachine).may_run).toEqual(OWN.people[0].agents.map((agent) => agent.id));
  expect(posted.some((entry) => entry.path.endsWith('/join-code'))).toBe(false);
  expect(host.querySelector('select[aria-label="What kind of computer is My laptop?"]')).not.toBeNull();
  const download = host.querySelector<HTMLButtonElement>('button[aria-label="Download the installer for My laptop"]');
  expect(download?.disabled).toBe(true);
  expect(host.querySelector('[data-join="code"]')).toBeNull();
  const done = host.querySelector<HTMLButtonElement>('button[aria-label="Done"]');
  if (!done) throw new Error('Done missing');
  await act(async () => { done.click(); });
  expect(host.querySelector('select[aria-label="What kind of computer is My laptop?"]')).toBeNull();
});

it('chooses the computer Lys runs on first for an administrator whose page is open on that computer, and says it is this one', async () => {
  // jsdom opens the page at a localhost address: only the computer Lys runs on answers it, so the browser is on that computer.
  const { host } = await draw({ '/people': ok(DIRECTORY) });
  const buttons = [...host.querySelectorAll('button')];
  expect(buttons.find((button) => button.textContent === 'The computer Lys runs on')?.disabled).toBe(false);
  expect(buttons.find((button) => button.textContent === 'The computer Lys runs on')?.getAttribute('aria-pressed')).toBe('true');
  expect(host.querySelector('.add-computer-hint')?.textContent).toContain('This computer: Lys runs here');
  expect(buttons.some((button) => button.textContent === 'This computer')).toBe(false);
  expect(host.textContent).toContain('No computer yet. Add this one, where Lys runs, to start agents here.');
});

it('chooses Another computer first for an administrator whose page is open from elsewhere, and names the Lys host by its name', async () => {
  vi.stubGlobal('location', { ...window.location, origin: 'https://lys.example.test', protocol: 'https:' });
  const { host } = await draw({ '/people': ok(DIRECTORY), '/network': ok({ machines: [], reports_served: false, host_name: 'Mac Studio' }) });
  vi.unstubAllGlobals();
  const buttons = [...host.querySelectorAll('button')];
  expect(buttons.find((button) => button.textContent === 'Another computer')?.getAttribute('aria-pressed')).toBe('true');
  expect(buttons.find((button) => button.textContent === 'Mac Studio, where Lys runs')?.getAttribute('aria-pressed')).toBe('false');
  expect(host.querySelector('.add-computer-hint')?.textContent).toContain('including the one you are using now');
  expect(host.textContent).toContain('No computer yet. Add a computer below, the one you are using or another, to start agents on it.');
});

it('downloads the chosen Mac client with the install commit and names a missing platform', async () => {
  serve({ '/clients': ok({ platforms: { 'aarch64-apple-darwin': { commit: '0123456789abcdef', files: { lys: 'digest', 'lys-containment-entry': 'digest' } } } }) });
  const host = document.createElement('div'); document.body.append(host);
  root = createRoot(host);
  await act(async () => { root?.render(<ClientDownload />); });
  const choice = host.querySelector<HTMLSelectElement>('select[aria-label="This Mac"]');
  if (!choice) throw new Error('Mac choice missing');
  await act(async () => { choice.value = 'aarch64-apple-darwin'; choice.dispatchEvent(new Event('change', { bubbles: true })); });
  expect(host.querySelector('a[download="lys"]')?.getAttribute('href')).toBe('/api/clients/aarch64-apple-darwin/lys');
  expect(host.textContent).toContain('commit 0123456789abcdef');
  await act(async () => { choice.value = 'x86_64-apple-darwin'; choice.dispatchEvent(new Event('change', { bubbles: true })); });
  expect(host.querySelector('a[download="lys"]')).toBeNull();
  expect(host.textContent).toContain('Lys has no download for Intel Macs yet. Ask the administrator to add one.');
});

it('shows a non-administrator the own-agent add row and the own-agent certificate issue act', async () => {
  serve({ ...SERVICE, '/people': ok(OWN), '/directory/people': refused(403, 'NotAdmitted', 'not administrator'),
    ['/agents/' + SCRIBE + '/certificates']: ok({ agent: SCRIBE, certificates: [], claims_are_live: false }) });
  const host = document.createElement('div'); document.body.append(host);
  root = createRoot(host);
  await act(async () => { root?.render(<MemoryRouter><AddAgent /><AgentCertificates id={SCRIBE} /></MemoryRouter>); });
  expect(host.querySelector('form[aria-label="Add an agent"]')).not.toBeNull();
  const targets = [...host.querySelectorAll<HTMLSelectElement>('select[name="answers_to"] option')];
  expect(targets.map((option) => option.value)).toEqual([ADA]);
  expect(host.querySelector('form[aria-label="Issue capability certificate"]')).not.toBeNull();
});
