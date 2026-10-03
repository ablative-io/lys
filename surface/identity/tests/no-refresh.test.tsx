/** No screen offers Refresh or Reload: each read is current when it opens, and each write shows its own answer. */
import { describe, expect, it } from 'vitest';
import { $$, mount } from './harness';
import { ME, SERVICE, ok } from './fixtures';

const secret = { name: 'Calendar', class: 'oauth', owner: ME.person.id, sequence: 2, upstream: null, header: null };
const routes = { ...SERVICE,
  '/sessions': ok({ person: ME.person.id, sessions: [] }),
  '/reviews': ok({ scope: 'personal', revision: 1, judged_at: 1790000000, due: [], unanswered: [] }),
  '/requests': ok({ requests: [] }),
  '/connections': ok({ connections: [], health_checked: false }),
  '/sign-in-providers': ok({ providers: [], offered: ['google'], redirect_address: 'http://localhost/callback' }),
  '/secrets': ok({ secrets: [secret] }), '/secrets/grants': ok({ grants: [] }),
  '/secrets/audit': ok({ verified_by: 'key', lines: [] }),
  '/secrets/settings?secret=Calendar': ok({ secret: 'Calendar', scope: null, recipients: 'anyone', last_operation: null }),
};

// The agents' runtime list at the foot of #/sessions is features/runtime, changed elsewhere.
const elsewhere = ['Refresh runtime reports'];

describe('No Refresh or Reload button', () => {
  it.each(['#/reviews', '#/requests', '#/connections', '#/secrets/entries', '#/secrets/grants', '#/secrets/audit', '#/secrets/manage?secret=Calendar'])('on %s', async (hash) => {
    await mount(hash, routes);
    expect($$('button').map((button) => button.textContent ?? '').filter((words) => /refresh|reload/i.test(words) && !elsewhere.includes(words))).toEqual([]);
  });
});
