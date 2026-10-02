import { beforeEach, describe, expect, it, vi } from 'vitest';
import { $, $$, choose, click, mount, press, text, unmountAll, unreachable, settle } from './harness';
import {
  ADA, BEA, BEA_DIRECTORY, BEA_GRANTS, BEA_REVIEWER_G, BEA_ROOT_G, BEA_SERVICE as BEA_BASE, DIRECTORY, GRANTS, LEDGER_G,
  AGENT_MODEL as MODEL, ROOT_G, SCRIBE, SCRIBE_G, SERVICE as BASE, ok, refused,
} from './fixtures';
import type { DelegateBody, LastUse } from '../src/generated/grants';

beforeEach(() => sessionStorage.clear());

const SERVICE = { ...BASE, '/grants/model': ok(MODEL) };
const BEA_SERVICE = { ...BEA_BASE, '/grants/model': ok(MODEL) };

const holdRows = () => $$('.grid2 > div:first-child table')[0].querySelectorAll('tbody tr');

describe('What you hold', () => {
  it('row_1_4_what_you_hold_shows_each_grant_its_source_and_whether_it_may_be_passed_on', async () => {
    const { requests } = await mount('#/me?tab=account');
    expect(requests).toContain('/grants');
    const rows = holdText();
    expect(rows).toEqual([
      ['You can do everything here (project identity).', 'owner', 'project:identity', 'root', 'yes', 'Give to an agent…'],
      ['View this resource (project ledger).', 'viewer', 'project:ledger', 'root', 'no', ''],
    ]);
    expect(unreachable()).toEqual([]);
  });
});

describe('The delegation form', () => {
  const open = async (routes = SERVICE) => {
    const m = await mount('#/me?tab=account', routes);
    await click($(`[data-act="delegate"][data-g="${ROOT_G}"]`));
    return m;
  };

  it('row_2_3_the_form_shows_the_source_grant_its_actions_may_pass_on_and_the_effective_end', async () => {
    await open();
    expect($('#drawer')?.classList.contains('open')).toBe(true);
    expect(document.activeElement?.id).toBe('dTo');
    expect($('#drawer h2')?.textContent).toBe('Give part of owner of project:identity to an agent');
    const from = $$('#drawer .card')[0].textContent ?? '';
    expect(from).toContain('Source grant');
    expect(from).toContain('owner of project:identity');
    expect(from).toContain('Actions it allowseverything here');
    expect(from).toContain('You may pass it onagent');
    expect(from).toContain('Ends no later than27 Oct');
    const ends = () => $$('#drawer .card')[0].querySelectorAll('.row')[3];
    expect(ends().textContent).toBe('Ends no later than27 Oct');
    expect(ends().querySelector('.open-q')).toBeNull();
    expect(from).not.toContain('when yours does');
    expect($$('#dLease option').map((o) => o.textContent)).toEqual(['7 days', 'ends with assignment', 'no end']);
    expect(unreachable()).toEqual([]);

    unmountAll();
    const endless = GRANTS.map((g) => (g.id === ROOT_G ? { ...g, window: { ...g.window, ends_at: null }, effective_ends_at: null } : g));
    await open({ ...SERVICE, '/grants': ok({ grants: endless, revision: 7 }) });
    expect($$('#drawer .card')[0].textContent).toContain('Ends no later thanno end');
    expect(ends().textContent).toBe('Ends no later thanno end');
  });

  const tick = async (...actions: string[]) => { for (const action of actions) await click($(`#drawer input[name="action"][value="${action}"]`)); };

  it('offers an agent one action at a time, only what may be passed on and is not withheld, and never a wider relation', async () => {
    const { requests } = await open();
    expect(requests).toContain('/grants/model');
    expect($$('#drawer input[name="action"]').map((c) => (c as HTMLInputElement).value)).toEqual(['edit', 'view']);
    expect($$('#drawer input[name="action"]').filter((c) => (c as HTMLInputElement).checked)).toEqual([]);
    expect($$('[data-pickrel]')).toEqual([]);
    expect(($('[data-act="delegatedo"]') as HTMLButtonElement).disabled).toBe(true);
  });

  it('offers nothing to an agent until the service names what it withholds from agents', async () => {
    const { withheld_from_agents: _withheld, ...silent } = MODEL;
    await open({ ...SERVICE, '/grants/model': ok(silent) });
    expect($$('#drawer input[name="action"]')).toEqual([]);
    expect($('#drawer')?.textContent).toContain('has not said which actions an agent may hold');
  });

  it("offers an agent nothing on an app's resource, since the service refuses every agent grant there until the app allows it", async () => {
    const onApp = GRANTS.map((g) => (g.id === ROOT_G ? { ...g, resource: { kind: 'fixture.doc', id: 'd1' } } : g));
    await open({ ...SERVICE, '/grants': ok({ grants: onApp, revision: 7 }) });
    expect($$('#drawer input[name="action"]')).toEqual([]);
    expect($('#drawer')?.textContent).toContain("can't give an agent this app's actions until the app allows it");
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
    expect($('#drawer')?.classList.contains('open')).toBe(false);
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
    const { posted } = await open({
      ...SERVICE,
      'POST /grants': (body) => (++calls === 2 ? refused(503, 'AppendUncertain', 'AppendUncertain: not known') : ok({ operation: (body as DelegateBody).operation, grant: SCRIBE_G, index: 1, receipt: { caller: ADA } })),
    });
    await tick('edit', 'view');
    await click($('[data-act="delegatedo"]'));
    expect(posted.filter((p) => p.path === '/grants')).toHaveLength(2);
    expect($('#dAnswer b')?.textContent).toBe('pending');
    expect($('#drawer')?.classList.contains('open')).toBe(true);
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
    const { posted } = await open({
      ...SERVICE,
      'POST /grants': (body) => (++calls === 1 ? refused(503, 'AppendUncertain', 'AppendUncertain: not known') : ok({ operation: (body as DelegateBody).operation, grant: SCRIBE_G, receipt: { caller: ADA } })),
    });
    await choose($('#dLease'), 'no end');
    await tick('edit', 'view');
    await click($('[data-act="delegatedo"]'));
    expect($('#dAnswer b')?.textContent).toBe('pending');
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
    expect($('#dAnswer b')?.textContent).toBe('Expired');
    expect($('#toast')?.textContent).toContain('Given before the refusal: Edit this resource project:identity.');
    expect(sessionStorage.getItem('lys.pending.grant.' + ADA + '.' + ROOT_G + '.rest')).toBeNull();
  });

  it('names a recipient who is not active as the service refuses it', async () => {
    await open({ ...SERVICE, 'POST /grants': refused(409, 'IdentityNotActive', `IdentityNotActive: ${SCRIBE} is suspended, and only an active identity's grants are effective`) });
    await tick('view');
    await click($('[data-act="delegatedo"]'));
    expect($('#dAnswer b')?.textContent).toBe('IdentityNotActive');
    expect($('#dAnswer .note')?.textContent).toContain('is suspended');
    expect($('#drawer')?.classList.contains('open')).toBe(true);
  });

  it('shows the service refusal by name and records nothing as given', async () => {
    await open({ ...SERVICE, 'POST /grants': refused(409, 'RecipientRefused', `RecipientRefused: ${ROOT_G} may not be passed on to a agent`) });
    await tick('view');
    await click($('[data-act="delegatedo"]'));
    expect($('#dAnswer b')?.textContent).toBe('RecipientRefused');
    expect($('#drawer')?.classList.contains('open')).toBe(true);
    expect($('#toast')?.textContent).not.toContain('Given');
  });

  it('keeps an unconfirmed gift pending and retries it under the same operation', async () => {
    let calls = 0;
    const { posted } = await open({
      ...SERVICE,
      'POST /grants': (body) => (++calls === 1 ? refused(503, 'AppendUncertain', 'AppendUncertain: not known') : ok({ operation: (body as DelegateBody).operation, grant: SCRIBE_G, index: 1, receipt: { caller: ADA } })),
    });
    await tick('view');
    await click($('[data-act="delegatedo"]'));
    expect($('#dAnswer b')?.textContent).toBe('pending');
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
    const { posted } = await open({ ...SERVICE, 'POST /grants': (body) => ++count === 1
      ? refused(503, 'AppendUncertain', 'outcome unknown')
      : ok({ operation: (body as DelegateBody).operation, grant: SCRIBE_G, receipt: { caller: ADA } }) });
    await tick('view');
    await click($('[data-act="delegatedo"]'));
    await press('Escape');
    vi.spyOn(Date, 'now').mockReturnValue(Date.now() + 600000);
    await click($(`[data-act="delegate"][data-g="${ROOT_G}"]`));
    expect($('#dAnswer')?.textContent).toContain('pending');
    await click($('[data-act="delegatedo"]'));
    expect(posted.filter((entry) => entry.path === '/grants')).toHaveLength(2);
    expect(posted[1].body).toEqual(posted[0].body);
    expect($('#toast')?.textContent).toContain('Given.');
  });

  it('keeps an earlier unknown grant held when a retry answers a definite refusal', async () => {
    let count = 0;
    const { posted } = await open({ ...SERVICE, 'POST /grants': () => ++count === 1
      ? refused(503, 'AppendUncertain', 'outcome unknown') : refused(409, 'Expired', 'source expired') });
    await tick('view');
    await click($('[data-act="delegatedo"]'));
    await click($('[data-act="delegatedo"]'));
    expect($('#dAnswer b')?.textContent).toBe('pending');
    expect(posted[1].body).toEqual(posted[0].body);
    expect(sessionStorage.getItem('lys.pending.grant.' + ADA + '.' + ROOT_G)).not.toBeNull();
    expect($('#toast')?.textContent).not.toContain('Given.');
  });

  it('does not claim a malformed success is this operation or send when retention fails', async () => {
    const { posted } = await open({ ...SERVICE, 'POST /grants': ok({ operation: 'wrong', grant: SCRIBE_G, receipt: { caller: ADA } }) });
    await tick('view');
    await click($('[data-act="delegatedo"]'));
    expect($('#dAnswer b')?.textContent).toBe('pending');
    expect($('#toast')?.textContent).not.toContain('Given.');
    expect(posted).toHaveLength(1);
  });

  it('sends nothing if the browser cannot retain the original grant request', async () => {
    const { posted } = await open();
    await tick('view');
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => { throw new Error('storage unavailable'); });
    await click($('[data-act="delegatedo"]'));
    expect(posted).toEqual([]);
    expect($('#dAnswer b')?.textContent).toBe('RequestNotRetained');
  });

  it('opens the real bounded grant form from temporary access on an agent file', async () => {
    await mount('#/file/' + SCRIBE + '/access');
    await click($('[data-act="temporary-access"]'));
    expect($('#drawer')?.classList.contains('open')).toBe(true);
    expect(($('#dTo') as HTMLSelectElement).value).toBe(SCRIBE);
    expect(($('#dLease') as HTMLSelectElement).value).toBe('7 days');
  });

  it('Escape closes the drawer and focus returns', async () => {
    await mount('#/me?tab=account');
    const give = $(`[data-act="delegate"][data-g="${ROOT_G}"]`);
    give?.focus();
    await click(give);
    expect(document.activeElement?.id).toBe('dTo');
    await press('Escape');
    expect($('#drawer')?.classList.contains('open')).toBe(false);
    expect(document.activeElement).toBe($(`[data-act="delegate"][data-g="${ROOT_G}"]`));
  });
});

describe("What you can't give (conformance 2.4)", () => {
  it('lists each with its reason, as the service answers it', async () => {
    const { requests } = await mount('#/me?tab=account');
    await click($(`[data-act="delegate"][data-g="${ROOT_G}"]`));
    expect(requests).toContain(`/grants/cannot-give?route=browser&source=${ROOT_G}&recipient=${SCRIBE}`);
    const reasons = $$('#drawer .why-not').map((d) => [d.querySelector('b')?.textContent, d.querySelector('.note')?.textContent]);
    expect(reasons).toEqual([
      ['viewer of project:ledger', 'You may use it; it does not let you pass it on.'],
      ['Your sign-in identities', 'They prove who you are. No agent can hold them.'],
    ]);
  });
});

describe('Can X do this? (conformance 8.1)', () => {
  it("answers yes with the path to a person on an agent's file", async () => {
    const { posted } = await mount(`#/file/${SCRIBE}/access`);
    expect($$('.file .card .chain .pill').map((p) => p.textContent)).toEqual(['Ada (test person) · owner of project:identity', "Scribe · viewer of project:identity"]);
    await choose($('#cPerm'), 'view');
    await press('c', {}, document.body);
    expect(posted.some((p) => p.path === '/grants/who')).toBe(true);
    expect(posted.some((p) => p.path === '/grants/check')).toBe(false);
    expect($('#answer .verdict-mark')?.textContent).toBe('Yes');
    expect($('#answer .why')?.textContent).toBe('view needs viewers or editors or owners.');
    expect($('#answer .chain')?.textContent).toContain("Scribe · viewer of project:identity");
    expect($('#answer .meta-line')?.textContent).toContain('model v1');
    expect($('#answer .meta-line')?.textContent).toContain('change 7');
  });

  it('answers no, and says it was not found among who can', async () => {
    await mount(`#/file/${SCRIBE}/access`);
    await choose($('#cPerm'), 'edit');
    await click($('[data-act="check"]'));
    expect($('#answer .verdict-mark')?.textContent).toBe('No');
    expect($('#answer .tag')?.textContent).toBe('no grant');
    expect($('#answer .meta-line')?.textContent).toContain('model v3');
    expect($('#answer .meta-line')?.textContent).toContain('change 7');
  });

  it('tags a refusal for a suspended identity with the open question', async () => {
    await mount('#/access/can', { ...SERVICE, 'POST /grants/why': refused(409, 'IdentityNotActive', `IdentityNotActive: ${ADA} is suspended, and only an active identity's grants are effective`) });
    await click($('[data-act="check"]'));
    expect($('#answer .tag')?.textContent).toBe('IdentityNotActive');
    expect($('#answer .why .open-q')?.textContent).toBe('what suspension refuses: open');
  });

  it('names each resource as the mock-up does: a project by its id, anything else as name (type)', async () => {
    const channel = { ...GRANTS[1], id: 'grant-' + '9'.repeat(32), resource: { kind: 'channel', id: 'general' } };
    await mount('#/access/can', { ...SERVICE, '/grants': ok({ grants: [...GRANTS, channel], revision: 7 }) });
    expect($$('#cRes option').map((o) => o.textContent)).toEqual(['project:identity', 'project:ledger', 'general (channel)']);
    expect($$('#cRes option').map((o) => (o as HTMLOptionElement).value)).toEqual(['project:identity', 'project:ledger', 'channel:general']);
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
    expect($('#answer .tag')?.textContent).toBe('Revoked');
    expect($('#answer .why')?.textContent).toContain('was revoked');
  });
});

describe('Who can reach this? (conformance 8.2)', () => {
  it('lists everyone with what they can do, from the same answers', async () => {
    await mount('#/access/who/project:identity');
    const rows = $$('#whoCan .row').map((r) => r.textContent);
    expect(rows).toEqual(['Ada (test person)everything here', "ScribeView this resource"]);
    await choose($('select[aria-label="Resource"]'), 'project:ledger');
    expect(location.hash).toBe('#/access/who/project:ledger');
    expect($$('#whoCan .row').map((r) => r.textContent)).toEqual(['Ada (test person)View this resource']);
  });

  it('lists every grant, with where it derives from, and keeps every control reachable', async () => {
    await mount('#/access/reach/' + SCRIBE);
    expect($$('.check .card tr').map((r) => r.textContent)).toEqual(['project identitybuilt inView this resource']);
    const grants = $$('table tbody tr[data-href]').map((r) => r.querySelectorAll('td')[4].textContent);
    expect(grants).toEqual(['root', 'root', `G/${ROOT_G.slice(6, 14)} · Ada (test person)`]);
    const used = $$('table tbody tr[data-href]').map((r) => r.querySelectorAll('td')[7].textContent);
    expect(used).toEqual(['not seen', 'not seen', '27 Sep 12:00 · tool']);
    expect(unreachable()).toEqual([]);
    expect(SCRIBE_G).toMatch(/^grant-/);
  });
});

describe('A grant card', () => {
  it('shows a void grant of a suspended holder with the open question, and no Allows line', async () => {
    const suspend = (v: typeof DIRECTORY) => ({ ...v, people: v.people.map((p) => ({ ...p, agents: p.agents.map((a) => (a.id === SCRIBE ? { ...a, state: 'suspended' as const } : a)) })) });
    const reason = `IdentityNotActive: ${SCRIBE} is suspended, and only an active identity's grants are effective`;
    const refusedScribe = GRANTS.map((g) => (g.id === SCRIBE_G ? { ...g, standing: { stands: false as const, refusal: 'IdentityNotActive', grant: null, reason } } : g));
    await mount(`#/file/${SCRIBE}/access`, { ...SERVICE, '/grants': ok({ grants: refusedScribe, revision: 7 }), '/directory/people': ok(suspend(DIRECTORY)), '/people': ok(suspend({ ...DIRECTORY, scope: 'personal', people: [DIRECTORY.people[0]] })) });
    const card = $$('.file .card').find((c) => c.querySelector('.verdict-mark.no'));
    expect(card?.textContent).toContain(`${reason} what suspension refuses: open`);
    expect(text()).not.toContain('Allows:');
    expect(card?.textContent).toContain('Last used: 27 Sep 12:00 · tool');
  });
});

describe('A grant card, its use and its window', () => {
  it('row_8_4_a_grant_card_shows_last_used_its_source_and_window_and_never_never_used', async () => {
    const unseen = GRANTS.map((g) => (g.id === SCRIBE_G ? { ...g, last_use: { seen: false as const, recorded: 0, source: 'reported' as const } } : g));
    await mount(`#/file/${SCRIBE}/access`, { ...SERVICE, '/grants': ok({ grants: unseen, revision: 7 }) });
    const card = $$('.file .card').find((c) => c.querySelector('.chain') && c.textContent?.includes('Last used'));
    expect(card?.textContent).toContain('Last used: not seen');
    expect([...(card?.querySelectorAll('.chain .pill') ?? [])].map((p) => p.textContent)).toEqual(['Ada (test person) · owner of project:identity', 'Scribe · viewer of project:identity']);
    expect(card?.textContent).toContain('Window: 27 Sep to 4 Oct');
    expect(card?.textContent).not.toContain('never used');
  });
});

describe('A grant that ends with a role assignment (conformance 4.5)', () => {
  const ROLE_END = 1_760_000_000;
  const routes = {
    ...SERVICE,
    '/roles': ok({ roles: [{
      id: 'role-0000000000000000000000000000000000000000000000000000000000000001', name: 'Scribe', latest: 1, policy: 'stays_until_moved', versions: [],
      holders: [{
        assignment: 'op-00000000000000000000000000000001', holder: SCRIBE, display_name: 'Scribe', version: 1, behind: false, assigned_by: ADA,
        assigned_at: 1_759_000_000, ends_at: ROLE_END, moves_at: null, state: 'holding', moves: [], ended_by: null, ended_at: null,
      }],
    }] }),
  };

  it('offers each dated holding of the recipient as an end, and posts that end', async () => {
    const { posted } = await mount('#/me?tab=account', routes);
    await click($(`[data-act="delegate"][data-g="${ROOT_G}"]`));
    await choose($('#dTo'), SCRIBE);
    expect($$('#dLease option').map((o) => o.textContent)).toEqual(['7 days', 'ends with Scribe assignment (9 Oct)', 'no end']);
    await choose($('#dLease'), 'assignment:op-00000000000000000000000000000001');
    await click($('#drawer input[name="action"][value="view"]'));
    await click($('[data-act="delegatedo"]'));
    const body = posted.find((p) => p.path === '/grants')?.body as DelegateBody;
    expect(body.window.ends_at).toBe(ROLE_END);
  });

  it('greys the choice, saying why, when the recipient holds no dated role', async () => {
    await mount('#/me?tab=account');
    await click($(`[data-act="delegate"][data-g="${ROOT_G}"]`));
    const option = $$('#dLease option')[1];
    expect(option?.hasAttribute('disabled')).toBe(true);
    expect(option?.getAttribute('title')).toBe('the agent holds no role with an end date');
  });
});

/** The hold table as text, one array per row. */
const holdText = () => [...holdRows()].map((tr) => { const [first, ...rest] = [...tr.querySelectorAll('td')]; return [first.firstChild?.textContent, ...[...first.querySelectorAll('.mono')].map((span) => span.textContent), ...rest.map((td) => td.textContent)]; });

/** Every grant id the page put in the DOM, from the buttons that carry one. */
const grantIdsOnScreen = () => $$('[data-g]').map((el) => el.dataset.g);

/** Start the next mount of a test from an empty page, as a new session would. */
const fresh = () => {
  unmountAll();
  document.body.innerHTML = '';
  sessionStorage.clear();
};

describe('Two people and their agents (conformance 1.4, 1.5)', () => {
  it('renders one person from their fixture ids, and only the other after the signed-in person changes', async () => {
    const ada = await mount('#/me?tab=account');
    const meReads = ada.requests.filter((r) => r === '/me').length;
    expect(meReads).toBeGreaterThan(0);
    expect($('h1')?.textContent).toBe('Ada (test person)');
    expect(holdText()).toEqual([
      ['You can do everything here (project identity).', 'owner', 'project:identity', 'root', 'yes', 'Give to an agent…'],
      ['View this resource (project ledger).', 'viewer', 'project:ledger', 'root', 'no', ''],
    ]);
    expect(grantIdsOnScreen()).toEqual([ROOT_G]);
    await click($(`[data-act="delegate"][data-g="${ROOT_G}"]`));
    expect($('#drawer')?.textContent).toContain('Scribe');
    const adaSource = $$('#drawer .card')[0].textContent ?? '';
    expect(adaSource).toContain('owner of project:identity');
    expect(adaSource).toContain('Actions it allowseverything here');
    expect(adaSource).toContain('You may pass it onagent');
    expect(adaSource).toContain('Ends no later than27 Oct');

    fresh();

    const bea = await mount('#/me?tab=account', BEA_SERVICE);
    // The second session read its own identity for itself; nothing was carried over.
    expect(bea.requests.filter((r) => r === '/me')).toHaveLength(meReads);
    expect($('h1')?.textContent).toBe('Bea (test person)');
    expect(holdText()).toEqual([['Edit this resource; View this resource (project ledger).', 'editor', 'project:ledger', 'root', 'yes', 'Give to an agent…']]);
    expect(grantIdsOnScreen()).toEqual([BEA_ROOT_G]);
    // Her agent, and what it holds under her root grant, not Ada's.
    location.hash = '#/me';
    await settle();
    expect(text()).toContain('Reviewer');
    expect($$('tr[data-href]').map((tr) => tr.querySelectorAll('td')[2].getAttribute('title'))).toContain('viewer of project:ledger');
    location.hash = '#/me?tab=account';
    await settle();
    await click($(`[data-act="delegate"][data-g="${BEA_ROOT_G}"]`));
    const beaSource = $$('#drawer .card')[0].textContent ?? '';
    expect(beaSource).toContain('editor of project:ledger');
    expect(beaSource).toContain('Actions it allowsEdit this resource; View this resource');
    expect(beaSource).toContain('You may pass it onagent');
    expect(beaSource).toContain('Ends no later than15 Nov');

    // Nothing of the first person survived the change of session. Bea's own
    // directory names Ada, so Ada is offered as a recipient in To and nowhere else.
    const offered = $$('#dTo option').map((o) => (o as HTMLOptionElement).value);
    expect(offered).toContain(ADA);
    const rest = document.body.cloneNode(true) as HTMLElement;
    rest.querySelector('#dTo')?.remove();
    for (const id of [ROOT_G, LEDGER_G, SCRIBE_G, ADA, SCRIBE]) expect(rest.innerHTML).not.toContain(id);
    expect(rest.textContent).not.toContain('Ada (test person)');
    expect(rest.textContent).not.toContain('Scribe');
    expect(text()).not.toContain('project:identity');
  });

  it("shows another person's grants through the administrator's own directory route", async () => {
    const seen = { ...SERVICE, '/directory/people': ok(BEA_DIRECTORY), '/grants': ok({ grants: [...GRANTS, ...BEA_GRANTS], revision: 7 }) };
    const { requests } = await mount(`#/file/${BEA}/access`, seen);
    // The wider route answered, so the personal one was never asked for.
    expect(requests).toContain('/directory/people');
    expect(requests).not.toContain('/people');
    expect($('h1')?.textContent).toBe('Bea (test person)');
    const cards = $$('.file .card').filter((c) => (c.textContent ?? '').includes('Passable:'));
    expect(cards).toHaveLength(1);
    expect(cards[0].textContent).toContain('editor of project:ledger');
    expect(cards[0].textContent).toContain('Passable: agent');
    expect(cards[0].textContent).toContain('Window: 27 Sep to 15 Nov');
    expect(cards[0].textContent).toContain('Last used: not seen');
    expect(grantIdsOnScreen()).toEqual([BEA_ROOT_G]);
    // The administrator sees her grants, not her agent's: only what she holds.
    expect(document.body.innerHTML).not.toContain(BEA_REVIEWER_G);
  });

  it('refuses that same route by name when the caller is not a directory administrator', async () => {
    const seen = {
      ...SERVICE,
      '/directory/people': refused(403, 'NotAdmitted', 'the wider directory is for its administrators'),
      '/grants': ok({ grants: [...GRANTS, ...BEA_GRANTS], revision: 7 }),
    };
    const { requests, posted } = await mount(`#/file/${BEA}/access`, seen);
    // The wider route was asked, refused, and the personal scope answered instead.
    expect(requests.filter((r) => r === '/directory/people').length).toBeGreaterThan(0);
    expect(requests).toContain('/people');
    expect($('.why-not details')?.textContent).toContain('PersonNotVisible');
    expect(text()).toContain('not among the records you may see');
    expect(document.body.innerHTML).not.toContain(BEA_ROOT_G);
    expect(grantIdsOnScreen()).toEqual([]);
    expect($$('[data-act="revoke"]')).toEqual([]);
    expect(posted).toEqual([]);
  });
});

/**
 * The refusals the service answers a delegation with. The reason is the whole
 * text `GrantError`'s `Display` writes (crates/lys-identity/src/grants/error.rs);
 * the refusal name is its first word, which is what `ServerError::name()` sends
 * (crates/lys-identity-server/src/error_status.rs), and the status is the one
 * `grant_status` gives that variant.
 */
interface RefusalCase {
  /** What the server refused. */
  what: string;
  status: number;
  /** The server's whole reason text; its first word is the refusal name. */
  reason: string;
  /** What the screen puts in bold: the refusal's name, or `pending` when the outcome is unknown. */
  marker: string;
  /** Whether the original request stays retained, pending its outcome. */
  retained: boolean;
}

/** `ServerError::name()`: the refusal's name is the first word of its message. */
const refusalName = (reason: string): string => reason.split(':')[0];

const REFUSALS: RefusalCase[] = [
  { what: 'use-only source', status: 403, marker: 'UseOnly', retained: false,
    reason: `UseOnly: ${LEDGER_G} may be exercised and not passed on` },
  { what: 'excessive requested scope', status: 403, marker: 'ActionsOutside', retained: false,
    reason: 'ActionsOutside: `owner` in model version 3 carries grant, outside the authority held' },
  { what: 'people-only policy', status: 403, marker: 'RecipientRefused', retained: false,
    reason: `RecipientRefused: ${ROOT_G} may not be passed on to a agent` },
  { what: 'expired ancestor', status: 403, marker: 'Expired', retained: false,
    reason: `Expired: ${ROOT_G} ended at ${String(GRANTS[0].window.ends_at)}` },
  { what: 'service outage', status: 503, marker: 'pending', retained: true,
    reason: 'PermissionEngineUnavailable: the permission relationships could not be read' },
];

describe('Every refusal of a delegation (conformance 2.3, 2.4)', () => {
  it('names the server refusal, sends exactly one request, and records no grant as given', async () => {
    let ran = 0;
    for (const c of REFUSALS) {
      const name = refusalName(c.reason);
      const { requests, posted } = await mount('#/me?tab=account', { ...SERVICE, 'POST /grants': refused(c.status, name, c.reason) });
      const before = holdText();
      expect(before).toHaveLength(2);
      const reads = requests.filter((r) => r === '/grants').length;
      await click($(`[data-act="delegate"][data-g="${ROOT_G}"]`));
      await click($('#drawer input[name="action"][value="view"]'));
    await click($('[data-act="delegatedo"]'));

      // One request, no retry, nothing else posted.
      expect(posted.filter((p) => p.path === '/grants')).toHaveLength(1);
      expect(posted).toHaveLength(1);
      expect(requests.filter((r) => r === 'POST /grants')).toHaveLength(1);
      // The refusal, exactly as the API answered it.
      expect($('#dAnswer b')?.textContent).toBe(c.marker);
      expect($('#dAnswer')?.textContent).toContain(c.reason);
      expect(text()).toContain(name);
      // Nothing given: no re-read of the list, no success, the rows as they were.
      expect(requests.filter((r) => r === '/grants')).toHaveLength(reads);
      expect($('#toast')?.textContent ?? '').not.toContain('Given');
      expect($('#drawer')?.classList.contains('open')).toBe(true);
      expect(holdText()).toEqual(before);
      expect(sessionStorage.getItem('lys.pending.grant.' + ADA + '.' + ROOT_G) !== null).toBe(c.retained);

      ran += 1;
      fresh();
    }
    expect(ran).toBe(5);
    expect(REFUSALS).toHaveLength(5);
    expect(new Set(REFUSALS.map((c) => c.what)).size).toBe(5);
  });
});

describe('A grant whose uses were not all recorded (conformance 8.4)', () => {
  const card = () => $$('.file .card').filter((c) => (c.textContent ?? '').includes('Last used:'));
  const withLastUse = (last_use: LastUse) => GRANTS.map((g) => (g.id === SCRIBE_G ? { ...g, last_use } : g));

  it('counts the uses it could not record beside "not seen", and a reported zero says only "not seen"', async () => {
    const unreported = { count: 1, at: GRANTS[2].window.starts_at, route: 'tool' as const, reason: 'the use log was unavailable' };
    const missing = withLastUse({ seen: false, recorded: 0, source: 'missing', unreported });
    await mount(`#/file/${SCRIBE}/access`, { ...SERVICE, '/grants': ok({ grants: missing, revision: 7 }) });
    expect(card()).toHaveLength(1);
    expect(card()[0].textContent).toContain('Last used: not seen · 1 use not recorded');
    expect(text()).not.toContain('never used');

    fresh();

    // A reported zero is a zero, and is never told as a missing one.
    const reported = withLastUse({ seen: false, recorded: 0, source: 'reported' });
    await mount(`#/file/${SCRIBE}/access`, { ...SERVICE, '/grants': ok({ grants: reported, revision: 7 }) });
    expect(card()).toHaveLength(1);
    expect(card()[0].textContent).toContain('Last used: not seen');
    expect(text()).not.toContain('not recorded');
    expect(text()).not.toContain('never used');
  });
});
