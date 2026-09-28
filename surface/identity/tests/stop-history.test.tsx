/** Saved stop evidence stays distinct from current authority and from runtime confirmation. */
import { describe, expect, it } from 'vitest';
import { $, $$, click, mount, text, unreachable } from './harness';
import { ADA, SCRIBE, SERVICE, ok, refused } from './fixtures';

const path = '/agents/' + SCRIBE + '/stops';
const route = '#/file/' + SCRIBE + '/record';
const stop = {
  agent: SCRIBE, operation: 'op-saved-stop', state: 'suspended', done: true, by: ADA, at: 1790553600,
  reason: 'Unexpected activity', certificates_withdrawn: ['serial-withdrawn'],
  credentials_ended: ['handle-ended'], credentials_refused: null, sessions_asked: ['session-asked'],
};
const empty = 'No emergency-stop requests have been recorded';
describe('Saved emergency-stop history', () => {
  it('reads the agent’s own saved outcomes without writing or claiming sessions ended', async () => {
    const { requests, posted } = await mount(route, { ...SERVICE, [path]: ok({ stops: [stop] }) });
    expect(requests).toContain(path); expect(posted).toEqual([]);
    for (const value of ['Unexpected activity', 'serial-withdrawn', 'handle-ended', 'session-asked', 'op-saved-stop', 'do not describe the agent’s current state', 'does not confirm a session stopped']) expect(text()).toContain(value);
    expect($('section[aria-label="Emergency-stop history"] a[href="#/file/' + SCRIBE + '/sessions"]')?.textContent).toBe('Check runtime reports');
    expect(unreachable()).toEqual([]);
  });
  it('shows the original broker refusal and never calls unconfirmed credentials ended', async () => {
    await mount(route, { ...SERVICE, [path]: ok({ stops: [{ ...stop, credentials_ended: null, credentials_refused: 'BrokerUnavailable: connection refused' }] }) });
    expect(text()).toContain('Not confirmed: BrokerUnavailable: connection refused');
    expect(text()).not.toContain('handle-ended'); expect(text()).not.toContain(empty);
  });
  it('shows an interrupted request without claiming any effects completed', async () => {
    const { posted } = await mount(route, { ...SERVICE, [path]: ok({ stops: [{ ...stop, done: false, state: 'asked', certificates_withdrawn: [], credentials_ended: null, sessions_asked: [] }] }) });
    const history = $('section[aria-label="Emergency-stop history"]')?.textContent ?? '';
    expect(history).toContain('Stop requested — outcome not confirmed');
    expect(history).toContain('Unexpected activity'); expect(history).toContain('op-saved-stop');
    expect(history).not.toContain('Authority after this stop');
    expect(history).not.toContain('Certificates withdrawn'); expect(history).not.toContain('Credentials ended');
    expect(history).not.toContain('Sessions asked to end'); expect(posted).toEqual([]);
  });
  it('distinguishes an empty record from a refusal and lets a subsequent read recover', async () => {
    let answered = false;
    const { requests, posted } = await mount(route, { ...SERVICE, [path]: () => answered ? ok({ stops: [] }) : refused(503, 'StopsUnavailable', 'Stop log could not be read') });
    expect(text()).toContain('StopsUnavailable'); expect(text()).not.toContain(empty);
    answered = true;
    await click($$('button').find((button) => button.textContent === 'Refresh stop history') ?? null);
    expect(text()).toContain(empty); expect(text()).not.toContain('StopsUnavailable');
    expect(requests.filter((value) => value === path)).toHaveLength(2); expect(posted).toEqual([]);
  });
  it.each([
    { stops: [{ ...stop, agent: ADA }] },
    { stops: [{ ...stop, state: 'active' }] },
    { stops: [{ ...stop, done: false }] },
    { stops: [{ ...stop, done: undefined }] },
    { stops: [{ ...stop, credentials_ended: null }] },
    { stops: [{ ...stop, certificates_withdrawn: [false] }] },
    { stops: [stop, stop] },
    {},
  ])('refuses invalid or mismatched evidence without showing its contents: %j', async (answer) => {
    await mount(route, { ...SERVICE, [path]: ok(answer) });
    expect(text()).toContain('StopHistoryUnreadable'); expect(text()).not.toContain('Unexpected activity'); expect(text()).not.toContain(empty);
  });
  it('does not ask for agent stop history on a person’s record', async () => {
    const { requests } = await mount('#/file/' + ADA + '/record');
    expect(requests.some((value) => value.endsWith('/stops'))).toBe(false);
    expect(text()).not.toContain('Emergency-stop history');
  });
});
