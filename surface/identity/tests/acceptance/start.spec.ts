/**
 * DIRECTORY-029 R13 acceptance fixtures: the start route's answers in each
 * state the Start drawer reaches, as the library writes them, and the
 * clipboard the browser tests read with navigator.clipboard.readText().
 * tests/start.test.tsx runs every acceptance line against them.
 */
import type { LaunchStateView, StartCheck, StartGiven, StartRefused } from '../../src/generated';
import { START_CHECKS } from '../../src/generated';
import type { Answer } from '../fixtures';

export const AGENT = 'agent-fixture-1';
export const L1 = 'launch-00000000000000000000000000000001';
/** The values the handle record fixture keeps beside the credential ids. */
export const VALUES = ['fixture-credential-value-1f3a', 'fixture-credential-value-2b7c'];
export const COMMAND = "env LYS_AGENT_ID=agent-fixture-1 LYS_LAUNCH_RECORD=" + L1 + " LYS_CREDENTIAL_IDS=vc-fixture-1,vc-fixture-2 fixture-exec --fixture-arg 'two words'";

const passed = (check: string): StartCheck => ({ check, result: 'passed', words: check + ': passed' });

export const GIVEN: StartGiven = {
  command: COMMAND,
  working_directory: 'fixture-cwd',
  launch_record: {
    id: L1, agent: AGENT, machine: 'machine-fixture-1', executable: 'fixture-exec', working_directory: 'fixture-cwd',
    profile_version: 'pv-fixture-1', given_by: 'admin-fixture', arguments: ['--fixture-arg', 'two words'],
    credential_ids: ['vc-fixture-1', 'vc-fixture-2'], given_at: 1790000000, copied_from: null,
  },
  state: 'unconfirmed',
  words: 'Unconfirmed: no signed report names launch record ' + L1 + ' yet. The request stands. Asking another machine could start it twice: wait for its report, or withdraw it first.',
  checks: START_CHECKS.map(passed),
};

const suspendedWords = "agent_not_active: the check 'the agent is active' failed: the agent is suspended";

export const SUSPENDED: StartRefused = {
  refused: [{ refusal: 'agent_not_active', words: suspendedWords }],
  checks: START_CHECKS.map((check, at) => (at === 0 ? { check, result: 'agent_not_active', words: suspendedWords } : passed(check))),
};

export const UNCONFIRMED: LaunchStateView = { launch_record: L1, agent: AGENT, state: 'unconfirmed', words: GIVEN.words, session: null, withdrawal: null };
export const WITHDRAWN: LaunchStateView = { ...UNCONFIRMED, state: 'withdrawn', words: 'Withdrawn: launch record ' + L1 + "'s request no longer stands. Withdrawn by admin-fixture at 1790000100.", withdrawal: { by: 'admin-fixture', at: 1790000100 } };
export const RUNNING_WITHDRAWN: LaunchStateView = { ...WITHDRAWN, state: 'running', session: 'sess-fixture-1', words: 'Running: the agent\'s verified signed report of session sess-fixture-1 names launch record ' + L1 + '. Withdrawn by admin-fixture at 1790000100.' };

export const answer = (status: number, body: unknown): Answer => ({ status, body });

/** A clipboard the page writes and the test reads, as a browser's would. */
export function installClipboard(): { written: string[] } {
  const held = { text: '', written: [] as string[] };
  Object.defineProperty(navigator, 'clipboard', {
    configurable: true,
    value: {
      writeText: async (text: string) => { held.text = text; held.written.push(text); },
      readText: async () => held.text,
    },
  });
  return held;
}

/** Whether `text` says, in any case, that the agent was not started. */
export const saysNotStarted = (text: string): boolean => text.toLowerCase().includes('not started');
