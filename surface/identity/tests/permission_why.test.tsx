import { describe, expect, it, vi } from 'vitest';
import { $, click, mount, settle, text } from './harness';
import { ADA, ROOT_G, SCRIBE, SERVICE, ok, refused } from './fixtures';
import type { Route } from './fixtures';

/** Grant PA, from P (Ada) to A (the scribe), of read on project X. */
const PA = 'grant-' + 'ab'.repeat(16);
const X = { kind: 'project', id: 'x' };
const LIST = {
  grants: [
    {
      id: PA, issuer: ADA, holder: SCRIBE, responsible: ADA, resource: X, relation: 'reader', actions: ['read'],
      pass_on: { kind: 'use_only' }, source: ROOT_G, window: { starts_at: 0, ends_at: null }, model_version: 1,
      operation: 'op-' + 'cd'.repeat(16), revoked: false, revoked_at: null, revoked_revision: null, last_use: { seen: false },
    },
  ],
  revision: 7,
};
const POLICY = { schema_sha256: 'c0ffee'.repeat(10) + 'c0ff', revision: 'GhUKEzE3OTA1OTU3NTE1MzYzMTgwMDA' };
const YES = {
  permitted: true, identity: SCRIBE, resource: X, action: 'read', grant: PA,
  path: [{ identity: SCRIBE, responsible: false }, { identity: ADA, responsible: true }],
  responsible: ADA, reason: null, policy: POLICY,
};
const REVOKED = {
  permitted: false, identity: SCRIBE, resource: X, action: 'read', grant: PA, path: [], responsible: null,
  reason: 'permission_revoked', policy: POLICY,
};

const open = (explain: Route) => mount('#/file/' + SCRIBE + '/why', { ...SERVICE, '/grants': ok(LIST), 'POST /grants/explain': explain });
const ask = () => click($('[data-act="why"]'));
const explained = (requests: string[]) => requests.filter((r) => r === 'POST /grants/explain').length;
const answered = () => $('#why-answer')?.textContent ?? '';
const verdictShown = () => answered().includes('Permitted') || answered().includes('Refused');

describe('Why an identity can or cannot do a thing (R7)', () => {
  it('shows the path to the responsible person, the resource, the action and the policy revision of a yes', async () => {
    const { requests } = await open(ok(YES));
    await ask();
    expect(explained(requests)).toBe(1);
    expect(text()).toContain('Permitted');
    expect(text()).toContain(SCRIBE);
    expect(text()).toContain('read');
    expect(text()).toContain('project:x');
    expect(text()).toContain(POLICY.revision);
    expect($('[data-responsible="true"]')?.textContent).toContain(ADA);
    expect($('[data-responsible="true"]')?.textContent).toContain('responsible');
    expect($('[data-responsible="false"]')?.textContent).toContain(SCRIBE);
  });

  it('shows a refusal by its name and the grant it concerns, and no yes, whatever the loaded grants say', async () => {
    const { posted } = await open(ok(REVOKED));
    await ask();
    expect(posted.find((p) => p.path === '/grants/explain')?.body).toEqual({ identity: SCRIBE, resource: X, action: 'read' });
    expect(text()).toContain('Refused');
    expect(text()).toContain('permission_revoked');
    expect(text()).toContain(PA);
    expect(answered()).not.toContain('Permitted');
  });

  it('asks once, and shows no verdict before the server answers', async () => {
    await open(ok(YES));
    const served = globalThis.fetch;
    let answer: (response: Response) => void = () => undefined;
    const pending = new Promise<Response>((resolve) => {
      answer = resolve;
    });
    let asked = 0;
    vi.stubGlobal('fetch', (input: string, init?: RequestInit) => {
      if (String(input).endsWith('/grants/explain')) {
        asked += 1;
        return pending;
      }
      return served(input, init);
    });
    await ask();
    expect(asked).toBe(1);
    expect(verdictShown()).toBe(false);
    expect(text()).toContain('Asking the server');
    answer(new Response(JSON.stringify(YES), { status: 200, headers: { 'content-type': 'application/json' } }));
    await settle();
    expect(asked).toBe(1);
    expect(text()).toContain('Permitted');
  });

  it('shows the unreachable refusal and no verdict when the server cannot be reached', async () => {
    await open(ok(YES));
    const served = globalThis.fetch;
    vi.stubGlobal('fetch', (input: string, init?: RequestInit) => {
      if (String(input).endsWith('/grants/explain')) return Promise.reject(new TypeError('connection refused'));
      return served(input, init);
    });
    await ask();
    expect(text()).toContain('ServiceUnreachable');
    expect(verdictShown()).toBe(false);
  });

  it('shows StaleDecision by name and no verdict when the projection has not caught up', async () => {
    await open(refused(503, 'StaleDecision', `StaleDecision: the decision needs revision 9 and the permission relationships stand at 8; the unapplied event changes ${PA}`));
    await ask();
    expect(text()).toContain('StaleDecision');
    expect(verdictShown()).toBe(false);
  });
});

describe('The why view is reached from the identity’s record (R7)', () => {
  it('links the record to the view for that identity', async () => {
    await mount('#/file/' + SCRIBE + '/profile');
    const link = [...document.querySelectorAll<HTMLAnchorElement>('a')].find((a) => a.getAttribute('href') === '#/file/' + SCRIBE + '/why');
    expect(link?.textContent).toBe('Why can it…');
  });
});
