/** The canvas starts with a returned running session and never turns membership or missing reads into authority. */
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { $, click, mount, text } from './harness';
import { GRANTS, SCRIBE, SERVICE, ok, refused } from './fixtures';
import { mockTerminal } from './terminal-double';
vi.mock('@gespenst/core', () => ({ createTerminal: mockTerminal }));

const session = 'op-' + '7'.repeat(32);
const running = { session, agent: SCRIBE, machine: 'machine-one', machine_name: 'Test runner', runtime: 'sh', shown: 'running', last_reported: 'running', first_report_at: 1790000000, last_report_at: 1790000001, what: 'process 4242', stopped: null, reported_by: 'test runner' };
const routes = {
  ...SERVICE,
  '/runtime/live': ok({ sessions: [running], unanswered: [] }),
  '/teams': ok({ teams: [{ id: 'team-a', name: 'Delivery', description: '', state: 'active', members: [SCRIBE] }] }),
};

beforeEach(() => {
  vi.stubGlobal('ResizeObserver', class { observe() {} disconnect() {} });
});

describe('Agent canvas', () => {
  it('shows only recorded team and grant connections for returned sessions', async () => {
    const { posted } = await mount('#/runtime/canvas', routes);
    expect(text()).toContain('Scribe');
    expect(text()).toContain('Test runner');
    expect(text()).toContain('Delivery');
    expect(text()).toContain('Member; no permission implied');
    expect(text()).toContain('view on project:identity');
    expect(text()).toContain('Message connections are unavailable');
    expect(document.querySelectorAll('.canvas-connections li')).toHaveLength(2);
    expect(posted).toEqual([]);
  });

  it('keeps the server refusal on a grant instead of drawing it as authority', async () => {
    await mount('#/runtime/canvas', { ...routes, '/grants': ok({ grants: GRANTS.map((grant) => grant.holder === SCRIBE ? { ...grant, standing: { stands: false, refusal: 'GrantRevoked', grant: grant.id, reason: 'Its source was revoked' } } : grant), revision: 8 }) });
    expect(text()).toContain('GrantRevoked: Its source was revoked');
    expect($('.session-canvas-lines path[data-kind="grant"]')?.getAttribute('data-standing')).toBe('false');
  });

  it('keeps sessions and teams visible when reading grants is refused', async () => {
    await mount('#/runtime/canvas', { ...routes, '/grants': refused(403, 'GrantReadRefused', 'These grants are not visible') });
    expect(text()).toContain('GrantReadRefused: These grants are not visible');
    expect(text()).toContain('Delivery');
    expect(text()).toContain('Test runner');
    expect(document.querySelectorAll('.canvas-connections li[data-kind="grant"]')).toHaveLength(0);
  });

  it('does not use a failed live-session read as an empty graph', async () => {
    await mount('#/runtime/canvas', { ...routes, '/runtime/live': refused(503, 'RuntimeUnavailable', 'Reports could not be read') });
    expect(text()).toContain('RuntimeUnavailable');
    expect(text()).not.toContain('No running sessions were returned');
    expect($('.session-canvas')).toBeNull();
  });

  it('does not label an unanswered runner as currently running', async () => {
    await mount('#/runtime/canvas', { ...routes, '/runtime/live': ok({ sessions: [running], unanswered: [{ session, machine: 'machine-one', refusal: 'runner_unreachable', reason: 'Socket closed' }] }) });
    expect(text()).toContain('Runner did not answer; current state unknown');
    expect(text()).toContain('runner_unreachable: Socket closed');
  });

  it('opens the returned session terminal and closing the view never ends its process', async () => {
    const base = '/runtime/sessions/' + session;
    const { posted } = await mount('#/runtime/canvas', {
      ...routes,
      ['POST ' + base + '/resize']: ok({ receipt: { index: 1 } }),
      ['POST ' + base + '/read-bytes']: ok({ session, answer: { kind: 'bytes', output: { session, from: 0, cursor: 0, oldest: 0, data: [], ended: { how: 'exited', at: 1, status: 0, signal: null } } }, receipt: { index: 2 } }),
    });
    await click($('.session-canvas-node button'));
    expect($('.terminal')).not.toBeNull();
    expect(posted.some((entry) => entry.path === base + '/read-bytes')).toBe(true);
    await click($('.session-canvas-node button'));
    expect($('.terminal')).toBeNull();
    expect(posted.some((entry) => entry.path.endsWith('/end'))).toBe(false);
  });
});
