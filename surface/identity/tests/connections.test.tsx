/** Installation connections never imply a live probe or expose an unserved inventory. */
import { describe, expect, it } from 'vitest';
import { $, $$, click, mount, text, unreachable } from './harness';
import { SERVICE, ok, refused } from './fixtures';

const connections = {
  connections: [
    { id: 'sign_in', name: 'Sign-in provider', purpose: 'Authenticates people.', state: 'configured', endpoint: 'http://localhost:18080' },
    { id: 'permissions', name: 'Permission engine', purpose: 'Checks grants.', state: 'local', endpoint: null },
    { id: 'secrets', name: 'Secrets broker', purpose: 'Provides secret access.', state: 'unconfigured', endpoint: null },
  ], health_checked: false,
};

describe('Connections', () => {
  it('shows configured, local and absent integrations without claiming health', async () => {
    const { requests, posted } = await mount('#/connections', { ...SERVICE, '/connections': ok(connections), '/sign-in-providers': ok({ providers: [], offered: ['google', 'microsoft', 'github'], redirect_address: 'http://localhost:8490/auth/v1/providers/callback' }) });
    expect($$('section.card:not([aria-label])')).toHaveLength(3);
    expect(text()).toContain('Local to Lys');
    expect(text()).toContain('Not configured');
    expect(text()).toContain('does not confirm that service is reachable');
    expect(text()).toContain('http://localhost:18080');
    expect($$('details[open]')).toHaveLength(0);
    expect(unreachable()).toEqual([]);
    await click($('.head button'));
    expect(requests.filter((path) => path === '/connections')).toHaveLength(2);
    expect(posted).toEqual([]);
  });
  it('names the admission refusal without showing a default integration list', async () => {
    await mount('#/connections', { ...SERVICE, '/connections': refused(403, 'NotAdmitted', 'Only the administrator can read installation connections.') });
    expect(text()).toContain('NotAdmitted');
    expect($$('section.card')).toHaveLength(0);
  });
});
