import { describe, expect, it } from 'vitest';
import { RAIL } from '../src/shell/railItems';
import { $, $$, click, mount, press, unreachable } from './harness';
import { unmountAll } from './harness';

describe('shell (conformance 9.1)', () => {
  it('draws the rail item for item, each screen a deep link', async () => {
    await mount('#/people');
    const links = $$('#rail a[data-nav]');
    const expected = RAIL.filter((i) => i.t === 'a');
    expect(links.map((a) => a.getAttribute('href'))).toEqual(expected.map((i) => 'href' in i && i.href));
    expect(links.map((a) => a.title)).toEqual(expected.map((i) => 'title' in i && i.title));
    expect($$('#rail button.rb').map((b) => b.title)).toEqual(['Help (?)', 'Assistant', 'Command palette (⌘K)', 'Rail labels ([)']);
    expect($('#rail a.on')?.dataset.nav).toBe('people');
  });

  it('goes to a screen with g then its letter', async () => {
    await mount('#/people');
    await press('g', {}, document.body);
    await press('u', {}, document.body);
    expect(location.hash).toBe('#/me');
    expect($('#rail a.on')?.dataset.nav).toBe('me');
    await press('g', {}, document.body);
    await press('o', {}, document.body);
    expect(location.hash).toBe('#/roles');
    expect($('.page h1')?.textContent).toBe('Roles');
  });

  it('opens and collapses the rail with [ and keeps the choice', async () => {
    await mount('#/people');
    expect($('#rail')?.classList.contains('open')).toBe(false);
    await press('[', {}, document.body);
    expect($('#rail')?.classList.contains('open')).toBe(true);
    expect(localStorage.getItem('iam.labels')).toBe('labels');
    await click($('#railBtn'));
    expect($('#rail')?.classList.contains('open')).toBe(false);
  });

  it('docks left or right with \\ and from Configuration', async () => {
    await mount('#/settings');
    expect($('#shell')?.classList.contains('dock-right')).toBe(false);
    await press('\\', {}, document.body);
    expect($('#shell')?.classList.contains('dock-right')).toBe(true);
    await click($('[data-dock="left"]'));
    expect($('#shell')?.classList.contains('dock-right')).toBe(false);
    await click($('[data-labels="labels"]'));
    expect($('#rail')?.classList.contains('open')).toBe(true);
  });

  it('opens the palette with Command K, moves with arrows, and returns focus on Escape', async () => {
    await mount('#/people');
    const before = $('#railBtn');
    before?.focus();
    await press('k', { metaKey: true });
    expect($('#palette')?.classList.contains('open')).toBe(true);
    expect(document.activeElement?.id).toBe('palIn');
    await press('Escape');
    expect($('#palette')?.classList.contains('open')).toBe(false);
    expect(document.activeElement).toBe(before);
  });

  it('finds a person in the palette from the service and opens their file', async () => {
    await mount('#/people');
    await press('k', { ctrlKey: true }, document.body);
    const input = $('#palIn') as HTMLInputElement;
    expect($$('#palList .it').some((i) => i.textContent?.includes("Scribe"))).toBe(true);
    expect($$('#palList .it').find((i) => i.textContent?.startsWith("Scribe"))?.textContent).toContain('agent · active');
    await press('ArrowDown', {}, input);
    expect($('#palList .it.sel')?.textContent).toContain('Bea (test person)');
    await press('ArrowUp', {}, input);
    await press('Enter', {}, input);
    expect(location.hash).toMatch(/^#\/file\/person-/);
  });

  it('shows every unbuilt screen as not built, with the mock-up heading', async () => {
    for (const view of ['roles', 'resources', 'graph', 'requests', 'reviews', 'secrets', 'connections', 'network', 'sessions', 'model']) {
      unmountAll();
      document.body.innerHTML = '';
      await mount('#/' + view);
      expect($('.empty-note')?.textContent).toContain('not built yet');
      expect(unreachable()).toEqual([]);
    }
  });

  it('keeps every control reachable by keyboard on Configuration (9.3)', async () => {
    await mount('#/settings');
    expect(unreachable()).toEqual([]);
  });
});
