/**
 * DIRECTORY-006 R6, GRANT_CONFORMANCE: the You and delegation screens against the
 * accepted mock-up's numbered conformance rows.
 *
 * **Runner: vitest + jsdom, not a browser.** No browser runner is approved for
 * this surface and none was added, so every check here is a jsdom check driven
 * through the same harness the unit tests use. Say which axis the independence
 * is on: this is independence of *statement* — the conformance rows are written
 * down here, numbered, and each one has to name a check that ran — not
 * independence of *platform*. One machine, one toolchain, one dependency
 * resolution, and a DOM implementation that is not a browser's.
 *
 * Two consequences of that are load-bearing, and are stated rather than hidden:
 *
 * - jsdom does not perform a user agent's default activation of a native
 *   `<button>` when Enter is pressed on it. Every step the screens implement
 *   themselves is driven by keyboard events below; the two native-button
 *   activations are clicks, and the keyboard check asserts instead that the
 *   shell's key registry yields Enter to a focused button rather than acting on
 *   it (`src/shell/keys.ts`), which is the part this codebase owns.
 * - Visual similarity is not checked and is not claimed. What is pinned is the
 *   mock-up *file*, by its SHA-256: if the accepted mock-up changes, this test
 *   fails and the rows below have to be read against it again.
 */
import { beforeEach, describe, expect, it } from 'vitest';
import { $, $$, choose, click, mount, press, text, unmountAll, unreachable, settle } from '../harness';
import { BEA, BEA_DIRECTORY, BEA_GRANTS, BEA_ROOT_G, BEA_SERVICE, GRANTS, ROOT_G, SCRIBE, SERVICE, ok, refused } from '../fixtures';

beforeEach(() => sessionStorage.clear());

/**
 * `node:fs` and `node:crypto`, typed here rather than from `@types/node`: this
 * surface has no node type package and no npm dependency was added for an
 * acceptance test. The specifier is a variable so the type checker does not try
 * to resolve a module it has no declarations for; the runner is node, which
 * resolves it. Only the three calls this file makes are declared.
 */
interface NodeFs {
  readFileSync(path: string): Uint8Array;
  existsSync(path: string): boolean;
}
interface NodeHash {
  update(data: Uint8Array): NodeHash;
  digest(encoding: 'hex'): string;
}
interface NodeCrypto {
  createHash(algorithm: 'sha256'): NodeHash;
}
const builtin = async <T>(specifier: string): Promise<T> => (await import(/* @vite-ignore */ specifier)) as T;

/** The path the runner was started from. */
const cwd = (): string => (globalThis as { process?: { cwd(): string } }).process?.cwd() ?? '';

/**
 * The nearest enclosing directory holding `relative`. Under jsdom
 * `import.meta.url` is an http URL, so the repository is found by walking up
 * from the working directory, which makes the path independent of where the
 * runner was started. The walk ends at the filesystem root, by a named refusal.
 */
function findUp(fs: NodeFs, relative: string): string {
  let dir = cwd();
  for (;;) {
    const candidate = dir + '/' + relative;
    if (fs.existsSync(candidate)) return candidate;
    const up = dir.slice(0, dir.lastIndexOf('/'));
    if (up === dir || up === '') throw new Error(`no ${relative} in any directory above ${cwd()}`);
    dir = up;
  }
}

/** The accepted mock-up's SHA-256. A different mock-up is a different acceptance. */
const MOCKUP_SHA256 = 'e67622e19f450e2335029f293f18b4757801f3245758a8b667521afa120ab91f';

/** Start the next mount of a test from an empty page, as a new visit would. */
const fresh = () => {
  unmountAll();
  document.body.innerHTML = '';
  sessionStorage.clear();
};

const holdRows = () =>
  [...($$('.grid2 > div:first-child table')[0]?.querySelectorAll('tbody tr') ?? [])].map((tr) =>
    { const [first, ...rest] = [...tr.querySelectorAll('td')]; return [first.firstChild?.textContent, ...[...first.querySelectorAll('.mono')].map((span) => span.textContent), ...rest.map((td) => td.textContent)]; });

/** The administrator's wider view: both people, and the grants of both. */
const ADMIN: Record<string, (typeof SERVICE)[string]> = {
  ...SERVICE,
  '/directory/people': ok(BEA_DIRECTORY),
  '/grants': ok({ grants: [...GRANTS, ...BEA_GRANTS], revision: 7 }),
};

describe('The accepted mock-up', () => {
  it('is the file this acceptance was written against, pinned by its SHA-256', async () => {
    const fs = await builtin<NodeFs>('node:fs');
    const crypto = await builtin<NodeCrypto>('node:crypto');
    const path = findUp(fs, 'docs/design/identity/mockup/index.v5.html');
    const bytes = fs.readFileSync(path);
    expect(bytes.byteLength).toBeGreaterThan(0);
    expect(crypto.createHash('sha256').update(bytes).digest('hex')).toBe(MOCKUP_SHA256);
  });
});

/** Which numbered rows a check actually ran for. Counted, not assumed. */
const exercised = new Set<string>();

interface ConformanceRow {
  /** The row's number in docs/design/identity/CONFORMANCE.md. */
  row: string;
  /** The rule, in the row's own words, shortened. */
  rule: string;
  check: () => Promise<void>;
}

const ROWS: ConformanceRow[] = [
  {
    row: '1.3',
    rule: 'service accounts a person may use are a separate list, permission checked separately',
    check: async () => {
      await mount('#/me?tab=account');
      const accounts = $('#service-accounts');
      expect(accounts).not.toBeNull();
      expect(accounts?.textContent).toContain('Permission to use or lend their credentials is checked separately');
      // Read from the service, never from sample data: the empty answer says so.
      expect(accounts?.textContent).toContain('No service-account records were returned for you');
      // A separate list: no grant of the hold table is inside it.
      expect(accounts?.querySelector('[data-act="delegate"]')).toBeNull();
      await click($(`[data-act="delegate"][data-g="${ROOT_G}"]`));
      // What cannot be given is the service's answer, and it lists no service account.
      const cannot = $$('.act-panel .why-not').map((d) => d.querySelector('b')?.textContent);
      expect(cannot).toEqual(['viewer of project:ledger', 'Your sign-in identities']);
    },
  },
  {
    row: '1.4',
    rule: '"What you hold" shows each grant, its source, and whether it may be passed on',
    check: async () => {
      const { requests } = await mount('#/me?tab=account');
      expect(requests).toContain('/grants');
      expect(holdRows()).toEqual([
        ['You can do everything here (project identity).', 'owner', 'project:identity', 'root', 'yes', 'Give to an agent…'],
        ['View this resource (project ledger).', 'viewer', 'project:ledger', 'root', 'no', ''],
      ]);
    },
  },
  {
    row: '1.5',
    rule: 'personal scope for the signed-in person; an administrator may lawfully see others',
    check: async () => {
      await mount('#/me?tab=account', BEA_SERVICE);
      expect($('h1')?.textContent).toBe('Bea (test person)');
      expect(text()).not.toContain('Ada (test person)');
      expect(document.body.innerHTML).not.toContain(ROOT_G);
      fresh();
      const { requests } = await mount(`#/file/${BEA}/access`, ADMIN);
      expect(requests).toContain('/directory/people');
      expect(requests).not.toContain('/people');
      expect(document.body.innerHTML).toContain(BEA_ROOT_G);
      expect(text()).toContain('editor of project:ledger');
    },
  },
  {
    row: '2.3',
    rule: 'the form shows the source grant, its actions, may-pass-on, and that it ends no later than its source',
    check: async () => {
      await mount('#/me?tab=account');
      await click($(`[data-act="delegate"][data-g="${ROOT_G}"]`));
      const source = $$('.act-panel .card')[0].textContent ?? '';
      expect(source).toContain('Source grant');
      expect(source).toContain('owner of project:identity');
      expect(source).toContain('Actions it allowseverything here');
      expect(source).toContain('You may pass it onagent');
      expect(source).toContain('Ends no later than27 Oct');
    },
  },
  {
    row: '2.4',
    rule: 'everything the person cannot give is listed with its reason',
    check: async () => {
      await mount('#/me?tab=account');
      await click($(`[data-act="delegate"][data-g="${ROOT_G}"]`));
      const reasons = $$('.act-panel .why-not').map((d) => [d.querySelector('b')?.textContent, d.querySelector('.note')?.textContent]);
      expect(reasons).toEqual([
        ['viewer of project:ledger', 'You may use it; it does not let you pass it on.'],
        ['Your sign-in identities', 'They prove who you are. No agent can hold them.'],
      ]);
    },
  },
  {
    row: '8.1',
    rule: '"Can X do this?" answers yes with the path to a person, or no with the named reason and the model version',
    check: async () => {
      const { posted } = await mount(`#/file/${SCRIBE}/access`);
      await choose($('#cPerm'), 'view');
      await click($('[data-act="check"]'));
      expect(posted.some((p) => p.path === '/grants/who')).toBe(true);
      expect($('#answer .verdict-mark')?.textContent).toBe('Yes');
      expect($('#answer .chain')?.textContent).toContain('Ada (test person) · owner of project:identity');
      expect($('#answer .meta-line')?.textContent).toContain('model v1');
      await choose($('#cPerm'), 'edit');
      await click($('[data-act="check"]'));
      expect($('#answer .verdict-mark')?.textContent).toBe('No');
      expect($('#answer .tag')?.textContent).toBe('no grant');
      expect($('#answer .meta-line')?.textContent).toContain('model v3');
    },
  },
  {
    row: '8.4',
    rule: 'every grant shows last used, its source and window; "not seen" is never shown as "never used"',
    check: async () => {
      await mount(`#/file/${SCRIBE}/access`);
      const card = $$('.file .card').filter((c) => (c.textContent ?? '').includes('Passable:'));
      expect(card).toHaveLength(1);
      expect(card[0].textContent).toContain('Last used: 27 Sep 12:00 · tool');
      expect(card[0].textContent).toContain('Window: 27 Sep to 4 Oct');
      // Its source, as the chain from the person who issued it.
      expect(card[0].querySelector('.chain')?.textContent).toBe('Ada (test person) · owner of project:identity→Scribe · viewer of project:identity');
      expect(text()).not.toContain('never used');
      fresh();
      // A grant with no observed use reads "not seen", never "never used".
      await mount('#/me?tab=account', ADMIN);
      fresh();
      await mount(`#/file/${BEA}/access`, ADMIN);
      expect(text()).toContain('Last used: not seen');
      expect(text()).not.toContain('never used');
    },
  },
];

describe('Numbered conformance rows (docs/design/identity/CONFORMANCE.md)', () => {
  it.each(ROWS)('$row — $rule', async (r) => {
    await r.check();
    exercised.add(r.row);
  });

  it('exercised every row R6 cites, each through a check that ran', () => {
    const cited = ['1.3', '1.4', '1.5', '2.3', '2.4', '8.1', '8.4'];
    expect(ROWS.map((r) => r.row)).toEqual(cited);
    expect([...exercised].sort()).toEqual(cited);
    expect(exercised.size).toBe(7);
  });
});

describe('Keyboard operation (conformance 9.3)', () => {
  it('reaches You by its key registry, picks a relation with Enter and Space, and Escape closes and returns focus', async () => {
    await mount('#/people');
    await press('g', {}, document.body);
    await press('u', {}, document.body);
    expect(location.hash).toBe('#/me');
    location.hash = '#/me?tab=account';
    await settle();

    const give = $(`[data-act="delegate"][data-g="${ROOT_G}"]`);
    expect(give?.tagName).toBe('BUTTON');
    give?.focus();
    // The shell yields Enter to a focused button (src/shell/keys.ts): it neither
    // opens the drawer itself nor navigates a cursor row in its place.
    await press('Enter');
    expect($('.act-panel')).toBeNull();
    expect(location.hash).toBe('#/me?tab=account');

    // jsdom does not perform the user agent's default activation of a button.
    await click(give);
    expect($('.act-panel')).not.toBeNull();
    expect(document.activeElement?.id).toBe('dTo');
    expect(unreachable()).toEqual([]);

    // An agent is offered one native checkbox per action, none ticked, so the keyboard reaches each and Space ticks it.
    const boxes = () => $$('.act-panel input[name="action"]') as HTMLInputElement[];
    const offered = boxes().map((box) => box.value);
    expect(offered).toContain('view');
    expect(boxes().filter((box) => box.checked)).toEqual([]);
    await click($('.act-panel input[name="action"][value="view"]'));
    expect(boxes().filter((box) => box.checked).map((box) => box.value)).toEqual(['view']);

    // The whole form is in the tab order, in the order it reads.
    const order = $$('.act-panel select, .act-panel input[name="action"], .act-panel button').map((el) => el.id || (el as HTMLInputElement).value || el.dataset.act);
    expect(order).toEqual(['dTo', ...offered, 'dLease', 'dPass', 'delegatedo', 'close']);
    for (const el of $$('.act-panel select, .act-panel input[name="action"], .act-panel button')) expect(el.tabIndex).toBeGreaterThanOrEqual(0);

    const submit = $('[data-act="delegatedo"]');
    expect(submit?.tagName).toBe('BUTTON');
    expect((submit as HTMLButtonElement).disabled).toBe(false);
    await click(submit);
    expect($('#toast')?.textContent).toContain('Given.');

    // Escape closes the form and focus returns to what opened it.
    fresh();
    await mount('#/me?tab=account');
    const again = $(`[data-act="delegate"][data-g="${ROOT_G}"]`);
    again?.focus();
    await click(again);
    await press('Escape');
    expect($('.act-panel')).toBeNull();
    expect(document.activeElement).toBe($(`[data-act="delegate"][data-g="${ROOT_G}"]`));
  });
});

describe('Deep linking (conformance 9.1)', () => {
  it('opens at a deep link, reads that identity, and carries its delegation target into the form', async () => {
    const { requests } = await mount(`#/file/${SCRIBE}/access`);
    expect(location.hash).toBe(`#/file/${SCRIBE}/access`);
    expect(requests).toContain('/directory/agents/' + SCRIBE);
    expect(text()).toContain('Scribe');
    await click($('[data-act="grant"]'));
    expect($('.act-panel')).not.toBeNull();
    expect(($('#dTo') as HTMLSelectElement).value).toBe(SCRIBE);

    fresh();
    await mount(`#/access/reach/${SCRIBE}`);
    expect(location.hash).toBe(`#/access/reach/${SCRIBE}`);
    expect(text()).toContain('project identity');

    fresh();
    // A deep link to a record the caller may not see is refused by name, not shown empty.
    const closed = { ...ADMIN, '/directory/people': refused(403, 'NotAdmitted', 'the wider directory is for its administrators') };
    const { posted } = await mount(`#/file/${BEA}/access`, closed);
    expect($('.why-not .refusal-name')?.textContent).toContain('PersonNotVisible');
    expect(document.body.innerHTML).not.toContain(BEA_ROOT_G);
    expect(posted).toEqual([]);
  });
});

describe('API refusal parity', () => {
  it('shows the refusal body the API answered, word for word, and nothing else', async () => {
    const reason = `RecipientRefused: ${ROOT_G} may not be passed on to a agent`;
    // `ServerError::name()`: the refusal's name is the first word of its message.
    const name = reason.split(':')[0];
    const { posted } = await mount('#/me?tab=account', { ...SERVICE, 'POST /grants': refused(403, name, reason) });
    await click($(`[data-act="delegate"][data-g="${ROOT_G}"]`));
    await click($('.act-panel input[name="action"][value="view"]'));
    await click($('[data-act="delegatedo"]'));
    expect(posted).toHaveLength(1);
    expect($('#dAnswer b')?.textContent).toBe(name);
    expect($('#dAnswer .note')?.textContent).toBe(reason);
    // Nothing added, reworded or dropped between the body and the screen.
    expect($('#dAnswer')?.textContent).toBe(name + reason);
    expect($('#toast')?.textContent ?? '').not.toContain('Given');
  });
});
