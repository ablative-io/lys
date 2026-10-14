import { act } from 'react';
import type { ReactNode } from 'react';
import { createRoot } from 'react-dom/client';
import type { Root } from 'react-dom/client';
import { afterEach, describe, expect, it } from 'vitest';
import { Refused, request, useLoad } from '../src/api';
import { DirectoryGate, ErrorWords, problemWords, refusalWords } from '../src/features/people/Words';
import { refused } from './fixtures';
import type { Route } from './fixtures';
import { serve, settle } from './harness';

let root: Root | undefined;
afterEach(() => {
  if (root) act(() => root?.unmount());
  root = undefined;
  document.body.innerHTML = '';
});

async function show(page: ReactNode, routes: Record<string, Route> = {}) {
  const requests = serve(routes);
  const container = document.createElement('div');
  document.body.append(container);
  root = createRoot(container);
  await act(async () => root?.render(page));
  await settle();
  return requests;
}

function AgentSettingsRead() {
  const load = useLoad(() => request('/agent-settings'), 'refusal-words-settings');
  return <DirectoryGate load={load} title="Settings" ok={() => <p>Settings read.</p>} />;
}

function headline(): string {
  const paragraph = document.querySelector('[role="alert"] > p');
  if (!paragraph) throw new Error('The refusal has no sentence.');
  return paragraph.textContent ?? '';
}

describe('agent page refusal wording', () => {
  it('asks for the working folder and keeps the complete server refusal in closed details', async () => {
    const reason = "WorkingFolderUnnamed: the launch names no working folder and the agent's profile has no working_folder; name one on the launch or set the agent's default";
    const requests = await show(<AgentSettingsRead />, { '/agent-settings': refused(422, 'WorkingFolderUnnamed', reason) });
    expect(requests.filter((path) => path === '/agent-settings')).toHaveLength(1);
    expect(headline()).toBe('Choose the folder this agent works in before it can start.');
    expect(headline()).not.toContain('WorkingFolderUnnamed');
    const details = document.querySelector('details');
    expect(details?.hasAttribute('open')).toBe(false);
    expect(details?.querySelector('summary')?.textContent).toBe('Error details');
    expect(details?.querySelector('.refusal-name')?.textContent).toBe('WorkingFolderUnnamed: ' + reason);
    expect(document.querySelector('[role="alert"]')?.getAttribute('title')).toBe('WorkingFolderUnnamed: ' + reason);
  });

  it('uses an unfamiliar refusal’s server reason without putting its code in the headline', async () => {
    await show(<AgentSettingsRead />, { '/agent-settings': refused(409, 'FutureAgentRule', 'FutureAgentRule: this computer is reserved for a different agent') });
    expect(headline()).toBe('Lys refused: this computer is reserved for a different agent');
    expect(headline()).not.toContain('FutureAgentRule');
    expect(document.querySelector('details')?.textContent).toContain('FutureAgentRule: this computer is reserved for a different agent');
  });

  it('keeps an unfamiliar unavailable refusal’s actual reason instead of inventing a service explanation', async () => {
    await show(<ErrorWords problem={new Refused(503, { refusal: 'FutureUnavailable', reason: 'the selected account has been removed' })} />);
    expect(headline()).toBe('Lys refused: the selected account has been removed');
  });

  it.each([
    'WorkingFolderUnnamed: choose a folder',
    new Error('WorkingFolderUnnamed: choose a folder'),
    'Error: WorkingFolderUnnamed: choose a folder',
    'Error: WorkingFolderUnnamed',
  ])('recognizes named refusals retained as strings or Error messages: %s', (problem) => {
    expect(problemWords(problem)).toBe('Choose the folder this agent works in before it can start.');
  });

  it('keeps ordinary form instructions and unnamed error explanations as written', () => {
    expect(problemWords('Choose a model this program lists.')).toBe('Choose a model this program lists.');
    expect(problemWords(new Error('This browser could not retain the settings request.'))).toBe('This browser could not retain the settings request.');
    expect(problemWords('Saved')).toBe('Saved');
  });

  it.each(['FutureCode', 'future_refusal', new Error('FutureCode'), new Error('future_refusal')])('names a missing reason instead of showing an unfamiliar bare code: %s', async (problem) => {
    await show(<ErrorWords problem={problem} />);
    expect(headline()).toBe('Lys refused without explaining why.');
    expect(document.querySelector('details .refusal-name')?.textContent).toBe(problem instanceof Error ? problem.message : problem);
  });

  it('removes whitespace and nested refusal prefixes from the headline while preserving exact original details', async () => {
    const reason = '  FutureAgentRule:  FutureAgentRule: the computer is reserved  ';
    await show(<ErrorWords problem={new Refused(409, { refusal: 'FutureAgentRule', reason })} />);
    expect(headline()).toBe('Lys refused: the computer is reserved  ');
    expect(document.querySelector('[role="alert"]')?.getAttribute('title')).toBe('FutureAgentRule: ' + reason);
    expect(document.querySelector('details .refusal-name')?.textContent).toBe('FutureAgentRule: ' + reason);
  });

  it('uses a retained unknown code’s reason, including further colons in that reason', () => {
    expect(problemWords(new Error('FutureAgentRule: the computer refused: its folder is read only'))).toBe('Lys refused: the computer refused: its folder is read only');
    expect(problemWords(new Refused(409, { refusal: 'FutureAgentRule', reason: '' }))).toBe('Lys refused without explaining why.');
  });

  it('does not treat inherited object members as refusal sentences', () => {
    expect(refusalWords('toString')).toBeUndefined();
    expect(problemWords(new Refused(409, { refusal: 'constructor', reason: 'this computer cannot be chosen' }))).toBe('Lys refused: this computer cannot be chosen');
  });

  it.each([
    ['InvalidProgramCommand', 'An installed program’s command is invalid.'],
    ['UnsafeSearchPath', 'The runner’s program search folders include an invalid path.'],
    ['ProgramMetadataUnreadable', 'Lys could not inspect the installed program.'],
    ['ProgramPathUnreadable', 'Lys could not read the installed program’s full path.'],
    ['CommandNotFound', 'The chosen command was not found on this computer.'],
    ['VersionCommandUnreadable', 'The computer could not run the program’s version command.'],
    ['VersionCommandFailed', 'The program’s version command failed.'],
    ['VersionOutputUnreadable', 'The program did not return a readable version.'],
  ])('explains installed-copy discovery refusal %s while keeping its source details closed', async (code, sentence) => {
    const reason = code + ': the executable could not be read';
    await show(<ErrorWords problem={reason} />);
    expect(headline()).toContain(sentence);
    expect(headline()).not.toContain(code);
    expect(document.querySelector('details')?.hasAttribute('open')).toBe(false);
    expect(document.querySelector('details .refusal-name')?.textContent).toBe(reason);
  });

  it('has an actionable sentence for the agent settings, start, folder and terminal refusal contracts', () => {
    const codes = [
      'NotSignedIn', 'NotAdmitted', 'NoPerson', 'RequestMalformed', 'AgentNotVisible', 'IdentifierMalformed',
      'WorkingFolderUnnamed', 'LaunchRecordMissing', 'HarnessUndeclared', 'LaunchUnrenderable',
      'AgentHasNoPolicy', 'AgentNotActive', 'AgentSignatureRefused', 'CertificatesUnavailable', 'PolicyUnavailable',
      'MachineUnknown', 'MachineRetired', 'MachineNotForAgent', 'MachineCannotReach', 'MachineWithoutRuntime', 'MachineWithoutRunner',
      'MachineAgentsReused', 'not_your_computer', 'agent_not_yours', 'NetworkUnavailable', 'RuntimeUnavailable',
      'ProvisioningUnavailable', 'ProvisioningChanged', 'ProvisioningReused', 'ProfileVersionUnknown', 'ProfileVersionReplaced',
      'ProfileNotReviewed', 'profile_version_unreviewed', 'account_handle_unheld', 'account_provider_mismatch', 'SecretsUnavailable',
      'McpCredentialInline', 'McpHandleUnsupported', 'McpSettingUnrepresentable', 'ModelUnrepresentable', 'PolicyUnrepresentable', 'SkillUnknown',
      'harness_program_unknown', 'harness_programs_unreadable', 'harness_catalogue_unreadable',
      'InvalidProgramCommand', 'UnsafeSearchPath', 'ProgramMetadataUnreadable', 'ProgramPathUnreadable',
      'CommandNotFound', 'VersionCommandUnreadable', 'VersionCommandFailed', 'VersionOutputUnreadable',
      'runner_absent', 'runner_request_unsigned', 'runner_request_malformed', 'runner_protocol_mismatch',
      'runner_request_replayed', 'runner_request_misaddressed', 'runner_unreachable', 'runner_reply_malformed',
      'runner_state_unavailable', 'runner_stopping', 'session_invalid', 'session_unknown', 'session_ended',
      'spawn_failed', 'launch_without_directory', 'launch_config_refused', 'size_invalid', 'rotation_invalid',
      'trust_file_invalid', 'trust_file_unreadable', 'trust_file_unwritable', 'trust_home_unknown', 'trust_watch_failed',
      'policy_invalid', 'policy_rule_duplicate', 'policy_target_ambiguous', 'policy_target_uninspectable', 'policy_digest_mismatch',
      'everything_stopped', 'RuntimeSessionUnknown', 'RuntimeSessionStopped', 'RuntimeReportReused',
      'BudgetExhausted', 'BudgetsUnavailable', 'ConfigurationUnavailable', 'TeamsUnavailable', 'TeamUnknown', 'TeamRetired',
      'plugins_unreadable', 'plugin_not_installed', 'home_unknown', 'folder_unreadable', 'FoldersUnreadable',
      'PendingSaveUnreadable', 'ProfileReadIncomplete', 'ProfileReceiptMismatch', 'PendingStartUnreadable',
      'StartReceiptMismatch', 'StartVersionChanged', 'RunnerStartUnconfirmed', 'ProfileMissing', 'PolicyUnreadable',
      'AgentSettingsMismatch', 'AgentSettingsUnreadable', 'AgentTeamsUnreadable', 'AgentComputerUnreadable', 'AgentModelsUnreadable',
      'ServiceUnreachable', 'UnreadableResponse', 'Unanswered', 'TerminalUnavailable', 'RolesUnavailable',
      'not_permitted', 'CallUnknown', 'CallKeptElsewhere', 'CallRecordsUnavailable', 'holder_unknown',
    ];
    for (const code of codes) {
      const sentence = refusalWords(code);
      expect(sentence, code).toBeDefined();
      expect(sentence, code).toMatch(/^[A-Z].*[.!?]$/);
      expect(sentence, code).not.toContain(code);
      expect(problemWords(new Refused(409, { refusal: code, reason: 'server detail' })), code).toBe(sentence);
    }
  });
});
