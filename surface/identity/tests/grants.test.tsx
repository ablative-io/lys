import { describe, expect, it } from 'vitest';
import { $, $$, choose, click, mount, press, text, unreachable } from './harness';
import { ADA, DIRECTORY, GRANTS, LEDGER_G, ROOT_G, SCRIBE, SCRIBE_G, SERVICE, ok, refused } from './fixtures';
import type { DelegateBody } from '../src/generated/grants';

const holdRows = () => $$('.grid2 > div:first-child table')[0].querySelectorAll('tbody tr');

describe('What you hold (conformance 1.4)', () => {
  it('shows each grant, its source, and whether it may be passed on', async () => {
    const { requests } = await mount('#/me');
    expect(requests).toContain('/grants');
    const rows = [...holdRows()].map((tr) => [...tr.querySelectorAll('td')].map((td) => td.textContent));
    expect(rows).toEqual([
      ['owner', 'project:identity', 'root', 'yes, to agents', 'Give to an agent…'],
      ['viewer', 'project:ledger', 'root', 'no', ''],
    ]);
    expect(unreachable()).toEqual([]);
  });
});

describe('The delegation form (conformance 2.1 to 2.3)', () => {
  const open = async (routes = SERVICE) => {
    const m = await mount('#/me', routes);
    await click($(`[data-act="delegate"][data-g="${ROOT_G}"]`));
    return m;
  };

  it('opens in the drawer, focused, and shows the source grant, its actions, may-pass-on and its end', async () => {
    await open();
    expect($('#drawer')?.classList.contains('open')).toBe(true);
    expect(document.activeElement?.id).toBe('dTo');
    expect($('#drawer h2')?.textContent).toBe('Give part of owner of project:identity to an agent');
    const from = $$('#drawer .card')[0].textContent ?? '';
    expect(from).toContain('Source grant');
    expect(from).toContain('owner of project:identity');
    expect(from).toContain('Actions it allowsedit, grant, view');
    expect(from).toContain('You may pass it onagent');
    expect(from).toContain('Ends no later thanillustrative27 Oct, when yours does');
    expect($$('#dLease option').map((o) => o.textContent)).toEqual(['7 days', 'ends with assignment', 'no end']);
    expect(unreachable()).toEqual([]);
  });

  it('offers only relations within what may be passed on, greying the rest', async () => {
    await open();
    expect($$('[data-pickrel]').map((c) => c.textContent)).toEqual(['viewer']);
    expect($('[data-pickrel].on')?.textContent).toBe('viewer');
    const greyed = $$('#drawer .chk').filter((c) => !c.dataset.pickrel).map((c) => c.textContent);
    expect(greyed).toEqual(['owner']);
  });

  it('gives through the service, naming source, recipient, relation and an end no later than the source', async () => {
    const { posted } = await open();
    await choose($('#dTo'), SCRIBE);
    await click($('[data-act="delegatedo"]'));
    const body = posted.find((p) => p.path === '/grants')?.body as DelegateBody;
    expect(body.source).toBe(ROOT_G);
    expect(body.recipient).toBe(SCRIBE);
    expect(body.responsible).toBe(ADA);
    expect(body.relation).toBe('viewer');
    expect(body.pass_on).toEqual({ kind: 'use_only' });
    expect(body.route).toBe('browser');
    expect(body.operation).toMatch(/^op-[0-9a-f]{32}$/);
    expect(body.window.ends_at).not.toBeNull();
    expect($('#drawer')?.classList.contains('open')).toBe(false);
    expect($('#toast')?.textContent).toContain("Given. Scribe can now view project:identity, through you.");
  });

  it('shows the service refusal by name and records nothing as given', async () => {
    await open({ ...SERVICE, 'POST /grants': refused(409, 'RecipientRefused', `RecipientRefused: ${ROOT_G} may not be passed on to a agent`) });
    await click($('[data-act="delegatedo"]'));
    expect($('#dAnswer b')?.textContent).toBe('RecipientRefused');
    expect($('#drawer')?.classList.contains('open')).toBe(true);
    expect($('#toast')?.textContent).not.toContain('Given');
  });

  it('keeps an unconfirmed gift pending and retries it under the same operation', async () => {
    let calls = 0;
    const { posted } = await open({
      ...SERVICE,
      'POST /grants': () => (++calls === 1 ? refused(503, 'AppendUncertain', 'AppendUncertain: not known') : ok({ operation: 'x', grant: 'g', index: 1, receipt: {} })),
    });
    await click($('[data-act="delegatedo"]'));
    expect($('#dAnswer b')?.textContent).toBe('pending');
    await click($('[data-act="delegatedo"]'));
    const ops = posted.filter((p) => p.path === '/grants').map((p) => (p.body as DelegateBody).operation);
    expect(ops).toHaveLength(2);
    expect(ops[0]).toBe(ops[1]);
    expect($('#toast')?.textContent).toContain('Given.');
  });

  it('Escape closes the drawer and focus returns', async () => {
    await mount('#/me');
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
  it('lists each with its reason', async () => {
    await mount('#/me');
    await click($(`[data-act="delegate"][data-g="${ROOT_G}"]`));
    const reasons = $$('#drawer .why-not').map((d) => [d.querySelector('b')?.textContent, d.querySelector('.note')?.textContent]);
    expect(reasons).toEqual([
      ['viewer of project:ledger', `You may use it; G/${LEDGER_G.slice(6, 14)} does not let you pass it on.`],
      ['owner of project:identity', 'More than you hold.'],
      ['Service accounts', 'not built yet The directory records no service accounts yet, so none is listed here.'],
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
    expect($('#answer .verdict-mark')?.textContent).toBe('Yes');
    expect($('#answer .why')?.textContent).toBe('view needs viewers or owners.');
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
    expect($('#answer .meta-line')?.textContent).toContain('model not named in a refusal');
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
    expect(rows).toEqual(['Ada (test person)edit, grant, view', "Scribeview"]);
    await choose($('select[aria-label="Resource"]'), 'project:ledger');
    expect(location.hash).toBe('#/access/who/project:ledger');
    expect($$('#whoCan .row').map((r) => r.textContent)).toEqual(['Ada (test person)view']);
  });

  it('lists every grant, with where it derives from, and keeps every control reachable', async () => {
    await mount('#/access/reach/' + SCRIBE);
    expect($$('.check .card tr').map((r) => r.textContent)).toEqual(['project:identitybuilt inview']);
    const grants = $$('table tbody tr[data-href]').map((r) => r.querySelectorAll('td')[4].textContent);
    expect(grants).toEqual(['root', 'root', `G/${ROOT_G.slice(6, 14)} · Ada (test person)`]);
    expect(text()).toContain('not reported');
    expect(unreachable()).toEqual([]);
    expect(SCRIBE_G).toMatch(/^grant-/);
  });
});

describe('A grant card', () => {
  it('shows a void grant of a suspended holder with the open question, and no Allows line', async () => {
    const suspend = (v: typeof DIRECTORY) => ({ ...v, people: v.people.map((p) => ({ ...p, agents: p.agents.map((a) => (a.id === SCRIBE ? { ...a, state: 'suspended' as const } : a)) })) });
    await mount(`#/file/${SCRIBE}/access`, { ...SERVICE, '/directory/people': ok(suspend(DIRECTORY)), '/people': ok(suspend({ ...DIRECTORY, scope: 'personal', people: [DIRECTORY.people[0]] })) });
    const card = $$('.file .card').find((c) => c.querySelector('.verdict-mark.no'));
    expect(card?.textContent).toContain('Scribe is suspended what suspension refuses: open');
    expect(text()).not.toContain('Allows:');
  });
});
