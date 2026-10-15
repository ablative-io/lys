import { readFileSync } from 'node:fs';
import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, expect, it } from 'vitest';
import { App } from '../src/App';
import { $, serve } from './harness';
import { COURIER, SCRIBE, SERVICE, dashboard, ok } from './fixtures';

let root: ReturnType<typeof createRoot> | undefined;
let style: HTMLStyleElement | undefined;
afterEach(() => {
  act(() => root?.unmount());
  root = undefined;
  style?.remove();
});

async function show() {
  const alarms = {
    open_count: 1,
    newest: [
      {
        id: 'alarm-1',
        kind: 'session_ungoverned',
        session: 'session-1',
        agent: SCRIBE,
        machine: 'machine-1',
        raised_at: 100,
        detail: 'Proxy run key has no usage file',
        acknowledged_by: null,
        acknowledged_at: null,
        cleared_at: null,
      },
    ],
  };
  const answer = { ...dashboard(), alarms };
  answer.agents = answer.agents.map((row) => ([SCRIBE, COURIER].includes(row.agent.id) ? { ...row, agent: { ...row.agent, display_name: 'Claude Code (lys)' } } : row));

  serve({
    ...SERVICE,
    '/dashboard': ok(answer),
    '/security-events': ok({ administrator: true, events: [{ id: 'event-1', kind: 'password_reset_completed', at: 100, detail: 'The password reset completed.' }] }),
  });
  style = document.createElement('style');
  style.textContent = ['features/dashboard/dashboard.css', 'styles/screen.css'].map((path) => readFileSync(new URL('../src/' + path, import.meta.url), 'utf8')).join('\n');
  document.head.appendChild(style);
  history.replaceState(null, '', '#/dashboard');
  const container = document.createElement('div');
  document.body.appendChild(container);
  root = createRoot(container);
  await act(async () => {
    root?.render(<App />);
  });
}

it('W5 distinguishes duplicate agent names even when neither has a runtime location', async () => {
  await show();
  const labels = [...document.querySelectorAll<HTMLElement>('.you-tree .you-agent-context')];
  expect(labels).toHaveLength(2);
  expect(labels.every((label) => label.textContent?.startsWith('Record A/'))).toBe(true);
  expect(new Set(labels.map((label) => label.textContent)).size).toBe(2);
  expect(labels.map((label) => label.title).sort()).toEqual([SCRIBE, COURIER].sort());
  expect($('.you-tree')?.textContent).toContain('Not running');
});

it('W6 gives the alarm text room beside its acknowledgement control', async () => {
  await show();
  const widgets = $('.dash-widgets');
  if (!widgets) throw new Error('Dashboard widgets did not render');
  expect(getComputedStyle(widgets).gridTemplateColumns).toBe('repeat(auto-fit, minmax(min(100%, 260px), 1fr))');
  const alarms = $('section[aria-label="Alarms"]');
  if (!alarms) throw new Error('Alarms did not render');
  expect(getComputedStyle(alarms).gridColumn).toBe('1 / -1');
  expect(widgets.firstElementChild).toBe(alarms);
  expect($('[data-alarm] td:first-child')?.textContent).toContain('Its model calls are not going through Lys, so its use is not counted');
  expect($('[data-alarm] button[aria-label="Acknowledge"]')).not.toBeNull();
});
