import { describe, expect, it } from 'vitest';
import { $, $$, click, mount, press, unreachable } from './harness';
import { ADA, GRANTS, ROOT_G, SCRIBE, SERVICE, ok, refused } from './fixtures';
import type { Route } from './fixtures';
import type { RevokeBody } from '../src/generated/grants';

/** The service after a revoke answers the grant as revoked; what derives from it is left to the reader. */
function revocable(): Record<string, Route> {
  let revoked = false;
  return {
    ...SERVICE,
    '/grants': () => ok({ grants: GRANTS.map((g) => (revoked && g.id === ROOT_G ? { ...g, revoked: true } : g)), revision: revoked ? 8 : 7 }),
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

describe('Revoke (conformance 2.5)', () => {
  it('asks for the reason and shows everything derived from it before it goes', async () => {
    await open(revocable());
    expect($('#drawer')?.classList.contains('open')).toBe(true);
    expect($('#drawer h2')?.textContent).toBe('Revoke owner of project:identity');
    expect(document.activeElement?.id).toBe('why');
    expect($('#derived')?.textContent).toBe("Scribe · viewerrevoked with it");
    expect(unreachable()).toEqual([]);
  });

  it('withdraws it through the service, and what derives from it goes too', async () => {
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

describe('Names as the mock-up writes them', () => {
  it("uses the first word of the name, exactly as x.name.split(' ')[0]", async () => {
    await mount(`#/file/${SCRIBE}/access`);
    expect($('.check h2')?.textContent).toBe("Can Scribe do this?");
    expect($('.section-h span')?.textContent).toBe('Grants');
    expect($$('.section-h span').map((s) => s.textContent)).toContain("What Scribe can reach");
  });
});
