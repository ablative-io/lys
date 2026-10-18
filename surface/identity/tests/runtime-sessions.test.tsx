/** Runtime reads preserve unknown outcomes and require a real stop report before showing stopped. */
import { describe, expect, it } from 'vitest';
import { $, click, mount, settle, text } from './harness';
import { SCRIBE, SERVICE, ok, refused } from './fixtures';
import type { RuntimeSession } from '../src/features/runtime/RuntimeSessions';
const path = '/agents/' + SCRIBE + '/runtime/sessions';
const session: RuntimeSession = { session: 'reported-session', agent: SCRIBE, machine: 'machine-one', machine_name: 'Dean laptop', runtime: 'test-runtime', shown: 'unconfirmed', last_reported: 'starting', first_report_at: 1790000000, last_report_at: 1790000001, what: 'Launch received', stopped: null, reported_by: SCRIBE };
describe('Runtime sessions', () => {
  it('shows unconfirmed without assuming a launch succeeded', async () => {
    const { posted, requests } = await mount('#/file/' + SCRIBE + '/sessions', { ...SERVICE, [path]: ok({ sessions: [session] }) });
    expect(requests).toContain(path); expect(posted).toEqual([]); expect(text()).toContain('Not yet confirmed by its runner'); expect(text()).toContain('Dean laptop'); expect(text()).not.toContain('Running, as its runner reported');
  });
  it('shows the runtime’s explicit stop evidence', async () => {
    await mount('#/file/' + SCRIBE + '/sessions', { ...SERVICE, [path]: ok({ sessions: [{ ...session, shown: 'stopped', last_reported: 'stopped', stopped: { at: 1790000010, confirmation: 'Process exited with status 0' } }] }) });
    expect(text()).toContain('Stopped, as its runner confirmed'); expect(text()).toContain('Process exited with status 0');
  });
  it('refuses a stopped claim without confirmation', async () => {
    await mount('#/file/' + SCRIBE + '/sessions', { ...SERVICE, [path]: ok({ sessions: [{ ...session, shown: 'stopped' }] }) });
    expect(text()).toContain('valid status and stop confirmation'); expect(text()).not.toContain('Stopped, as its runner confirmed');
  });
  it('does not turn no reports into not running', async () => {
    await mount('#/file/' + SCRIBE + '/sessions', { ...SERVICE, [path]: ok({ sessions: [] }) });
    expect(text()).toContain('does not show whether a process is running');
  });
  it('shows found sessions without registering an identity', async () => {
    const { requests, posted } = await mount('#/people', { ...SERVICE, '/runtime/found': ok({ sessions: [{ ...session, agent: null, shown: 'running' }] }) });
    posted.length = 0;
    // They are listed with the running agents: the People stat links there, and People has no Found tab.
    expect($('a[data-kind="found"]')).toBeNull();
    await click($('.stat a[href="#/canvas?view=sessions"]'));
    await settle();
    expect(location.hash).toBe('#/canvas?view=sessions');
    expect(requests).toContain('/runtime/found'); expect(posted).toEqual([]);
    expect($('section[aria-label="Sessions with no agent"] h2')?.textContent).toBe('Sessions with no agent');
    expect(text()).toContain('No agent attached'); expect(text()).toContain('Running, as its runner reported');
  });
  it('sends the old Found address to the running agents’ Sessions view', async () => {
    const { requests } = await mount('#/people/view/found', { ...SERVICE, '/runtime/found': ok({ sessions: [{ ...session, agent: null, shown: 'running' }] }) });
    await settle();
    expect(location.hash).toBe('#/canvas?view=sessions');
    expect(requests).toContain('/runtime/found');
  });
  it('names each session by its agent and computer, with no session id shown beside a named agent (#124)', async () => {
    await mount('#/file/' + SCRIBE + '/sessions', { ...SERVICE, [path]: ok({ sessions: [session] }) });
    const row = $('tbody tr');
    expect(row?.querySelector('td')?.textContent).toContain('Scribe');
    expect(row?.querySelector('td a')?.textContent).toBe('Scribe');
    expect(row?.querySelector('td details')).toBeNull();
    expect(row?.textContent).not.toContain(session.session);
    expect(row?.querySelector('td > .mono')).toBeNull();
    expect(text()).not.toContain('Refresh runtime reports');
  });
  it('names an unavailable runtime store', async () => {
    await mount('#/file/' + SCRIBE + '/sessions', { ...SERVICE, [path]: refused(503, 'RuntimeUnavailable', 'Store unavailable') });
    expect(text()).toContain('RuntimeUnavailable'); expect(text()).not.toContain('No runner has reported');
  });
});
