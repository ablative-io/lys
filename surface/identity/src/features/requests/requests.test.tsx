import { afterEach, describe, expect, it } from 'vitest';
import { $, $$, mount, settle, text, unmountAll } from '../../../tests/harness';
import { ADA, SERVICE, ok } from '../../../tests/fixtures';
import type { AccessRequest } from './contract';

// ACCESS-011 R3: a request whose relation the model no longer defines is shown in its place as one that
// cannot be granted, with Decline and not Approve; every other request is shown as today.

const ADA_SUMMARY = { id: ADA, display_name: 'Ada (test person)', state: 'active' as const };
const BEA_ASKS = 'person-00000000000000000000000000000002';

const request = (id: string, relation: string, standing: AccessRequest['standing'], actions: string[]): AccessRequest => ({
  id, asked_by: BEA_ASKS, asked_by_name: 'Bea (test person)', responsible: ADA_SUMMARY,
  resource: { kind: 'cambium.workspace', id: 'ablative' }, relation, actions, standing,
  ends_at: null, why: 'to post in the workspace', asked_at: 1790000000, state: 'waiting',
  approvers: [ADA_SUMMARY], sources: [], can_issue_root: true, can_decide: true, held_by: null, decision: null,
});

const GRANTABLE = 'op-0123456789abcdef0123456789abcdef';
const OUTLIVED = 'op-fedcba9876543210fedcba9876543210';

const ROUTES = {
  ...SERVICE,
  '/requests': ok({ requests: [
    request(GRANTABLE, 'member', 'grantable', ['post', 'read']),
    request(OUTLIVED, 'owner', 'relation_unknown', []),
  ] }),
};

afterEach(() => unmountAll());

describe('Requests page with a request the model no longer defines', () => {
  it('lists both requests and shows the outlived one as not grantable, with Decline only', async () => {
    await mount('#/requests/' + OUTLIVED, ROUTES);
    await settle();
    const rows = $$('tr[data-href^="#/requests/op-"]').map((row) => row.getAttribute('data-href'));
    expect(rows).toContain('#/requests/' + GRANTABLE);
    expect(rows).toContain('#/requests/' + OUTLIVED);
    expect(text()).toContain('Cannot be granted as the model stands: cambium.workspace no longer defines owner.');
    expect($('[data-outlived]')).not.toBeNull();
    expect(text()).not.toContain('Allows ');
    expect($('[aria-label="Decline request"], button[name="Decline request"]') ?? $$('button').find((button) => button.textContent?.includes('Decline'))).toBeTruthy();
    expect($$('button').some((button) => button.textContent?.includes('Approve'))).toBe(false);
  });

  it('shows a grantable request as today, with its Allows line and Approve', async () => {
    await mount('#/requests/' + GRANTABLE, ROUTES);
    await settle();
    expect(text()).toContain('Allows ');
    expect($('[data-outlived]')).toBeNull();
    expect($$('button').some((button) => button.textContent?.includes('Approve'))).toBe(true);
    expect($$('button').some((button) => button.textContent?.includes('Decline'))).toBe(true);
  });
});
