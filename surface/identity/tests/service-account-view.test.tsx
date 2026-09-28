/** Service-account records display the served schema and never invent pass-on authority. */
import { describe, expect, it } from 'vitest';
import { mount, text } from './harness';
import { ADA, ME, SERVICE, ok } from './fixtures';
describe('Own service-account records', () => {
  it('shows the recorded name and status without claiming delegation permission', async () => {
    await mount('#/me', { ...SERVICE, '/me': ok({ ...ME, service_accounts: [{ id: 'service-test', owner: ADA, name: 'Invoice processing', description: 'Test account record', state: 'active', created_by: ME.signed_in, created_at: 1790000000, retired_by: null, retired_at: null }] }) });
    expect(text()).toContain('Invoice processing'); expect(text()).toContain('Test account record'); expect(text()).toContain('Registered'); expect(text()).not.toContain('passable to agents');
  });
  it('keeps retired records visible as retired', async () => {
    await mount('#/me', { ...SERVICE, '/me': ok({ ...ME, service_accounts: [{ id: 'service-retired', owner: ADA, name: 'Old account', description: '', state: 'retired', created_by: ME.signed_in, created_at: 1790000000, retired_by: ME.signed_in, retired_at: 1790000010 }] }) });
    expect(text()).toContain('Old account'); expect(text()).toContain('Retired');
  });
});
