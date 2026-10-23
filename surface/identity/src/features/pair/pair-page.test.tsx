import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { $, $$, click, mount, settle, text, type, unmountAll } from '../../../tests/harness';
import { SERVICE, ok, refused } from '../../../tests/fixtures';
import { PAIR_CODE } from './PairPage';

// AGENTS-020 R2: the Pair page. A known code shows the agent's proposed name and the roots choice; Approve posts
// the name and roots to /pair/{code}/approve; Decline posts to /pair/{code}/decline; an unknown code says so; the
// words survive the sign-in round trip in the page's own storage, never in the address.

const CODE = 'amber bison cedar delta';
const PATH = '/pair/' + encodeURIComponent(CODE);
const AGENT = 'agent-' + '1'.repeat(32);
const SERIAL = 'op-' + '2'.repeat(32);

const ROUTES = {
  ...SERVICE,
  [PATH + '/waiting']: ok({ code: CODE, name: 'Pancake', asked_at: 1790000000 }),
  ['POST ' + PATH + '/approve']: ok({ code: CODE, agent: AGENT, serial: SERIAL, apps: 1 }),
  ['POST ' + PATH + '/decline']: ok({ code: CODE, decision: 'declined' }),
  ['/pair/' + encodeURIComponent('no such words here') + '/waiting']: refused(404, 'pair_unknown', 'no agent is waiting with these words'),
};

const button = (name: string) => $$('button').find((each) => each.getAttribute('aria-label') === name) ?? null;

async function findAgent(words: string) {
  await type($('input[name="code"]'), words);
  await click(button('Find the agent'));
  await settle();
}

beforeEach(() => sessionStorage.removeItem(PAIR_CODE));
afterEach(() => unmountAll());

describe('Pair page', () => {
  it('shows the proposed name and the roots choice for a known code, and Approve posts the name and roots', async () => {
    const page = await mount('#/pair', ROUTES);
    await findAgent('Amber  Bison cedar delta ');
    expect(page.requests).toContain(PATH + '/waiting');
    const name = $('input[name="display_name"]') as HTMLInputElement | null;
    expect(name?.value).toBe('Pancake');
    expect(text()).toContain('It proposed the name Pancake');
    expect($('table.agent-grants')).not.toBeNull();
    await type(name, 'Pancake the Second');
    await click(button('Approve the pairing'));
    await settle();
    const approval = page.posted.find((entry) => entry.path === PATH + '/approve');
    expect(approval).toBeTruthy();
    const body = approval?.body as Record<string, unknown>;
    expect(body.display_name).toBe('Pancake the Second');
    expect(Array.isArray(body.grants)).toBe(true);
    for (const key of ['operation', 'activate', 'certificate']) expect(body[key]).toMatch(/^op-[0-9a-f]{32}$/);
    expect(new Set([body.operation, body.activate, body.certificate]).size).toBe(3);
    expect(text()).toContain('Paired: Pancake the Second can now act as itself');
    expect(sessionStorage.getItem(PAIR_CODE)).toBeNull();
  });

  it('Decline posts to /pair/{code}/decline and says nothing of the key was kept', async () => {
    const page = await mount('#/pair', ROUTES);
    await findAgent(CODE);
    await click(button('Decline the pairing'));
    await settle();
    expect(page.posted.some((entry) => entry.path === PATH + '/decline')).toBe(true);
    expect(page.posted.some((entry) => entry.path === PATH + '/approve')).toBe(false);
    expect(text()).toContain("Declined. Nothing of the agent's key was kept.");
  });

  it('says when no agent is waiting with these words', async () => {
    await mount('#/pair', ROUTES);
    await findAgent('no such words here');
    expect(text()).toContain('No agent is waiting with these words');
    expect($('input[name="display_name"]')).toBeNull();
  });

  it('keeps the words across the sign-in round trip and looks them up again on return', async () => {
    const stranger = {
      ...ROUTES,
      '/me': refused(401, 'NotSignedIn', 'sign in first'),
      [PATH + '/waiting']: refused(401, 'NotSignedIn', 'sign in first'),
    };
    await mount('#/pair', stranger);
    await findAgent(CODE);
    expect(text()).toContain('Sign in');
    expect(sessionStorage.getItem(PAIR_CODE)).toBe(CODE);
    expect(location.hash).toBe('#/pair');
    unmountAll();
    const page = await mount('#/pair', ROUTES);
    await settle();
    expect(page.requests).toContain(PATH + '/waiting');
    expect(($('input[name="code"]') as HTMLInputElement | null)?.value).toBe(CODE);
    expect(($('input[name="display_name"]') as HTMLInputElement | null)?.value).toBe('Pancake');
  });
});
