import { readFileSync } from 'node:fs';
import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, expect, it } from 'vitest';
import { App } from '../src/App';
import { $, serve } from './harness';
import { GRANTS, ME, SCRIBE, SERVICE, ok } from './fixtures';

let root: ReturnType<typeof createRoot> | undefined;
let style: HTMLStyleElement | undefined;
afterEach(() => {
  act(() => root?.unmount());
  root = undefined;
  style?.remove();
});

it('W4 gives the Reviews list the full width and puts the selected grant below it', async () => {
  const grant = { ...GRANTS[0], relation: 'template_owner', resource: { kind: 'liminal.network', id: 'ablative' } };
  serve({
    ...SERVICE,
    '/reviews': ok({
      scope: 'directory',
      revision: 4,
      judged_at: 1790000000,
      due: [{ grant, agent: { id: SCRIBE, display_name: 'Claude Code (lys)', state: 'active' }, reviewer: ME.person }],
      unanswered: [],
    }),
  });
  style = document.createElement('style');
  style.textContent = ['features/grants/grants.css', 'styles/screen.css'].map((path) => readFileSync(new URL('../src/' + path, import.meta.url), 'utf8')).join('\n');
  document.head.appendChild(style);
  history.replaceState(null, '', '#/reviews');
  const container = document.createElement('div');
  document.body.appendChild(container);
  root = createRoot(container);
  await act(async () => {
    root?.render(<App />);
  });
  const body = $('.page > .body');
  if (!body) throw new Error('The Reviews list did not render');
  expect(getComputedStyle(body).display).toBe('flex');
  expect(getComputedStyle(body).flexDirection).toBe('column');
  expect(getComputedStyle(body).overflow).toBe('auto');
  const table = $('.listing table');
  if (!table) throw new Error('The review table did not render');
  expect(getComputedStyle(table).width).toBe('100%');
  expect(getComputedStyle(table).tableLayout).toBe('fixed');
  expect($('.listing td[data-col="Access"]')?.textContent).toBe('template_owner on liminal.network ablative');
  expect($('.listing td[data-col="Access"] wbr')).not.toBeNull();
  expect($('[aria-label="Grant to review"]')).not.toBeNull();
});
