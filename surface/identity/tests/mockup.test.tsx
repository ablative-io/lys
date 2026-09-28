import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';

// The mock-up's two fixes, proved in the mock-up itself: index.v6.html loaded into
// its own jsdom window with its scripts running, each case checked there and shown
// failing on index.v5.html by the same steps. The independence claimed is of the
// mock-up against the built shell, on one toolchain, not of browser platform.

type Win = Window & typeof globalThis;
type Version = 'v5' | 'v6';
type Check = [string, boolean];

interface JsdomModule {
  JSDOM: {
    fromFile(
      path: string,
      options: { url: string; runScripts: 'dangerously'; pretendToBeVisual: boolean; beforeParse: (window: Win) => void },
    ): Promise<{ window: Win }>;
  };
}

const opened: Win[] = [];

afterEach(() => {
  for (const win of opened.splice(0)) win.close();
});

const file = (version: Version) =>
  decodeURIComponent(new URL(`../../../docs/design/identity/mockup/index.${version}.html`, import.meta.url).pathname);

// jsdom lays nothing out: every element gets a box on screen, so explain mode
// places its numbers, and scrollIntoView is a no-op.
const BOX = { x: 100, y: 100, left: 100, top: 100, right: 180, bottom: 120, width: 80, height: 20, toJSON: () => ({}) } as DOMRect;

async function load(version: Version, hash: string): Promise<Win> {
  const name = 'jsdom';
  const { JSDOM } = (await import(/* @vite-ignore */ name)) as JsdomModule;
  const dom = await JSDOM.fromFile(file(version), {
    url: 'http://mockup.test/' + hash,
    runScripts: 'dangerously',
    pretendToBeVisual: true,
    beforeParse(win) {
      win.Element.prototype.getBoundingClientRect = () => BOX;
      win.Element.prototype.scrollIntoView = () => undefined;
    },
  });
  opened.push(dom.window);
  await tick();
  return dom.window;
}

/** Let queued tasks run: mutation observers, hashchange events, re-renders. */
async function tick(): Promise<void> {
  for (let i = 0; i < 6; i += 1) await new Promise<void>((resolve) => setTimeout(resolve, 0));
}

async function key(win: Win, target: Element, k: string, init: KeyboardEventInit = {}): Promise<void> {
  target.dispatchEvent(new win.KeyboardEvent('keydown', { key: k, bubbles: true, cancelable: true, ...init }));
  await tick();
}

/** Every hashchange's newURL, in order, from now on. */
function navigations(win: Win): string[] {
  const urls: string[] = [];
  win.addEventListener('hashchange', (e) => urls.push(e.newURL));
  return urls;
}

function need<T>(value: T | null | undefined, what: string): T {
  if (value === null || value === undefined) throw new Error('setup: ' + what);
  return value;
}

const peopleRows = (win: Win) => [...win.document.querySelectorAll<HTMLElement>('#screen tr[data-href]')];

/** The open palette's Go to rows, in order. */
function goToRows(win: Win): HTMLElement[] {
  const rows: HTMLElement[] = [];
  let group = '';
  for (const el of win.document.querySelectorAll<HTMLElement>('#palList > *')) {
    if (el.classList.contains('grp')) group = el.textContent ?? '';
    else if (group === 'Go to' && el.matches('.it[data-n]')) rows.push(el);
  }
  return rows;
}

async function openPalette(win: Win): Promise<HTMLInputElement> {
  await key(win, win.document.body, 'k', { metaKey: true });
  const input = need(win.document.querySelector<HTMLInputElement>('#palIn'), 'palette input');
  if (!win.document.querySelector('#palette')?.classList.contains('open')) throw new Error('setup: palette did not open');
  if (win.document.activeElement !== input) throw new Error('setup: palette input not focused');
  return input;
}

const CASES: [string, (version: Version) => Promise<Check[]>][] = [
  ['case 1 focus return', async (version) => {
    const win = await load(version, '#/people');
    const row = need(peopleRows(win)[2], 'third People row');
    row.focus();
    if (win.document.activeElement !== row) throw new Error('setup: third row not focused');
    await key(win, row, '?');
    if (!win.document.querySelector('#xlayer')) throw new Error('setup: overlay did not open');
    await key(win, need(win.document.activeElement, 'focus in the overlay'), 'Escape');
    return [
      ['overlay closed', win.document.querySelector('#xlayer') === null],
      ['focus back on the third row', win.document.activeElement === row],
    ];
  }],
  ['case 2 network node', async (version) => {
    const win = await load(version, '#/network');
    const nodes = [...win.document.querySelectorAll<SVGGElement>('g.gn[data-act="node"]')];
    if (!nodes.length) throw new Error('setup: no network map nodes');
    const first = nodes[0];
    first.focus();
    const reachable = nodes.every((n) => n.tabIndex === 0 && n.getAttribute('role') === 'button');
    const focused = win.document.activeElement === first;
    await key(win, first, 'Enter');
    return [
      [`every one of ${nodes.length} nodes has tabIndex 0 and role button`, reachable],
      ['the first node takes focus', focused],
      ['Enter opens its drawer', Boolean(win.document.querySelector('#drawer')?.classList.contains('open'))],
    ];
  }],
  ['case 3 palette Tab', async (version) => {
    const win = await load(version, '#/people');
    await openPalette(win);
    const first = need(win.document.querySelector('#palList .it[data-n]'), 'a palette row');
    await userEvent.setup({ document: win.document }).tab();
    await tick();
    return [['Tab from the input reaches the first row', win.document.activeElement === first]];
  }],
  ['case 4 palette Enter', async (version) => {
    const win = await load(version, '#/access');
    const input = await openPalette(win);
    const total = win.document.querySelectorAll('#palList .it[data-n]').length;
    for (let n = 0; n < total && !goToRows(win)[1]?.classList.contains('sel'); n += 1) await key(win, input, 'ArrowDown');
    const [first, second] = goToRows(win);
    if (!second?.classList.contains('sel')) throw new Error('setup: second Go to row never selected');
    if (first.lastElementChild?.textContent !== '#/people' || second.lastElementChild?.textContent !== '#/roles') {
      throw new Error('setup: Go to rows are not People then Roles');
    }
    if (!first.hasAttribute('tabindex')) first.tabIndex = -1;
    first.focus();
    if (win.document.activeElement !== first) throw new Error('setup: first Go to row not focused');
    const urls = navigations(win);
    await key(win, first, 'Enter');
    return [
      ['went to the focused row, #/people', urls.some((u) => u.endsWith('#/people'))],
      ['never went to the selected row, #/roles', !urls.some((u) => u.endsWith('#/roles'))],
    ];
  }],
  ['case 5 j and k', async (version) => {
    const win = await load(version, '#/people');
    const rows = peopleRows(win);
    if (rows.length < 3) throw new Error('setup: fewer than three People rows');
    const screenChild = win.document.querySelector('#screen')?.firstElementChild;
    rows[0].focus();
    if (win.document.activeElement !== rows[0]) throw new Error('setup: first row not focused');
    await key(win, rows[0], 'j');
    await key(win, win.document.activeElement ?? win.document.body, 'j');
    return [
      ['focus on the third row', win.document.activeElement === rows[2]],
      ['the third row carries the cursor', rows[2].classList.contains('cursor')],
      ['the screen kept its elements', win.document.querySelector('#screen')?.firstElementChild === screenChild],
    ];
  }],
  ['case 6 Enter on a focused row', async (version) => {
    const win = await load(version, '#/people');
    const rows = peopleRows(win);
    if (rows.length < 2 || !rows[0].classList.contains('cursor')) throw new Error('setup: cursor not on the first of two rows');
    const firstHref = need(rows[0].dataset.href, 'first row href');
    const secondHref = need(rows[1].dataset.href, 'second row href');
    rows[1].focus();
    if (win.document.activeElement !== rows[1]) throw new Error('setup: second row not focused');
    const urls = navigations(win);
    await key(win, rows[1], 'Enter');
    return [
      ['opened the focused row exactly once', urls.filter((u) => u.endsWith(secondHref)).length === 1],
      ['never opened the cursor row', !urls.some((u) => u.endsWith(firstHref))],
    ];
  }],
];

const failed = (checks: Check[]) => checks.filter(([, passed]) => !passed).map(([what]) => what);

describe('mock-up index.v6.html (conformance 9.2, 9.3)', () => {
  for (const [title, run] of CASES) {
    it(title, async () => {
      const checks = await run('v6');
      expect(checks.length).toBeGreaterThan(0);
      expect(failed(checks)).toEqual([]);
    });
  }

  it('all six cases fail on v5', async () => {
    const failing: string[] = [];
    for (const [title, run] of CASES) {
      if (failed(await run('v5')).length > 0) failing.push(title);
    }
    expect(CASES).toHaveLength(6);
    expect(failing).toEqual(CASES.map(([title]) => title));
    expect(failing).toHaveLength(6);
  });
});
