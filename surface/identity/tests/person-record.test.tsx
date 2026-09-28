/** Person audit history reads the administrator record and each signed receipt without changing it. */
import { describe, expect, it } from 'vitest';
import { $, $$, mount, text } from './harness';
import { ADA, SERVICE, RECEIPTS, ok, refused } from './fixtures';

const record = { id: ADA, display_name: 'Ada (test person)', state: 'active', responsible: null, logins: [], events: [4, 5] };
const routes = {
  ...SERVICE,
  ['/identities/' + ADA]: ok(record),
  '/receipts/4': ok({ ...RECEIPTS[4], receipt: { ...RECEIPTS[4].receipt, identity: ADA } }),
  '/receipts/5': ok({ ...RECEIPTS[5], receipt: { ...RECEIPTS[5].receipt, identity: ADA } }),
};

describe('Person record', () => {
  it('shows the returned event history and links to its receipts', async () => {
    const app = await mount('#/file/' + ADA + '/record', routes);
    expect($$('.timeline .tl')).toHaveLength(2);
    expect($('a[href="/api/receipts/4"]')).not.toBeNull();
    expect(text()).not.toContain('does not carry its events yet');
    expect(app.posted).toHaveLength(0);
  });
  it('preserves the named administrator refusal', async () => {
    await mount('#/file/' + ADA + '/record', { ...routes, ['/identities/' + ADA]: refused(403, 'NotAdmitted', 'directory administrator only') });
    expect(text()).toContain('directory administrator only');
    expect($('.timeline')).toBeNull();
  });
  it('names an unreadable receipt instead of drawing partial history', async () => {
    await mount('#/file/' + ADA + '/record', { ...routes, '/receipts/5': refused(503, 'ReceiptUnreadable', 'receipt 5 unavailable') });
    expect(text()).toContain('receipt 5 unavailable');
    expect($('.timeline')).toBeNull();
  });
});
