import { describe, expect, it, vi } from 'vitest';
import { $, $$, click, mount, text } from './harness';
import { ADA, SCRIBE, SERVICE, dashboard, ok, refused } from './fixtures';
import { mockTerminal } from './terminal-double';
vi.mock('@gespenst/core', () => ({ createTerminal: mockTerminal }));

const alarm = (id: string) => ({ id, kind: 'seat_stalled', session: 'session-1', agent: SCRIBE,
  machine: 'machine-1', raised_at: 100, detail: 'No signal from Scribe', acknowledged_by: null,
  acknowledged_at: null, cleared_at: null });
const routes = { ...SERVICE, '/dashboard': ok({ ...dashboard(), alarms: {
  open_count: 4, newest: [alarm('alarm-4'), alarm('alarm-3'), alarm('alarm-2')],
} }) };

describe('Dashboard alarms', () => {
  it('lists the newest alarms, their detail, and the open count', async () => {
    await mount('#/', routes);
    expect($('section[aria-label="Alarms"] .dash-count')?.textContent).toBe('4');
    expect($$('tr[data-alarm]').map((row) => row.dataset.alarm)).toEqual(['alarm-4', 'alarm-3', 'alarm-2']);
    expect(text()).toContain('No signal from Scribe');
    expect($$('tr[data-alarm] button').map((button) => button.textContent)).toEqual(['Acknowledge', 'Acknowledge', 'Acknowledge']);
  });

  it('acknowledges one alarm and records the returned person in its row', async () => {
    const { posted } = await mount('#/', { ...routes,
      'POST /alarms/alarm-4/acknowledge': ok({ ...alarm('alarm-4'), acknowledged_by: ADA, acknowledged_at: 110 }),
    });
    await click($('tr[data-alarm="alarm-4"] button'));
    expect(posted.filter((call) => call.path === '/alarms/alarm-4/acknowledge')).toHaveLength(1);
    expect($('tr[data-alarm="alarm-4"]')?.textContent).toContain('Acknowledged');
    expect($('tr[data-alarm="alarm-4"] button')).toBeNull();
  });

  it('shows an alarm read refusal by name', async () => {
    await mount('#/', { ...SERVICE, '/dashboard': ok({ ...dashboard(), alarms: {
      refusal: 'AlarmsUnavailable', reason: 'the alarm log cannot be read',
    } }) });
    expect($('section[aria-label="Alarms"]')?.textContent).toContain('AlarmsUnavailable');
    expect($('section[aria-label="Alarms"]')?.textContent).not.toContain('No open alarms');
  });

  it('keeps an unacknowledged alarm available when acknowledgement is refused', async () => {
    await mount('#/', { ...routes, 'POST /alarms/alarm-4/acknowledge': refused(503, 'AlarmsUnavailable', 'write failed') });
    await click($('tr[data-alarm="alarm-4"] button'));
    expect($('tr[data-alarm="alarm-4"] button')?.textContent).toBe('Acknowledge');
    expect($('section[aria-label="Alarms"] [role="alert"]')?.textContent).toContain('AlarmsUnavailable: write failed');
  });

  it('shows the retained list including cleared and acknowledged alarms', async () => {
    await mount('#/', { ...routes, '/alarms': ok({ revision: 8, alarms: [
      { ...alarm('alarm-4'), acknowledged_by: ADA, acknowledged_at: 110 },
      { ...alarm('alarm-old'), cleared_at: 120 },
    ] }) });
    await click($('section[aria-label="Alarms"] .section-h button'));
    expect($$('tr[data-alarm]').map((row) => row.dataset.alarm)).toEqual(['alarm-4', 'alarm-old']);
    expect($('tr[data-alarm="alarm-4"]')?.textContent).toContain('Acknowledged');
    expect($('tr[data-alarm="alarm-old"]')?.textContent).toContain('Cleared');
    expect($$('tr[data-alarm] button')).toHaveLength(0);
  });

  it('shows recovery after this tab acknowledged the alarm', async () => {
    await mount('#/', { ...routes,
      'POST /alarms/alarm-4/acknowledge': ok({ ...alarm('alarm-4'), acknowledged_by: ADA, acknowledged_at: 110 }),
      '/alarms': ok({ revision: 9, alarms: [{ ...alarm('alarm-4'), acknowledged_by: ADA, acknowledged_at: 110, cleared_at: 120 }] }),
    });
    await click($('tr[data-alarm="alarm-4"] button'));
    await click($('section[aria-label="Alarms"] .section-h button'));
    expect($('tr[data-alarm="alarm-4"]')?.textContent).toContain('Cleared');
  });

  it('names malformed alarm records instead of drawing an incomplete row', async () => {
    await mount('#/', { ...SERVICE, '/dashboard': ok({ ...dashboard(), alarms: {
      open_count: 1, newest: [{ id: 'alarm-incomplete' }],
    } }) });
    expect($('section[aria-label="Alarms"] [role="alert"]')?.textContent).toContain('AlarmsUnreadable');
    expect($$('tr[data-alarm]')).toHaveLength(0);
  });
});
