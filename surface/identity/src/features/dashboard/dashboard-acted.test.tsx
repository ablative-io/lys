/**
 * AGENTS-016 R5: what the person's agents registered and started for them, on the Dashboard under one heading, in
 * plain words, with Stop beside each line that posts the agent's stop after the same confirmation the agent file's
 * Stop uses, and the same receipt words. No lines, no heading. No clocks: every wait is the page changing.
 */
import { createRoot } from 'react-dom/client';
import type { Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { ActedByAgents } from './ActedByAgents';
import type { ActedLine } from './contract';
import { ShellProvider } from '../../shell/ShellContext';
import { ADA, ok } from '../../../tests/fixtures';
import { serve } from '../../../tests/harness';

const hex = (n: number) => n.toString(16).padStart(32, '0');
const SCRIBE = `agent-${hex(201)}`;
const HELPER = `agent-${hex(202)}`;
const RUNNER = `agent-${hex(203)}`;
const NAMES = new Map([[SCRIBE, 'Scribe'], [HELPER, 'Helper'], [RUNNER, 'Runner']]);
const AT = 1_760_850_000;
const LINES: ActedLine[] = [
  { kind: 'started', acting_agent: SCRIBE, agent: RUNNER, at: AT + 60, session: `op-${hex(301)}` },
  { kind: 'registered', acting_agent: SCRIBE, agent: HELPER, at: AT },
];

let root: Root | null = null;
let actEnvironment: boolean | undefined;
beforeEach(() => {
  sessionStorage.clear();
  const scope = globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean };
  actEnvironment = scope.IS_REACT_ACT_ENVIRONMENT;
  scope.IS_REACT_ACT_ENVIRONMENT = false;
});
afterEach(() => {
  root?.unmount();
  root = null;
  (globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = actEnvironment;
});

/** Resolves when `ready` holds of the page: checked now, then on every change to it. */
function until(container: HTMLElement, ready: () => boolean): Promise<void> {
  return new Promise((resolve) => {
    if (ready()) { resolve(); return; }
    const watch = new MutationObserver(() => { if (ready()) { watch.disconnect(); resolve(); } });
    watch.observe(container, { childList: true, subtree: true, characterData: true, attributes: true });
  });
}

async function drawn(lines: ActedLine[]) {
  const posted: { path: string; body: unknown }[] = [];
  serve({
    [`POST /agents/${HELPER}/stop`]: (body) => ok({
      agent: HELPER, operation: (body as { operation: string }).operation, state: 'suspended', by: ADA, at: AT + 120,
      certificates_withdrawn: ['serial-1'], credentials_ended: [], credentials_refused: null, sessions_asked: [],
      reason: (body as { reason: string }).reason, sessions_confirmed: [], sessions_refused: [],
    }),
  }, posted);
  const container = document.createElement('div');
  document.body.appendChild(container);
  root = createRoot(container);
  root.render(<ShellProvider><ActedByAgents lines={lines} names={NAMES} changed={() => undefined} /></ShellProvider>);
  await until(container, () => container.querySelector('[data-acted-ready]') !== null);
  return { container, posted };
}

const typed = (input: HTMLInputElement, value: string) => {
  Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')?.set?.call(input, value);
  input.dispatchEvent(new Event('input', { bubbles: true }));
};

describe('Acted for you by your agents, on the Dashboard', () => {
  it('lists each line in plain words under one heading, newest first, with Stop beside each', async () => {
    const { container } = await drawn(LINES);
    const section = container.querySelector('section[aria-label="Acted for you by your agents"]');
    expect(section?.querySelector('h2, .section-h')?.textContent).toContain('Acted for you by your agents');
    const rows = [...container.querySelectorAll('[data-acted-line]')];
    expect(rows.map((row) => row.querySelector('[data-acted-words]')?.textContent)).toEqual([
      expect.stringMatching(/^Scribe started Runner for you, /),
      expect.stringMatching(/^Scribe registered Helper for you, /),
    ]);
    expect(rows.map((row) => row.querySelector('button[data-act="stop"]')?.getAttribute('aria-label'))).toEqual(['Stop Runner', 'Stop Helper']);
  });

  it('Stop asks the agent file’s confirmation, posts the agent’s stop, and shows the same receipt', async () => {
    const { container, posted } = await drawn(LINES);
    container.querySelectorAll<HTMLButtonElement>('button[data-act="stop"]')[1]?.click();
    await until(container, () => container.querySelector('form[aria-label="Confirm pause"]') !== null);
    expect(posted).toEqual([]);
    const reason = container.querySelector<HTMLInputElement>('form[aria-label="Confirm pause"] input');
    expect(reason).not.toBeNull();
    if (reason) typed(reason, 'I did not ask for this');
    await until(container, () => !container.querySelector<HTMLButtonElement>('button[aria-label="Pause this agent now"]')?.disabled);
    container.querySelector<HTMLButtonElement>('button[aria-label="Pause this agent now"]')?.click();
    await until(container, () => container.querySelector('[aria-label="Agent suspension recorded"]') !== null);
    expect(posted.map((each) => each.path)).toEqual([`/agents/${HELPER}/stop`]);
    expect(posted[0].body).toMatchObject({ reason: 'I did not ask for this' });
    expect(container.textContent).toContain('The agent’s access is suspended.');
  });

  it('draws no heading when no agent acted for the person', async () => {
    const { container } = await drawn([]);
    expect(container.textContent).not.toContain('Acted for you by your agents');
    expect(container.querySelector('[data-acted-line]')).toBeNull();
  });
});
