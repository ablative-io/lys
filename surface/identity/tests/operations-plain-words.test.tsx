/**
 * Operations and Network as a first-time person meets them: each panel holds only what it is for, a saved layout is
 * not lost to one stray press, a role is named by its name, and the own-agent control says what it does.
 */
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { $, $$, click, mount, settle, text, unmountAll } from './harness';
import { ADA, SCRIBE, SERVICE, ok } from './fixtures';
import type { Machine } from '../src/features/network/contract';
import { mockTerminal } from './terminal-double';
vi.mock('@gespenst/core', () => ({ createTerminal: mockTerminal }));

const session = 'op-' + '7'.repeat(32);
const running = { session, agent: SCRIBE, machine: 'machine-one', machine_name: 'Test runner', runtime: 'sh', shown: 'running', last_reported: 'running', first_report_at: 1790000000, last_report_at: 1790000001, what: 'process 4242', stopped: null, reported_by: 'test runner' };
const empty = { boxes: {}, open: [], groups: [], notes: [], links: [], widgets: [] };
const layout = { name: 'Morning', saved_at: 1790000000, arrangement: empty };

beforeEach(() => {
  vi.stubGlobal('ResizeObserver', class { observe() {} disconnect() {} });
  localStorage.clear(); sessionStorage.clear(); unmountAll();
});

describe('The canvas panels', () => {
  it('finds agents in the Agents panel without laying the seats table over the canvas', async () => {
    await mount('#/canvas?view=canvas&panel=agents', { ...SERVICE, '/runtime/live': ok({ sessions: [running], unanswered: [] }), '/canvas': ok({ arrangement: empty, layouts: [] }) });
    const panel = $('.canvas-pop-body:not([hidden])');
    expect(panel?.querySelector('input[aria-label="Find an agent"]')).not.toBeNull();
    // The seats, their processes and their acts are the Sessions view's, never a slide-out panel's.
    expect(panel?.querySelector('section.seats')).toBeNull();
    expect($('.canvas-pop section[aria-label="Seats"]')).toBeNull();
  });

  it('asks before a saved layout is removed, and keeps it when the person says Keep', async () => {
    const { posted } = await mount('#/canvas?view=canvas&panel=layouts', { ...SERVICE, '/runtime/live': ok({ sessions: [running], unanswered: [] }), '/canvas': ok({ arrangement: empty, layouts: [layout] }), 'PUT /canvas': ok({ arrangement: empty, layouts: [] }) });
    await click($('button[aria-label="Remove Morning"]'));
    expect(posted.filter((each) => each.path === 'PUT /canvas')).toHaveLength(0);
    expect($('button[aria-label="Confirm removing Morning"]')).not.toBeNull();
    await click($('button[aria-label="Keep it"]'));
    expect($('button[aria-label="Confirm removing Morning"]')).toBeNull();
    expect($('tr[data-layout="Morning"]')).not.toBeNull();
    expect(posted.filter((each) => each.path === 'PUT /canvas')).toHaveLength(0);
  });
});

describe('A computer on the Network page', () => {
  const role = 'role-' + 'e'.repeat(32);
  const computer: Machine = { id: 'op-' + 'd'.repeat(32), name: 'Studio Mac mini', kind: 'Computer', runtime: 'lys-runner', slots: 0, may_run: [], may_run_roles: [role, 'role-' + 'f'.repeat(32)], may_reach: [], named_by: ADA, named_at: 1790000000, state: 'in_use', retired_at: null, last_report_at: null };
  const routes = {
    ...SERVICE,
    '/network': ok({ machines: [computer], reports_served: true }),
    ['/network/machines/' + computer.id + '/runner']: ok({ machine: computer.id, runner: { kind: 'lys' }, answers: true }),
    '/roles': ok({ roles: [{ id: role, name: 'Reviewer', latest: 1, policy: 'stays_until_moved', versions: [], holders: [] }] }),
  };

  it('names a role allowance by the role\'s name, and a role it may not read as "a role", never by its identifier', async () => {
    await mount('#/network', routes);
    await settle();
    expect($$('#screen tbody tr[data-href]')[0].textContent).toContain('anyone holding the Reviewer role, anyone holding a role');
    const panel = $('section[aria-label="Studio Mac mini"]');
    expect(panel?.textContent).toContain('Anyone holding the Reviewer role');
    expect(text()).not.toContain(role);
    expect(text()).not.toContain('role-' + 'f'.repeat(32));
  });

  it('lets the person who named it allow their own agent with a labelled choice and an act that says what it does', async () => {
    await mount('#/network', routes);
    await settle();
    const form = $('form[aria-label="Your agents on this computer"]');
    expect(form?.querySelector('label')?.textContent).toContain('Your agent');
    expect(form?.querySelector('button[type="submit"]')?.textContent).toBe('Allow here');
  });
});
