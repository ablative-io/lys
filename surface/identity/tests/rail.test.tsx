import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { HashRouter, Route, Routes } from 'react-router';
import { describe, expect, it } from 'vitest';
import { Settings } from '../src/features/settings/Settings';
import { Shell } from '../src/shell/Shell';
import { ShellProvider } from '../src/shell/ShellContext';
import { NavigationProvider } from '../src/shell/NavigationAccess';
import { SERVICE, ok } from './fixtures';
import { $, $$, click, mount, press, serve, settle, unmountAll } from './harness';

// The rail and the dock side, as the mock-up keeps them (index.v5.html:39, 41, 42,
// 52, 285, 453-457, 1092, 1127-1128): one preference key each, iam.labels and iam.dock.

const railOpen = () => $('#rail')?.classList.contains('open');
const dockRight = () => $('#shell')?.classList.contains('dock-right');

/** Every iam. key in storage, so a preference kept under any other key shows. */
function prefKeys(): string[] {
  const keys: string[] = [];
  for (let i = 0; i < localStorage.length; i += 1) {
    const key = localStorage.key(i);
    if (key?.startsWith('iam.')) keys.push(key);
  }
  return keys.sort();
}

describe('rail and dock side (conformance 9.1)', () => {
  it('rail button toggles the labels, which start shown', async () => {
    await mount('#/people');
    expect(localStorage.getItem('iam.labels')).toBeNull();
    expect(railOpen()).toBe(true);
    await click($('#railBtn'));
    expect(railOpen()).toBe(false);
    expect(localStorage.getItem('iam.labels')).toBe('icons');
    await click($('#railBtn'));
    expect(railOpen()).toBe(true);
    expect(localStorage.getItem('iam.labels')).toBe('labels');
    expect(prefKeys()).toEqual(['iam.labels']);
  });

  it('bracket toggles the rail outside a text field', async () => {
    await mount('#/people');
    await press('[', {}, document.body);
    expect(railOpen()).toBe(false);
    expect(localStorage.getItem('iam.labels')).toBe('icons');
    await press('[', {}, document.body);
    expect(railOpen()).toBe(true);
    expect(localStorage.getItem('iam.labels')).toBe('labels');
    await press('k', { metaKey: true }, document.body);
    expect(document.activeElement?.id).toBe('palIn');
    await press('[', {}, $('#palIn'));
    expect(railOpen()).toBe(true);
    expect(localStorage.getItem('iam.labels')).toBe('labels');
  });

  it('rail choice survives a remount', async () => {
    await mount('#/people');
    await press('[', {}, document.body);
    expect(localStorage.getItem('iam.labels')).toBe('icons');
    unmountAll();
    expect($$('#rail')).toHaveLength(0);
    await mount('#/people');
    expect(railOpen()).toBe(false);
    await press('[', {}, document.body);
    expect(localStorage.getItem('iam.labels')).toBe('labels');
    unmountAll();
    await mount('#/people');
    expect(railOpen()).toBe(true);
  });

  it('Menu opens the whole rail as a drawer, and choosing an entry, Escape or Menu again closes it', async () => {
    await mount('#/people');
    const menu = () => $('#rail .menu-btn');
    const drawer = () => $('#rail')?.classList.contains('drawer');
    expect(menu()?.getAttribute('aria-expanded')).toBe('false');
    expect(menu()?.textContent).toBe('Menu');
    expect(menu()?.getAttribute('aria-label')).toBe('Open the menu');
    await click(menu());
    expect(drawer()).toBe(true);
    expect(railOpen()).toBe(true);
    expect(menu()?.textContent).toBe('Close');
    expect(menu()?.getAttribute('aria-label')).toBe('Close the menu');
    await click($('#rail a[data-nav="network"]'));
    expect(location.hash).toBe('#/network');
    expect(drawer()).toBe(false);
    await click(menu());
    await press('Escape', {}, document.body);
    expect(drawer()).toBe(false);
    await click(menu());
    await click(menu());
    expect(drawer()).toBe(false);
    // The drawer is not the labels preference: opening it stores nothing.
    expect(prefKeys()).toEqual([]);
  });

  it('Menu carries the count of what waits on the person while the drawer is shut', async () => {
    const line = (kind: string, id: string) => ({ kind, id, asked_by: { id: 'person-sam', name: 'Sam' }, about: 'Sam asks to edit project logo', asked_at: '2026-10-18T01:00:00Z', can_decide: true });
    await mount('#/people', { ...SERVICE, '/waiting': ok({ waiting: [line('request', 'r1'), line('review', 'g1')] }) });
    expect($('#rail .menu .cnt')?.textContent).toBe('2');
    expect($('#rail .menu .cnt')?.title).toBe('2 things wait on you');
    expect($('#rail a[data-nav="waiting"] .cnt')?.textContent).toBe('2');
  });

  it('opens Home with Waiting on you focused and shares its count and list links', async () => {
    const waiting = [{ kind: 'request', id: 'r1', asked_by: { id: 'sam', name: 'Sam' }, about: 'Sam asks for access', asked_at: '2026-10-26T01:00:00Z', can_decide: true }];
    const { requests } = await mount('#/people', { ...SERVICE, '/waiting': ok({ waiting }) });
    const before = requests.filter((path) => path === '/waiting').length;
    await click($('#rail a[data-nav="waiting"]'));
    expect(location.hash).toBe('#/?section=waiting-on-you');
    const section = $('section[aria-label="Waiting on you"]');
    expect(document.activeElement).toBe(section);
    expect(requests.filter((path) => path === '/waiting')).toHaveLength(before + 1);
    expect(section?.querySelector('[data-waiting="count"]')?.textContent).toBe($('#rail a[data-nav="waiting"] .cnt')?.textContent);
    expect([...section?.querySelectorAll('footer a') ?? []].map((link) => [link.textContent, link.getAttribute('href')])).toEqual([
      ['Requests', '#/requests'], ['Checks', '#/reviews'], ['Drafts', '#/access/drafts'],
    ]);
    await click($('#rail a[data-nav="waiting"]'));
    expect(document.activeElement).toBe(section);
  });

  it('palette act and backslash switch the dock side', async () => {
    await mount('#/people');
    expect(localStorage.getItem('iam.dock')).toBeNull();
    expect(dockRight()).toBe(false);
    await press('k', { metaKey: true }, document.body);
    const act = $$('#palette .it[data-n]').filter((row) => row.firstElementChild?.textContent === 'Move the menu and Help to the other side');
    expect(act).toHaveLength(1);
    await click(act[0]);
    expect(dockRight()).toBe(true);
    expect(localStorage.getItem('iam.dock')).toBe('right');
    await press('\\', {}, document.body);
    expect(dockRight()).toBe(false);
    expect(localStorage.getItem('iam.dock')).toBe('left');
    await press('k', { metaKey: true }, document.body);
    await press('\\', {}, $('#palIn'));
    expect(dockRight()).toBe(false);
    expect(localStorage.getItem('iam.dock')).toBe('left');
    expect(prefKeys()).toEqual(['iam.dock']);
  });

  it('Layout segment sets the dock side', async () => {
    serve(SERVICE);
    location.hash = '#/settings/layout';
    const container = document.createElement('div');
    document.body.appendChild(container);
    const root = createRoot(container);
    try {
      await act(async () => {
        root.render(
          <HashRouter>
            <ShellProvider>
              <NavigationProvider>
                <Shell>
                  <Routes>
                    <Route path="/settings/:sec?" element={<Settings />} />
                  </Routes>
                </Shell>
              </NavigationProvider>
            </ShellProvider>
          </HashRouter>,
        );
      });
      await settle();
      expect(localStorage.getItem('iam.dock')).toBeNull();
      expect(dockRight()).toBe(false);
      await click($('button[data-dock="right"]'));
      expect(dockRight()).toBe(true);
      expect(localStorage.getItem('iam.dock')).toBe('right');
      await click($('button[data-dock="left"]'));
      expect(dockRight()).toBe(false);
      expect(localStorage.getItem('iam.dock')).toBe('left');
    } finally {
      act(() => root.unmount());
    }
  });
});
