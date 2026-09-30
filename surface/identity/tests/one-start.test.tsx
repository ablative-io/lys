/** An agent has one start: the start address opens the agent's settings page, whose first card is that start (#117). */
import { describe, expect, it } from 'vitest';
import { $, mount, text } from './harness';
import { SCRIBE, SERVICE, ok } from './fixtures';

describe('one start', () => {
  it('sends the start address to the settings page, not a second start screen', async () => {
    const answer = { agent: SCRIBE, profile: null, versions: [], enforced: false };
    await mount('#/file/' + SCRIBE + '/start', { ...SERVICE, ['/agents/' + SCRIBE + '/provisioning']: ok(answer), '/network': ok({ machines: [], reports_served: false }), '/roles': ok({ roles: [] }) });
    expect(location.hash).toBe('#/file/' + SCRIBE + '/provisioning');
    expect(text()).toContain('MachineUnavailable');
    expect(text()).not.toContain('Start ' + SCRIBE);
    expect($('section[aria-label="Start this agent"]')).not.toBeNull();
  });
});
