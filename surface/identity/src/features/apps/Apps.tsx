/**
 * The Apps screen: every app that signs in with Lys and has its permissions
 * checked here. An app registers itself through the API and nothing it
 * registers takes effect until the administrator approves it on this screen,
 * which shows its name, its redirect addresses and its whole schema in words
 * before the button. Broker-backed approval saves credentials in Lys secrets
 * before activation. A proposed schema change waits here the same way. The permission template is built and edited here too, and saved
 * through the same routes an upload uses.
 */
import { ConfigTabs } from '../settings/ConfigTabs';
import { useState } from 'react';
import { Listing } from '../../shell/Listing';
import type { Column } from '../../shell/Listing';
import { Refused, operationId, useLoad } from '../../api';
import { Gate } from '../signin/Gate';
import { SchemaBuilder, send } from './SchemaBuilder';
import type { SchemaJson } from './SchemaBuilder';
import './apps.css';
import { SaveCredentials, SavedCredentials } from './SaveCredentials';
import type { StoredCredentials } from './SaveCredentials';

/** Who made an act on the apps. */
export interface By { kind: 'person' | 'operator' | 'service_account' | 'start'; login?: { provider: string; subject: string }; id?: string }
/** An app as the service answers it. */
export interface AppRecord {
  id: string; name: string; state: 'pending' | 'approved' | 'declined' | 'retired'; redirects: string[];
  schema: unknown; version: number; versions: number[];
  pending: { operation: string; replaces: number; schema: unknown; by: By; at: number } | null;
  client_id: string | null; service_account: string | null; registered_by: By; registered_at: number;
}
/** The client an approval creates, answered once. */
export interface ClientIssued { client_id: string; client_secret: string; credential: string }

interface ApprovalAnswer { app: AppRecord; client: ClientIssued | null; credentials: StoredCredentials | null }
function approvalAnswer(value: unknown, id: string): ApprovalAnswer {
  const answer = value as Partial<ApprovalAnswer> | null;
  const app = answer?.app;
  const client = answer?.client ?? null;
  const credentials = answer?.credentials ?? null;
  const strings = (values: unknown): values is string[] => Array.isArray(values) && values.every((entry) => typeof entry === 'string');
  const pending = app?.pending;
  const validPending = pending === null || (pending && typeof pending.operation === 'string' && Number.isSafeInteger(pending.replaces) && typeof pending.schema === 'object' && pending.by && typeof pending.by.kind === 'string' && Number.isSafeInteger(pending.at));
  const validApp = app && app.id === id && app.state === 'approved' && typeof app.name === 'string'
    && strings(app.redirects) && typeof app.version === 'number' && Number.isSafeInteger(app.version)
    && Array.isArray(app.versions) && app.versions.every((version) => Number.isSafeInteger(version))
    && app.schema !== null && typeof app.schema === 'object' && !Array.isArray(app.schema) && validPending
    && (app.client_id === null || typeof app.client_id === 'string')
    && (app.service_account === null || typeof app.service_account === 'string')
    && app.registered_by && ['person', 'operator', 'service_account', 'start'].includes(app.registered_by.kind)
    && typeof app.registered_at === 'number';
  const validClient = client === null || (client && typeof client.client_id === 'string' && typeof client.client_secret === 'string' && typeof client.credential === 'string');
  const validCredentials = credentials === null || (credentials && credentials.app === id && typeof credentials.client_secret_ref === 'string' && typeof credentials.api_credential_ref === 'string');
  if (!app || !validApp || !validClient || !validCredentials) throw new Error('Lys answered, but did not confirm this app’s approval. Open Apps again to check its saved state before approving again.');
  return { app, client: client ?? null, credentials: credentials ?? null };
}

const LYS = 'lys';
const who = (by: By) => (by.kind === 'operator' ? 'the install operator for ' + (by.login?.subject ?? 'the administrator') : by.kind === 'person' ? by.login?.subject ?? 'a person' : by.kind === 'service_account' ? 'service account ' + (by.id ?? '') : 'Lys at start');
const listed = (items: string[]) => (items.length ? items.join(', ') : 'nothing');
const path = (id: string, rest: string) => '/apps/' + encodeURIComponent(id) + rest;
const refusalWords = (error: unknown) => (error instanceof Refused ? error.refusal.refusal + ': ' + error.refusal.reason : String(error));

/** The schema of `app` in words, one line per kind, so it is read before it is approved. */
export function schemaWords(app: string, schema: unknown): string[] {
  const value = schema as { kinds?: Record<string, { actions?: string[]; relations?: Record<string, string[]>; parents?: string[] }>; relations?: Record<string, string[]> } | null;
  if (app === LYS && value?.relations) {
    return Object.entries(value.relations).map(([relation, actions]) => 'On every kind of Lys\'s own, ' + relation + ' may ' + listed(actions) + '.');
  }
  return Object.entries(value?.kinds ?? {}).map(([kind, body]) => {
    const relations = Object.entries(body.relations ?? {}).map(([relation, actions]) => relation + ' may ' + listed(actions));
    const parents = body.parents?.length ? ' What is held on ' + body.parents.join(' or ') + ' reaches it.' : '';
    return kind + ': its actions are ' + listed(body.actions ?? []) + '. ' + (relations.length ? relations.join('; ') + '.' : 'It has no relation of its own.') + parents;
  });
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
  return <div className="page fill apps">
    <ConfigTabs on="apps" />
    <div className="head"><div><h1>Apps</h1><p className="sub">Every app that signs in with Lys and has its permissions checked here. An app registers itself through the API; nothing it registers takes effect until you approve it here.</p></div>
      <button type="button" className="btn primary" onClick={() => setRegistering(!registering)}>{registering ? 'Close the new app' : 'Register an app'}</button></div>
    {notice ? <p role="status">{notice}</p> : null}
    <Gate load={load} title="Apps" ok={(answer) => {
      const apps = answer.apps.map((app) => approved[app.id] ?? app);
      const open = apps.find((app) => app.id === picked) ?? apps.find((app) => app.state === 'pending') ?? apps[0] ?? null;
      const group = [{ id: '', name: 'Apps', lead: null, depth: 0, items: apps, within: apps }];
      const columns: Column<AppRecord>[] = [
        { head: 'App', cell: (app) => app.name },
        { head: 'State', cell: (app) => <span className={'app-state app-' + app.state}>{app.state === 'pending' ? 'waiting for approval' : app.state}</span> },
        { head: 'Signs in to', cell: (app) => <span className="sec">{app.redirects.length} {app.redirects.length === 1 ? 'address' : 'addresses'}</span> },
      ];
      return <div className="body work">
        <Listing<AppRecord> groups={group} columns={columns} id={(app) => app.id} href={(app) => '#/apps?app=' + encodeURIComponent(app.id)} words={(app) => app.name + ' ' + app.id}
          noun="apps" holds={(items) => items.length + ' apps'} selected={registering ? null : open?.id ?? null} select={() => undefined} open={(app) => { setPicked(app.id); setRegistering(false); }} />
        <div className="detail">
          {registering ? <Register changed={changed} /> : open ? <AppCard key={open.id + ':' + revision} app={open} approved={(answer) => { setApproved((held) => ({ ...held, [answer.id]: answer })); setNotice(answer.name + ' is approved: its sign-in client and its schema now take effect.'); }} stored={stored[open.id] ?? null} saved={(answer) => { setStored((held)=>({...held,[open.id]:answer})); setIssued((held)=>{const next={...held};delete next[open.id];return next;}); }} issued={issued[open.id] ?? null} issue={(client) => setIssued((held) => ({ ...held, [open.id]: client }))} changed={changed} /> : <p>No app is registered.</p>}
        </div>
      </div>;
    }} />
  </div>;
}

function AppCard({ app, issued, issue, changed, stored, saved, approved }: { approved: (answer: AppRecord) => void; stored:StoredCredentials|null; saved:(answer:StoredCredentials)=>void; app: AppRecord; issued: ClientIssued | null; issue: (client: ClientIssued) => void; changed: (message: string) => void }) {
  const [editing, setEditing] = useState(false);
  const [reason, setReason] = useState('');
  const [refusal, setRefusal] = useState('');
  const [busy, setBusy] = useState(false);
  const act = async (route: string, body: Record<string, unknown>, done: (answer: unknown) => void) => {
    setBusy(true); setRefusal('');
    try { done(await send('POST', path(app.id, route), { operation: operationId(), ...body })); } catch (error) { setRefusal(refusalWords(error)); }
    setBusy(false);
  };
  const words = schemaWords(app.id, app.schema);
  return <article className="card app-card" aria-label={'App ' + app.id}>
    <h3>{app.name} <span className="app-id mono">{app.id}</span> <span className={'app-state app-' + app.state}>{app.state}</span></h3>
    <p className="note">Registered by {who(app.registered_by)}{app.version ? ', schema version ' + app.version : ''}{app.client_id ? ', sign-in client ' + app.client_id : ''}{app.service_account ? ', acting as ' + app.service_account : ''}.</p>
    {app.redirects.length ? <div><b>Redirect addresses</b><ul className="app-list">{app.redirects.map((address) => <li className="mono" key={address}>{address}</li>)}</ul></div> : null}
    <div><b>{app.state === 'pending' ? 'The schema it asks for' : 'Its schema'}</b><ul className="app-list" aria-label={'Schema of ' + app.id}>{words.map((line) => <li key={line}>{line}</li>)}</ul></div>
    {issued ? <SaveCredentials app={app.id} client={issued} saved={saved} /> : null}
    {stored ? <SavedCredentials answer={stored} /> : null}
    {app.state === 'pending' ? <div className="app-actions">
      <label className="field">Why, if you decline<input value={reason} onChange={(event) => setReason(event.target.value)} /></label>
      <button type="button" className="btn primary" disabled={busy} onClick={() => { void act('/approve', {}, (answer) => { const approval = approvalAnswer(answer, app.id); if (approval.client) issue(approval.client); if (approval.credentials) saved(approval.credentials); approved(approval.app); }); }}>Approve {app.name}</button>
      <button type="button" className="btn danger" disabled={busy} onClick={() => { void act('/decline', { reason }, () => changed(app.name + ' was declined and never took effect.')); }}>Decline {app.name}</button>
    </div> : null}
    {app.pending ? <div className="app-waiting" aria-label={'Change waiting for ' + app.id}>
      <b>A change waits for you, replacing version {app.pending.replaces}, from {who(app.pending.by)}</b>
      <ul className="app-list">{schemaWords(app.id, app.pending.schema).map((line) => <li key={line}>{line}</li>)}</ul>
      <button type="button" className="btn primary" disabled={busy} onClick={() => { void act('/schema/approve', {}, () => changed('The change to ' + app.name + ' is approved.')); }}>Approve the change</button>
      <button type="button" className="btn danger" disabled={busy} onClick={() => { void act('/schema/decline', { reason }, () => changed('The change to ' + app.name + ' was declined.')); }}>Decline the change</button>
    </div> : null}
    {app.state === 'approved' && app.id !== LYS ? <div className="app-actions">
      <button type="button" className="btn" onClick={() => setEditing(!editing)}>{editing ? 'Close the template' : 'Edit the permission template'}</button>
      <button type="button" className="btn danger" disabled={busy} onClick={() => { void act('/retire', { reason }, () => changed(app.name + ' is retired: its client is disabled, and its grants stay readable.')); }}>Retire {app.name}</button>
    </div> : null}
    {editing ? <SchemaBuilder app={app.id} version={app.version} initial={app.schema as SchemaJson} onSaved={(message) => { setEditing(false); changed(message); }} /> : null}
    {refusal ? <p role="alert" className="app-refused">{refusal}</p> : null}
  </article>;
}

/** Register an app from this screen: the same route an app registers itself through. */
function Register({ changed }: { changed: (message: string) => void }) {
  const [id, setId] = useState('');
  const [name, setName] = useState('');
  const [redirects, setRedirects] = useState('');
  const ready = /^[a-z][a-z0-9_]{2,39}$/.test(id) && id !== LYS && name.trim() !== '';
  const register = async (schema: SchemaJson) => {
    await send('POST', '/apps', { operation: operationId(), id, name: name.trim(), redirects: redirects.split('\n').map((line) => line.trim()).filter(Boolean), schema });
  };
  return <section className="card" aria-label="Register an app">
    <h3>Register an app</h3>
    <label className="field">App id, its kinds' prefix<input value={id} onChange={(event) => setId(event.target.value.trim())} placeholder="lowercase letters, digits and underscores" /></label>
    <label className="field">Name<input value={name} onChange={(event) => setName(event.target.value)} /></label>
    <label className="field">Redirect addresses, one to a line<textarea value={redirects} onChange={(event) => setRedirects(event.target.value)} /></label>
    {ready ? <SchemaBuilder key={id} app={id} version={null} register={register} onSaved={changed} /> : <p className="note">Name the app and give it an id to build its permission template.</p>}
  </section>;
}
