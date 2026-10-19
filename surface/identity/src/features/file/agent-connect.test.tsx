/**
 * AGENTS-016 R2: an agent's file shows the two lines that connect Claude Code and Codex to it through lys mcp, built
 * from the page's own origin and the agent's id, each with a copy act; the sentence under them says which origin works
 * now and which needs the https bridge of AGENTS-015 R3. A person's file shows none. No clocks: every wait here is
 * the page changing.
 */
import { createRoot } from 'react-dom/client';
import type { Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { AgentConnect } from './AgentConnect';
import { ShellProvider } from '../../shell/ShellContext';

const hex = (n: number) => n.toString(16).padStart(32, '0');
const AGENT = `agent-${hex(401)}`;
const PERSON = `person-${hex(402)}`;
const KEY = '<path to this agent’s key file on that computer>';

let root: Root | null = null;
let copied: string[] = [];
let actEnvironment: boolean | undefined;
beforeEach(() => {
  copied = [];
  Object.defineProperty(navigator, 'clipboard', { configurable: true, value: { writeText: async (text: string) => { copied.push(text); } } });
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

async function drawn(id: string, origin?: string) {
  const container = document.createElement('div');
  document.body.appendChild(container);
  root = createRoot(container);
  root.render(<ShellProvider><AgentConnect id={id} origin={origin} /></ShellProvider>);
  await until(container, () => container.querySelector('[data-connect-ready]') !== null);
  return container;
}

const lines = (container: HTMLElement) => [...container.querySelectorAll('[data-connect-line]')].map((line) => line.textContent);

describe('Connect on an agent’s file', () => {
  it('shows both lines with the page’s origin, /api/mcp and the agent’s id, and each copy act copies its exact line', async () => {
    const origin = window.location.origin;
    const container = await drawn(AGENT);
    const claude = `claude mcp add --transport stdio lys -- lys mcp --url ${origin}/api/mcp --agent ${AGENT} --key-file ${KEY}`;
    const codex = `codex mcp add lys -- lys mcp --url ${origin}/api/mcp --agent ${AGENT} --key-file ${KEY}`;
    expect(lines(container)).toEqual([claude, codex]);
    expect(container.textContent).toContain('Capability certificates');
    const acts = [...container.querySelectorAll<HTMLButtonElement>('button[data-symbol="copy"]')];
    expect(acts.map((act) => act.getAttribute('aria-label'))).toEqual(['Copy the claude code line', 'Copy the codex line']);
    acts[0]?.click();
    await until(container, () => copied.length === 1);
    acts[1]?.click();
    await until(container, () => copied.length === 2);
    expect(copied).toEqual([claude, codex]);
  });

  it('says the line works now for http on this computer, and names AGENTS-015 R3 for https', async () => {
    const here = await drawn(AGENT, 'http://127.0.0.1:8490');
    expect(lines(here)[0]).toContain('--url http://127.0.0.1:8490/api/mcp');
    expect(here.querySelector('[data-connect-works]')?.textContent).toMatch(/works now/);
    expect(here.textContent).not.toContain('AGENTS-015 R3');
    root?.unmount();
    const away = await drawn(AGENT, 'https://lys.example.test');
    expect(lines(away)[1]).toContain('--url https://lys.example.test/api/mcp');
    expect(away.querySelector('[data-connect-works]')?.textContent).toContain('AGENTS-015 R3');
  });

  it('draws nothing on a person’s file', async () => {
    const container = await drawn(PERSON);
    expect(container.querySelector('section[aria-label="Connect"]')).toBeNull();
    expect(lines(container)).toEqual([]);
  });
});
