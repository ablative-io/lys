import { describe, expect, it } from 'vitest';
import { act } from 'react';
import { refreshLive } from '../src/live';
import { mount, settle } from './harness';
import { SERVICE, ok, refused } from './fixtures';

describe('Administrator security events', () => {
  it('shows issuance and completed recovery events without asking the person to find a log', async () => {
    const { container, posted } = await mount('#/', { ...SERVICE, '/security-events': ok({ administrator: true, events: [
      { id: 'completed', kind: 'password_reset_completed', at: 20, detail: 'The administrator password was reset. Other sign-in factors were kept.' },
      { id: 'issued', kind: 'password_code_issued', at: 10, detail: 'An administrator password recovery code was requested on this Mac after system authentication.' },
    ] }) });
    const events = container.querySelector('section[aria-label="Security events"]');
    expect(events?.textContent).toContain('The administrator password was reset.');
    expect(events?.textContent).toContain('recovery code was requested on this Mac after system authentication.');
    expect(events?.querySelectorAll('li')).toHaveLength(2);
    expect(posted).toEqual([]);
  });

  it('does not show administrator events to another person', async () => {
    const { container } = await mount('#/', SERVICE);
    expect(container.querySelector('section[aria-label="Security events"]')).toBeNull();
  });

  it('shows a newly recorded recovery on an already open dashboard when changes are signalled', async () => {
    const events: { id: string; kind: string; at: number; detail: string }[] = [];
    const { container } = await mount('#/', { ...SERVICE, '/security-events': () => ok({ administrator: true, events }) });
    expect(container.querySelector('section[aria-label="Security events"]')).toBeNull();
    events.push({ id: 'completed', kind: 'password_reset_completed', at: 20, detail: 'The administrator password was reset.' });
    await act(async () => { refreshLive(); });
    await settle();
    expect(container.querySelector('section[aria-label="Security events"]')?.textContent).toContain('The administrator password was reset.');
  });

  it('names an unreadable event answer instead of silently hiding the security history', async () => {
    const { container } = await mount('#/', { ...SERVICE, '/security-events': refused(503, 'SetupUnavailable', 'RecoveryEventsUnavailable: the record is unreadable') });
    expect(container.textContent).toContain('RecoveryEventsUnavailable');
  });
});
