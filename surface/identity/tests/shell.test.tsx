import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { HashRouter } from 'react-router';
import { describe, expect, it, vi } from 'vitest';
import { Shell } from '../src/shell/Shell';
import { ShellProvider, useShell } from '../src/shell/ShellContext';
import { NavigationProvider } from '../src/shell/NavigationAccess';
import { SERVICE } from './fixtures';
import { RAIL } from '../src/shell/railItems';
import { $, $$, click, mount, press, serve, type, unreachable } from './harness';

describe('shell (conformance 9.1)', () => {
  it('draws the rail item for item, each screen a deep link', async () => {
    await mount('#/people');
    const links = $$('#rail a[data-nav]');
    const expected = RAIL.filter((i) => i.t === 'a');
    expect(links.map((a) => a.getAttribute('href'))).toEqual(expected.map((i) => 'href' in i && i.href));
    expect(links.map((a) => a.title)).toEqual(expected.map((i) => 'title' in i && i.title));
    // jsdom's navigator is not a Mac, so Search names Ctrl K, the key a person off a Mac presses.
    expect($$('#rail button.rb').map((b) => b.title)).toEqual(['Help (?)', 'Search (Ctrl K)', 'Menu labels ([)']);
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
    expect($('.page h1')?.textContent).toBe('Jobs');
  });

  it('opens with its labels, collapses them with [ and keeps the choice', async () => {
    await mount('#/people');
    expect($('#rail')?.classList.contains('open')).toBe(true);
    await press('[', {}, document.body);
    expect($('#rail')?.classList.contains('open')).toBe(false);
    expect(localStorage.getItem('iam.labels')).toBe('icons');
    await click($('#railBtn'));
    expect($('#rail')?.classList.contains('open')).toBe(true);
  });

  it('docks left or right with \\ and from Settings, For you', async () => {
    await mount('#/settings/account');
    expect($('#shell')?.classList.contains('dock-right')).toBe(false);
    await press('\\', {}, document.body);
    expect($('#shell')?.classList.contains('dock-right')).toBe(true);
    await click($('[data-dock="left"]'));
    expect($('#shell')?.classList.contains('dock-right')).toBe(false);
    await click($('[data-labels="icons"]'));
    expect($('#rail')?.classList.contains('open')).toBe(false);
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
    // Nothing typed: what can be done, then the pages in the rail's order; no person yet, however long the directory.
    expect($('#palList .it.sel')?.dataset.to).toBe('#/people/new');
    expect($$('#palList .grp').map((g) => g.textContent)).toEqual(['Do', 'Go to']);
    expect($$('#palList .it').some((i) => i.textContent?.includes('Scribe'))).toBe(false);
    await type(input, 'Scribe');
    expect($$('#palList .it').find((i) => i.textContent?.startsWith('Scribe'))?.textContent).toContain('agent · active');
    await type(input, 'test person');
    expect($$('#palList .grp').map((g) => g.textContent)).toEqual(['People and agents', 'Ask']);
    expect($('#palList .it.sel')?.textContent).toContain('Ada (test person)');
    await press('ArrowDown', {}, input);
    expect($('#palList .it.sel')?.textContent).toContain('Bea (test person)');
    await press('ArrowUp', {}, input);
    await press('Enter', {}, input);
    expect(location.hash).toMatch(/^#\/file\/person-/);
  });

  it('keeps every control reachable by keyboard on Configuration (9.3)', async () => {
    await mount('#/settings');
    expect(unreachable()).toEqual([]);
  });
});

describe('no fixed waits in the shell (CONSENSUS.md)', () => {
  it('keeps a toast until it is dismissed or replaced, on no clock', async () => {
    serve(SERVICE);
    location.hash = '#/me';
    const container = document.createElement('div');
    document.body.appendChild(container);
    const root = createRoot(container);
    let said: ((text: string) => void) | null = null;
    function Says() {
      said = useShell().toast;
      return null;
    }
    await act(async () => {
      root.render(<HashRouter><ShellProvider><NavigationProvider><Shell><Says /></Shell></NavigationProvider></ShellProvider></HashRouter>);
    });
    const clock = vi.spyOn(globalThis, 'setTimeout');
    act(() => said?.('Given.'));
    expect(clock).not.toHaveBeenCalled();
    clock.mockRestore();
    expect($('#toast.show')?.textContent).toBe('Given.');
    act(() => said?.('Taken back.'));
    expect($('#toast.show')?.textContent).toBe('Taken back.');
    await click($('#toast [aria-label="Dismiss this message"]'));
    expect($('#toast.show')).toBeNull();
    act(() => root.unmount());
  });

  it('lets g wait for its letter until a pointer press, not a clock', async () => {
    await mount('#/people');
    await press('g', {}, document.body);
    document.body.dispatchEvent(new Event('pointerdown', { bubbles: true }));
    await press('u', {}, document.body);
    expect(location.hash).toBe('#/people');
    await press('g', {}, document.body);
    await press('u', {}, document.body);
    expect(location.hash).toBe('#/me');
  });
});
