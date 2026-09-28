import { describe, expect, it } from 'vitest';
import { $, $$, click, mount, press } from './harness';

// Every rail screen is one palette entry and one g letter away (index.v5.html:1085,
// 1094, 1112, 1125-1126, 1177-1178). The screens and letters are written out here,
// from the mock-up, rather than read from the shell's own tables.

const SCREENS = [
  'me', 'people', 'roles', 'resources', 'access', 'graph', 'requests',
  'reviews', 'secrets', 'connections', 'network', 'sessions', 'model', 'settings',
];

const LETTERS: Record<string, string> = {
  p: 'people', o: 'roles', r: 'resources', a: 'access', q: 'requests', w: 'reviews', v: 'secrets',
  n: 'connections', x: 'sessions', m: 'model', s: 'settings', h: 'graph', t: 'network', u: 'me',
};

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
  it('palette Go to reaches all 14 screens', async () => {
    await mount('#/people');
    await press('k', { metaKey: true }, document.body);
    const count = goToRows().length;
    expect(count).toBeGreaterThanOrEqual(14);
    await press('Escape');
    const reached = new Set<string>();
    for (let n = 0; n < count; n += 1) {
      await press('k', { metaKey: true }, document.body);
      const entry = goToRows()[n];
      await click(entry.row);
      expect($('#palette')?.classList.contains('open')).toBe(false);
      const nav = current();
      expect(nav, entry.hash).toBeDefined();
      if (nav) reached.add(nav);
    }
    expect(reached.size).toBe(14);
    expect([...reached].sort()).toEqual([...SCREENS].sort());
  });

  it('g letters reach all 14 screens', async () => {
    await mount('#/people');
    const reached = new Map<string, string>();
    for (const [letter, screen] of Object.entries(LETTERS)) {
      await press('g', {}, document.body);
      await press(letter, {}, document.body);
      const nav = current();
      expect(nav, 'g ' + letter).toBe(screen);
      if (nav) reached.set(letter, nav);
    }
    expect(reached.size).toBe(14);
    expect(new Set(reached.values()).size).toBe(14);
    expect([...new Set(reached.values())].sort()).toEqual([...SCREENS].sort());
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
