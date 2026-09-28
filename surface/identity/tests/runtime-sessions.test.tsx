/** Runtime reads preserve unknown outcomes and require a real stop report before showing stopped. */
import { describe, expect, it } from 'vitest';
import { mount, text } from './harness';
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
  it('names an unavailable runtime store', async () => {
    await mount('#/file/' + SCRIBE + '/sessions', { ...SERVICE, [path]: refused(503, 'RuntimeUnavailable', 'Store unavailable') });
    expect(text()).toContain('RuntimeUnavailable'); expect(text()).not.toContain('No runtime reports were returned');
  });
});
