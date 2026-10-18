/** One pair of verbs for every decision (CONSENSUS): approve or decline, give or take back. No decide surface says Keep, Revoke or Refuse. */
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { $, mount, unmountAll } from './harness';
import { ADA, ME, SCRIBE, SERVICE, ok } from './fixtures';

beforeEach(() => sessionStorage.clear());
afterEach(() => unmountAll());
const OLD = /\b(keep|kept|revoke|revoked|refuse|refused)\b/i;

/** Every word a person reads on the page, and every label a screen reader says. */
const said = (): string => {
  const page = $('.page');
  if (!page) throw new Error('No page');
  return (page.textContent ?? '') + ' ' + [...page.querySelectorAll('[aria-label], [title]')].map((each) => (each.getAttribute('aria-label') ?? '') + ' ' + (each.getAttribute('title') ?? '')).join(' ');
};
const review = { scope: 'personal', revision: 4, judged_at: 1790000000, decisions_recorded: true, unanswered: [],
  due: [{ agent: { id: SCRIBE, display_name: 'Builder', state: 'active' }, reviewer: ME.person, last_kept: null, grant: { id: 'grant-1', relation: 'editor', resource: { kind: 'project', id: 'Lys' }, actions: ['edit'] } }] };

const ADA_NAMED = { id: ADA, display_name: 'Ada (test person)' };
const draft = (id: string, state: string) => ({
  id, agent: { id: SCRIBE, display_name: 'Scribe' }, responsible: { ...ADA_NAMED, state: 'active' },
  target: { kind: 'project', id: 'identity', action: 'edit' }, method: 'POST', path: '/grants', body: '{}',
  note: 'To fix the typo', created_at: 1790000000, creation_hash: 'ab'.repeat(32), state,
  decided: state === 'waiting' ? undefined : { operation: 'op-' + '9'.repeat(32), by: ADA_NAMED, login: { provider: 'https://issuer.test', subject: 'ada' }, at: 1790000100, application: null, replacement: null, reason: 'Not now' },
});

describe('Decide surfaces say approve, decline and take back', () => {
  it.each([
    ['the grants list', '#/access', SERVICE],
    ['reviews', '#/reviews', { ...SERVICE, '/reviews': ok(review) }],
    ['requests', '#/requests', { ...SERVICE, '/requests': ok({ requests: [] }) }],
    ['drafts waiting', '#/access/drafts', { ...SERVICE, '/drafts?state=waiting': ok({ drafts: [draft('op-' + '7'.repeat(32), 'waiting')] }) }],
  ])('%s', async (_name, hash, routes) => {
    await mount(hash, routes);
    expect(said().match(OLD)).toBeNull();
  });
});
