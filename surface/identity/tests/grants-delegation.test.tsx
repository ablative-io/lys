import { beforeEach, describe, expect, it, vi } from 'vitest';
import { $, $$, choose, click, mount, press, unmountAll, unreachable } from './harness';
import { ADA, GRANTS, AGENT_MODEL as MODEL, ORGANISATION, ROOT_G, SCRIBE, SCRIBE_G, SERVICE as BASE, ok, refused } from './fixtures';
import type { DelegateBody } from '../src/generated/grants';

beforeEach(() => sessionStorage.clear());

const SERVICE: typeof BASE = { ...BASE, '/grants/model': ok(MODEL) };

describe('The delegation form', () => {
  const open = async (routes = SERVICE) => {
    const m = await mount('#/me?tab=account', routes);
    await click($(`[data-act="delegate"][data-g="${ROOT_G}"]`));
    return m;
  };

  it('row_2_3_the_form_shows_the_source_grant_its_actions_may_pass_on_and_the_effective_end', async () => {
    await open();
    expect($('.act-panel')).not.toBeNull();
    expect(document.activeElement?.id).toBe('dTo');
    expect($('.act-panel h2')?.textContent).toBe('Give part of owner of project:identity to an agent');
    const from = $$('.act-panel .card')[0].textContent ?? '';
    expect(from).toContain('Source grant');
    expect(from).toContain('owner of project:identity');
    expect(from).toContain('Actions it allowseverything here');
    expect(from).toContain('You may pass it onagent');
    expect(from).toContain('Ends no later than27 Oct');
    const ends = () => $$('.act-panel .card')[0].querySelectorAll('.row')[3];
    expect(ends().textContent).toBe('Ends no later than27 Oct');
    expect(ends().querySelector('.open-q')).toBeNull();
    expect(from).not.toContain('when yours does');
    expect($$('#dLease option').map((o) => o.textContent)).toEqual(['7 days', 'Ends with a role assignment (none with an end date)', 'When your own access ends']);
    expect(unreachable()).toEqual([]);

    unmountAll();
    const endless = GRANTS.map((g) => (g.id === ROOT_G ? { ...g, window: { ...g.window, ends_at: null }, effective_ends_at: null } : g));
    await open({ ...SERVICE, '/grants': ok({ grants: endless, revision: 7 }) });
    expect($$('.act-panel .card')[0].textContent).toContain('Ends no later thanno end');
    expect(ends().textContent).toBe('Ends no later thanno end');
  });

  const tick = async (...actions: string[]) => {
    for (const action of actions) await click($(`.act-panel input[name="action"][value="${action}"]`));
  };

  it('offers an agent one action at a time, only what may be passed on and is not withheld, and never a wider relation', async () => {
    const { requests } = await open();
    expect(requests).toContain('/grants/model');
    expect($$('.act-panel input[name="action"]').map((c) => (c as HTMLInputElement).value)).toEqual(['edit', 'view']);
    expect($$('.act-panel input[name="action"]').filter((c) => (c as HTMLInputElement).checked)).toEqual([]);
    expect($$('[data-pickrel]')).toEqual([]);
    expect(($('[data-act="delegatedo"]') as HTMLButtonElement).disabled).toBe(true);
  });

  it('offers nothing to an agent until the service names what it withholds from agents', async () => {
    const { withheld_from_agents: _withheld, ...silent } = MODEL;
    await open({ ...SERVICE, '/grants/model': ok(silent) });
    expect($$('.act-panel input[name="action"]')).toEqual([]);
    expect($('.act-panel')?.textContent).toContain('has not said which actions an agent may hold');
  });

  it("offers an agent nothing on an app's resource, since the service refuses every agent grant there until the app allows it", async () => {
    const onApp = GRANTS.map((g) => (g.id === ROOT_G ? { ...g, resource: { kind: 'fixture.doc', id: 'd1' } } : g));
    await open({ ...SERVICE, '/grants': ok({ grants: onApp, revision: 7 }) });
    expect($$('.act-panel input[name="action"]')).toEqual([]);
    expect($('.act-panel')?.textContent).toContain("can't give an agent this app's actions until the app allows it");
    expect(($('[data-act="delegatedo"]') as HTMLButtonElement).disabled).toBe(true);
  });

  it('gives through the service, naming source, recipient, relation and an end no later than the source', async () => {
    const { posted } = await open();
    await choose($('#dTo'), SCRIBE);
    await tick('view');
    await click($('[data-act="delegatedo"]'));
    const body = posted.find((p) => p.path === '/grants')?.body as DelegateBody;
    expect(body.source).toBe(ROOT_G);
    expect(body.recipient).toBe(SCRIBE);
    expect(body.responsible).toBe(ADA);
    expect(body.relation).toBe('viewer');
    expect(body.pass_on).toEqual({ kind: 'use_only' });
    expect(body.route).toBe('browser');
    expect(body.operation).toMatch(/^op-[0-9a-f]{32}$/);
    const E = GRANTS.find((g) => g.id === ROOT_G)?.effective_ends_at ?? NaN;
    expect(body.window.ends_at).toBe(Math.min(E, body.window.starts_at + 604800));
    expect($('.act-panel')).toBeNull();
    expect($('#toast')?.textContent).toContain('Given. Scribe can now View this resource project:identity, through you.');

    await click($(`[data-act="delegate"][data-g="${ROOT_G}"]`));
    await choose($('#dTo'), SCRIBE);
    await choose($('#dLease'), 'no end');
    await tick('view');
    await click($('[data-act="delegatedo"]'));
    const given = posted.filter((p) => p.path === '/grants').map((p) => p.body as DelegateBody);
    expect(given).toHaveLength(2);
    expect(given[1].window.ends_at).toBe(E);
  });

  it('gives each ticked action as its own grant, one operation each, and names them all', async () => {
    const { posted } = await open();
    await tick('edit', 'view');
    await click($('[data-act="delegatedo"]'));
    const given = posted.filter((p) => p.path === '/grants').map((p) => p.body as DelegateBody);
    expect(given.map((b) => b.relation)).toEqual(['only.edit', 'viewer']);
    expect(given.map((b) => b.pass_on)).toEqual([{ kind: 'use_only' }, { kind: 'use_only' }]);
    expect(new Set(given.map((b) => b.operation)).size).toBe(2);
    expect($('#toast')?.textContent).toContain('Given. Scribe can now Edit this resource; View this resource project:identity, through you.');
    expect(sessionStorage.getItem('lys.pending.grant.' + ADA + '.' + ROOT_G + '.rest')).toBeNull();
  });

  it('stops at an unconfirmed grant, retains the rest, and finishes them under new operations after the retry confirms', async () => {
    let calls = 0;
    const { posted } = await open({ ...SERVICE, 'POST /grants': (body) => (++calls === 2 ? refused(503, 'AppendUncertain', 'AppendUncertain: not known') : ok({ operation: (body as DelegateBody).operation, grant: SCRIBE_G, index: 1, receipt: { caller: ADA } })) });
    await tick('edit', 'view');
    await click($('[data-act="delegatedo"]'));
    expect(posted.filter((p) => p.path === '/grants')).toHaveLength(2);
    expect($('#dAnswer[data-pending] b')?.textContent).toBe('Not confirmed yet');
    expect($('.act-panel')).not.toBeNull();
    expect(sessionStorage.getItem('lys.pending.grant.' + ADA + '.' + ROOT_G + '.rest')).toBeNull();
    expect(sessionStorage.getItem('lys.pending.grant.' + ADA + '.' + ROOT_G)).not.toBeNull();
    await click($('[data-act="delegatedo"]'));
    const given = posted.filter((p) => p.path === '/grants').map((p) => p.body as DelegateBody);
    expect(given).toHaveLength(3);
    expect(given[2]).toEqual(given[1]);
    expect(given.map((b) => b.relation)).toEqual(['only.edit', 'viewer', 'viewer']);
    expect($('#toast')?.textContent).toContain('Given.');
  });

  it('keeps the rest of a run whole across a reopen, and a definite refusal mid-run names what was given and what was not', async () => {
    let calls = 0;
    const { posted } = await open({ ...SERVICE, 'POST /grants': (body) => (++calls === 1 ? refused(503, 'AppendUncertain', 'AppendUncertain: not known') : ok({ operation: (body as DelegateBody).operation, grant: SCRIBE_G, receipt: { caller: ADA } })) });
    await choose($('#dLease'), 'no end');
    await tick('edit', 'view');
    await click($('[data-act="delegatedo"]'));
    expect($('#dAnswer[data-pending] b')?.textContent).toBe('Not confirmed yet');
    const restRaw = sessionStorage.getItem('lys.pending.grant.' + ADA + '.' + ROOT_G + '.rest');
    expect(restRaw).not.toBeNull();
    expect((JSON.parse(restRaw ?? '[]') as DelegateBody[]).map((b) => [b.relation, b.window.ends_at])).toEqual([['viewer', GRANTS.find((g) => g.id === ROOT_G)?.effective_ends_at ?? NaN]]);
    expect($('#dAnswer .note')?.textContent).toContain('then gives the 1 remaining');
    await press('Escape');
    await click($(`[data-act="delegate"][data-g="${ROOT_G}"]`));
    await click($('[data-act="delegatedo"]'));
    const given = posted.filter((p) => p.path === '/grants').map((p) => p.body as DelegateBody);
    expect(given.map((b) => b.relation)).toEqual(['only.edit', 'only.edit', 'viewer']);
    expect(given[2].window.ends_at).toBe(given[0].window.ends_at);
    expect(sessionStorage.getItem('lys.pending.grant.' + ADA + '.' + ROOT_G + '.rest')).toBeNull();
    expect($('#toast')?.textContent).toContain('Given. Scribe can now Edit this resource; View this resource project:identity, through you.');

    unmountAll();
    sessionStorage.clear();
    let count = 0;
    const second = await open({ ...SERVICE, 'POST /grants': (body) => (++count === 1 ? ok({ operation: (body as DelegateBody).operation, grant: SCRIBE_G, receipt: { caller: ADA } }) : refused(409, 'Expired', 'source expired')) });
    await tick('edit', 'view');
    await click($('[data-act="delegatedo"]'));
    expect(second.posted.filter((p) => p.path === '/grants')).toHaveLength(2);
    expect($('#dAnswer')?.dataset.refusal).toBe('Expired');
    expect($('#toast')?.textContent).toContain('Given before the refusal: Edit this resource project:identity.');
    expect(sessionStorage.getItem('lys.pending.grant.' + ADA + '.' + ROOT_G + '.rest')).toBeNull();
  });

  it('names a recipient who is not active as the service refuses it', async () => {
    await open({ ...SERVICE, 'POST /grants': refused(409, 'IdentityNotActive', `IdentityNotActive: ${SCRIBE} is suspended, and only an active identity's grants are effective`) });
    await tick('view');
    await click($('[data-act="delegatedo"]'));
    expect($('#dAnswer')?.dataset.refusal).toBe('IdentityNotActive');
    expect($('#dAnswer b')?.textContent).toContain('Scribe is suspended');
    expect($('#dAnswer b')?.textContent).not.toContain('IdentityNotActive');
    expect($('.act-panel')).not.toBeNull();
  });

  it('shows the service refusal by name and records nothing as given', async () => {
    await open({ ...SERVICE, 'POST /grants': refused(409, 'RecipientRefused', `RecipientRefused: ${ROOT_G} may not be passed on to a agent`) });
    await tick('view');
    await click($('[data-act="delegatedo"]'));
    expect($('#dAnswer')?.dataset.refusal).toBe('RecipientRefused');
    expect($('.act-panel')).not.toBeNull();
    expect($('#toast')?.textContent).not.toContain('Given');
  });

  it('keeps an unconfirmed gift pending and retries it under the same operation', async () => {
    let calls = 0;
    const { posted } = await open({ ...SERVICE, 'POST /grants': (body) => (++calls === 1 ? refused(503, 'AppendUncertain', 'AppendUncertain: not known') : ok({ operation: (body as DelegateBody).operation, grant: SCRIBE_G, index: 1, receipt: { caller: ADA } })) });
    await tick('view');
    await click($('[data-act="delegatedo"]'));
    expect($('#dAnswer[data-pending] b')?.textContent).toBe('Not confirmed yet');
    vi.spyOn(Date, 'now').mockReturnValue(Date.now() + 120000);
    expect(($('#dTo') as HTMLSelectElement).closest('fieldset')?.disabled).toBe(true);
    await click($('[data-act="delegatedo"]'));
    expect(posted[0].body).toEqual(posted[1].body);
    const ops = posted.filter((p) => p.path === '/grants').map((p) => (p.body as DelegateBody).operation);
    expect(ops).toHaveLength(2);
    expect(ops[0]).toBe(ops[1]);
    expect($('#toast')?.textContent).toContain('Given.');
  });

  it('retains the exact grant after closing and reopening the drawer, even after the clock changes', async () => {
    let count = 0;
    const { posted } = await open({ ...SERVICE, 'POST /grants': (body) => (++count === 1 ? refused(503, 'AppendUncertain', 'outcome unknown') : ok({ operation: (body as DelegateBody).operation, grant: SCRIBE_G, receipt: { caller: ADA } })) });
    await tick('view');
    await click($('[data-act="delegatedo"]'));
    await press('Escape');
    vi.spyOn(Date, 'now').mockReturnValue(Date.now() + 600000);
    await click($(`[data-act="delegate"][data-g="${ROOT_G}"]`));
    expect($('#dAnswer[data-pending]')?.textContent).toContain('Not confirmed yet');
    await click($('[data-act="delegatedo"]'));
    expect(posted.filter((entry) => entry.path === '/grants')).toHaveLength(2);
    expect(posted[1].body).toEqual(posted[0].body);
    expect($('#toast')?.textContent).toContain('Given.');
  });

  it('keeps an earlier unknown grant held when a retry answers a definite refusal', async () => {
    let count = 0;
    const { posted } = await open({ ...SERVICE, 'POST /grants': () => (++count === 1 ? refused(503, 'AppendUncertain', 'outcome unknown') : refused(409, 'Expired', 'source expired')) });
    await tick('view');
    await click($('[data-act="delegatedo"]'));
    await click($('[data-act="delegatedo"]'));
    expect($('#dAnswer[data-pending] b')?.textContent).toBe('Not confirmed yet');
    expect(posted[1].body).toEqual(posted[0].body);
    expect(sessionStorage.getItem('lys.pending.grant.' + ADA + '.' + ROOT_G)).not.toBeNull();
    expect($('#toast')?.textContent).not.toContain('Given.');
  });

  it('does not claim a malformed success is this operation or send when retention fails', async () => {
    const { posted } = await open({ ...SERVICE, 'POST /grants': ok({ operation: 'wrong', grant: SCRIBE_G, receipt: { caller: ADA } }) });
    await tick('view');
    await click($('[data-act="delegatedo"]'));
    expect($('#dAnswer[data-pending] b')?.textContent).toBe('Not confirmed yet');
    expect($('#toast')?.textContent).not.toContain('Given.');
    expect(posted).toHaveLength(1);
  });

  it('after one failed rest write, Give again sends every ticked action, not only the tail', async () => {
    const KEY = 'lys.pending.grant.' + ADA + '.' + ROOT_G;
    const { posted } = await open();
    await tick('edit', 'view');
    const real = Storage.prototype.setItem;
    let failed = false;
    const spy = vi.spyOn(Storage.prototype, 'setItem').mockImplementation(function (this: Storage, k: string, v: string) {
      if (k.endsWith('.rest') && !failed) {
        failed = true;
        throw new Error('storage unavailable');
      }
      real.call(this, k, v);
    });
    await click($('[data-act="delegatedo"]'));
    spy.mockRestore();
    const given = () => posted.filter((p) => p.path === '/grants').map((p) => p.body as DelegateBody);
    expect(given()).toEqual([]);
    expect($('#dAnswer')?.dataset.refusal).toBe('RequestNotRetained');
    // State says what storage says: nothing is held, so nothing is offered as an earlier run.
    expect(sessionStorage.getItem(KEY)).toBeNull();
    expect(sessionStorage.getItem(KEY + '.rest')).toBeNull();
    expect($('[data-rest-held]')).toBeNull();
    await click($('[data-act="delegatedo"]'));
    expect(
      given()
        .map((b) => b.relation)
        .sort()
    ).toEqual(['only.edit', 'viewer']);
    expect($('#toast')?.textContent).toContain('Given.');
  });

  it('names a rest that could not be cleared after a definite refusal mid-run, and offers it again', async () => {
    const KEY = 'lys.pending.grant.' + ADA + '.' + ROOT_G;
    let calls = 0;
    const { posted } = await open({ ...SERVICE, 'POST /grants': (body) => (++calls === 2 ? refused(403, 'RecipientRefused', 'RecipientRefused: not now') : ok({ operation: (body as DelegateBody).operation, grant: SCRIBE_G, receipt: { caller: ADA } })) });
    await tick('edit', 'view');
    const real = Storage.prototype.removeItem;
    const spy = vi.spyOn(Storage.prototype, 'removeItem').mockImplementation(function (this: Storage, k: string) {
      // The clear that follows the refusal is the one that fails; the shortening before the second send succeeds.
      if (k.endsWith('.rest') && calls >= 2) throw new Error('storage unavailable');
      real.call(this, k);
    });
    await click($('[data-act="delegatedo"]'));
    spy.mockRestore();
    expect(posted.filter((p) => p.path === '/grants')).toHaveLength(2);
    expect($('#dAnswer')?.dataset.refusal).toBe('RecipientRefused');
    expect($('#toast')?.textContent).toContain('could not clear them');
    expect(sessionStorage.getItem(KEY)).toBeNull();
    expect(sessionStorage.getItem(KEY + '.rest')).toBeNull();
  });

  it('sends nothing and names it when the rest of a two-action run cannot be retained', async () => {
    const KEY = 'lys.pending.grant.' + ADA + '.' + ROOT_G;
    const { posted } = await open();
    await tick('edit', 'view');
    const real = Storage.prototype.setItem;
    const spy = vi.spyOn(Storage.prototype, 'setItem').mockImplementation(function (this: Storage, k: string, v: string) {
      if (k.endsWith('.rest')) throw new Error('storage unavailable');
      real.call(this, k, v);
    });
    await click($('[data-act="delegatedo"]'));
    spy.mockRestore();
    expect(posted.filter((p) => p.path === '/grants')).toEqual([]);
    expect($('#dAnswer')?.dataset.refusal).toBe('RequestNotRetained');
    expect(sessionStorage.getItem(KEY)).toBeNull();
    expect(sessionStorage.getItem(KEY + '.rest')).toBeNull();
  });

  it('retains the next body before shortening the rest, names a failure to do so, and resumes the rest exactly', async () => {
    const KEY = 'lys.pending.grant.' + ADA + '.' + ROOT_G;
    const { posted } = await open();
    await tick('edit', 'view');
    const real = Storage.prototype.setItem;
    const spy = vi.spyOn(Storage.prototype, 'setItem').mockImplementation(function (this: Storage, k: string, v: string) {
      if (k === KEY && v.includes('"viewer"')) throw new Error('storage unavailable');
      real.call(this, k, v);
    });
    await click($('[data-act="delegatedo"]'));
    spy.mockRestore();
    const given = () => posted.filter((p) => p.path === '/grants').map((p) => p.body as DelegateBody);
    expect(given().map((b) => b.relation)).toEqual(['only.edit']);
    expect($('#dAnswer')?.dataset.refusal).toBe('RequestNotRetained');
    expect($('#toast')?.textContent).toContain('Not sent: View this resource');
    expect(sessionStorage.getItem(KEY)).toBeNull();
    const rest = JSON.parse(sessionStorage.getItem(KEY + '.rest') ?? 'null') as DelegateBody[] | null;
    expect(rest?.map((b) => b.relation)).toEqual(['viewer']);
    // Nothing is picked, yet the held rest is offered and sent exactly as retained, under its own operation.
    expect($('[data-rest-held]')?.textContent).toContain('1 of an earlier run not yet sent');
    expect(($('[data-act="delegatedo"]') as HTMLButtonElement).disabled).toBe(false);
    await click($('[data-act="delegatedo"]'));
    expect(given().map((b) => b.relation)).toEqual(['only.edit', 'viewer']);
    expect(given()[1].operation).toBe(rest?.[0].operation);
    expect(sessionStorage.getItem(KEY + '.rest')).toBeNull();
    expect($('#toast')?.textContent).toContain('Given.');
  });

  it('sends nothing if the browser cannot retain the original grant request', async () => {
    const { posted } = await open();
    await tick('view');
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new Error('storage unavailable');
    });
    await click($('[data-act="delegatedo"]'));
    expect(posted).toEqual([]);
    expect($('#dAnswer')?.dataset.refusal).toBe('RequestNotRetained');
  });

  it('opens the real bounded grant form from temporary access on an agent file', async () => {
    await mount('#/file/' + SCRIBE + '/access');
    await click($('[data-act="grant"]'));
    expect($('.act-panel')).not.toBeNull();
    expect(($('[aria-label="Grant to give"]') as HTMLSelectElement).value).toBe('');
    expect($('#dTo')).toBeNull();
    await choose($('[aria-label="Grant to give"]'), ROOT_G);
    expect(($('#dTo') as HTMLSelectElement).value).toBe(SCRIBE);
    expect(($('#dLease') as HTMLSelectElement).value).toBe('7 days');
    expect($('.act-panel h2')?.textContent).toContain('owner of project:identity');
  });

  it('Escape closes the drawer and focus returns', async () => {
    await mount('#/me?tab=account');
    const give = $(`[data-act="delegate"][data-g="${ROOT_G}"]`);
    give?.focus();
    await click(give);
    expect(document.activeElement?.id).toBe('dTo');
    await press('Escape');
    expect($('.act-panel')).toBeNull();
    expect(document.activeElement).toBe($(`[data-act="delegate"][data-g="${ROOT_G}"]`));
  });
});

describe("What you can't give (conformance 2.4)", () => {
  it('lists each with its reason, as the service answers it', async () => {
    const { requests } = await mount('#/me?tab=account');
    await click($(`[data-act="delegate"][data-g="${ROOT_G}"]`));
    expect(requests).toContain(`/grants/cannot-give?route=browser&source=${ROOT_G}&recipient=${SCRIBE}`);
    const reasons = $$('.act-panel [data-cannot-give]').map((d) => [d.querySelector('b')?.textContent, d.querySelector('.note')?.textContent]);
    expect(reasons).toEqual([
      ['viewer of project:ledger', 'You may use it; it does not let you pass it on.'],
      ['Your sign-in identities', 'They prove who you are. No agent can hold them.'],
    ]);
  });
});

describe('Can X do this? (conformance 8.1)', () => {
  it("answers yes with the path to a person on an agent's file", async () => {
    const { posted } = await mount(`#/file/${SCRIBE}/access`);
    expect($$('tr[data-grant] .chain .pill').map((p) => p.textContent)).toEqual(['Ada (test person)', 'Scribe']);
    expect($('tr[data-grant] .chain')?.getAttribute('title')).toBe('Ada (test person), P/00000000 · owner of project:identity → Scribe, agent of Ada (test person), A/00000000 · viewer of project:identity');
    await choose($('#cPerm'), 'view');
    await press('c', {}, document.body);
    expect(posted.some((p) => p.path === '/grants/who')).toBe(true);
    expect(posted.some((p) => p.path === '/grants/check')).toBe(false);
    expect($('#answer .verdict-mark')?.textContent).toBe('Yes');
    expect($('#answer .why')?.textContent).toBe('view needs viewers or editors or owners.');
    expect($('#answer .chain')?.textContent).toBe('Ada (test person)→Scribe');
    expect($('#answer .chain')?.getAttribute('title')).toContain('Scribe, agent of Ada (test person), A/00000000 · viewer of project:identity');
    expect($('#answer .meta-line')?.textContent).toContain('model v1');
    expect($('#answer .meta-line')?.textContent).toContain('change 7');
  });

  it('answers no, and says it was not found among who can', async () => {
    await mount(`#/file/${SCRIBE}/access`);
    await choose($('#cPerm'), 'edit');
    await click($('[data-act="check"]'));
    expect($('#answer .verdict-mark')?.textContent).toBe('No');
    expect($('#answer .tag')?.textContent).toBe('No grant');
    expect($('#answer .refusal-name')).toBeNull();
    expect($('#answer .meta-line')?.textContent).toContain('model v3');
    expect($('#answer .meta-line')?.textContent).toContain('change 7');
  });

  it('names a refusal for a suspended identity, with no design note beside it', async () => {
    await mount('#/access/can', { ...SERVICE, 'POST /grants/why': refused(409, 'IdentityNotActive', `IdentityNotActive: ${ADA} is suspended, and only an active identity's grants are effective`) });
    await click($('[data-act="check"]'));
    expect($('#answer .refusal-name')?.textContent).toBe('IdentityNotActive');
    expect($('#answer .why')?.textContent).not.toContain('IdentityNotActive');
    expect($('#answer .open-q')).toBeNull();
  });

  it('names each resource as the mock-up does: a project by its id, anything else as name (type)', async () => {
    const channel = { ...GRANTS[1], id: 'grant-' + '9'.repeat(32), resource: { kind: 'channel', id: 'general' } };
    await mount('#/access/can', { ...SERVICE, '/grants': ok({ grants: [...GRANTS, channel], revision: 7 }) });
    expect($$('#cRes option').map((o) => o.textContent)).toEqual(['project:identity', 'project:ledger', 'organisation:' + ORGANISATION, 'general (channel)']);
    expect($$('#cRes option').map((o) => (o as HTMLOptionElement).value)).toEqual(['project:identity', 'project:ledger', 'organisation:' + ORGANISATION, 'channel:general']);
  });

  it("asks /grants/why for the caller's own question and shows its named refusal", async () => {
    const { posted } = await mount('#/access/can');
    await choose($('#cRes'), 'project:ledger');
    await choose($('#cPerm'), 'view');
    await click($('[data-act="check"]'));
    expect(posted.at(-1)?.path).toBe('/grants/why');
    expect(posted.some((p) => p.path === '/grants/check')).toBe(false);
    expect($('#answer .verdict-mark')?.textContent).toBe('Yes');
  });

  it('shows the refusal /grants/why names', async () => {
    await mount('#/access/can', { ...SERVICE, 'POST /grants/why': refused(409, 'Revoked', `Revoked: ${ROOT_G} was revoked, and nothing derived from it is effective`) });
    await click($('[data-act="check"]'));
    expect($('#answer .verdict-mark')?.textContent).toBe('No');
    expect($('#answer .refusal-name')?.textContent).toBe('Revoked');
    expect($('#answer .why')?.textContent).not.toContain('Revoked:');
    expect($('#answer .why')?.textContent).toContain('was revoked');
  });
});
