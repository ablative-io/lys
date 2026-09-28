/** Runtime reads preserve unknown outcomes and require a real stop report before showing stopped. */
import { act } from 'react';
import { describe, expect, it } from 'vitest';
import { $, click, mount, text } from './harness';
import { SCRIBE, SERVICE, ok, refused } from './fixtures';
import type { RuntimeSession } from '../src/features/runtime/RuntimeSessions';
const path = '/agents/' + SCRIBE + '/runtime/sessions';
const session: RuntimeSession = { session: 'reported-session', agent: SCRIBE, machine: 'machine-one', machine_name: 'Dean laptop', runtime: 'test-runtime', shown: 'unconfirmed', last_reported: 'starting', first_report_at: 1790000000, last_report_at: 1790000001, what: 'Launch received', stopped: null, reported_by: SCRIBE };
describe('Runtime sessions', () => {
  it('shows unconfirmed without assuming a launch succeeded', async () => {
    const { posted, requests } = await mount('#/file/' + SCRIBE + '/sessions', { ...SERVICE, [path]: ok({ sessions: [session] }) });
    expect(requests).toContain(path); expect(posted).toEqual([]); expect(text()).toContain('Unconfirmed'); expect(text()).toContain('Dean laptop'); expect(text()).not.toContain('Reported running');
  });
  it('shows the runtime’s explicit stop evidence', async () => {
    await mount('#/file/' + SCRIBE + '/sessions', { ...SERVICE, [path]: ok({ sessions: [{ ...session, shown: 'stopped', last_reported: 'stopped', stopped: { at: 1790000010, confirmation: 'Process exited with status 0' } }] }) });
    expect(text()).toContain('Stop confirmed'); expect(text()).toContain('Process exited with status 0');
  });
  it('refuses a stopped claim without confirmation', async () => {
    await mount('#/file/' + SCRIBE + '/sessions', { ...SERVICE, [path]: ok({ sessions: [{ ...session, shown: 'stopped' }] }) });
    expect(text()).toContain('valid status and stop confirmation'); expect(text()).not.toContain('Stop confirmed');
  });
  it('does not turn no reports into not running', async () => {
    await mount('#/file/' + SCRIBE + '/sessions', { ...SERVICE, [path]: ok({ sessions: [] }) });
    expect(text()).toContain('does not establish whether a process is running');
  });
  it('shows found sessions without registering an identity', async () => {
    const { requests, posted } = await mount('#/people', { ...SERVICE, '/runtime/found': ok({ sessions: [{ ...session, agent: null, shown: 'running' }] }) });
    posted.length = 0;
    await click($('button[data-kind="found"]'));
    expect(requests).toContain('/runtime/found'); expect(posted).toEqual([]);
    expect(text()).toContain('No identity attached'); expect(text()).toContain('Reported running');
  });
  it('names an unavailable runtime store', async () => {
    await mount('#/file/' + SCRIBE + '/sessions', { ...SERVICE, [path]: refused(503, 'RuntimeUnavailable', 'Store unavailable') });
    expect(text()).toContain('RuntimeUnavailable'); expect(text()).not.toContain('No runtime reports were returned');
  });
  it('asks for a start command on a chosen machine and shows it without claiming it ran', async () => {
    const machine = { id: 'machine-one', name: 'Dean laptop', kind: 'laptop', runtime: 'test-runtime', slots: 1, may_run: [], may_run_roles: [], may_reach: [], named_by: SCRIBE, named_at: 1790000000, state: 'in_use', retired_at: null, last_report_at: null };
    const { posted } = await mount('#/file/' + SCRIBE + '/sessions', { ...SERVICE, [path]: ok({ sessions: [] }), '/network': ok({ machines: [machine], reports_served: true }),
      ['POST /agents/' + SCRIBE + '/start-command']: (body) => ok({ agent: SCRIBE, machine: 'machine-one', runtime: 'test-runtime', session: (body as { operation: string }).operation, provisioning_version: 2, harness: 'claude', handles: [], template: 'run', template_sha256: 'ab', command: 'claude --session x', left_out: [], executed: false }) });
    const select = $('section[aria-label="Start this agent"] select');
    if (!(select instanceof HTMLSelectElement)) throw new Error('Machine choice missing');
    await act(async () => { select.value = 'machine-one'; select.dispatchEvent(new Event('change', { bubbles: true })); });
    await click([...document.querySelectorAll('button')].find((entry) => entry.textContent === 'Get start command') ?? null);
    expect(posted).toEqual([{ path: '/agents/' + SCRIBE + '/start-command', body: { machine: 'machine-one', operation: expect.stringMatching(/^op-[0-9a-f]{32}$/) } }]);
    expect(text()).toContain('claude --session x'); expect(text()).toContain('It does not run the command');
  });
});
