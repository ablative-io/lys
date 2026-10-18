import { expect, it } from 'vitest';
import { $, $$, click, mount, text } from './harness';
import { ADA, SCRIBE, SERVICE, ok } from './fixtures';

it('shows carried out with its receipt, execution refusal, and an uncarried approval as a fault', async () => {
  const rows = ['executed', 'refused_on_execution', 'approved'].map((state, index) => ({
    id: 'op-' + String(index + 1).repeat(32), agent: { id: SCRIBE, display_name: 'Scribe' }, responsible: { id: ADA, display_name: 'Ada', state: 'active' },
    target: { kind: 'agent', id: SCRIBE, action: 'agent.provisioning.set' }, method: 'POST', path: '/agents/' + SCRIBE + '/provisioning', body: '{}', note: '', created_at: 1, creation_hash: 'ab'.repeat(32), state,
    execution: index === 0 ? { state, status: 200, receipt_index: 7 } : index === 1 ? { state, refusal: 'NotHeld', reason: 'This scope is not held.' } : undefined,
  }));
  await mount('#/access/drafts', { ...SERVICE, '/drafts?state=waiting': ok({ drafts: [] }), '/drafts?state=decided': ok({ drafts: rows }) });
  await click($$('button').find((button) => button.textContent === 'Decided') ?? null);
  expect(text()).toContain('Approved and carried out');
  expect($('a[href="/api/receipts/7"]')?.textContent).toContain('receipt');
  expect(text()).toContain('Approved, but stopped when carried out');
  expect(text()).toContain('NotHeld');
  expect(text()).toContain('This scope is not held.');
  expect($('[role="alert"]')?.textContent).toContain('Approved, not yet carried out');
});
