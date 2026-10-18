import { beforeEach, describe, expect, it } from 'vitest';
import { $, $$, choose, click, mount, text, unmountAll, unreachable, settle } from './harness';
import { ADA, BEA, BEA_DIRECTORY, BEA_GRANTS, BEA_REVIEWER_G, BEA_ROOT_G, BEA_SERVICE as BEA_BASE, DIRECTORY, GRANTS, LEDGER_G, AGENT_MODEL as MODEL, ME, REVIEWER, ROOT_G, SCRIBE, SCRIBE_G, SERVICE as BASE, ok, refused } from './fixtures';
import type { DelegateBody, LastUse } from '../src/generated/grants';

beforeEach(() => sessionStorage.clear());

const SERVICE: typeof BASE = { ...BASE, '/grants/model': ok(MODEL) };
const BEA_SERVICE: typeof BEA_BASE = { ...BEA_BASE, '/grants/model': ok(MODEL) };

const holdRows = () => document.querySelectorAll('table[aria-label="Grants"] tbody tr[data-grant]');
const cell = (tr: Element, col: string) => tr.querySelector('td[data-col="' + col + '"]')?.textContent ?? '';

describe('What you hold', () => {
  it('row_1_4_what_you_hold_shows_each_grant_its_source_and_whether_it_may_be_passed_on', async () => {
    const { requests } = await mount('#/me?tab=account');
    expect(requests).toContain('/grants');
    const rows = holdText();
    expect(rows).toEqual([
      ['Everything here', 'owner', 'project:identity', 'root', 'yes', 'Give'],
      ['View this resource', 'viewer', 'project:ledger', 'root', 'no', ''],
    ]);
    expect(unreachable()).toEqual([]);
  });
});

describe('Who can reach this? (conformance 8.2)', () => {
  it('lists everyone with what they can do, from the same answers', async () => {
    await mount('#/access/who/project:identity');
    const rows = $$('#whoCan .row').map((r) => r.textContent);
    expect(rows).toEqual(['Ada (test person)everything hereoutright', 'ScribeView this resourceoutright']);
    await choose($('select[aria-label="Resource"]'), 'project:ledger');
    expect(location.hash).toBe('#/access/who/project:ledger');
    expect($$('#whoCan .row').map((r) => r.textContent)).toEqual(['Ada (test person)View this resourceoutright']);
  });

  it('lists every grant, with where it derives from, and keeps every control reachable', async () => {
    await mount('#/access/reach/' + SCRIBE);
    expect($$('.check .card tr').map((r) => r.textContent)).toEqual(['project identityView this resourceoutright']);
    // The grants themselves are the Grants tab of the same page.
    location.hash = '#/access';
    await settle();
    const grants = $$('table tbody tr[data-href]').map((r) => [...r.querySelectorAll('.chain .pill')].map((pill) => pill.textContent));
    expect(grants).toEqual([['Ada (test person)'], ['Ada (test person)'], ['Ada (test person)', 'Scribe']]);
    expect($$('table tbody tr[data-href] .chain').map((chain) => chain.getAttribute('title'))).toEqual([
      'Ada (test person), P/00000000 · owner of project:identity',
      'Ada (test person), P/00000000 · viewer of project:ledger',
      'Ada (test person), P/00000000 · owner of project:identity → Scribe, agent of Ada (test person), A/00000000 · viewer of project:identity',
    ]);
    const used = $$('table tbody tr[data-href]').map((r) => r.querySelectorAll('td')[8].textContent);
    expect(used).toEqual(['not seen', 'not seen', '27 Sep 12:00 · tool']);
    expect(unreachable()).toEqual([]);
    expect(SCRIBE_G).toMatch(/^grant-/);
  });
});

describe('A grant card', () => {
  it('shows a void grant of a suspended holder with its reason and no design note, and no Allows line', async () => {
    const suspend = (v: typeof DIRECTORY) => ({ ...v, people: v.people.map((p) => ({ ...p, agents: p.agents.map((a) => (a.id === SCRIBE ? { ...a, state: 'suspended' as const } : a)) })) });
    const reason = `IdentityNotActive: ${SCRIBE} is suspended, and only an active identity's grants are effective`;
    const refusedScribe = GRANTS.map((g) => (g.id === SCRIBE_G ? { ...g, standing: { stands: false as const, refusal: 'IdentityNotActive', grant: null, reason } } : g));
    await mount(`#/file/${SCRIBE}/access`, { ...SERVICE, '/grants': ok({ grants: refusedScribe, revision: 7 }), '/directory/people': ok(suspend(DIRECTORY)), '/people': ok(suspend({ ...DIRECTORY, scope: 'personal', people: [DIRECTORY.people[0]] })) });
    const card = $$('tr[data-grant]').find((c) => c.querySelector('.verdict-mark.no'));
    // The service's reason, with the raw identifier it names put as the name a person reads.
    expect(card?.textContent).toContain("IdentityNotActive: Scribe is suspended, and only an active identity's grants are effective");
    expect(card?.textContent).not.toContain(SCRIBE);
    expect(card?.textContent).not.toContain('what suspension refuses');
    expect(text()).not.toContain('Allows:');
    expect(card?.textContent).toContain('27 Sep 12:00 · tool');
  });
});

describe('A grant card, its use and its window', () => {
  it('row_8_4_a_grant_card_shows_last_used_its_source_and_window_and_never_never_used', async () => {
    const unseen = GRANTS.map((g) => (g.id === SCRIBE_G ? { ...g, last_use: { seen: false as const, recorded: 0, source: 'reported' as const } } : g));
    await mount(`#/file/${SCRIBE}/access`, { ...SERVICE, '/grants': ok({ grants: unseen, revision: 7 }) });
    const card = $$('tr[data-grant]').find((c) => c.querySelector('.chain'));
    expect(card?.textContent).toContain('not seen');
    expect([...(card?.querySelectorAll('.chain .pill') ?? [])].map((p) => p.textContent)).toEqual(['Ada (test person)', 'Scribe']);
    expect(card?.querySelector('.chain')?.getAttribute('title')).toBe('Ada (test person), P/00000000 · owner of project:identity → Scribe, agent of Ada (test person), A/00000000 · viewer of project:identity');
    expect(card?.textContent).toContain('27 Sep to 4 Oct');
    expect(card?.textContent).not.toContain('never used');
  });
});

describe('A grant that ends with a role assignment (conformance 4.5)', () => {
  const ROLE_END = 1_760_000_000;
  const routes = {
    ...SERVICE,
    '/roles': ok({
      roles: [
        {
          id: 'role-0000000000000000000000000000000000000000000000000000000000000001',
          name: 'Scribe',
          latest: 1,
          policy: 'stays_until_moved',
          versions: [],
          holders: [{ assignment: 'op-00000000000000000000000000000001', holder: SCRIBE, display_name: 'Scribe', version: 1, behind: false, assigned_by: ADA, assigned_at: 1_759_000_000, ends_at: ROLE_END, moves_at: null, state: 'holding', moves: [], ended_by: null, ended_at: null }],
        },
      ],
    }),
  };

  it('offers each dated holding of the recipient as an end, and posts that end', async () => {
    const { posted } = await mount('#/me?tab=account', routes);
    await click($(`[data-act="delegate"][data-g="${ROOT_G}"]`));
    await choose($('#dTo'), SCRIBE);
    expect($$('#dLease option').map((o) => o.textContent)).toEqual(['7 days', 'Ends with the Scribe assignment (9 Oct)', 'When your own access ends']);
    await choose($('#dLease'), 'assignment:op-00000000000000000000000000000001');
    await click($('.act-panel input[name="action"][value="view"]'));
    await click($('[data-act="delegatedo"]'));
    const body = posted.find((p) => p.path === '/grants')?.body as DelegateBody;
    expect(body.window.ends_at).toBe(ROLE_END);
  });

  it('greys the choice, saying why, when the recipient holds no dated role', async () => {
    await mount('#/me?tab=account');
    await click($(`[data-act="delegate"][data-g="${ROOT_G}"]`));
    const option = $$('#dLease option')[1];
    expect(option?.hasAttribute('disabled')).toBe(true);
    expect(option?.textContent).toBe('Ends with a role assignment (none with an end date)');
  });
});

/** The hold table as text, one array per row. */
const holdText = () =>
  [...holdRows()].map((tr) => [
    cell(tr, 'Allows'),
    cell(tr, 'Relation'),
    tr.querySelector<HTMLElement>('td[data-col="On"] span')?.title ?? '',
    ((pills) => (pills.length === 1 ? 'root' : (pills[pills.length - 2].textContent ?? '').split(' · ')[0]))([...tr.querySelectorAll('.chain .pill')]),
    cell(tr, 'May pass on').includes('agent') ? 'yes' : 'no',
    tr.querySelector('[data-act="delegate"]')?.textContent ?? '',
  ]);

/** Every grant id the page put in the DOM, from the buttons that carry one. */
const grantIdsOnScreen = () => $$('[data-g]').map((el) => el.dataset.act + ':' + el.dataset.g);

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
      ['Everything here', 'owner', 'project:identity', 'root', 'yes', 'Give'],
      ['View this resource', 'viewer', 'project:ledger', 'root', 'no', ''],
    ]);
    expect(grantIdsOnScreen()).toEqual(['delegate:' + ROOT_G, 'revoke:' + ROOT_G, 'revoke:grant-00000000000000000000000000000020']);
    await click($(`[data-act="delegate"][data-g="${ROOT_G}"]`));
    expect($('.act-panel')?.textContent).toContain('Scribe');
    const adaSource = $$('.act-panel .card')[0].textContent ?? '';
    expect(adaSource).toContain('owner of project:identity');
    expect(adaSource).toContain('Actions it allowseverything here');
    expect(adaSource).toContain('You may pass it onagent');
    expect(adaSource).toContain('Ends no later than27 Oct');

    fresh();

    const bea = await mount('#/me?tab=account', BEA_SERVICE);
    // The second session read its own identity for itself; nothing was carried over.
    expect(bea.requests.filter((r) => r === '/me')).toHaveLength(meReads);
    expect($('h1')?.textContent).toBe('Bea (test person)');
    expect(holdText()).toEqual([['Edit this resource; View this resource', 'editor', 'project:ledger', 'root', 'yes', 'Give']]);
    expect(grantIdsOnScreen()).toEqual(['delegate:' + BEA_ROOT_G, 'revoke:' + BEA_ROOT_G]);
    // Her agent, and what it holds under her root grant, not Ada's, on its own Access tab where what an agent holds lives.
    location.hash = '#/dashboard';
    await settle();
    expect(text()).toContain('Reviewer');
    location.hash = '#/file/' + REVIEWER + '/access';
    await settle();
    expect([...document.querySelectorAll('table[aria-label="Grants"] tbody tr[data-grant]')].map((tr) => `${tr.querySelector('td[data-col="Relation"]')?.textContent} of ${tr.querySelector<HTMLElement>('td[data-col="On"] span')?.title}`)).toContain('viewer of project:ledger');
    location.hash = '#/me?tab=account';
    await settle();
    await click($(`[data-act="delegate"][data-g="${BEA_ROOT_G}"]`));
    const beaSource = $$('.act-panel .card')[0].textContent ?? '';
    expect(beaSource).toContain('editor of project:ledger');
    expect(beaSource).toContain('Actions it allowsEdit this resource; View this resource');
    expect(beaSource).toContain('You may pass it onagent');
    expect(beaSource).toContain('Ends no later than15 Nov');

    // Nothing of the first person survived the change of session. Bea's own
    // directory names Ada, but this grant passes on only to agents, so Ada is
    // not offered in To either: a person here is a give the service refuses.
    const offered = $$('#dTo option').map((o) => (o as HTMLOptionElement).value);
    expect(offered).not.toContain(ADA);
    expect(offered).toContain(REVIEWER);
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
    const cards = $$('tr[data-grant]').filter((c) => Boolean(c));
    expect(cards).toHaveLength(1);
    expect(cards[0].querySelector('.chain')?.getAttribute('title')).toContain('editor of project:ledger');
    expect(cards[0].textContent).toContain('agent');
    expect(cards[0].textContent).toContain('27 Sep to 15 Nov');
    expect(cards[0].textContent).toContain('not seen');
    expect(grantIdsOnScreen()).toEqual(['revoke:' + BEA_ROOT_G]);
    // The administrator sees her grants, not her agent's: only what she holds.
    expect(document.body.innerHTML).not.toContain(BEA_REVIEWER_G);
  });

  it('refuses that same route by name when the caller is not a directory administrator', async () => {
    const seen = { ...SERVICE, '/directory/people': refused(403, 'NotAdmitted', 'the wider directory is for its administrators'), '/grants': ok({ grants: [...GRANTS, ...BEA_GRANTS], revision: 7 }) };
    const { requests, posted } = await mount(`#/file/${BEA}/access`, seen);
    // The wider route was asked, refused, and the personal scope answered instead.
    expect(requests.filter((r) => r === '/directory/people').length).toBeGreaterThan(0);
    expect(requests).toContain('/people');
    expect($('.why-not .refusal-name')?.textContent).toContain('PersonNotVisible');
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
  { what: 'use-only source', status: 403, marker: 'UseOnly', retained: false, reason: `UseOnly: ${LEDGER_G} may be exercised and not passed on` },
  { what: 'excessive requested scope', status: 403, marker: 'ActionsOutside', retained: false, reason: 'ActionsOutside: `owner` in model version 3 carries grant, outside the authority held' },
  { what: 'people-only policy', status: 403, marker: 'RecipientRefused', retained: false, reason: `RecipientRefused: ${ROOT_G} may not be passed on to a agent` },
  { what: 'expired ancestor', status: 403, marker: 'Expired', retained: false, reason: `Expired: ${ROOT_G} ended at ${String(GRANTS[0].window.ends_at)}` },
  { what: 'service outage', status: 503, marker: 'pending', retained: true, reason: 'PermissionEngineUnavailable: the permission relationships could not be read' },
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
      await click($('.act-panel input[name="action"][value="view"]'));
      await click($('[data-act="delegatedo"]'));

      // One request, no retry, nothing else posted.
      expect(posted.filter((p) => p.path === '/grants')).toHaveLength(1);
      expect(posted).toHaveLength(1);
      expect(requests.filter((r) => r === 'POST /grants')).toHaveLength(1);
      // The refusal, exactly as the API answered it.
      // The refusal's name is in the Error details fold only; its reason is read with each grant by its short name, never its raw id.
      if (c.marker === 'pending') {
        expect($('#dAnswer[data-pending] b')?.textContent).toBe('Not confirmed yet');
        expect($('#dAnswer')?.textContent).not.toContain(name);
      } else {
        expect($('#dAnswer')?.dataset.refusal).toBe(c.marker);
        expect($('#dAnswer b')?.textContent).not.toContain(c.marker);
        expect($('#dAnswer .refusal-name')?.textContent).toContain(c.marker + ':');
      }
      // Read in words with each grant by its short name; the fold keeps the body word for word.
      if (c.marker === 'pending') expect($('#dAnswer')?.textContent).toContain(c.reason.replace(/^[A-Za-z]+: /, '').replace(/grant-[0-9a-f]{32}/g, (id) => 'G/' + id.slice(6, 14)));
      else expect($('#dAnswer .refusal-name')?.textContent).toBe(c.reason);
      expect($('#dAnswer b')?.textContent ?? '').not.toMatch(/grant-[0-9a-f]{32}/);
      if (c.marker !== 'pending') expect(text()).toContain(name);
      // Nothing given: no re-read of the list, no success, the rows as they were.
      expect(requests.filter((r) => r === '/grants')).toHaveLength(reads);
      expect($('#toast')?.textContent ?? '').not.toContain('Given');
      expect($('.act-panel')).not.toBeNull();
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
  const card = () => $$('tr[data-grant]').filter((c) => Boolean(c));
  const withLastUse = (last_use: LastUse) => GRANTS.map((g) => (g.id === SCRIBE_G ? { ...g, last_use } : g));

  it('counts the uses it could not record beside "not seen", and a reported zero says only "not seen"', async () => {
    const unreported = { count: 1, at: GRANTS[2].window.starts_at, route: 'tool' as const, reason: 'the use log was unavailable' };
    const missing = withLastUse({ seen: false, recorded: 0, source: 'missing', unreported });
    await mount(`#/file/${SCRIBE}/access`, { ...SERVICE, '/grants': ok({ grants: missing, revision: 7 }) });
    expect(card()).toHaveLength(1);
    expect(card()[0].textContent).toContain('not seen · 1 use not recorded');
    expect(text()).not.toContain('never used');

    fresh();

    // A reported zero is a zero, and is never told as a missing one.
    const reported = withLastUse({ seen: false, recorded: 0, source: 'reported' });
    await mount(`#/file/${SCRIBE}/access`, { ...SERVICE, '/grants': ok({ grants: reported, revision: 7 }) });
    expect(card()).toHaveLength(1);
    expect(card()[0].textContent).toContain('not seen');
    expect(text()).not.toContain('not recorded');
    expect(text()).not.toContain('never used');
  });
});

/** A raw identity or grant id, which a person never reads as a label (Tom, 4 Oct 2026). */
const RAW = /\b(?:person|agent|grant)-[0-9a-f]{8,}/;
/** A grant held on a person: the resource is someone, so it is said by their name. */
const ON_ADA_G = 'grant-' + '7'.repeat(32);
const onAda = (routes = SERVICE) => ({ ...routes, '/grants': ok({ grants: [...GRANTS, { ...GRANTS[1], id: ON_ADA_G, resource: { kind: 'person', id: ADA } }], revision: 7 }) });

describe('The grant table says each fact once, by name (walk of 4 Oct)', () => {
  it('Allows carries the actions only; the resource is said in On', async () => {
    await mount('#/me?tab=account');
    for (const tr of holdRows()) {
      expect(cell(tr, 'Allows')).not.toMatch(/identity|ledger/);
      expect(cell(tr, 'On')).toMatch(/identity|ledger/);
    }
  });

  it('the path to a person carries names only, the whole path kept in its title', async () => {
    await mount(`#/file/${SCRIBE}/access`);
    const chain = $('tr[data-grant] .chain');
    expect([...(chain?.querySelectorAll('.pill') ?? [])].map((pill) => pill.textContent)).toEqual(['Ada (test person)', 'Scribe']);
    expect(chain?.textContent).not.toMatch(/ of |:|owner|viewer/);
    expect(chain?.getAttribute('title')).toBe('Ada (test person), P/00000000 · owner of project:identity → Scribe, agent of Ada (test person), A/00000000 · viewer of project:identity');
  });

  it('names a resource that is someone by their name, on the grants page and in every picker, never by a raw id', async () => {
    await mount('#/access', onAda());
    expect(text()).toContain('person Ada (test person)');
    expect(text()).not.toMatch(RAW);
    fresh();
    await mount('#/access/can', onAda());
    expect($$('#cRes option').map((o) => o.textContent)).toContain('Ada (test person) (person)');
    for (const option of $$('#cRes option, #cWho option, #cPerm option')) expect(option.textContent).not.toMatch(RAW);
    expect(text()).not.toMatch(RAW);
    fresh();
    await mount(`#/file/${SCRIBE}/access`, onAda());
    expect($$('#cRes option').map((o) => o.textContent)).toContain('Ada (test person) (person)');
    expect($$('.check .q .ask-field > .note').map((note) => note.textContent)).toEqual(['Who', 'Do what', 'On what']);
    for (const option of $$('#cRes option, #cPerm option')) expect(option.textContent).not.toMatch(RAW);
  });

  it('the Access grants list has its Change column, and a press there opens the form on the page, not the holder', async () => {
    await mount('#/access');
    expect(
      $$('thead th')
        .map((th) => th.textContent)
        .at(-1)
    ).toBe('Change');
    await click($(`[data-act="revoke"][data-g="${ROOT_G}"]`));
    expect(location.hash).toBe('#/access');
    expect($('.act-panel h2')?.textContent).toBe('Revoke owner of project:identity');
    expect(unreachable()).toEqual([]);
  });
});

describe('Ask is one sentence (walk of 4 Oct)', () => {
  it('reads Can [who] [do what] [on what]? [Check], who first, with no separate box to ask about someone else', async () => {
    const { posted } = await mount('#/access/can');
    const line = $('.check .q');
    const fields = [...(line?.querySelectorAll('select, button') ?? [])].map((el) => el.id || el.getAttribute('data-act'));
    expect(fields).toEqual(['cWho', 'cPerm', 'cRes', 'check']);
    expect(line?.textContent).toMatch(/^Can/);
    expect($('input[aria-label="Ask about someone else"]')).toBeNull();
    // Every field carries its visible name, the same on Access > Ask and on an agent's Access tab.
    expect($$('.check .q .ask-field > .note').map((note) => note.textContent)).toEqual(['Who', 'Do what', 'On what']);
    expect($$('.check .q select').every((select) => select.closest('label')?.querySelector('.note'))).toBe(true);
    expect($$('#cWho option')[0].textContent).toBe('Ada (test person)');
    expect($$('#cWho option').map((o) => o.textContent)).toContain('Scribe (agent of Ada (test person))');
    await choose($('#cWho'), SCRIBE);
    await choose($('#cPerm'), 'view');
    await click($('[data-act="check"]'));
    expect(posted.at(-1)?.path).toBe('/grants/who');
    expect($('#answer .verdict-mark')?.textContent).toBe('Yes');
  });

  it('asks what someone can reach in the same sentence, choosing who by name', async () => {
    await mount('#/access/reach/' + SCRIBE);
    expect($('.check .q')?.textContent).toMatch(/^What can.*reach\?$/);
    expect(($('#rWho') as HTMLSelectElement | null)?.value).toBe(SCRIBE);
    await choose($('#rWho'), ADA);
    expect(location.hash).toBe('#/access/reach/' + ADA);
  });
});

describe('Give access is one row (walk of 4 Oct)', () => {
  it('puts every field in one row of the one form, in plain words, gives with Give access, and closes with a plain button', async () => {
    await mount('#/access/issue');
    const close = [...document.querySelectorAll('.head a')].find((a) => a.textContent === 'Close the form');
    expect(close?.classList.contains('primary')).toBe(false);
    const form = $('.act-panel.issue-root form[aria-label="Give access"]');
    expect(form?.querySelectorAll('.card, .act-panel')).toHaveLength(0);
    const fields = [...(form?.querySelector('fieldset')?.children ?? [])].filter((el) => el.classList.contains('field')).map((el) => el.firstChild?.textContent);
    expect(fields).toEqual(['Person', 'Give them access to', 'Which one', 'They may', 'May they pass it on', 'Expires (your local time)', 'More']);
    expect(form?.querySelector('button[type="submit"]')?.textContent).toBe('Give access');
    // None of the grant model's own words reach the person giving access.
    for (const word of ['Holder', 'root grant', 'root authority', 'relation from the permission model']) expect(form?.textContent).not.toContain(word);
  });
});

describe('Reviews shows the chosen grant whole (walk of 4 Oct)', () => {
  const view = { scope: 'personal', revision: 4, judged_at: 1790000000, decisions_recorded: true, unanswered: [], due: [{ agent: { id: SCRIBE, display_name: 'Scribe', state: 'active' }, reviewer: ME.person, last_kept: null, grant: GRANTS[2] }] };

  it('as facts beside the list, never the eleven-column table squeezed into the panel, with Keep access and Revoke', async () => {
    await mount('#/reviews', { ...SERVICE, '/reviews': ok(view) });
    const panel = $('section[aria-label="Grant to review"]');
    expect(panel?.querySelector('table')).toBeNull();
    expect($('table[aria-label="Grants"]')).toBeNull();
    expect([...(panel?.querySelectorAll('dt') ?? [])].map((dt) => dt.textContent)).toEqual(['Holder', 'Allows', 'On', 'Access level', 'Path to a person', 'Window', 'Last used', 'Stands']);
    expect([...(panel?.querySelectorAll('.chain .pill') ?? [])].map((pill) => pill.textContent)).toEqual(['Ada (test person)', 'Scribe']);
    expect(panel?.textContent).toContain('27 Sep to 4 Oct');
    expect([...(panel?.querySelectorAll('button') ?? [])].map((b) => b.getAttribute('aria-label'))).toEqual(['Keep access', 'Revoke']);
    expect([...(panel?.querySelectorAll('button') ?? [])].map((b) => b.textContent)).toEqual(['Keep', 'Revoke']);
    await click($(`section[aria-label="Grant to review"] [data-act="revoke"][data-g="${SCRIBE_G}"]`));
    expect($('.act-panel h2')?.textContent).toBe('Revoke viewer of project:identity');
    expect(unreachable()).toEqual([]);
  });
});
