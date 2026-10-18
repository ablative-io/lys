/**
 * The Apps screen: every app that signs in with Lys and has its permissions
 * checked here. An app registers itself through the API and nothing it
 * registers takes effect until the administrator approves it on this screen,
 * which shows its name, the return addresses it asked for and its whole schema
 * in words before the button. The approval takes the app's sign-in settings:
 * the exact return addresses and whether it is given the person's name. Those
 * settings are changed here too, for an approved app, and the next sign-in
 * follows the change; nothing is restarted. Approval has Lys secrets make and
 * keep the app's secrets before it takes effect, and nobody is shown them; an
 * approved app is issued client credentials here instead, each shown once, and
 * they are taken back here. A proposed schema change waits
 * here the same way, each change it makes to a role named in words ("adds
 * seat_retire to administrator") so a widening is seen before it is approved
 * (ACCESS-004 R1); a schema's roles are listed under its matrix. The
 * permission template is built and edited here too, and
 * saved through the same routes an upload uses.
 */
import { PageHeader } from '../../shell/PageHeader';
import { useState } from 'react';
import { Refused, api, operationId, useLoad } from '../../api';
import { SchemaMatrix } from './SchemaMatrix';
import type { MatrixKind } from './SchemaMatrix';
import { Gate } from '../signin/Gate';
import { SchemaBuilder, send } from './SchemaBuilder';
import type { SchemaJson } from './SchemaBuilder';
import './apps.css';
import { ClientCredentials, CustodyConfirmed, credentialRecord } from './ClientCredentials';
import type { CredentialRecord, StoredCredentials } from './ClientCredentials';
import { Act } from '../../shell/Act';
import { roleChanges } from './roleChanges';
import { KNOWN_APPS, KnownApps } from './ConnectApp';
import { WhoCanUse } from './WhoCanUse';
import { ErrorWords, IdentityName, problemWords } from '../people/Words';
import type { MeView } from '../../generated';

/** Who made an act on the apps. */
export interface By { kind: 'person' | 'operator' | 'service_account' | 'start'; login?: { provider: string; subject: string }; id?: string }
/** One return address declared rather than saved in the list (DIRECTORY-098): who declared it and when. */
export interface Declared { address: string; by: By; at: number }
/** An approved app's sign-in settings as last set: what the provider admits at its next request. */
export interface SignIn { redirects: string[]; profile: boolean; operation: string; by: By; at: number; declared?: Declared[] }
/** An app as the service answers it. */
export interface AppRecord {
  id: string; name: string; state: 'pending' | 'approved' | 'declined' | 'retired'; redirects: string[];
  sign_in: SignIn | null;
  schema: unknown; version: number; versions: number[];
  pending: { operation: string; replaces: number; schema: unknown; by: By; at: number } | null;
  client_id: string | null; service_account: string | null; client_credentials: CredentialRecord[]; registered_by: By; registered_at: number;
}

const strings = (values: unknown): values is string[] => Array.isArray(values) && values.every((entry) => typeof entry === 'string');
const validBy = (by: unknown): by is By => !!by && typeof by === 'object' && ['person', 'operator', 'service_account', 'start'].includes((by as By).kind);
const validDeclared = (value: unknown): value is Declared => !!value && typeof value === 'object' && typeof (value as Declared).address === 'string' && validBy((value as Declared).by) && Number.isSafeInteger((value as Declared).at);
const validSignIn = (value: unknown): value is SignIn | null => value === null
  || (!!value && typeof value === 'object' && strings((value as SignIn).redirects) && typeof (value as SignIn).profile === 'boolean'
    && typeof (value as SignIn).operation === 'string' && validBy((value as SignIn).by) && Number.isSafeInteger((value as SignIn).at)
    && ((value as SignIn).declared === undefined || (Array.isArray((value as SignIn).declared) && (value as SignIn).declared!.every(validDeclared))));
/** When a declaration was made, as a date and time a person reads the same everywhere. */
const declaredWhen = (at: number): string => new Date(at * 1000).toISOString().replace('T', ' ').slice(0, 16) + ' UTC';
/** The line under a declared address: who declared it (the app itself, or the person by their login) and when. */
function DeclaredBy({ declared }: { declared: Declared }) {
  return <div className="note declared-by">declared by {declared.by.kind === 'service_account' ? 'the app' : who(declared.by)}, {declaredWhen(declared.at)}</div>;
}

/** The app as an answer carries it, checked whole, or thrown with words for the screen. */
function appAnswer(value: unknown, id: string, state: AppRecord['state'], what: string): AppRecord {
  const app = value as Partial<AppRecord> | null;
  const pending = app?.pending;
  const validPending = pending === null || (pending && typeof pending.operation === 'string' && Number.isSafeInteger(pending.replaces) && typeof pending.schema === 'object' && validBy(pending.by) && Number.isSafeInteger(pending.at));
  const validApp = app && app.id === id && app.state === state && typeof app.name === 'string'
    && strings(app.redirects) && validSignIn(app.sign_in) && typeof app.version === 'number' && Number.isSafeInteger(app.version)
    && Array.isArray(app.versions) && app.versions.every((version) => Number.isSafeInteger(version))
    && app.schema !== null && typeof app.schema === 'object' && !Array.isArray(app.schema) && validPending
    && (app.client_id === null || typeof app.client_id === 'string')
    && (app.service_account === null || typeof app.service_account === 'string')
    && Array.isArray(app.client_credentials) && app.client_credentials.every(credentialRecord)
    && validBy(app.registered_by) && typeof app.registered_at === 'number';
  if (!app || !validApp) throw new Error('Lys answered, but did not confirm this app’s ' + what + '. Open Apps again to check its saved state before trying again.');
  return app as AppRecord;
}

interface ApprovalAnswer { app: AppRecord; credentials: StoredCredentials | null }
function approvalAnswer(value: unknown, id: string): ApprovalAnswer {
  const answer = value as Partial<ApprovalAnswer> | null;
  const app = appAnswer(answer?.app, id, 'approved', 'approval');
  const credentials = answer?.credentials ?? null;
  const validCredentials = credentials === null || (credentials && credentials.app === id && typeof credentials.client_secret_ref === 'string' && typeof credentials.api_credential_ref === 'string');
  if (!validCredentials) throw new Error('Lys answered, but did not confirm this app’s approval. Open Apps again to check its saved state before approving again.');
  return { app, credentials: credentials ?? null };
}

const LYS = 'lys';
/** Who made an act, in words: a sign-in by the account name its provider gave, never a raw id. */
export const who = (by: By) => (by.kind === 'operator' ? 'the install operator for ' + (by.login?.subject ?? 'the administrator') : by.kind === 'person' ? by.login?.subject ?? 'a person' : by.kind === 'service_account' ? 'a service account' : 'Lys at start');
/** Whether `by` signed in with one of the signed-in person's own sign-ins: the same account, or the one account of that name they hold. */
const isMe = (by: By, me: MeView | null): boolean => {
  if (!me || !by.login || (by.kind !== 'person' && by.kind !== 'operator')) return false;
  const login = by.login;
  if (me.sign_in_identities.some((held) => held.provider === login.provider && held.subject === login.subject)) return true;
  return me.sign_in_identities.filter((held) => held.subject === login.subject).length === 1;
};
/** Who made an act, named through the directory where Lys knows them: the signed-in person by name and file, anyone else in words. */
function ByName({ by, me }: { by: By; me: MeView | null }) {
  if (!me || !isMe(by, me)) return <>{who(by)}</>;
  return by.kind === 'operator' ? <>the install operator for <IdentityName id={me.person.id} /></> : <IdentityName id={me.person.id} />;
}
/** An app's state as a person reads it. */
export const STATE_WORDS: Record<AppRecord['state'], string> = { pending: 'Waiting for your approval', approved: 'Approved', declined: 'Declined', retired: 'Retired' };
const path = (id: string, rest: string) => '/apps/' + encodeURIComponent(id) + rest;
/** A refusal as a sentence a person reads: the plain reason only, never its code. A screen that can fold the code away shows ErrorWords instead. */
export const refusalWords = (error: unknown): string => problemWords(error);
/** A refusal with its name said once in its details: Lys words some reasons as "name: words" already. */
const namedOnce = (error: unknown): unknown => {
  if (!(error instanceof Refused)) return error;
  const { refusal, reason } = error.refusal;
  return reason.startsWith(refusal + ': ') ? new Refused(error.status, { ...error.refusal, reason: reason.slice(refusal.length + 2) }) : error;
};
const lines = (text: string) => text.split('\n').map((line) => line.trim()).filter(Boolean);

type SchemaShape = { kinds?: Record<string, MatrixKind>; relations?: Record<string, string[]> } | null;

/** The schema of `app` as the one permissions matrix. Lys's own actions read as the model's sentences. */
export function SchemaTable({ app, schema, label }: { app: string; schema: unknown; label: string }) {
  const value = schema as SchemaShape;
  if (app === LYS && value?.relations) return <LysSchema relations={value.relations} label={label} />;
  return <SchemaMatrix kinds={Object.entries(value?.kinds ?? {})} label={label} />;
}

/** Lys's own schema: every relation on every kind of Lys's own, each action in the words the permission model gives it. */
function LysSchema({ relations, label }: { relations: Record<string, string[]>; label: string }) {
  const model = useLoad(api.model, 'permission-model');
  const sentences = model.status === 'ok' ? model.data.action_sentences : {};
  return <SchemaMatrix kinds={[['Every kind of Lys’s own', { relations }]]} label={label} sentence={(action) => sentences[action]} />;
}

export function Apps() {
  const [revision, setRevision] = useState(0);
  const [approved, setApproved] = useState<Record<string, AppRecord>>({});
  const [notice, setNotice] = useState('');
  // What approval's custody answered, shown until the page is left.
  const [stored, setStored] = useState<Record<string, StoredCredentials>>({});
  const load = useLoad(() => send<{ apps: AppRecord[] }>('GET', '/apps'), 'apps:' + revision);
  const changed = (message: string) => { setApproved({}); setNotice(message); setRevision((value) => value + 1); };
  const [picked, setPicked] = useState<string | null>(null);
  const [editing, setEditing] = useState<string | null>(null);
  // The signed-in person, so an act they made is named as them; without it every act is still said in words.
  const self = useLoad(api.me, 'me');
  const me = self.status === 'ok' ? self.data : null;
  return <div className="page fill apps">
    <PageHeader title="Apps" subtitle="Apps such as Cambium sign people in with Lys, and Lys decides what each person and agent may do in them. Connect an app below; nothing it asks for takes effect until you approve it here." />
    {notice ? <p role="status">{notice}</p> : null}
    <Gate load={load} title="Apps" ok={(answer) => {
      const apps = answer.apps.map((app) => approved[app.id] ?? app);
      const open = apps.find((app) => app.id === picked) ?? apps.find((app) => app.state === 'pending') ?? apps[0] ?? null;
      const edited = apps.find((app) => app.id === editing);
      if (edited) return <div className="pane"><section className="card" aria-label={'Permission template of ' + edited.id}>
        <div className="row"><h3>Permission template of {edited.name}</h3><Act symbol="close" name="Close the template" word="Close" onClick={() => setEditing(null)} /></div>
        <SchemaBuilder app={edited.id} version={edited.version} initial={edited.schema as SchemaJson} onSaved={(message) => { setEditing(null); changed(message); }} />
      </section></div>;
      // The app is chosen above; its card and its matrix take the whole width below and scroll inside their own pane.
      return <>
        <KnownApps apps={apps} />
        {apps.length ? <nav className="app-picker" aria-label="Apps">
          <span className="note">Apps on this Lys</span>
          {apps.map((app) => <button type="button" key={app.id} className={'app-choice' + (app.id === open?.id ? ' on' : '')} aria-pressed={app.id === open?.id} onClick={() => setPicked(app.id)}>
{app.name}</button>)}
        </nav> : null}
        <div className="pane app-pane">
          <details className="card" aria-label="Add your own app (for developers)"><summary>Add your own app (for developers)</summary>
            {apps.some((app) => app.id !== LYS && !KNOWN_APPS.some((known) => known.id === app.id)) ? null : <p className="dim">No other apps yet.</p>}
            <Register changed={changed} /></details>
          {open ? <AppCard me={me} edit={() => setEditing(open.id)} key={open.id + ':' + revision} app={open} approved={(answer) => { setApproved((held) => ({ ...held, [answer.id]: answer })); setNotice(answer.name + ' is approved. People can sign in to it now, and can do things there once you give them access, under Who can use ' + answer.name + ' below.'); }} stored={stored[open.id] ?? null} saved={(answer) => setStored((held) => ({ ...held, [open.id]: answer }))} changed={changed} /> : null}
        </div>
      </>;
    }} />
  </div>;
}

/** The sign-in settings' two fields: the return addresses, one to a line, and whether the app is given the person's name. */
function SignInFields({ redirects, setRedirects, profile, setProfile, app, name }: { redirects: string; setRedirects: (text: string) => void; profile: boolean; setProfile: (on: boolean) => void; app: string; name: string }) {
  return <>
    <label className="field app-addresses-field">Return addresses the sign-in may send a person back to, one to a line <span className="hint">An https address may name every machine of a domain with *. as its first label, such as https://*.example.com/signin/return; http addresses are admitted only on this machine.</span><textarea value={redirects} onChange={(event) => setRedirects(event.target.value)} aria-label={'Return addresses of ' + app} /></label>
    <label className="tick" title={app}><input type="checkbox" checked={profile} onChange={(event) => setProfile(event.target.checked)} /> Give {name} the person’s name</label>
  </>;
}

function AppCard({ app, changed, stored, saved, approved, edit, me }: { me: MeView | null; edit: () => void; approved: (answer: AppRecord) => void; stored: StoredCredentials | null; saved: (answer: StoredCredentials) => void; app: AppRecord; changed: (message: string) => void }) {
  const [reason, setReason] = useState('');
  // Retiring ends every client credential the app holds, so it is asked about first, with its reason, and can be cancelled.
  const [retiring, setRetiring] = useState(false);
  const [refusal, setRefusal] = useState<unknown>(null);
  const [busy, setBusy] = useState(false);
  // The settings offered to approve are the registration's; what is sent is what is kept.
  const [redirects, setRedirects] = useState(app.redirects.join('\n'));
  const [profile, setProfile] = useState(false);
  const act = async (route: string, body: Record<string, unknown>, done: (answer: unknown) => void) => {
    setBusy(true); setRefusal(null);
    try { done(await send('POST', path(app.id, route), { operation: operationId(), ...body })); } catch (error) { setRefusal(namedOnce(error)); }
    setBusy(false);
  };
  return <article className="card app-card" aria-label={'App ' + app.id}>
    <h3>{app.name} <span className={'app-state app-' + app.state}>{STATE_WORDS[app.state]}</span></h3>
    <p className="note">Registered by <ByName by={app.registered_by} me={me} />.</p>
    {app.state === 'approved' && app.id !== LYS ? <WhoCanUse app={app.id} name={app.name} /> : null}
    <details aria-label={'Details of ' + app.id}><summary>Details</summary><dl className="app-details">
      <dt>App id</dt><dd className="mono">{app.id}</dd>
      {app.version ? <><dt>Permissions version</dt><dd>{app.version}</dd></> : null}
      {app.client_id ? <><dt>Sign-in client id</dt><dd className="mono">{app.client_id}</dd></> : null}
      {app.service_account ? <><dt>Acts as service account</dt><dd className="mono">{app.service_account}</dd></> : null}
      {app.registered_by.login ? <><dt>Registered with the sign-in</dt><dd className="mono">{app.registered_by.login.subject} at {app.registered_by.login.provider}</dd></> : null}
    </dl></details>
    {app.redirects.length ? <div><b>Return addresses its registration asked for</b><ul className="app-list">{app.redirects.map((address) => <li className="mono" key={address}>{address}</li>)}</ul></div> : null}
    {app.sign_in ? <div aria-label={'Sign-in settings of ' + app.id}><b>Its sign-in, as last set by <ByName by={app.sign_in.by} me={me} /></b>
      <ul className="app-list">{app.sign_in.redirects.map((address) => { const declared = app.sign_in?.declared?.find((entry) => entry.address === address); return <li className="mono" key={address}>{address}{declared ? <DeclaredBy declared={declared} /> : null}</li>; })}</ul>
      <p className="note">{app.sign_in.profile ? 'It is given the person’s name.' : 'It is not given the person’s name.'}</p></div> : null}
    <div><b>{app.state === 'pending' ? 'The permissions it asks for' : 'The permissions it has'}</b><SchemaTable app={app.id} schema={app.schema} label={'Schema of ' + app.id} /></div>
    {stored ? <CustodyConfirmed answer={stored} /> : null}
    {app.state === 'pending' ? <div className="app-actions app-signin">
      <SignInFields app={app.id} name={app.name} redirects={redirects} setRedirects={setRedirects} profile={profile} setProfile={setProfile} />
      <label className="field">Why, if you decline <span className="hint">optional; the app is told</span><input value={reason} onChange={(event) => setReason(event.target.value)} /></label>
      <Act symbol="approve" name={'Approve ' + app.name} word="Approve" tone="primary" disabled={busy} onClick={() => { void act('/approve', { redirects: lines(redirects), profile }, (answer) => { const approval = approvalAnswer(answer, app.id); if (approval.credentials) saved(approval.credentials); approved(approval.app); }); }} />
      <Act symbol="decline" name={'Decline ' + app.name} word="Decline" tone="danger" disabled={busy} onClick={() => { void act('/decline', { reason }, () => changed(app.name + ' was declined and never took effect.')); }} />
    </div> : null}
    {app.pending ? <div className="app-waiting" aria-label={'Change waiting for ' + app.id}>
      <b>A change to its permissions waits for you, from <ByName by={app.pending.by} me={me} /></b>
      <RoleChanges before={app.schema} after={app.pending.schema} app={app.id} />
      <SchemaTable app={app.id} schema={app.pending.schema} label={'Schema waiting for ' + app.id} />
      {app.state === 'pending' ? null : <label className="field">Why, if you decline the change <span className="hint">optional; the app is told</span><input value={reason} onChange={(event) => setReason(event.target.value)} /></label>}
      <Act symbol="approve" name={'Approve the change'} word="Approve" tone="primary" disabled={busy} onClick={() => { void act('/schema/approve', {}, () => changed('The change to ' + app.name + ' is approved.')); }} />
      <Act symbol="decline" name={'Decline the change'} word="Decline" tone="danger" disabled={busy} onClick={() => { void act('/schema/decline', { reason }, () => changed('The change to ' + app.name + ' was declined.')); }} />
    </div> : null}
    {app.state === 'approved' && app.id !== LYS ? <SignInSettings app={app} busy={busy} act={act} changed={changed} /> : null}
    {(app.state === 'approved' || app.state === 'retired') && app.id !== LYS ? <ClientCredentials app={app} /> : null}
    {app.state === 'approved' && app.id !== LYS ? <div className="app-actions">
      <Act symbol="edit" name="Edit the permission template" word="Template" onClick={edit} />
      {retiring ? null : <Act symbol="retire" name={'Retire ' + app.name} word="Retire" tone="danger" disabled={busy} onClick={() => { setReason(''); setRetiring(true); }} />}
    </div> : null}
    {app.state === 'approved' && app.id !== LYS && retiring ? <div className="app-actions app-retire" role="group" aria-label={'Retire ' + app.name + '?'}>
      <p>Retire {app.name}? Nobody can sign in to it with Lys again, and every client credential it holds is refused. Its grants stay readable. This cannot be undone.</p>
      <label className="field">Why <span className="hint">optional; kept with the retirement</span><input value={reason} onChange={(event) => setReason(event.target.value)} /></label>
      <Act symbol="retire" name={'Retire ' + app.name + ' for good'} word="Retire for good" tone="danger" disabled={busy} onClick={() => { void act('/retire', { reason }, () => changed(app.name + ' is retired: every client credential it held is refused, and its grants stay readable.')); }} />
      <Act symbol="close" name={'Keep ' + app.name} word="Cancel" disabled={busy} onClick={() => setRetiring(false)} />
    </div> : null}
    {refusal ? <ErrorWords problem={refusal} /> : null}
  </article>;
}

/** Each role a waiting change adds, removes, widens or narrows, in words, before its matrix. */
function RoleChanges({ before, after, app }: { before: unknown; after: unknown; app: string }) {
  const words = roleChanges(before, after);
  if (!words.length) return null;
  return <div aria-label={'Role changes waiting for ' + app}><b>What it does to roles</b>
    <ul className="app-list">{words.map((sentence) => <li key={sentence}>{sentence}</li>)}</ul></div>;
}

/** Change an approved app's sign-in settings; the next sign-in follows them, with nothing restarted. */
function SignInSettings({ app, busy, act, changed }: { app: AppRecord; busy: boolean; act: (route: string, body: Record<string, unknown>, done: (answer: unknown) => void) => Promise<void>; changed: (message: string) => void }) {
  const [redirects, setRedirects] = useState((app.sign_in?.redirects ?? app.redirects).join('\n'));
  const [profile, setProfile] = useState(app.sign_in?.profile ?? false);
  return <div className="app-actions app-signin" aria-label={'Change the sign-in of ' + app.id}>
    <SignInFields app={app.id} name={app.name} redirects={redirects} setRedirects={setRedirects} profile={profile} setProfile={setProfile} />
    <Act symbol="save" name={'Save the sign-in of ' + app.name} word="Save" tone="primary" disabled={busy} onClick={() => { void act('/sign_in', { redirects: lines(redirects), profile }, (answer) => { const kept = appAnswer(answer, app.id, 'approved', 'sign-in settings'); changed(kept.name + '’s sign-in now sends people back to ' + kept.sign_in?.redirects.join(', ') + (kept.sign_in?.profile ? ' and gives it their name.' : ' and withholds their name.')); }); }} />
  </div>;
}

/** Register an app from this screen: the same route an app registers itself through. */
function Register({ changed }: { changed: (message: string) => void }) {
  const [id, setId] = useState('');
  const [name, setName] = useState('');
  const [redirects, setRedirects] = useState<string[]>(['']);
  const ready = /^[a-z][a-z0-9_]{2,39}$/.test(id) && id !== LYS && name.trim() !== '';
  const register = async (schema: SchemaJson) => {
    await send('POST', '/apps', { operation: operationId(), id, name: name.trim(), redirects: redirects.map((address) => address.trim()).filter(Boolean), schema });
  };
  const idWhy = id === '' ? 'Give the app an id.' : id === LYS ? 'The id lys is Lys’s own. Choose another.'
    : /^[a-z][a-z0-9_]{2,39}$/.test(id) ? '' : 'An id is 3 to 40 characters: lowercase letters, digits and underscores, starting with a letter.';
  return <section className="card" aria-label="Register an app">
    <h3>Register an app</h3>
    <div className="form-grid">
      <label className="field">App id <span className="hint">3 to 40 lowercase letters, digits or underscores; it starts every permission name the app uses, such as notes.document</span><input value={id} onChange={(event) => setId(event.target.value.trim())} autoComplete="off" spellCheck={false} required aria-invalid={id !== '' && idWhy !== ''} /></label>
      <label className="field">Name <span className="hint">as people will see it</span><input value={name} onChange={(event) => setName(event.target.value)} required /></label>
    </div>
    <fieldset className="field"><legend>Return addresses it asks for <span className="hint">where the app’s sign-in sends a person back to</span> <span className="hint">An https address may name every machine of a domain with *. as its first label, such as https://*.example.com/signin/return; http addresses are admitted only on this machine.</span></legend>
      {redirects.map((address, at) => <div className="app-address-row" key={at}>
        <input value={address} type="url" inputMode="url" aria-label={'Return address ' + (at + 1)} placeholder="https://app.example.com/callback" onChange={(event) => setRedirects(redirects.map((held, index) => (index === at ? event.target.value : held)))} />
        {redirects.length > 1 ? <Act symbol="remove" name={'Remove return address ' + (at + 1)} onClick={() => setRedirects(redirects.filter((_, index) => index !== at))} /> : null}
      </div>)}
      <Act symbol="add" name="Add another address" word="Add another address" onClick={() => setRedirects([...redirects, ''])} />
    </fieldset>
    {ready ? <SchemaBuilder key={id} app={id} version={null} register={register} onSaved={changed} /> : <p className="note why-not">{idWhy || 'Give the app a name.'} Its permission template is built here once it has an id and a name.</p>}
  </section>;
}
