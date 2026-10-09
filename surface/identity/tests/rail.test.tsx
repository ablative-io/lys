import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { HashRouter, Route, Routes } from 'react-router';
import { describe, expect, it } from 'vitest';
import { Settings } from '../src/features/settings/Settings';
import { Shell } from '../src/shell/Shell';
import { ShellProvider } from '../src/shell/ShellContext';
import { NavigationProvider } from '../src/shell/NavigationAccess';
import { SERVICE } from './fixtures';
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
  it('rail button toggles the labels', async () => {
    await mount('#/people');
    expect(localStorage.getItem('iam.labels')).toBeNull();
    expect(railOpen()).toBe(false);
    await click($('#railBtn'));
    expect(railOpen()).toBe(true);
    expect(localStorage.getItem('iam.labels')).toBe('labels');
    await click($('#railBtn'));
    expect(railOpen()).toBe(false);
    expect(localStorage.getItem('iam.labels')).toBe('icons');
    expect(prefKeys()).toEqual(['iam.labels']);
  });

  it('bracket toggles the rail outside a text field', async () => {
    await mount('#/people');
    await press('[', {}, document.body);
    expect(railOpen()).toBe(true);
    expect(localStorage.getItem('iam.labels')).toBe('labels');
    await press('[', {}, document.body);
    expect(railOpen()).toBe(false);
    expect(localStorage.getItem('iam.labels')).toBe('icons');
    await press('k', { metaKey: true }, document.body);
    expect(document.activeElement?.id).toBe('palIn');
    await press('[', {}, $('#palIn'));
    expect(railOpen()).toBe(false);
    expect(localStorage.getItem('iam.labels')).toBe('icons');
  });

  it('rail choice survives a remount', async () => {
    await mount('#/people');
    await press('[', {}, document.body);
    expect(localStorage.getItem('iam.labels')).toBe('labels');
    unmountAll();
    expect($$('#rail')).toHaveLength(0);
    await mount('#/people');
    expect(railOpen()).toBe(true);
    await press('[', {}, document.body);
    expect(localStorage.getItem('iam.labels')).toBe('icons');
    unmountAll();
    await mount('#/people');
    expect(railOpen()).toBe(false);
  });

  it('palette act and backslash switch the dock side', async () => {
    await mount('#/people');
    expect(localStorage.getItem('iam.dock')).toBeNull();
    expect(dockRight()).toBe(false);
    await press('k', { metaKey: true }, document.body);
    const act = $$('#palette .it[data-n]').filter((row) => row.firstElementChild?.textContent === 'Move Help to the other side');
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
