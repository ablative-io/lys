import { describe, expect, it } from 'vitest';
import { $, $$, click, mount, press, unmountAll, unreachable, settle } from './harness';
import { ADA, GRANTS, LEDGER_G, ROOT_G, SCRIBE, SCRIBE_G, SERVICE, ok, refused } from './fixtures';
import type { Route } from './fixtures';
import type { Grant, RevokeBody } from '../src/generated/grants';

/** The reason the service gives for every grant on a chain through a revoked grant. */
const revokedReason = (grant: string) => `Revoked: ${grant} was revoked, and nothing derived from it is effective`;

/** The standing the service answers for a grant on a chain through the revoked `grant`. */
const voidThrough = (grant: string): Grant['standing'] => ({ stands: false, refusal: 'Revoked', grant, reason: revokedReason(grant) });

/**
 * The service after a revoke answers the grant as revoked, and it and the
 * grant derived from it as void, naming the revoked grant.
 */
function revocable(): Record<string, Route> {
  let revoked = false;
  const after = (g: Grant): Grant => {
    if (g.id === ROOT_G) return { ...g, revoked: true, revoked_at: Math.floor(Date.now() / 1000), revoked_revision: 8, standing: voidThrough(ROOT_G) };
    if (g.id === SCRIBE_G) return { ...g, standing: voidThrough(ROOT_G) };
    return g;
  };
  return {
    ...SERVICE,
    '/grants': () => ok({ grants: revoked ? GRANTS.map(after) : GRANTS, revision: revoked ? 8 : 7 }),
    [`POST /grants/${ROOT_G}/revoke`]: () => {
      revoked = true;
      return ok({ operation: 'op-x', grant: ROOT_G, index: 3, receipt: { revision: 8 } });
    },
  };
}

const open = async (routes: Record<string, Route>) => {
  const m = await mount(`#/file/${ADA}/access`, routes);
  const link = $(`[data-act="revoke"][data-g="${ROOT_G}"]`);
  link?.focus();
  await click(link);
  return m;
};

describe('Revoke', () => {
  it('asks for the reason and shows everything derived from it before it goes', async () => {
    await open(revocable());
    expect($('#drawer')?.classList.contains('open')).toBe(true);
    expect($('#drawer h2')?.textContent).toBe('Revoke owner of project:identity');
    expect(document.activeElement?.id).toBe('why');
    expect($('#derived')?.textContent).toBe("Scribe · viewerrevoked with it");
    expect(unreachable()).toEqual([]);
  });

  it('withdraws it through the service, and on the Access tab what derives from it stays, void', async () => {
    const { posted } = await open(revocable());
    await click($('[data-act="revokedo"]'));
    const body = posted.find((p) => p.path === `/grants/${ROOT_G}/revoke`)?.body as RevokeBody;
    expect(body.reason).toBe('No longer needed.');
    expect(body.route).toBe('browser');
    expect(body.operation).toMatch(/^op-[0-9a-f]{32}$/);
    expect($('#toast')?.textContent).toBe(`Revoked G/${ROOT_G.slice(6, 14)}. Everything derived from it goes with it.`);
    expect($('#drawer')?.classList.contains('open')).toBe(false);
    const card = $$('.file .card').find((c) => c.textContent?.includes('owner of project:identity'));
    expect(card?.textContent).toContain('void');
    expect(card?.textContent).toMatch(/Policy changed at \d\d:\d\d, change 8\. Every check from here on refuses\./);
    expect(card?.querySelector('[data-act="revoke"]')).toBeNull();
    location.hash = `#/file/${SCRIBE}/access`;
    await press('Escape', {}, document.body);
    const derived = $$('.file .card').find((c) => c.textContent?.includes('viewer of project:identity'));
    expect(derived?.textContent).toContain('void');
    expect(derived?.textContent).toContain('which no longer stands');
    expect(derived?.textContent).toContain(revokedReason(ROOT_G));
    location.hash = '#/me';
    await press('Escape', {}, document.body);
    const scribe = $(`tr[data-href="#/file/${SCRIBE}"]`);
    expect(scribe?.textContent).not.toContain('viewer of project:identity');
    expect(scribe?.textContent).toContain('no access');
  });

  it('shows the refusal by the service name, and nothing is withdrawn', async () => {
    await open({ ...SERVICE, [`POST /grants/${ROOT_G}/revoke`]: refused(409, 'RevokeRefused', `RevokeRefused: ${ADA} neither issued ${ROOT_G} nor holds a grant it derives from`) });
    await click($('[data-act="revokedo"]'));
    expect($('#rAnswer b')?.textContent).toBe('RevokeRefused');
    expect($('#drawer')?.classList.contains('open')).toBe(true);
    expect($('#toast')?.textContent).not.toContain('Revoked');
  });

  it('keeps an unconfirmed revoke pending under the same operation', async () => {
    let calls = 0;
    const { posted } = await open({
      ...SERVICE,
      [`POST /grants/${ROOT_G}/revoke`]: () => (++calls === 1 ? refused(503, 'AppendUncertain', 'AppendUncertain: not known') : ok({ operation: 'x', grant: ROOT_G, index: 3, receipt: { revision: 8 } })),
    });
    await click($('[data-act="revokedo"]'));
    expect($('#rAnswer b')?.textContent).toBe('pending');
    await click($('[data-act="revokedo"]'));
    const ops = posted.filter((p) => p.path.endsWith('/revoke')).map((p) => (p.body as RevokeBody).operation);
    expect(ops).toHaveLength(2);
    expect(ops[0]).toBe(ops[1]);
  });
});

const HANDBOOK_G = 'grant-' + '51'.padStart(32, '0');
const LENT_G = 'grant-' + '52'.padStart(32, '0');
const RETURNED_G = 'grant-' + '53'.padStart(32, '0');

/**
 * A chain on project:handbook every holder and delegator of which is active:
 * Ada's root, lent to Scribe, and returned by Scribe to Ada. Each hop lies
 * within its source's pass-on, so admission admits all three.
 */
function handbook(): Record<string, Route> {
  const base = GRANTS.find((g) => g.id === LEDGER_G) as Grant;
  const on = { kind: 'project', id: 'handbook' };
  const chain: Grant[] = [
    { ...base, id: HANDBOOK_G, issuer: ADA, holder: ADA, responsible: ADA, resource: on, relation: 'editor', actions: ['edit', 'view'], pass_on: { kind: 'to', actions: ['view'], recipients: ['person', 'agent'] }, source: null },
    { ...base, id: LENT_G, issuer: ADA, holder: SCRIBE, responsible: ADA, resource: on, relation: 'viewer', actions: ['view'], pass_on: { kind: 'to', actions: ['view'], recipients: ['person'] }, source: HANDBOOK_G },
    { ...base, id: RETURNED_G, issuer: SCRIBE, holder: ADA, responsible: ADA, resource: on, relation: 'viewer', actions: ['view'], pass_on: { kind: 'use_only' }, source: LENT_G },
  ];
  let revoked = false;
  const after = (g: Grant): Grant => ({ ...g, ...(g.id === HANDBOOK_G ? { revoked: true, revoked_at: Math.floor(Date.now() / 1000), revoked_revision: 8 } : {}), standing: voidThrough(HANDBOOK_G) });
  return {
    ...SERVICE,
    '/grants': () => ok({ grants: [...GRANTS, ...(revoked ? chain.map(after) : chain)], revision: revoked ? 8 : 7 }),
    [`POST /grants/${HANDBOOK_G}/revoke`]: () => {
      revoked = true;
      return ok({ operation: 'op-x', grant: HANDBOOK_G, index: 3, receipt: { revision: 8 } });
    },
  };
}

/** What you hold on You, each row by its relation and resource only. */
const held = () => [...$$('.grid2 > div:first-child table')[0].querySelectorAll('tbody tr')].map((tr) => {
  const cells = tr.querySelectorAll('td');
  return `${cells[0].textContent} of ${cells[1].textContent}`;
});

/** What Scribe holds, as its row under Your agents on You carries it: the list in the cell's title, the count in its text. */
const scribeHolds = () => ($(`tr[data-href="#/file/${SCRIBE}"]`)?.querySelectorAll('td')[2].getAttribute('title') ?? '').split('; ');

describe('What you hold after a revoke', () => {
  it('row_2_5_what_you_hold_drops_the_grants_derived_from_a_revoked_one', async () => {
    const routes = handbook();
    await mount('#/me?tab=account', routes);
    expect(held()).toEqual(['owner of project:identity', 'viewer of project:ledger', 'editor of project:handbook', 'viewer of project:handbook']);
    location.hash = '#/me';
    await settle();
    expect(scribeHolds()).toEqual(['viewer of project:identity', 'viewer of project:handbook']);

    unmountAll();
    const { posted } = await mount(`#/file/${ADA}/access`, routes);
    await click($(`[data-act="revoke"][data-g="${HANDBOOK_G}"]`));
    await click($('[data-act="revokedo"]'));
    expect(posted.filter((p) => p.path.endsWith('/revoke')).map((p) => p.path)).toEqual([`/grants/${HANDBOOK_G}/revoke`]);

    unmountAll();
    await mount('#/me?tab=account', routes);
    expect(held()).toEqual(['owner of project:identity', 'viewer of project:ledger']);
    location.hash = '#/me';
    await settle();
    expect(scribeHolds()).toEqual(['viewer of project:identity']);
  });
});

describe('Names as the mock-up writes them', () => {
  it("uses the first word of the name, exactly as x.name.split(' ')[0]", async () => {
    await mount(`#/file/${SCRIBE}/access`);
    expect($('.check h2')?.textContent).toBe("Can Scribe do this?");
    expect($('.section-h span')?.textContent).toBe('Grants');
    expect($$('.section-h span').map((s) => s.textContent)).toContain("What Scribe can reach");
  });
});
