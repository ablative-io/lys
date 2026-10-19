/**
 * AGENTS-016 R2, AGENTS-017 R1: an agent's file shows the enrol line first, whose --out is the seat file the agent's
 * profile names, then the two lines that connect Claude Code and Codex to it through lys mcp reading that same file,
 * built from the page's own origin and the agent's id, each with a copy act; no line carries a placeholder for the key
 * file. The sentence under them says which origin works now and which needs the https bridge of AGENTS-015 R3. A
 * person's file shows none. No clocks: every wait here is the page changing.
 */
import { createRoot } from 'react-dom/client';
import type { Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { AgentConnect } from './AgentConnect';
import { CONNECT_INTRO, REGISTRAR_SEAT, connectLines, defaultSeatFile } from '../network/connectLines';
import { ShellProvider } from '../../shell/ShellContext';
import { ok } from '../../../tests/fixtures';
import { serve } from '../../../tests/harness';

const hex = (n: number) => n.toString(16).padStart(32, '0');
const AGENT = `agent-${hex(401)}`;
const PERSON = `person-${hex(402)}`;
/** The placeholder the lines carried before AGENTS-017 R1, which no line may carry again. */
const OLD_PLACEHOLDER = '<path to this agent’s key file on that computer>';
const SEAT_FILE = '/Users/bea/.lys/seats/assistant.seat.json';

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

/** The agent's profile as `GET /agents/{id}/provisioning` answers it: naming `seatFile`, or no seat program. */
function profileNaming(seatFile: string | null) {
  const seat_program = seatFile === null ? undefined : {
    executable: '/usr/local/bin/seat', executable_sha256: 'a'.repeat(64), arguments: ['--seat', seatFile],
    seat_file: seatFile, seat_file_sha256: 'b'.repeat(64), adapter: 'stdio/v1',
  };
  serve({ [`/agents/${AGENT}/provisioning`]: ok({ agent: AGENT, recorded: null, enforced: false, versions: [],
    profile: { version: 1, harness: { name: 'seat', program: '/usr/local/bin/seat', package: 'seat', description: { seat_program } } } }) });
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

describe('The connect lines, built in one place', () => {
  it('builds the enrol line, then the Claude Code and Codex lines, from the agent’s id, the origin and the seat file', () => {
    expect(connectLines({ id: AGENT, origin: 'https://lys.example.test', keyFile: '/home/bea/assistant.seat.json' })).toEqual([
      { what: 'The enrol line', line: `lys agent enrol --url https://lys.example.test/api --agent ${AGENT} --as ${REGISTRAR_SEAT} --out /home/bea/assistant.seat.json` },
      { what: 'The Claude Code line', line: `claude mcp add --transport stdio lys -- lys mcp --url https://lys.example.test/api/mcp --agent ${AGENT} --key-file /home/bea/assistant.seat.json` },
      { what: 'The Codex line', line: `codex mcp add lys -- lys mcp --url https://lys.example.test/api/mcp --agent ${AGENT} --key-file /home/bea/assistant.seat.json` },
    ]);
  });
});

describe('The registering agent’s seat file in the enrol line', () => {
  it('says what goes there, and the words above the lines say how to fill it or enrol with a pass instead', () => {
    expect(REGISTRAR_SEAT).toBe('<path to the seat file of the agent that registered this one>');
    expect(CONNECT_INTRO).toContain('Put the path to the seat file of the agent that registered this one where the line has angle brackets');
    expect(CONNECT_INTRO).toContain('use --pass with your Lys app pass file in place of --as');
  });
});

describe('Connect on an agent’s file', () => {
  it('shows the enrol line first with --out the profile’s seat file, the mcp lines reading it, and no placeholder', async () => {
    profileNaming(SEAT_FILE);
    const origin = window.location.origin;
    const container = await drawn(AGENT);
    const enrol = `lys agent enrol --url ${origin}/api --agent ${AGENT} --as ${REGISTRAR_SEAT} --out ${SEAT_FILE}`;
    const claude = `claude mcp add --transport stdio lys -- lys mcp --url ${origin}/api/mcp --agent ${AGENT} --key-file ${SEAT_FILE}`;
    const codex = `codex mcp add lys -- lys mcp --url ${origin}/api/mcp --agent ${AGENT} --key-file ${SEAT_FILE}`;
    expect(lines(container)).toEqual([enrol, claude, codex]);
    for (const line of lines(container)) expect(line).not.toContain(OLD_PLACEHOLDER);
    expect(container.textContent).not.toContain(OLD_PLACEHOLDER);
    expect(container.querySelector('[data-connect-seat-file]')?.textContent).toContain(SEAT_FILE);
    const acts = [...container.querySelectorAll<HTMLButtonElement>('button[data-symbol="copy"]')];
    expect(acts.map((act) => act.getAttribute('aria-label'))).toEqual(['Copy the enrol line', 'Copy the claude code line', 'Copy the codex line']);
    for (const [index, act] of acts.entries()) {
      act.click();
      await until(container, () => copied.length === index + 1);
    }
    expect(copied).toEqual([enrol, claude, codex]);
  });

  it('with no seat file in the profile, every line uses the default path and says so', async () => {
    profileNaming(null);
    const container = await drawn(AGENT);
    const seat = defaultSeatFile(AGENT);
    expect(lines(container)[0]?.endsWith('--out ' + seat)).toBe(true);
    for (const line of lines(container)) expect(line).toContain(seat);
    expect(container.querySelector('[data-connect-seat-file]')?.textContent).toContain('names no seat file');
  });

  it('says the lines work now for http on this computer, and names AGENTS-015 R3 for https', async () => {
    profileNaming(SEAT_FILE);
    const here = await drawn(AGENT, 'http://127.0.0.1:8490');
    expect(lines(here)[0]).toContain('--url http://127.0.0.1:8490/api ');
    expect(lines(here)[1]).toContain('--url http://127.0.0.1:8490/api/mcp');
    expect(here.querySelector('[data-connect-works]')?.textContent).toMatch(/work now/);
    expect(here.textContent).not.toContain('AGENTS-015 R3');
    root?.unmount();
    const away = await drawn(AGENT, 'https://lys.example.test');
    expect(lines(away)[2]).toContain('--url https://lys.example.test/api/mcp');
    expect(away.querySelector('[data-connect-works]')?.textContent).toContain('AGENTS-015 R3');
  });

  it('draws nothing on a person’s file', async () => {
    profileNaming(SEAT_FILE);
    const container = await drawn(PERSON);
    expect(container.querySelector('section[aria-label="Connect"]')).toBeNull();
    expect(lines(container)).toEqual([]);
  });
});
