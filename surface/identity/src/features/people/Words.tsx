import type { ReactNode } from 'react';
import { api, Refused, useLoad } from '../../api';
import type { Load } from '../../api';
import type { PeopleView } from '../../generated';
import { Gate } from '../signin/Gate';
import { entries } from './directory';

/** The one table of what a named refusal means and what to do next. */
const REFUSALS: Record<string, string> = {
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
  UnconfirmedReceipt: 'The answer did not confirm this change. Use Check whether Lys saved it before sending another.',
  StorageUncertain: 'The change may have been saved, but its outcome is unknown. Use Check whether Lys saved it before sending another.',
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
};

export const refusalWords = (code: string): string | undefined => REFUSALS[code];

export function problemWords(problem: unknown): string {
  const code = problem instanceof Refused ? problem.refusal.refusal : String(problem).split(':')[0];
  if (REFUSALS[code]) return REFUSALS[code];
  if (code.endsWith('Unavailable')) return 'This part of Lys cannot answer. Ask the administrator to check the service. Keep any change whose outcome is unknown.';
  return 'Lys could not complete this request. Ask the administrator to check the error details before repeating a change.';
}

export function ErrorWords({ problem }: { problem: unknown }) {
  const detail = problem instanceof Refused ? problem.refusal.refusal + ': ' + problem.refusal.reason : String(problem);
  return <div className="why-not" role="alert"><p>{problemWords(problem)} <small className="refusal-name" style={{ overflowWrap: 'anywhere' }}>{detail}</small></p></div>;
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
  return <><Named id={id} name={name} waiting={load.status === 'loading'} />{load.status === 'refused' ? <small className="refusal-name"> {load.refused.refusal.refusal}</small> : null}</>;
}

export const STATUS: Record<string, string> = { registered: 'Awaiting activation', active: 'Active', suspended: 'Access suspended', retired: 'Permanently retired' };
export const ACTION: Record<string, string> = { activate: 'Activate access', suspend: 'Suspend access', retire: 'Retire permanently', reinstate: 'Restore access', resume: 'Restore access' };
/** The same acts in one word, for a menu or a small button; the full words stay on the form each opens. */
export const ACTION_SHORT: Record<string, string> = { activate: 'Activate', suspend: 'Suspend', retire: 'Retire', reinstate: 'Restore', resume: 'Restore' };
export const PART: Record<string, string> = { responsibilities: 'Responsibilities', goals: 'Goals', practice: 'How the work is done', profile: 'Starting instructions for an agent' };
