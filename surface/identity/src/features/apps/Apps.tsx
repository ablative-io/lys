/**
 * The Apps screen: every app that signs in with Lys and has its permissions
 * checked here. An app registers itself through the API and nothing it
 * registers takes effect until the administrator approves it on this screen,
 * which shows its name, the return addresses it asked for and its whole schema
 * in words before the button. The approval takes the app's sign-in settings:
 * the exact return addresses and whether it is given the person's name. Those
 * settings are changed here too, for an approved app, and the next sign-in
 * follows the change; nothing is restarted. Broker-backed approval saves
 * credentials in Lys secrets before activation. A proposed schema change waits
 * here the same way. The permission template is built and edited here too, and
 * saved through the same routes an upload uses.
 */
import { ConfigTabs } from '../settings/ConfigTabs';
import { useState } from 'react';
import { Refused, api, operationId, useLoad } from '../../api';
import { SchemaMatrix } from './SchemaMatrix';
import type { MatrixKind } from './SchemaMatrix';
import { Gate } from '../signin/Gate';
import { SchemaBuilder, send } from './SchemaBuilder';
import type { SchemaJson } from './SchemaBuilder';
import './apps.css';
import { SaveCredentials, SavedCredentials } from './SaveCredentials';
import type { StoredCredentials } from './SaveCredentials';
import { Act } from '../../shell/Act';

/** Who made an act on the apps. */
export interface By { kind: 'person' | 'operator' | 'service_account' | 'start'; login?: { provider: string; subject: string }; id?: string }
/** An approved app's sign-in settings as last set: what the provider admits at its next request. */
export interface SignIn { redirects: string[]; profile: boolean; operation: string; by: By; at: number }
/** An app as the service answers it. */
export interface AppRecord {
  id: string; name: string; state: 'pending' | 'approved' | 'declined' | 'retired'; redirects: string[];
  sign_in: SignIn | null;
  schema: unknown; version: number; versions: number[];
  pending: { operation: string; replaces: number; schema: unknown; by: By; at: number } | null;
  client_id: string | null; service_account: string | null; registered_by: By; registered_at: number;
}
/** The client an approval creates, answered once. */
export interface ClientIssued { client_id: string; client_secret: string; credential: string }

const strings = (values: unknown): values is string[] => Array.isArray(values) && values.every((entry) => typeof entry === 'string');
const validBy = (by: unknown): by is By => !!by && typeof by === 'object' && ['person', 'operator', 'service_account', 'start'].includes((by as By).kind);
const validSignIn = (value: unknown): value is SignIn | null => value === null
  || (!!value && typeof value === 'object' && strings((value as SignIn).redirects) && typeof (value as SignIn).profile === 'boolean'
    && typeof (value as SignIn).operation === 'string' && validBy((value as SignIn).by) && Number.isSafeInteger((value as SignIn).at));

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
    && validBy(app.registered_by) && typeof app.registered_at === 'number';
  if (!app || !validApp) throw new Error('Lys answered, but did not confirm this app’s ' + what + '. Open Apps again to check its saved state before trying again.');
  return app as AppRecord;
}

interface ApprovalAnswer { app: AppRecord; client: ClientIssued | null; credentials: StoredCredentials | null }
function approvalAnswer(value: unknown, id: string): ApprovalAnswer {
  const answer = value as Partial<ApprovalAnswer> | null;
  const app = appAnswer(answer?.app, id, 'approved', 'approval');
  const client = answer?.client ?? null;
  const credentials = answer?.credentials ?? null;
  const validClient = client === null || (client && typeof client.client_id === 'string' && typeof client.client_secret === 'string' && typeof client.credential === 'string');
  const validCredentials = credentials === null || (credentials && credentials.app === id && typeof credentials.client_secret_ref === 'string' && typeof credentials.api_credential_ref === 'string');
  if (!validClient || !validCredentials) throw new Error('Lys answered, but did not confirm this app’s approval. Open Apps again to check its saved state before approving again.');
  return { app, client: client ?? null, credentials: credentials ?? null };
}

const LYS = 'lys';
const who = (by: By) => (by.kind === 'operator' ? 'the install operator for ' + (by.login?.subject ?? 'the administrator') : by.kind === 'person' ? by.login?.subject ?? 'a person' : by.kind === 'service_account' ? 'service account ' + (by.id ?? '') : 'Lys at start');
const path = (id: string, rest: string) => '/apps/' + encodeURIComponent(id) + rest;
/** A refusal in Lys's words, its name said once: Lys words each reason as "name: words" already. */
const refusalWords = (error: unknown) => {
  if (!(error instanceof Refused)) return String(error);
  const { refusal, reason } = error.refusal;
  return reason.startsWith(refusal + ': ') ? reason : refusal + ': ' + reason;
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
  const [registering, setRegistering] = useState(false);
  // An approval's secret stays only in memory until the encrypted broker confirms it.
  const [issued, setIssued] = useState<Record<string, ClientIssued>>({});
  const [stored, setStored] = useState<Record<string, StoredCredentials>>({});
  const load = useLoad(() => send<{ apps: AppRecord[] }>('GET', '/apps'), 'apps:' + revision);
  const changed = (message: string) => { setApproved({}); setNotice(message); setRegistering(false); setRevision((value) => value + 1); };
  const [picked, setPicked] = useState<string | null>(null);
  const [editing, setEditing] = useState<string | null>(null);
  return <div className="page fill apps">
    <div className="head"><div><h1>Configuration</h1><p className="sub">Every app that signs in with Lys and has its permissions checked here. An app registers itself through the API; nothing it registers takes effect until you approve it here.</p></div>
      <Act symbol={registering ? 'close' : 'add'} name={registering ? 'Close the new app' : 'Register an app'} word={registering ? 'Close' : 'Register'} tone="primary" onClick={() => setRegistering(!registering)} /></div>
    <ConfigTabs on="apps" />
    {notice ? <p role="status">{notice}</p> : null}
    <Gate load={load} title="Apps" ok={(answer) => {
      const apps = answer.apps.map((app) => approved[app.id] ?? app);
      const open = apps.find((app) => app.id === picked) ?? apps.find((app) => app.state === 'pending') ?? apps[0] ?? null;
      const edited = apps.find((app) => app.id === editing);
      if (registering) return <div className="pane"><Register changed={changed} /></div>;
      if (edited) return <div className="pane"><section className="card" aria-label={'Permission template of ' + edited.id}>
        <div className="row"><h3>Permission template of {edited.name}</h3><Act symbol="close" name="Close the template" word="Close" onClick={() => setEditing(null)} /></div>
        <SchemaBuilder app={edited.id} version={edited.version} initial={edited.schema as SchemaJson} onSaved={(message) => { setEditing(null); changed(message); }} />
      </section></div>;
      // The app is chosen above; its card and its matrix take the whole width below and scroll inside their own pane.
      return <>
        {apps.length ? <nav className="app-picker" aria-label="Apps">
          {apps.map((app) => <button type="button" key={app.id} className={'app-choice' + (app.id === open?.id ? ' on' : '')} aria-pressed={app.id === open?.id} onClick={() => setPicked(app.id)}>
{app.name}</button>)}
        </nav> : null}
        <div className="pane app-pane">
          {open ? <AppCard edit={() => setEditing(open.id)} key={open.id + ':' + revision} app={open} approved={(answer) => { setApproved((held) => ({ ...held, [answer.id]: answer })); setNotice(answer.name + ' is approved: its sign-in client and its schema now take effect.'); }} stored={stored[open.id] ?? null} saved={(answer) => { setStored((held)=>({...held,[open.id]:answer})); setIssued((held)=>{const next={...held};delete next[open.id];return next;}); }} issued={issued[open.id] ?? null} issue={(client) => setIssued((held) => ({ ...held, [open.id]: client }))} changed={changed} /> : <p className="dim">No app is registered.</p>}
        </div>
      </>;
    }} />
  </div>;
}

/** The sign-in settings' two fields: the return addresses, one to a line, and whether the app is given the person's name. */
function SignInFields({ redirects, setRedirects, profile, setProfile, app }: { redirects: string; setRedirects: (text: string) => void; profile: boolean; setProfile: (on: boolean) => void; app: string }) {
  return <>
    <label className="field">Return addresses the sign-in may send a person back to, one to a line<textarea value={redirects} onChange={(event) => setRedirects(event.target.value)} aria-label={'Return addresses of ' + app} /></label>
    <label className="tick"><input type="checkbox" checked={profile} onChange={(event) => setProfile(event.target.checked)} /> Give {app} the person’s name</label>
  </>;
}

function AppCard({ app, issued, issue, changed, stored, saved, approved, edit }: { edit: () => void; approved: (answer: AppRecord) => void; stored:StoredCredentials|null; saved:(answer:StoredCredentials)=>void; app: AppRecord; issued: ClientIssued | null; issue: (client: ClientIssued) => void; changed: (message: string) => void }) {
  const [reason, setReason] = useState('');
  const [refusal, setRefusal] = useState('');
  const [busy, setBusy] = useState(false);
  // The settings offered to approve are the registration's; what is sent is what is kept.
  const [redirects, setRedirects] = useState(app.redirects.join('\n'));
  const [profile, setProfile] = useState(false);
  const act = async (route: string, body: Record<string, unknown>, done: (answer: unknown) => void) => {
    setBusy(true); setRefusal('');
    try { done(await send('POST', path(app.id, route), { operation: operationId(), ...body })); } catch (error) { setRefusal(refusalWords(error)); }
    setBusy(false);
  };
  return <article className="card app-card" aria-label={'App ' + app.id}>
    <h3>{app.name} <span className={'app-state app-' + app.state}>{app.state}</span></h3>
    <p className="note">Registered by {who(app.registered_by)}{app.version ? ', schema version ' + app.version : ''}{app.client_id ? ', sign-in client ' + app.client_id : ''}{app.service_account ? ', acting as ' + app.service_account : ''}.</p>
    {app.redirects.length ? <div><b>Return addresses its registration asked for</b><ul className="app-list">{app.redirects.map((address) => <li className="mono" key={address}>{address}</li>)}</ul></div> : null}
    {app.sign_in ? <div aria-label={'Sign-in settings of ' + app.id}><b>Its sign-in, as last set by {who(app.sign_in.by)}</b>
      <ul className="app-list">{app.sign_in.redirects.map((address) => <li className="mono" key={address}>{address}</li>)}</ul>
      <p className="note">{app.sign_in.profile ? 'It is given the person’s name.' : 'It is not given the person’s name.'}</p></div> : null}
    <div><b>{app.state === 'pending' ? 'The schema it asks for' : 'Its schema'}</b><SchemaTable app={app.id} schema={app.schema} label={'Schema of ' + app.id} /></div>
    {issued ? <SaveCredentials app={app.id} client={issued} saved={saved} /> : null}
    {stored ? <SavedCredentials answer={stored} /> : null}
    {app.state === 'pending' ? <div className="app-actions app-signin">
      <SignInFields app={app.id} redirects={redirects} setRedirects={setRedirects} profile={profile} setProfile={setProfile} />
      <label className="field">Why, if you decline<input value={reason} onChange={(event) => setReason(event.target.value)} /></label>
      <Act symbol="approve" name={'Approve ' + app.name} word="Approve" tone="primary" disabled={busy} onClick={() => { void act('/approve', { redirects: lines(redirects), profile }, (answer) => { const approval = approvalAnswer(answer, app.id); if (approval.client) issue(approval.client); if (approval.credentials) saved(approval.credentials); approved(approval.app); }); }} />
      <Act symbol="decline" name={'Decline ' + app.name} word="Decline" tone="danger" disabled={busy} onClick={() => { void act('/decline', { reason }, () => changed(app.name + ' was declined and never took effect.')); }} />
    </div> : null}
    {app.pending ? <div className="app-waiting" aria-label={'Change waiting for ' + app.id}>
      <b>A change waits for you, replacing version {app.pending.replaces}, from {who(app.pending.by)}</b>
      <SchemaTable app={app.id} schema={app.pending.schema} label={'Schema waiting for ' + app.id} />
      <Act symbol="approve" name={'Approve the change'} word="Approve" tone="primary" disabled={busy} onClick={() => { void act('/schema/approve', {}, () => changed('The change to ' + app.name + ' is approved.')); }} />
      <Act symbol="decline" name={'Decline the change'} word="Decline" tone="danger" disabled={busy} onClick={() => { void act('/schema/decline', { reason }, () => changed('The change to ' + app.name + ' was declined.')); }} />
    </div> : null}
    {app.state === 'approved' && app.id !== LYS ? <SignInSettings app={app} busy={busy} act={act} changed={changed} /> : null}
    {app.state === 'approved' && app.id !== LYS ? <div className="app-actions">
      <Act symbol="edit" name="Edit the permission template" word="Template" onClick={edit} />
      <Act symbol="retire" name={'Retire ' + app.name} word="Retire" tone="danger" disabled={busy} onClick={() => { void act('/retire', { reason }, () => changed(app.name + ' is retired: its client is disabled, and its grants stay readable.')); }} />
    </div> : null}
    {refusal ? <p role="alert" className="app-refused">{refusal}</p> : null}
  </article>;
}

/** Change an approved app's sign-in settings; the next sign-in follows them, with nothing restarted. */
function SignInSettings({ app, busy, act, changed }: { app: AppRecord; busy: boolean; act: (route: string, body: Record<string, unknown>, done: (answer: unknown) => void) => Promise<void>; changed: (message: string) => void }) {
  const [redirects, setRedirects] = useState((app.sign_in?.redirects ?? app.redirects).join('\n'));
  const [profile, setProfile] = useState(app.sign_in?.profile ?? false);
  return <div className="app-actions app-signin" aria-label={'Change the sign-in of ' + app.id}>
    <SignInFields app={app.id} redirects={redirects} setRedirects={setRedirects} profile={profile} setProfile={setProfile} />
    <Act symbol="save" name={'Save the sign-in of ' + app.name} word="Save" tone="primary" disabled={busy} onClick={() => { void act('/sign_in', { redirects: lines(redirects), profile }, (answer) => { const kept = appAnswer(answer, app.id, 'approved', 'sign-in settings'); changed(kept.name + '’s sign-in now sends people back to ' + kept.sign_in?.redirects.join(', ') + (kept.sign_in?.profile ? ' and gives it their name.' : ' and withholds their name.')); }); }} />
  </div>;
}

/** Register an app from this screen: the same route an app registers itself through. */
function Register({ changed }: { changed: (message: string) => void }) {
  const [id, setId] = useState('');
  const [name, setName] = useState('');
  const [redirects, setRedirects] = useState('');
  const ready = /^[a-z][a-z0-9_]{2,39}$/.test(id) && id !== LYS && name.trim() !== '';
  const register = async (schema: SchemaJson) => {
    await send('POST', '/apps', { operation: operationId(), id, name: name.trim(), redirects: lines(redirects), schema });
  };
  return <section className="card" aria-label="Register an app">
    <h3>Register an app</h3>
    <label className="field">App id, its kinds' prefix<input value={id} onChange={(event) => setId(event.target.value.trim())} placeholder="lowercase letters, digits and underscores" /></label>
    <label className="field">Name<input value={name} onChange={(event) => setName(event.target.value)} /></label>
    <label className="field">Return addresses it asks for, one to a line<textarea value={redirects} onChange={(event) => setRedirects(event.target.value)} /></label>
    {ready ? <SchemaBuilder key={id} app={id} version={null} register={register} onSaved={changed} /> : <p className="note">Name the app and give it an id to build its permission template.</p>}
  </section>;
}
