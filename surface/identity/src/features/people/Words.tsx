import type { ReactNode } from 'react';
import { api, Refused, useLoad } from '../../api';
import type { Load } from '../../api';
import type { PeopleView } from '../../generated';
import { Gate } from '../signin/Gate';
import { entries } from './directory';

/** The one table of what a named refusal means and what to do next. */
const REFUSALS: Record<string, string> = {
  NotSignedIn: 'Sign in before changing this agent.',
  NotAdmitted: 'You do not have permission to do this. Ask the directory administrator to make the change.',
  not_permitted: 'You do not have permission to do this. Ask the responsible person or the administrator.',
  NoPerson: 'Your sign-in account is not connected to a person in the directory. Ask the administrator to connect it.',
  PolicyVersionConflict: 'Someone changed these rules first. Open this tab again to see the saved rules before editing.',
  BudgetVersionConflict: 'This budget changed before your confirmation. Open the Budgets tab again and review the current limits.',
  AgentNotVisible: 'This agent is not among the records you can see. Ask its responsible person or the administrator.',
  PersonNotVisible: 'This person is not among the records you can see. Ask the administrator to check your access.',
  RoleUnknown: 'This role could not be found. Open All roles and choose a current role.',
  HolderUnknown: 'This person, agent or role assignment could not be found. Open the role again and check its assignments.',
  RoleVersionUnknown: 'This role version could not be found. Open the role again and review its saved versions.',
  ServiceAccountUnknown: 'This account record could not be found or is not yours to manage. Ask the owner or administrator.',
  RequestMalformed: 'Lys could not accept these form details. Check the fields; the service’s explanation is under Error details.',
  UnreadableResponse: 'Lys answered, but its result could not be read. Keep the original change and check its outcome before sending another.',
  SavedRequestUnreleased: 'This browser would not remove the saved request. Close this tab and open Lys in a new one: a new tab starts with no saved requests.',
  AlarmsUnreadable: 'Lys answered, but the alarms could not be read. Reload the page; if it happens again, ask the administrator to check the service.',
  UnconfirmedReceipt: 'The answer did not confirm this change. Use Check to see whether Lys saved it before sending another.',
  StorageUncertain: 'The change may have been saved, but its outcome is unknown. Use Check to see whether Lys saved it before sending another.',
  ServiceUnreachable: 'Lys could not be reached. Ask the administrator to check the service. Keep any change whose outcome is unknown.',
  SecretsUnavailable: 'Lys could not reach or use its secret storage service. Ask the administrator to check it.',
  SignInProvidersUnavailable: 'Lys could not read or change its sign-in services. Ask the administrator to check them.',
  SignInProvidersRefused: 'The sign-in service did not accept these settings.',
  ProviderRefused: 'The provider did not accept these sign-in settings.',
  HandleUnknown: 'That permission to use a secret was not found in the records you can see.',
  RequestHeld: 'An earlier approval still has no confirmed result.',
  LeaseExhausted: 'The permitted number of uses has been reached.',
  app_exists: 'An app is already registered with that short name.',
  schema_version_moved: 'Someone saved a newer version of these permissions.',
  schema_change_strands_grants: 'These permissions are still in use and cannot be removed yet.',
  RootAuthorityRefused: 'Only the directory\'s root authority issues a root grant. Ask the administrator to issue it.',
  grant_mode_on_hot_action: 'This action is decided at once by the app, so it cannot be held for a draft. Issue it outright, or choose another action.',
  WorkingFolderUnnamed: 'Choose the folder this agent works in before it can start.',
  LaunchRecordMissing: 'Save this agent’s settings before starting it.',
  HarnessUndeclared: 'Choose the installed program this agent uses, then save its settings.',
  AgentNotActive: 'This agent is not ready to start. Check whether its access is awaiting activation, paused or retired.',
  AgentHasNoPolicy: 'Set this agent’s tool rules before starting it.',
  AgentSignatureRefused: 'Lys could not verify this agent’s identity. Ask the administrator to check its certificate.',
  AgentSettingsMismatch: 'Lys answered with settings for a different agent. Keep your changes and check this agent’s saved settings.',
  AgentSettingsUnreadable: 'Lys could not read this agent’s saved computer and model. Keep your changes and check its saved settings.',
  AgentTeamsUnreadable: 'Lys could not read this agent’s team memberships. Ask the administrator to check them.',
  AgentComputerUnreadable: 'Lys could not read this agent’s computers. Ask the administrator to check them.',
  AgentModelsUnreadable: 'Lys could not read the models available to this agent. Ask the administrator to check its programs.',
  MachineUnknown: 'This computer is no longer listed. Choose a computer Lys currently lists.',
  MachineRetired: 'This computer is retired. Choose a computer that is still in use.',
  MachineNotForAgent: 'Allow this agent to run on the chosen computer before starting it.',
  MachineUnavailable: 'No available computer is allowed to run this agent. Choose a computer and allow the agent there.',
  MachineCannotReach: 'This computer cannot reach a service the agent needs. Check its network access before starting it.',
  MachineWithoutRuntime: 'This computer has no service to run the agent. Ask the administrator to connect its runner.',
  MachineWithoutRunner: 'This computer has no connected runner. Connect its Lys runner before starting the agent.',
  MachineAgentsReused: 'This saved computer allowance names a different change. Check its earlier outcome before sending another.',
  not_your_computer: 'You do not manage this computer. Ask its owner or the administrator to make the change.',
  agent_not_yours: 'You do not manage this agent. Ask its responsible person or the administrator to make the change.',
  ProvisioningChanged: 'Someone saved newer settings first. Open this agent’s Settings again and review the saved version.',
  ProvisioningReused: 'This saved settings request names a different change. Check its earlier outcome before sending another.',
  ProfileMissing: 'Save this agent’s settings before choosing its folder.',
  ProfileNotReviewed: 'These settings need approval before the agent can use them.',
  ProfileVersionUnknown: 'This settings version could not be found. Open Settings again and choose a saved version.',
  ProfileVersionReplaced: 'Newer settings replaced the version being approved. Review the latest saved settings.',
  profile_version_unreviewed: 'Approve this agent’s current settings before starting it.',
  ProfileReceiptMismatch: 'Lys did not confirm these exact settings were saved. Check the original request’s outcome before sending another.',
  ProfileReadIncomplete: 'Lys did not return all the saved settings. Update Lys before changing them.',
  PendingSaveUnreadable: 'This browser could not read an earlier settings save. Check its outcome before saving another change.',
  PendingStartUnreadable: 'This browser could not read an earlier start request. Check its outcome before starting again.',
  StartReceiptMismatch: 'Lys did not confirm this exact start request. Check its original outcome before starting again.',
  StartVersionChanged: 'The start named a different settings version. Check that session before starting again.',
  RunnerStartUnconfirmed: 'The start was admitted, but the runner has not confirmed running it. Check the saved request before starting again.',
  RuntimeSessionUnknown: 'This session could not be found among those you can see. Open the agent again and choose its current session.',
  RuntimeSessionStopped: 'This session has ended and cannot take more input. Start a new session when you are ready.',
  RuntimeReportReused: 'This saved session report names a different change. Check its earlier outcome before sending another.',
  session_ended: 'This session has ended and cannot take more input. Start a new session when you are ready.',
  session_unknown: 'The runner could not find this session. Check the agent’s current session before sending more input.',
  session_invalid: 'The runner could not accept this session’s details. Ask the administrator to check the start request.',
  no_live_session: 'This agent has no running session to wake. Start it before sending a message.',
  everything_stopped: 'All agents are stopped. Ask the administrator to allow starts again.',
  harness_program_unknown: 'This computer did not report the chosen program. Choose one of its installed copies.',
  harness_absent_on_machine: 'The chosen program is not installed on this computer. Choose an installed copy or another computer.',
  harness_program_invalid: 'Enter the program’s command or its full path on this computer.',
  harness_programs_unreadable: 'Lys could not read this computer’s installed programs. Check its runner before choosing a copy.',
  harness_catalogue_unreadable: 'Lys could not read its program descriptions. Ask the administrator to check them.',
  CommandNotFound: 'The chosen command was not found on this computer. Choose an installed program or enter its full path.',
  InvalidProgramCommand: 'An installed program’s command is invalid. Ask the administrator to check its executable name.',
  UnsafeSearchPath: 'The runner’s program search folders include an invalid path. Ask the administrator to give each folder its full path.',
  ProgramMetadataUnreadable: 'Lys could not inspect the installed program. Ask the administrator to check access to its executable.',
  ProgramPathUnreadable: 'Lys could not read the installed program’s full path. Ask the administrator to check its executable.',
  VersionCommandUnreadable: 'The computer could not run the program’s version command. Check its installed copy before choosing it.',
  VersionCommandFailed: 'The program’s version command failed. Check its installed copy before choosing it.',
  VersionOutputUnreadable: 'The program did not return a readable version. Check its installed copy before choosing it.',
  LaunchUnrenderable: 'Lys could not turn these settings into a start command. Check the error details and correct the settings.',
  launch_config_refused: 'The runner could not use these program settings. Check the error details and correct them.',
  launch_without_directory: 'Choose the folder this agent works in before it can start.',
  spawn_failed: 'The computer could not start this program. Check its installed copy and working folder.',
  ModelUnrepresentable: 'This program cannot use the chosen model settings. Choose a model the program lists.',
  PolicyUnrepresentable: 'This program cannot enforce the chosen tool rules. Change the rules or choose a program that supports them.',
  SkillUnknown: 'A skill in these settings is no longer available. Remove it or ask the administrator to keep its text.',
  McpCredentialInline: 'Keep this connected tool’s credential in secret storage and choose its secret handle here.',
  McpSettingUnrepresentable: 'This program cannot use the connected tool’s settings. Check the tool details before saving.',
  McpHandleUnsupported: 'This agent cannot use a secret required by a connected tool. Ask its responsible person to grant that access.',
  account_handle_unheld: 'This agent cannot use the selected model account. Grant it access to that account before starting.',
  account_provider_mismatch: 'The selected model account belongs to a different provider. Choose an account for this program’s provider.',
  runner_absent: 'This computer has no connected Lys runner. Connect its runner before continuing.',
  runner_unreachable: 'Lys could not reach this computer’s runner. Check that the computer and its runner are connected.',
  runner_reply_malformed: 'The computer’s runner returned an unreadable answer. Check the request’s outcome before sending it again.',
  runner_state_unavailable: 'The runner could not confirm the session’s state. Check that session before starting another.',
  runner_stopping: 'This computer’s runner is stopping and cannot take this request. Check its connection before continuing.',
  runner_protocol_mismatch: 'This computer’s runner uses an incompatible version. Update it before continuing.',
  runner_request_unsigned: 'The runner could not verify who sent this request. Ask the administrator to check its connection.',
  runner_request_malformed: 'The runner could not read this request. Ask the administrator to check the error details.',
  runner_request_replayed: 'The runner has already seen this request. Check its earlier outcome before sending another.',
  runner_request_misaddressed: 'This request reached a different runner. Ask the administrator to check the computer’s connection.',
  FoldersUnreadable: 'The computer did not return a readable folder list. Check its runner before choosing a folder.',
  home_unknown: 'The runner could not find its home folder. Ask the administrator to check the account it runs under.',
  folder_unreadable: 'The runner could not read this folder. Choose a readable folder or ask its owner to check access.',
  plugins_unreadable: 'The runner could not read the installed plugins. Check the program’s configuration folder.',
  plugin_not_installed: 'This plugin is not installed on the chosen computer. Choose an installed plugin.',
  trust_file_invalid: 'The program’s folder approval file is invalid. Ask the administrator to check it before starting.',
  trust_file_unreadable: 'The runner could not read the program’s folder approval file. Ask the administrator to check its access.',
  trust_file_unwritable: 'The runner could not save the program’s folder approval. Ask the administrator to check its access.',
  trust_home_unknown: 'The runner could not find the home folder needed for program approval. Ask the administrator to check its account.',
  trust_watch_failed: 'The runner could not monitor the program’s folder approval. Ask the administrator to check the runner.',
  size_invalid: 'The terminal size could not be accepted. Open the session again and check the terminal.',
  rotation_invalid: 'The runner could not accept the output file limits. Ask the administrator to check its settings.',
  policy_invalid: 'These tool rules could not be accepted. Check their details before saving.',
  policy_rule_duplicate: 'Two tool rules use the same name. Give each rule its own name before saving.',
  policy_target_ambiguous: 'A tool rule names more than one possible target. Choose one target before saving.',
  policy_target_uninspectable: 'The runner cannot inspect a target named by a tool rule. Check that rule before starting.',
  policy_digest_mismatch: 'The runner received different tool rules from the saved ones. Ask the administrator to check them before starting.',
  PolicyUnreadable: 'Lys could not read this agent’s tool rules. Check the saved rules before changing them.',
  BudgetExhausted: 'This agent has reached a budget limit and cannot start. Ask its responsible person to review the limit.',
  CallUnknown: 'This model call could not be found. Open the call list again and choose a recorded call.',
  CallKeptElsewhere: 'This model call is kept on another computer. Read it from the computer that recorded it.',
  holder_unknown: 'This budget’s person, agent or team could not be found. Open the agent again and check its ownership.',
  IdentifierMalformed: 'A record identifier in this request could not be read. Open this agent again before continuing.',
  IdentityUnknown: 'This identity could not be found. Open People and agents again and choose a current record.',
  TeamUnknown: 'This team could not be found. Open Teams again and choose a current team.',
  TeamRetired: 'This team is retired. Choose a team that is still active.',
  TransitionRefused: 'This agent cannot make that change of state. Check its current state before continuing.',
  OperationReused: 'This saved request names a different change. Check its earlier outcome before sending another.',
  StopReused: 'This saved stop request names a different change. Check its earlier outcome before sending another.',
  Unanswered: 'Lys answered without confirming the request’s outcome. Check its original outcome before sending another.',
  DirectoryUnavailable: 'Lys could not read its directory. Ask the administrator to check the service.',
  RolesUnavailable: 'Lys could not read the roles that allow this agent to run. Ask the administrator to check the service.',
  MembershipUnavailable: 'Lys could not read this agent’s role memberships. Ask the administrator to check the service.',
  NetworkUnavailable: 'Lys could not read the computers. Ask the administrator to check the service.',
  RuntimeUnavailable: 'Lys could not read or change this agent’s sessions. Check the current session before repeating a change.',
  TerminalUnavailable: 'Lys could not read or use this terminal. Check the session before sending more input.',
  ProvisioningUnavailable: 'Lys could not read or save this agent’s settings. Keep your changes and check the saved version before trying again.',
  CertificatesUnavailable: 'Lys could not read or change this agent’s certificates. Ask the administrator to check the service.',
  PolicyUnavailable: 'Lys could not read this agent’s tool rules. Check the saved rules before starting it.',
  BudgetsUnavailable: 'Lys could not read this agent’s budgets. Check the limits before starting it.',
  ConfigurationUnavailable: 'Lys could not read its configuration. Ask the administrator to check the service.',
  TeamsUnavailable: 'Lys could not read the teams. Ask the administrator to check the service.',
  CallRecordsUnavailable: 'Lys could not read the recorded model calls. Check the recording service before relying on the totals.',
};

export const refusalWords = (code: string): string | undefined => Object.hasOwn(REFUSALS, code) ? REFUSALS[code] : undefined;

function problemParts(problem: unknown): { code: string; reason: string; detail: string } {
  const detail = problem instanceof Refused ? problem.refusal.refusal + ': ' + problem.refusal.reason
    : problem instanceof Error ? problem.message : String(problem);
  const words = detail.replace(/^Error:\s*/, '');
  const candidate = words.trim();
  const named = problem instanceof Refused ? [problem.refusal.refusal, problem.refusal.reason]
    : /^([A-Za-z][A-Za-z0-9_]*):\s*([\s\S]*)$/.exec(candidate)?.slice(1);
  const bareCode = /^(?:[A-Za-z][A-Za-z0-9]*[A-Z][A-Za-z0-9]*|[a-z][a-z0-9]*(?:_[a-z0-9]+)+)$/.test(candidate);
  const code = named?.[0] ?? (Object.hasOwn(REFUSALS, candidate) || bareCode ? candidate : '');
  let reason = named?.[1] ?? (code ? '' : words);
  // Some services repeat their refusal name inside the reason they send.
  while (code && reason.trimStart().startsWith(code + ':')) reason = reason.trimStart().slice(code.length + 1).trimStart();
  if (code && reason.trim() === code) reason = '';
  return { code, reason, detail };
}

/** The refusal name a problem carries, or empty when it is a plain sentence. */
export const refusalCode = (problem: unknown): string => problemParts(problem).code;

const STOPPED = 'The request was stopped before Lys answered. Check whether it was carried out, then try again.';
const UNEXPLAINED = 'Something went wrong and no reason was given. Try again; if it happens again, ask the administrator to check Lys.';
/** Names the browser gives its own failures: never a refusal from Lys, so never shown as one. */
const BROWSER_FAILURES = new Set(['Error', 'TypeError', 'RangeError', 'ReferenceError', 'SyntaxError', 'NetworkError', 'DOMException', 'NotAllowedError', 'InvalidStateError']);

export function problemWords(problem: unknown): string {
  const { code, reason } = problemParts(problem);
  const name = typeof problem === 'object' && problem !== null && 'name' in problem ? String(problem.name) : '';
  if (name === 'AbortError' || name === 'TimeoutError' || code === 'AbortError' || code === 'TimeoutError') return STOPPED;
  if (code && Object.hasOwn(REFUSALS, code)) return REFUSALS[code];
  if (problem instanceof Refused && problem.status >= 500) return reason.trim() ? 'Lys could not answer: ' + reason : 'Lys could not answer. Ask the administrator to check the service.';
  if (code && BROWSER_FAILURES.has(code)) return UNEXPLAINED;
  if (code) return reason.trim() ? 'Lys refused: ' + reason : 'Lys refused without explaining why.';
  // Never an empty sentence: callers read the words as the sign that something failed.
  return reason.trim() ? reason : UNEXPLAINED;
}

export function ErrorWords({ problem }: { problem: unknown }) {
  const { detail } = problemParts(problem);
  return <div className="why-not" role="alert" title={detail}>
    <p>{problemWords(problem)}</p>
    <details><summary>Error details</summary><p className="refusal-name" style={{ overflowWrap: 'anywhere' }}>{detail}</p></details>
  </div>;
}

export function DirectoryGate<T>({ load, title, ok }: { load: Load<T>; title: string; ok: (data: T) => ReactNode }) {
  return <Gate load={load} title={title} ok={ok} renderError={(problem) => <ErrorWords problem={problem} />} />;
}

/** The one namer of a person or agent: its name linked to its file, never its raw identifier. A screen that already holds the directory passes it; any other reads it here. */
export function IdentityName({ id, people }: { id: string; people?: PeopleView }) {
  return people ? <Named id={id} name={entries(people).find((entry) => entry.id === id)?.display_name} waiting={false} /> : <ReadName id={id} />;
}
function Named({ id, name, waiting }: { id: string; name: string | undefined; waiting: boolean }) {
  return <a href={'#/file/' + encodeURIComponent(id)} title={id}>{name ?? (waiting ? 'Reading name…' : 'Name unavailable')}</a>;
}
function ReadName({ id }: { id: string }) {
  const load = useLoad(api.people, 'identity-name');
  const name = load.status === 'ok' ? entries(load.data).find((entry) => entry.id === id)?.display_name : undefined;
  return <><Named id={id} name={name} waiting={load.status === 'loading'} />{load.status === 'refused' ? <small title={problemParts(load.refused).detail}> {problemWords(load.refused)}</small> : null}</>;
}

export const STATUS: Record<string, string> = { registered: 'Awaiting activation', active: 'Active', suspended: 'Access suspended', retired: 'Permanently retired' };
export const ACTION: Record<string, string> = { activate: 'Activate access', suspend: 'Suspend access', retire: 'Retire permanently', reinstate: 'Restore access', resume: 'Restore access' };
/** The same acts in one word, for a menu or a small button; the full words stay on the form each opens. */
export const ACTION_SHORT: Record<string, string> = { activate: 'Activate', suspend: 'Suspend', retire: 'Retire', reinstate: 'Restore', resume: 'Restore' };
export const PART: Record<string, string> = { responsibilities: 'Responsibilities', goals: 'Goals', practice: 'How the work is done', profile: 'Starting instructions for an agent' };
