import { useRef, useState } from 'react';
import { api, operationId, Refused, request, useLoad } from '../../api';
import type { GrantModel } from '../../generated/grants';
import { sendKept } from '../../kept';
import { fullName } from './person-name';
import { DirectoryGate, ErrorWords, refusalWords } from './Words';
import { ClearSavedRequest } from './SavedRequest';
import { identity } from '../access/Resources';
import { actionWords } from '../grants/action-words';
import { resourceNames, resourceWords } from './agent-resource-names';
import type { PeopleView } from '../../generated';
import type { AppRecord } from '../apps/Apps';
import type { Team } from '../teams/contract';
import type { ResourcesView } from '../access/Resources';
import './add-agent.css';
import { Act } from '../../shell/Act';
import { PageHeader } from '../../shell/PageHeader';
import { organisationActs, ORGANISATION_ADMIT } from '../grants/organisation-acts';
import type { OrganisationActs } from '../grants/organisation-acts';

/** Adding a person is one act, POST /people/admit: register them, make their sign-in account at the issuer by
 * email, bind it, activate them, when one is chosen issue their first grant, and write to them. A fresh install has
 * nothing to grant yet, so "No access yet" is the default and no grant is sent. The letter is Lys's own when their
 * sign-in account already existed, and the sign-in service's own when it made the account (DIRECTORY-110). The answer
 * is one receipt naming each step and the log leaf it was recorded at, and who wrote to them; asking again with the
 * saved request answers the same receipt and makes nothing twice. */
/** A saved request: `name` is the name people see, made from `given` and `family`; a request saved before the two
 * were asked for apart carries `name` alone. */
interface PendingPerson { name: string; given?: string; family?: string; email: string; kind: string; id: string; relation: string; operation: string }
type Asking = { given: string; family: string; email: string; kind: string; id: string; relation: string };
interface Logged { step: string; log: string; index: number }
/** The letter the act owed: who sent it (Lys, or the sign-in service when it made the account) and where. */
interface Letter { letter: string; display_name?: string; to?: string; sent: boolean; by: 'lys' | 'issuer'; reason?: string; handed?: string }
interface Existing { person: string; display_name: string }
interface Admitted { person: string; displayName: string | null; existing: Existing | null; logged: Logged[]; letter: Letter }
const STEPS = ['register', 'account', 'bind', 'activate', 'grant', 'letter'];
/** Each step as the act under way, for saying where a refusal stopped. */
const DOING: Record<string, string> = { register: 'registering them', account: 'finding or making their sign-in account', bind: 'binding their sign-in', activate: 'activating them', grant: 'giving their first access', letter: 'writing to them' };
const WORDS: Record<string, string> = { register: 'Registered', account: 'Sign-in account found or made', bind: 'Sign-in bound', activate: 'Activated', grant: 'First grant issued', letter: 'Letter sent' };

function object(value: unknown): value is Record<string, unknown> { return value !== null && typeof value === 'object' && !Array.isArray(value); }
function filled(value: Record<string, unknown>, key: string): string | null {
  const field = value[key];
  return typeof field === 'string' && field.trim() ? field : null;
}
function restored(key: string): PendingPerson | null {
  const raw = sessionStorage.getItem(key);
  if (raw === null) return null;
  const value: unknown = JSON.parse(raw);
  const unreadable = new Error('The saved request cannot be read. Check whether the person was added before submitting another request.');
  if (!object(value)) throw unreadable;
  const [name, email, kind, id, relation, operation] = ['name', 'email', 'kind', 'id', 'relation', 'operation'].map((each) => filled(value, each));
  // A saved request either names a whole first grant or none at all.
  const granting = Boolean(kind || id || relation);
  if (!name || !email || !operation || !/^op-[0-9a-f]{32}$/.test(operation) || (granting && !(kind && id && relation))) throw unreadable;
  const [given, family] = [filled(value, 'given'), filled(value, 'family')];
  return { name, email, kind: kind ?? '', id: id ?? '', relation: relation ?? '', operation, ...(given && family ? { given, family } : {}) };
}

/** The letter member of an answer, when it is one: which letter, to whom, whether it went and who sent it. */
function letterOf(value: unknown): Letter | null {
  if (!object(value) || typeof value.letter !== 'string' || typeof value.sent !== 'boolean' || (value.by !== 'lys' && value.by !== 'issuer')
    || (value.display_name !== undefined && typeof value.display_name !== 'string')
    || (value.to !== undefined && typeof value.to !== 'string') || (value.reason !== undefined && typeof value.reason !== 'string')
    || (value.handed !== undefined && typeof value.handed !== 'string')) return null;
  return { letter: value.letter, ...(typeof value.display_name === 'string' ? { display_name: value.display_name } : {}), sent: value.sent, by: value.by as 'lys' | 'issuer', ...(typeof value.to === 'string' ? { to: value.to } : {}),
    ...(typeof value.reason === 'string' ? { reason: value.reason } : {}), ...(typeof value.handed === 'string' ? { handed: value.handed } : {}) };
}

/** The act's answer, confirmed for `operation`: every step completed, none failed, each written step's leaf named, the letter sent or its link handed (DIRECTORY-112). */
function confirmed(answer: unknown, operation: string, granting: boolean): Admitted {
  const steps = granting ? STEPS : STEPS.filter((step) => step !== 'grant');
  const unconfirmed = new Refused(200, { refusal: 'UnconfirmedReceipt', reason: 'The answer did not confirm this person was added. The original request is saved.' });
  if (!object(answer) || typeof answer.person !== 'string' || !/^person-[0-9a-f]{32}$/.test(answer.person) || !object(answer.receipt)) throw unconfirmed;
  const receipt = answer.receipt;
  const logged = receipt.logged;
  if (receipt.operation !== operation || receipt.person !== answer.person || (receipt.failed !== null && receipt.failed !== undefined)
    || !Array.isArray(receipt.completed) || receipt.completed.join(' ') !== steps.join(' ') || !Array.isArray(logged)
    || !logged.every((leaf: unknown) => object(leaf) && typeof leaf.step === 'string' && typeof leaf.log === 'string'
      && typeof leaf.index === 'number' && Number.isSafeInteger(leaf.index) && leaf.index >= 0)) throw unconfirmed;
  const letter = letterOf(answer.letter);
  if (!letter || (!letter.sent && letter.handed === undefined)) throw unconfirmed;
  const displayName = filled(answer, 'display_name');
  let existing: Existing | null = null;
  if (receipt.existing !== undefined && receipt.existing !== null) {
    if (!object(receipt.existing) || receipt.existing.person !== answer.person || !filled(receipt.existing, 'display_name')) throw unconfirmed;
    existing = { person: answer.person, display_name: receipt.existing.display_name as string };
    if (displayName !== null && displayName !== existing.display_name) throw unconfirmed;
  }
  return { person: answer.person, displayName, existing, logged: logged as Logged[], letter };
}

/** What a refused act had already written: the step that failed and each earlier step's log leaf (`AdmitReceipt`). */
interface Stopped { refusal: string; reason: string; failed: string; completed: string[]; logged: Logged[] }

/** The receipt a refusal of the act carries for `operation`, or null when it carries none this act can read as its own. */
export function stoppedAt(error: unknown, operation: string | undefined): Stopped | null {
  if (!(error instanceof Refused) || !object(error.answer) || !object(error.answer.receipt)) return null;
  const receipt = error.answer.receipt;
  const logged = receipt.logged;
  if ((operation !== undefined && receipt.operation !== operation) || typeof receipt.failed !== 'string'
    || !Array.isArray(receipt.completed) || !receipt.completed.every((step: unknown) => typeof step === 'string')
    || !Array.isArray(logged) || !logged.every((leaf: unknown) => object(leaf) && typeof leaf.step === 'string' && typeof leaf.log === 'string'
      && typeof leaf.index === 'number' && Number.isSafeInteger(leaf.index) && leaf.index >= 0)) return null;
  return { refusal: error.refusal.refusal, reason: error.refusal.reason, failed: receipt.failed, completed: receipt.completed as string[], logged: logged as Logged[] };
}

/** The refusal's reason as a sentence: the refusal's own name, which some services repeat at its start, is left for Error details. */
function spoken(stopped: Stopped): string {
  let reason = stopped.reason.trim();
  while (reason.startsWith(stopped.refusal + ':')) reason = reason.slice(stopped.refusal.length + 1).trim();
  return refusalWords(stopped.refusal) ?? (reason || 'it gave no reason.');
}

/** A refused act said whole: its refusal by name and reason, the step it stopped at, and the leaf of each step already written. */
function RefusedReceipt({ stopped }: { stopped: Stopped }) {
  return <section aria-label="Refused receipt" className="why-not" role="alert">
    <p>{stopped.failed === 'letter' ? 'Added, but the letter was not sent.' : 'Not added.'} Lys refused while {DOING[stopped.failed] ?? 'adding them'}: {spoken(stopped)}</p>
    <details><summary>Error details</summary><p className="refusal-name" style={{ overflowWrap: 'anywhere' }}>{stopped.refusal}: {stopped.reason}</p></details>
    {stopped.logged.length
      ? <><p>These steps are already done and kept. Asking again finishes from here and does none of them twice:</p>
        <ul>{stopped.logged.map((leaf) => <li key={leaf.log + leaf.index} data-step={leaf.step}>{WORDS[leaf.step] ?? 'A step Lys did not name'}</li>)}</ul>
        <details><summary>Where Lys recorded them</summary><ul>{stopped.logged.map((leaf) => <li key={leaf.log + leaf.index}>{WORDS[leaf.step] ?? leaf.step}: {leaf.log} log, entry {leaf.index}</li>)}</ul></details></>
      : <p>Nothing was done before it.</p>}
  </section>;
}

/** Who wrote to the person, as the receipt says it: Lys itself, or the sign-in service when it made their account. */
function wrote(letter: Letter, name: string, email: string): string {
  if (letter.handed !== undefined) return name + ' is added. This Lys cannot send email, so give them this link; opening it signs them in once, and they set a password there.';
  return letter.by === 'issuer'
    ? name + ' is added. The sign-in service wrote to them at ' + email + ' with their first sign-in.'
    : name + ' is added. Lys wrote to them at ' + email + '.';
}

function Receipt({ admitted, name, email }: { admitted: Admitted; name: string; email: string }) {
  const granted = admitted.logged.some((leaf) => leaf.step === 'grant');
  const actualName = admitted.letter.display_name ?? admitted.existing?.display_name ?? admitted.displayName ?? name;
  const reuse = admitted.existing ? `${email} was already ${actualName}; ${actualName} now holds ${granted ? 'the access just given' : 'their existing access'}.` : null;
  return <section aria-label="Receipt" className="note">
    <p role="status">{reuse ?? wrote(admitted.letter, actualName, admitted.letter.to ?? email)}</p>
    {admitted.letter.handed !== undefined ? <div className="field">
      <p>This link signs in as {actualName}.</p>
      <label htmlFor="add-person-handed">Their sign-in link</label>
      <input id="add-person-handed" aria-label="Their sign-in link" type="text" readOnly value={admitted.letter.handed} onFocus={(event) => event.currentTarget.select()} />
    </div> : null}
    {granted || admitted.existing ? null : <p>They have no access yet. Give it from Access once there is something to give.</p>}
    <p><a href={'#/file/' + encodeURIComponent(admitted.person)}>Open their file</a></p>
    <details><summary>What Lys recorded</summary><ul>
      {admitted.logged.map((leaf) => <li key={leaf.log + leaf.index}>{WORDS[leaf.step] ?? leaf.step}: {leaf.log} log, entry {leaf.index}</li>)}
      <li>Sign-in account: the issuer account for their email, found or made there; the issuer keeps that record</li>
      {admitted.letter.handed !== undefined
        ? <li>Letter: handed to you here to give them; nothing was sent</li>
        : <li>Letter: {admitted.letter.by === 'issuer' ? 'the sign-in service wrote to them when it made their account' : 'Lys wrote to them'}; the send is recorded beside the directory, not in a log</li>}
    </ul></details>
  </section>;
}

/** A failure said with its receipt when the refusal carries one, else by its name and words alone. */
function Failure({ failure, operation }: { failure: unknown; operation: string | undefined }) {
  if (failure instanceof Refused && failure.refusal.refusal === 'EmailHeldByAnother' && object(failure.answer)) {
    const { person, display_name: displayName, email } = failure.answer;
    if (typeof person === 'string' && typeof displayName === 'string' && typeof email === 'string') {
      return <section className="why-not" role="alert"><p>{email} already signs in as <a href={'#/file/' + encodeURIComponent(person)}>{displayName}</a>.</p></section>;
    }
  }
  const stopped = stoppedAt(failure, operation);
  return stopped ? <RefusedReceipt stopped={stopped} /> : <ErrorWords problem={failure} />;
}

/** The kinds a first grant may name and, for each, the resources of it the caller sees; a person or an agent is never a first grant's resource. */
function grantable(view: ResourcesView): Map<string, string[]> {
  const kinds = new Map<string, string[]>();
  for (const resource of view.resources) {
    if (identity(resource.id)) continue;
    kinds.set(resource.kind, [...(kinds.get(resource.kind) ?? []), resource.id]);
  }
  return new Map([...kinds].sort(([a], [b]) => a.localeCompare(b)).map(([kind, ids]) => [kind, ids.sort()]));
}

/** The relations a first grant may name on `kind`: an app kind's are the app's own, as its approved schema declares
 * them in the grant model (DIRECTORY-116 R1); one of Lys's own kinds offers Lys's model relations. */
export const relationsOf = (model: GrantModel, kind: string): Record<string, string[]> => model.kinds?.[kind]?.relations ?? model.relations;

/** A relation choice in words: an app's relation by its own name, then what it carries; Lys's own by what it carries. */
export const relationWords = (model: GrantModel, resource: { kind: string; id: string }, relation: string, actions: string[]): string =>
  resource.kind.includes('.') ? relation.replaceAll('_', ' ') + ': ' + actionWords(model, resource, actions) : actionWords(model, resource, actions);

/** The relation an app declares as a newly admitted person's first on `kind`; none when it declares none (DIRECTORY-116 R2). */
export const firstGrantOf = (model: GrantModel, kind: string): string => model.kinds?.[kind]?.first_grant ?? '';

/** The app a kind belongs to, by the name Lys holds for it, else by its prefix. */
export const appOf = (apps: Map<string, string>, kind: string): string => {
  const dot = kind.indexOf('.');
  const prefix = dot < 0 ? kind : kind.slice(0, dot);
  return apps.get(prefix) ?? prefix;
};

/** A kind said as words: an app's `app.kind` reads as two words, underscores as spaces. */
export const kindWords = (kind: string) => kind.replaceAll('_', ' ').replaceAll('.', ' ');

/** The legacy key an earlier Lys kept an unanswered add under; nothing writes it now, and it cannot be sent again. */
const LEGACY = 'lys.pending.register-person';

function savedState(key: string): { pending: PendingPerson | null; error: unknown } {
  try {
    if (sessionStorage.getItem(LEGACY) !== null) throw new Error('SavedRequestUnreadable: an earlier request to add a person was kept in a form this Lys cannot send again.');
    return { pending: restored(key), error: null };
  } catch (error) { return { pending: null, error }; }
}

/** What stops Add, said beside it; empty when nothing does. */
export function missing(asking: Asking, granting: boolean): string {
  const unnamed = [asking.given.trim() ? '' : 'first name', asking.family.trim() ? '' : 'last name', asking.email.trim() ? '' : 'email'].filter(Boolean);
  if (unnamed.length) return 'Enter their ' + (unnamed.length === 1 ? unnamed[0] : unnamed.slice(0, -1).join(', ') + ' and ' + unnamed[unnamed.length - 1]) + '.';
  if (granting && !asking.id) return 'Choose which one they get access to, or choose No access yet.';
  if (granting && !asking.relation) return 'Choose what they may do there, or choose No access yet.';
  return '';
}

/** Why the form is not open: the right not held, who holds it, and what to do instead (ADR-011). */
function NotOpen({ acts }: { acts: OrganisationActs }) {
  const add = <>You can add agents of your own: <a href="#/agents/new">Add an agent</a>.</>;
  if (!acts.organisation) return <p>Lys names no organisation for you yet, so this form is not open to you: the Lys administrator brings the directory under one, and people are added within it. {add}</p>;
  return <p>Adding a person to {acts.organisation.display_name} needs {ORGANISATION_ADMIT} there, which you do not hold, so this form is not open to you. Whoever holds it adds people: send them the person's name and email address. {add}</p>;
}

function PersonForm({ person, organisation, model, resources, names, apps }: { person: string; organisation: string; model: GrantModel; resources: ResourcesView; names: Map<string, string>; apps: Map<string, string> }) {
  const choices = grantable(resources);
  const key = 'lys.add-person.' + person;
  const [saved, setSaved] = useState(() => savedState(key));
  const [pending, setPending] = useState(saved.pending);
  const [asking, setAsking] = useState<Asking>(() => {
    const held = saved.pending;
    if (!held) return { given: '', family: '', email: '', kind: '', id: '', relation: '' };
    return { given: held.given ?? held.name, family: held.family ?? '', email: held.email, kind: held.kind, id: held.id, relation: held.relation };
  });
  const [failure, setFailure] = useState<unknown>(saved.error);
  const [admitted, setAdmitted] = useState<{ admitted: Admitted; name: string; email: string } | null>(null);
  const [busy, setBusy] = useState(false);
  const sending = useRef(false);
  // A saved request is sent again as it was saved, whatever the fields now hold.
  // The first grant is optional; when a kind is chosen, the resource and how they hold it must be chosen too.
  const granting = Boolean(asking.kind);
  // The app's declared first grant is preselected when its kind is chosen; the person adding may change it.
  const usual = granting && asking.relation !== '' && asking.relation === firstGrantOf(model, asking.kind);
  const lacking = pending === null ? missing(asking, granting) : '';
  const complete = pending !== null || !lacking;
  const set = (field: keyof typeof asking) => (event: { target: { value: string } }) => setAsking({ ...asking, [field]: event.target.value });
  const locked = busy || pending !== null || Boolean(saved.error);
  const submit = async () => {
    if (sending.current || saved.error || !complete) return;
    sending.current = true; setBusy(true); setFailure(null);
    try {
      const given = asking.given.trim();
      const family = asking.family.trim();
      const asked: PendingPerson = pending ?? { ...asking, ...(granting ? {} : { kind: '', id: '', relation: '' }), name: fullName(given, family), given, family, email: asking.email.trim(), operation: operationId() };
      const first = Boolean(asked.kind && asked.id && asked.relation);
      const answer = await sendKept(key, asked, async () => {
        setPending(asked);
        return confirmed(await request<unknown>('/people/admit', {
          operation: asked.operation, display_name: asked.name,
          ...(asked.given && asked.family ? { given_name: asked.given, family_name: asked.family } : {}), email: asked.email,
          ...(first ? { grant: { route: 'browser', resource: { kind: asked.kind, id: asked.id }, relation: asked.relation } } : {}),
        }), asked.operation, first);
      }, false);
      setPending(null); setAdmitted({ admitted: answer, name: asked.name, email: asked.email });
    } catch (error) { setFailure(error); }
    finally { sending.current = false; setBusy(false); }
  };
  if (admitted) return <Receipt admitted={admitted.admitted} name={admitted.name} email={admitted.email} />;
  const clearSaved = saved.error ? <ClearSavedRequest what="request to add a person" keys={[LEGACY, key]}
    check={<>Look for them in <a href="#/people">People and agents</a> first. If they are there, they were added; if not, clear it and add them again. Adding them again with the same email finds their sign-in account rather than making a second one.</>}
    cleared={() => { setSaved(savedState(key)); setFailure(null); setPending(null); }} /> : null;
  return <form aria-label="Add a person" className="add-person-form" onSubmit={(event) => { event.preventDefault(); void submit(); }}>
    <p className="note">They join {organisation}.</p>
    <div className="form-grid">
    {pending && !(pending.given && pending.family)
      ? <label className="field wide">Name<input name="display_name" value={pending.name} readOnly disabled /></label>
      : <>
        <label className="field">First name<input name="given_name" autoFocus autoComplete="given-name" required value={asking.given} disabled={locked} onChange={set('given')} /></label>
        <label className="field">Last name<input name="family_name" autoComplete="family-name" required value={asking.family} disabled={locked} onChange={set('family')} /></label>
      </>}
    <label className="field wide">Email<input name="email" type="email" autoComplete="email" required value={asking.email} disabled={locked} onChange={set('email')} /><span className="hint">Their sign-in name. Lys makes their sign-in account with it, or finds the one they already have.</span></label>
    <label className="field wide">Give them access to<select name="kind" value={asking.kind} disabled={locked} onChange={(event) => setAsking({ ...asking, kind: event.target.value, id: '', relation: firstGrantOf(model, event.target.value) })}>
      <option value="">No access yet</option>
      {[...choices.keys()].map((kind) => <option key={kind} value={kind}>{kindWords(kind)}</option>)}
    </select></label>
    {choices.size ? <p className="note">Optional. More can be given from Access later.</p>
      : <p className="note">Nothing can be given yet, so they start with no access. Connect an app on <a href="#/apps">Apps</a>; what it shares appears here and on Access once it is approved.</p>}
    {granting ? <>
      <label className="field">Which one<select name="resource" value={asking.id} disabled={locked} onChange={set('id')}>
        <option value="" disabled>Choose one</option>
        {(choices.get(asking.kind) ?? []).map((id) => <option key={id} value={id}>{resourceWords({ kind: asking.kind, id }, names).words}</option>)}
      </select></label>
      <label className="field">As<select name="relation" value={asking.relation} disabled={locked} onChange={set('relation')}>
        <option value="" disabled>Choose what they may do</option>
        {Object.entries(relationsOf(model, asking.kind)).map(([relation, actions]) => <option key={relation} value={relation}>{relationWords(model, { kind: asking.kind, id: asking.id }, relation, actions)}</option>)}
      </select>{usual ? <span className="hint">{appOf(apps, asking.kind)}&apos;s usual first access</span> : null}</label>
    </> : null}
    </div>
    <div className="add-person-act">
    <Act symbol="add" name={busy ? 'Adding…' : pending ? 'Continue adding this person' : 'Add person'} word={busy ? 'Adding…' : pending ? 'Continue' : 'Add'} tone="primary" type="submit" disabled={busy || !complete || Boolean(saved.error)} aria-describedby={lacking ? 'add-person-missing' : undefined} />
    {!busy && !saved.error && lacking ? <p className="why-not" role="status" id="add-person-missing">{lacking}</p> : null}
    </div>
    {pending ? <p role="status">This request has no confirmed answer yet. Its original details are saved; trying again checks the same request.</p> : null}
    {clearSaved ?? (failure ? <Failure failure={failure} operation={pending?.operation} /> : null)}
  </form>;
}

export function AddPerson() {
  const load = useLoad(async () => {
    const [me, people, list, model, resources, teams, apps] = await Promise.all([api.me(), api.people(), api.grants(), api.model(), request<ResourcesView>('/resources'),
      // Team names only make the choices readable; without them a team is said by its kind and id.
      request<{ teams: Team[] }>('/teams').then((answer) => answer.teams, () => [] as Team[]),
      // App names only make the declared first grant readable; without them an app is said by its kinds' prefix.
      request<{ apps: AppRecord[] }>('/apps').then((answer) => new Map(answer.apps.map((app) => [app.id, app.name])), () => new Map<string, string>())]);
    return { me, acts: organisationActs(me, list.grants), model, resources, names: resourceNames(people as PeopleView, teams, null), apps };
  }, 'add-person');
  return <div className="page"><PageHeader breadcrumb={<a href="#/people">People and agents</a>} title="Add a person" subtitle="Add someone to the directory. Access is optional; they can start with none." />
    <DirectoryGate load={load} title="Add a person" ok={(data) => data.acts.admit && data.acts.organisation
      ? <PersonForm person={data.me.person.id} organisation={data.acts.organisation.display_name} model={data.model} resources={data.resources} names={data.names} apps={data.apps} /> : <NotOpen acts={data.acts} />} />
  </div>;
}
