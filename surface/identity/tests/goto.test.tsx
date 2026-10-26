import { describe, expect, it } from 'vitest';
import { $, $$, click, mount, press, type } from './harness';

// Every screen is one palette entry and one g letter away. The letters and where each goes are written out here
// rather than read from the shell's own tables. The rail is CONSENSUS.md's (18 October 2026): what waits on you is its
// own entry, Jobs and the access map light Access, Settings opens your own account, the computers are Computers, Apps has its own entry, and Running
// agents is the canvas.

const LETTERS: Record<string, [string, string]> = {
  p: ['#/people', 'people'], o: ['#/roles', 'access'], r: ['#/resources', 'access'], a: ['#/access', 'access'], q: ['#/?section=waiting-on-you', 'waiting'], w: ['#/reviews', 'waiting'],
  v: ['#/secrets', 'secrets'], n: ['#/apps', 'apps'], m: ['#/model', 'access'], s: ['#/settings/account', 'settings'],
  h: ['#/graph', 'access'], t: ['#/network', 'network'], u: ['#/me', 'me'], c: ['#/canvas', 'canvas'], d: ['#/dashboard', 'dashboard'],
};
const RAIL_ITEMS = ['dashboard', 'waiting', 'people', 'canvas', 'access', 'network', 'apps', 'secrets', 'settings', 'me'];

/** Every page Search names, written out here rather than read from the palette's own table. */
const PAGE_NAMES: [string, string][] = [
  ['Dashboard', '#/dashboard'], ['Waiting on you', '#/?section=waiting-on-you'], ['Requests', '#/requests'], ['People and agents', '#/people'], ['Running agents', '#/canvas'],
  ['Access', '#/access'], ['Access: ask', '#/access/can'], ['Access: drafts', '#/access/drafts'], ['Access: reviews', '#/reviews'],
  ['Access: resources', '#/resources'], ['Access: model', '#/model'], ['Access map', '#/graph'], ['Jobs', '#/roles'],
  ['Computers', '#/network'], ['Apps', '#/apps'], ['Secrets', '#/secrets'], ['Settings', '#/settings/account'], ['Settings: this service', '#/settings'],
  ['Settings: service accounts', '#/settings/service-accounts'],   ['Settings: services and sign-in providers', '#/connections'], ['You', '#/me'],
];

const current = () => $('#rail a.on')?.dataset.nav;

/** The open palette's Go to rows, in order, each with its destination hash (kept on the row, never shown). */
function goToRows(): { row: HTMLElement; hash: string }[] {
  const rows: { row: HTMLElement; hash: string }[] = [];
  let group = '';
  for (const el of $$('#palList > *')) {
    if (el.classList.contains('grp')) group = el.textContent ?? '';
    else if (group === 'Go to' && el.matches('.it[data-n]')) rows.push({ row: el, hash: el.dataset.to ?? '' });
  }
  return rows;
}

describe('go-to keys and palette (conformance 9.1)', () => {
  it('palette, with nothing typed, offers what can be done, then the rail\'s pages in the rail\'s order', async () => {
    await mount('#/people');
    await press('k', { metaKey: true }, document.body);
    expect(goToRows().map(({ hash }) => hash)).toEqual(['#/dashboard', '#/?section=waiting-on-you', '#/people', '#/canvas', '#/access', '#/network', '#/apps', '#/secrets', '#/settings/account', '#/me']);
    expect($$('#palList .grp').map((group) => group.textContent)).toEqual(['Do', 'Go to']);
  });

  it('palette Go to reaches every screen by its name, each row arriving at its own address with a rail item lit', async () => {
    await mount('#/people');
    const reached = new Set<string>();
    const lit = new Set<string>();
    for (const [name, hash] of PAGE_NAMES) {
      await press('k', { metaKey: true }, document.body);
      await type($('#palIn'), name);
      const entry = goToRows().find(({ row }) => row.firstElementChild?.textContent === name);
      expect(entry, name).toBeDefined();
      if (!entry) continue;
      expect(entry.hash, name).toBe(hash);
      await click(entry.row);
      expect($('#palette')?.classList.contains('open')).toBe(false);
      expect(location.hash, name).toBe(hash);
      reached.add(location.hash);
      const nav = current();
      expect(nav, hash).toBeDefined();
      if (nav) lit.add(nav);
    }
    expect(reached.size).toBe(PAGE_NAMES.length);
    expect([...lit].sort()).toEqual([...RAIL_ITEMS].sort());
  });

  it('g letters reach every screen', async () => {
    await mount('#/people');
    const lit = new Set<string>();
    for (const [letter, [hash, nav]] of Object.entries(LETTERS)) {
      await press('g', {}, document.body);
      await press(letter, {}, document.body);
      expect(location.hash, 'g ' + letter).toBe(hash);
      expect(current(), 'g ' + letter).toBe(nav);
      lit.add(nav);
    }
    expect([...lit].sort()).toEqual([...RAIL_ITEMS].sort());
  });

  it('g then an unmapped letter goes nowhere', async () => {
    await mount('#/people');
    const before = location.hash;
    expect(before).toBe('#/people');
    await press('g', {}, document.body);
    await press('z', {}, document.body);
    expect(location.hash).toBe(before);
    expect(current()).toBe('people');
  });
});
