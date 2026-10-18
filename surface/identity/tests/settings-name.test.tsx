import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, expect, it } from 'vitest';
import { App } from '../src/App';
import { $, serve } from './harness';
import { SERVICE } from './fixtures';

let root: ReturnType<typeof createRoot> | undefined;
afterEach(() => {
  act(() => root?.unmount());
  root = undefined;
});

// Apps has its own rail entry and page head (01 item 1); src/features/apps/apps-header.test.tsx pins it.
it.each(['/settings', '/settings/account', '/connections'])('W1 names %s Settings in the page, rail and tabs', async (path) => {
  serve(SERVICE);
  history.replaceState(null, '', '#' + path);
  const container = document.createElement('div');
  document.body.appendChild(container);
  root = createRoot(container);
  await act(async () => {
    root?.render(<App />);
  });
  expect($('.page h1')?.textContent).toBe('Settings');
  expect($('#rail [data-nav="settings"] .lbl')?.textContent).toBe('Settings');
  expect($('nav[aria-label="Settings views"]')).not.toBeNull();
  expect($('nav[aria-label="Configuration views"]')).toBeNull();
});
