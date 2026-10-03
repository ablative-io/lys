import { describe, expect, it } from 'vitest';
import { $, $$, click, mount, press } from './harness';

// Every screen is one palette entry and one g letter away. The letters and where each goes are written out here
// rather than read from the shell's own tables. Access is one rail item whose views are its tabs, and Running is the canvas.

const LETTERS: Record<string, [string, string]> = {
  p: ['#/people', 'people'], o: ['#/roles', 'roles'], r: ['#/resources', 'access'], a: ['#/access', 'access'], q: ['#/requests', 'access'], w: ['#/reviews', 'access'],
  v: ['#/secrets', 'secrets'], n: ['#/connections', 'connections'], x: ['#/sessions', 'sessions'], m: ['#/model', 'access'], s: ['#/settings', 'settings'],
  h: ['#/graph', 'access'], t: ['#/network', 'network'], u: ['#/me', 'me'], c: ['#/canvas', 'canvas'], l: ['#/canvas', 'canvas'],
};
const RAIL_ITEMS = ['me', 'people', 'canvas', 'roles', 'access', 'secrets', 'connections', 'network', 'sessions', 'settings'];

const current = () => $('#rail a.on')?.dataset.nav;

/** The open palette's Go to rows, in order, each with its destination hash. */
function goToRows(): { row: HTMLElement; hash: string }[] {
  const rows: { row: HTMLElement; hash: string }[] = [];
  let group = '';
  for (const el of $$('#palList > *')) {
    if (el.classList.contains('grp')) group = el.textContent ?? '';
    else if (group === 'Go to' && el.matches('.it[data-n]')) rows.push({ row: el, hash: el.lastElementChild?.textContent ?? '' });
  }
  return rows;
}

describe('go-to keys and palette (conformance 9.1)', () => {
  it('palette Go to reaches every screen, each row arriving at its own address with a rail item lit', async () => {
    await mount('#/people');
    await press('k', { metaKey: true }, document.body);
    const count = goToRows().length;
    expect(count).toBe(16);
    await press('Escape');
    const reached = new Set<string>();
    const lit = new Set<string>();
    for (let n = 0; n < count; n += 1) {
      await press('k', { metaKey: true }, document.body);
      const entry = goToRows()[n];
      await click(entry.row);
      expect($('#palette')?.classList.contains('open')).toBe(false);
      expect(location.hash, entry.hash).toBe(entry.hash);
      reached.add(location.hash);
      const nav = current();
      expect(nav, entry.hash).toBeDefined();
      if (nav) lit.add(nav);
    }
    expect(reached.size).toBe(16);
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
