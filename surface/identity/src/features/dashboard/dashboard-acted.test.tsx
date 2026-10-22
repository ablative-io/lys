/**
 * AGENTS-016 R5, AGENTS-017 R2: what the person's agents registered and started for them, on the Dashboard under one
 * heading, in plain words. A started line's Stop ends that run through the runtime session's end and never the
 * agent's stop; Withdraw, on every line, is the emergency act behind the agent file's confirmation, headed Withdraw,
 * its receipt saying the registrar cannot certify the agent again. A registered line has no session and shows
 * Withdraw alone. AGENTS-019 R5: an act an agent took with a grant is said in its shipped sentence, '<agent>
 * <sentence> for you, <time>.', with Withdraw for the agent that acted and no Stop. No lines, no heading. No clocks:
 * every wait is the page changing.
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
const SESSION = `op-${hex(301)}`;
const FINAL = 'Its certificates are withdrawn for good: the agent that registered it cannot certify it again, and a person issues its next certificate.';
const LINES: ActedLine[] = [
  { kind: 'started', acting_agent: SCRIBE, agent: RUNNER, at: AT + 60, session: SESSION },
  { kind: 'registered', acting_agent: SCRIBE, agent: HELPER, at: AT },
];
const ACTS: ActedLine[] = [
  { kind: 'person.email.set', acting_agent: HELPER, agent: HELPER, at: AT + 120, subject: { kind: 'account', id: `person-${hex(7)}` },
    index: 44, sentence: 'Change this person’s email' },
  { kind: 'organisation.admit', acting_agent: HELPER, agent: HELPER, at: AT + 90, subject: { kind: 'organisation', id: `organisation-${hex(1)}` },
    index: 41, sentence: 'Admit a person into this organisation' },
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
      final_words: FINAL,
    }),
    [`POST /runtime/sessions/${SESSION}/end`]: () => ok({
      session: SESSION, answer: { kind: 'ended', ended: { how: 'exited', at: AT + 90, status: 0, signal: null } },
      receipt: { act: 'end', session: SESSION },
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
  it('lists each line in plain words under one heading, newest first: Stop beside a started line, Withdraw beside each', async () => {
    const { container } = await drawn(LINES);
    const section = container.querySelector('section[aria-label="Acted for you by your agents"]');
    expect(section?.querySelector('h2, .section-h')?.textContent).toContain('Acted for you by your agents');
    const rows = [...container.querySelectorAll('[data-acted-line]')];
    expect(rows.map((row) => row.querySelector('[data-acted-words]')?.textContent)).toEqual([
      expect.stringMatching(/^Scribe started Runner for you, /),
      expect.stringMatching(/^Scribe registered Helper for you, /),
    ]);
    expect(rows.map((row) => row.querySelector('button[data-act="stop"]')?.getAttribute('aria-label') ?? null)).toEqual(['Stop Runner', null]);
    expect(rows.map((row) => row.querySelector('button[data-act="withdraw"]')?.getAttribute('aria-label'))).toEqual(['Withdraw Runner', 'Withdraw Helper']);
  });

  it('a started line’s Stop ends that run through the session’s end, never the agent’s stop, and the receipt reads ended', async () => {
    const { container, posted } = await drawn(LINES);
    const started = container.querySelector('[data-acted-line="started"]');
    started?.querySelector<HTMLButtonElement>('button[data-act="stop"]')?.click();
    await until(container, () => container.querySelector('[role="alertdialog"][aria-label="Stop Runner"]') !== null);
    expect(posted).toEqual([]);
    container.querySelector<HTMLButtonElement>('button[data-act="confirm-stop"]')?.click();
    await until(container, () => container.querySelector('[data-session-stopped]') !== null);
    expect(posted.map((each) => each.path)).toEqual([`/runtime/sessions/${SESSION}/end`]);
    expect(posted.some((each) => each.path.endsWith('/stop'))).toBe(false);
    const receipt = container.querySelector('[data-session-stopped]')?.textContent ?? '';
    expect(receipt).toContain('ended');
    expect(receipt).toContain('Its certificate stands');
    expect(container.querySelector('[aria-label="Agent suspension recorded"]')).toBeNull();
  });

  it('Withdraw asks the agent file’s confirmation headed Withdraw, posts the agent’s stop, and its receipt says it is final', async () => {
    const { container, posted } = await drawn(LINES);
    const registered = container.querySelector('[data-acted-line="registered"]');
    expect(registered?.querySelector('button[data-act="stop"]')).toBeNull();
    registered?.querySelector<HTMLButtonElement>('button[data-act="withdraw"]')?.click();
    await until(container, () => container.querySelector('form[aria-label="Confirm withdraw"]') !== null);
    expect(container.querySelector('form[aria-label="Confirm withdraw"] h3')?.textContent).toBe('Withdraw');
    expect(posted).toEqual([]);
    const reason = container.querySelector<HTMLInputElement>('form[aria-label="Confirm withdraw"] input');
    expect(reason).not.toBeNull();
    if (reason) typed(reason, 'I did not ask for this');
    await until(container, () => !container.querySelector<HTMLButtonElement>('button[aria-label="Withdraw this agent now"]')?.disabled);
    container.querySelector<HTMLButtonElement>('button[aria-label="Withdraw this agent now"]')?.click();
    await until(container, () => container.querySelector('[aria-label="Agent suspension recorded"]') !== null);
    expect(posted.map((each) => each.path)).toEqual([`/agents/${HELPER}/stop`]);
    expect(posted[0].body).toMatchObject({ reason: 'I did not ask for this' });
    expect(container.textContent).toContain('The agent’s access is suspended.');
    expect(container.querySelector('[data-withdraw-final]')?.textContent).toContain('cannot certify it again');
  });

  it('says what an agent did with a grant in its shipped sentence, newest first, with Withdraw for that agent and no Stop', async () => {
    const { container } = await drawn([...ACTS, ...LINES]);
    const rows = [...container.querySelectorAll('[data-acted-line]')];
    expect(rows.map((row) => row.getAttribute('data-acted-line'))).toEqual(['person.email.set', 'organisation.admit', 'started', 'registered']);
    expect(rows.map((row) => row.querySelector('[data-acted-words]')?.textContent)).toEqual([
      expect.stringMatching(/^Helper change this person’s email for you, .+\.$/),
      expect.stringMatching(/^Helper admit a person into this organisation for you, .+\.$/),
      expect.stringMatching(/^Scribe started Runner for you, /),
      expect.stringMatching(/^Scribe registered Helper for you, /),
    ]);
    const acts = rows.slice(0, 2);
    expect(acts.map((row) => row.querySelector('button[data-act="stop"]'))).toEqual([null, null]);
    expect(acts.map((row) => row.querySelector('button[data-act="withdraw"]')?.getAttribute('aria-label'))).toEqual(['Withdraw Helper', 'Withdraw Helper']);
  });

  it('draws no heading when no agent acted for the person', async () => {
    const { container } = await drawn([]);
    expect(container.textContent).not.toContain('Acted for you by your agents');
    expect(container.querySelector('[data-acted-line]')).toBeNull();
  });
});
