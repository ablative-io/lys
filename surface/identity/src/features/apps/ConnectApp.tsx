/**
 * Connect an app (the app-manifest pattern). The Apps page names the apps
 * Lys knows how to connect; Connect asks for the app's web address once and
 * hands the browser to the app, which sends it back to #/apps/connect with
 * its manifest's address, its return address and its state. There the
 * administrator reads the app's name, address and permissions in words and
 * approves or declines; approval is one act on Lys, and the browser goes back
 * to the app with a one-time code its server exchanges for its credentials.
 */
import { useEffect, useState } from 'react';
import { useNavigate, useSearchParams } from 'react-router';
import { API, Refused, operationId, useLoad } from '../../api';
import { rememberFileReturn } from '../signin/fileReturn';
import { Act } from '../../shell/Act';
import { send } from './SchemaBuilder';
import { STATE_WORDS, SchemaTable, refusalWords } from './Apps';
import type { AppRecord } from './Apps';

/** An app offered by name on the connect card: a suggestion only, since any app that serves the connect address connects. */
export interface KnownApp { id: string; name: string; manifest: string }
/** The one app offered by name (ADR-116: Lys holds no app's name; this line goes once apps register themselves first). */
export const KNOWN_APPS: KnownApp[] = [{ id: 'cambium', name: 'Cambium', manifest: '/identity/lys/manifest' }];
/** Where every app that connects to Lys starts the connect on its own side; Lys sends the browser there with its address. */
export const CONNECT_PATH = '/identity/lys/connect';
/** Any other app, by its web address alone: its name arrives from its manifest after the hand-off. */
const ANOTHER: KnownApp = { id: 'another', name: 'the app', manifest: '' };
/** `text` with its first letter capitalised, for a sentence or label that starts with an app's name. */
const opening = (text: string): string => text.charAt(0).toUpperCase() + text.slice(1);

/** The app's origin from what a person typed, or null when it is not a web address. */
export function appOrigin(typed: string): string | null {
  try {
    const url = new URL(typed.trim());
    return url.protocol === 'https:' || url.protocol === 'http:' ? url.origin : null;
  } catch { return null; }
}

/** Whether a web address names only the computer it is opened on: localhost, a .localhost name, 127.0.0.0/8 or ::1. */
export function onlyHere(origin: string): boolean {
  try {
    const host = new URL(origin).hostname.replace(/^\[|\]$/g, '').toLowerCase();
    return host === 'localhost' || host.endsWith('.localhost') || host === '::1' || /^127(\.\d{1,3}){3}$/.test(host);
  } catch { return false; }
}

/**
 * Why the app is unlikely to call Lys back at the address this browser is on, or null when it can be handed over: Lys open at
 * an address only its own computer answers, while the app is on another computer. An app on the same computer reaches it there.
 * Said, never enforced: a person who has carried that address to the app's computer (a tunnel) still goes on.
 */
export function strandedWhy(name: string, lys: string, app: string): string | null {
  if (!onlyHere(lys) || onlyHere(app)) return null;
  // A browser on a loopback address is on the computer Lys runs on, so "this computer" is both the device in hand and Lys's own.
  return opening(name) + ' is on another computer, but Lys is open here at ' + lys + ', which only this computer answers, so ' + name + ' will most likely not reach it. Open Lys at an address ' + name + '’s computer can reach, such as its https address, and connect from there.';
}

/** Connect an app: each app offered by name with its state on this Lys, then any other app by its web address. */
export function KnownApps({ apps }: { apps: AppRecord[] }) {
  return <section className="card" aria-label="Connect an app">
    <h3>Connect an app</h3>
    {KNOWN_APPS.map((known) => <KnownAppRow key={known.id} known={known} state={apps.find((app) => app.id === known.id)?.state ?? null} />)}
    <KnownAppRow known={ANOTHER} state={null} />
  </section>;
}

function KnownAppRow({ known, state }: { known: KnownApp; state: AppRecord['state'] | null }) {
  const [asking, setAsking] = useState(false);
  const [address, setAddress] = useState('');
  const origin = appOrigin(address);
  const lys = window.location.origin;
  const stranded = origin ? strandedWhy(known.name, lys, origin) : null;
  const whyNot = origin ? null : address.trim()
    ? 'That is not a whole web address. Type it as it appears in the browser, starting with https://.'
    : 'Type ' + known.name + '’s web address to continue.';
  const another = known === ANOTHER;
  const cancel = () => { setAsking(false); setAddress(''); };
  const connected = state === 'approved';
  // A connected app rests with nothing inviting it to be broken: connecting again is a quiet act whose cost is said before the address is asked for.
  // The address takes the card's whole width on its own line, its label above it and the acts under it.
  return <div aria-label={another ? 'Another app' : 'Known app ' + known.id}>
    <div className="row">
      {another ? <span><b>Another app</b> <span className="note">any app that connects to Lys, by its web address</span></span>
        : <span><b>{known.name}</b> <span className={'app-state app-' + (state ?? 'none')}>{connected ? 'Connected' : state ? STATE_WORDS[state] : 'Not connected'}</span></span>}
      {asking ? null : <Act symbol="add" name={another ? 'Connect another app' : 'Connect ' + known.name} word={connected ? 'Connect again' : 'Connect'} tone={connected || another ? 'plain' : 'primary'} onClick={() => setAsking(true)} />}
    </div>
    {asking ? <div className="app-connect">
      {connected ? <ReplacesCredentials name={known.name} /> : null}
      <p className="note">Lys sends your browser to {known.name}, which asks to connect and brings you back here to approve what it asks for.</p>
      <label className="field">{opening(known.name)}’s web address<input type="url" inputMode="url" autoComplete="url" autoFocus value={address} onChange={(event) => setAddress(event.target.value)} placeholder={another ? 'https://app.example.com' : 'https://' + known.id + '.example.com'} aria-label={opening(known.name) + ' web address'} /></label>
      {whyNot ? <p className="note" aria-live="polite">{whyNot}</p>
        : stranded ? <p className="app-replace" role="note">{stranded}</p>
        : <p className="note">{opening(known.name)} will call Lys back at <span className="mono">{lys}</span>.</p>}
      <div className="app-actions">
        <Act symbol="approve" name={'Continue to ' + known.name} word={'Continue to ' + known.name} tone="primary" disabled={!origin} hint={whyNot ?? undefined} onClick={() => { if (origin) window.location.assign(origin + CONNECT_PATH + '?lys=' + encodeURIComponent(lys)); }} />
        <Act symbol="close" name={'Cancel connecting ' + known.name} word="Cancel" onClick={cancel} />
      </div>
    </div> : null}
  </div>;
}

/** What connecting an already connected app costs, said before it is done: approval replaces its credentials at once. */
function ReplacesCredentials({ name }: { name: string }) {
  return <p className="app-replace" role="note">{name} is already connected. Connecting again ends the credentials the running {name} uses, so nobody can sign in to it until it finishes connecting again. Do this only when {name} says it has lost its connection to Lys, or to move to another {name}.</p>;
}

/** The name a refusal gives the app: a known app's name when the link names its manifest, else "the app". */
function appName(manifestUrl: string): string {
  try {
    const path = new URL(manifestUrl).pathname;
    return KNOWN_APPS.find((known) => known.manifest === path)?.name ?? 'the app';
  } catch { return 'the app'; }
}

/**
 * Where a decline sends the browser: the app's return address with error=access_denied and its state, as OAuth says a
 * refusal is returned. Only an address on the manifest's own origin is followed; anything else goes back to Apps.
 */
export function declinedAddress(back: string, manifestUrl: string, state: string): string | null {
  const origin = appOrigin(manifestUrl);
  try {
    const url = new URL(back);
    if (!origin || url.origin !== origin) return null;
    url.searchParams.set('error', 'access_denied');
    url.searchParams.set('state', state);
    return url.toString();
  } catch { return null; }
}

interface Manifest { id: string; name: string; redirects: string[]; post_logout_redirects?: string[]; profile: boolean; schema: unknown }
const validManifest = (value: unknown): value is Manifest => {
  const manifest = value as Partial<Manifest> | null;
  return !!manifest && typeof manifest.id === 'string' && typeof manifest.name === 'string'
    && Array.isArray(manifest.redirects) && manifest.redirects.every((entry) => typeof entry === 'string')
    && typeof manifest.profile === 'boolean' && !!manifest.schema && typeof manifest.schema === 'object';
};

/** #/apps/connect: approve or decline the app the browser was sent here for. */
export function ConnectApp() {
  const [params] = useSearchParams();
  const navigate = useNavigate();
  const manifestUrl = params.get('manifest') ?? '';
  const back = params.get('return') ?? '';
  const state = params.get('state') ?? '';
  const [manifest, setManifest] = useState<Manifest | null>(null);
  const [refusal, setRefusal] = useState('');
  const [busy, setBusy] = useState(false);
  const [signedOut, setSignedOut] = useState(false);
  // Whether the manifest could not be read from this page at all, so the person is offered the manifest itself to open.
  const [blocked, setBlocked] = useState(false);
  // Whether the app is already connected, so approving says it replaces the running app's credentials; unread, the words say "if".
  const apps = useLoad(() => send<{ apps: AppRecord[] }>('GET', '/apps'), 'apps');
  const held = apps.status === 'ok' ? apps.data.apps.find((app) => app.id === manifest?.id)?.state ?? null : undefined;
  useEffect(() => {
    const name = appName(manifestUrl);
    const from = name === 'the app' ? 'the app’s' : name + '’s';
    if (!appOrigin(manifestUrl) || !back || !state) { setRefusal('This link is incomplete. Start again from ' + from + ' Connect to Lys.'); return; }
    // An https page may not read a plain http address off its own computer: the browser blocks it unasked, so it is said before any read.
    if (window.location.protocol === 'https:' && manifestUrl.startsWith('http:') && !onlyHere(manifestUrl)) {
      setRefusal('Lys is open over https, and ' + name + ' at ' + appOrigin(manifestUrl) + ' is served over plain http, so the browser will not let this page read it. Serve ' + name + ' over https, then start again from ' + name + '.');
      return;
    }
    let live = true;
    // A read the browser rejects is either nothing answering or an answer this page is not allowed to read; the page cannot tell which.
    const unreachable = 'Lys could not read ' + name + '’s manifest at ' + appOrigin(manifestUrl) + ' from this page, so it cannot tell whether ' + name + ' is not answering there or does not let this Lys read it.';
    const notManifest = (said: string) => 'Lys reached ' + appOrigin(manifestUrl) + ', but ' + said + ', so it is not ' + name + ' as Lys knows it. Check that the address is ' + from + ', then start again from ' + name + '.';
    // Not answering at all, answering with a refusal, and answering with something else are three different faults, each said as itself.
    fetch(manifestUrl).then(async (answer) => {
      if (!live) return;
      if (!answer.ok) { setRefusal(notManifest('it answered ' + answer.status + ' for the manifest')); return; }
      const value: unknown = await answer.json().catch(() => undefined);
      if (!live) return;
      if (validManifest(value)) setManifest(value); else setRefusal(notManifest('its answer is not a manifest Lys reads'));
    }, () => { if (live) { setRefusal(unreachable); setBlocked(true); } });
    return () => { live = false; };
  }, [manifestUrl, back, state]);
  const approve = async () => {
    setBusy(true); setRefusal('');
    try {
      const answer = await send<{ redirect: string }>('POST', '/apps/connect', { operation: operationId(), manifest, return: back, state });
      window.location.assign(answer.redirect);
    } catch (error) {
      setRefusal(error instanceof Refused && error.status === 401
        ? 'Your sign-in to Lys ended before Approve reached it, so nothing was approved and ' + (manifest?.name ?? 'the app') + ' is still waiting. Sign in again; if Lys does not bring you back here, start again from ' + (manifest?.name ?? 'the app') + '’s Connect to Lys.'
        : refusalWords(error));
      setSignedOut(error instanceof Refused && error.status === 401);
      setBusy(false);
    }
  };
  return <div className="page apps"><section className="card" aria-label="Connect an app">
    <h1>Connect an app</h1>
    {manifest ? <>
      <h3>{manifest.name}</h3>
      <p className="note app-addresses">At <span className="mono">{appOrigin(manifestUrl)}</span>. It will send people back to <span className="mono">{back}</span>{manifest.profile ? ' and is given their name' : ''}.</p>
      <div><b>The permissions it asks for</b><SchemaTable app={manifest.id} schema={manifest.schema} label={'Permissions of ' + manifest.id} /></div>
      {held === 'approved' ? <ReplacesCredentials name={manifest.name} />
        : held === undefined ? <p className="note">If {manifest.name} is already connected, approving gives it new credentials and the {manifest.name} running now stops signing people in until it finishes connecting.</p> : null}
      <div className="app-actions">
        {signedOut ? <a className="btn primary" href={API + '/login'} onClick={(event) => {
          try { rememberFileReturn(); } catch (error) { event.preventDefault(); setRefusal('The browser could not keep this link for sign-in: ' + String(error) + '. Copy the address from the address bar, sign in, then open it again.'); }
        }}>Sign in again</a>
          : <Act symbol="approve" name={'Approve ' + manifest.name} word={held === 'approved' ? 'Approve and replace its credentials' : 'Approve'} tone="primary" disabled={busy} onClick={() => { void approve(); }} />}
        <Act symbol="decline" name={'Decline ' + manifest.name} word="Decline" tone="danger" disabled={busy} onClick={() => { const declined = declinedAddress(back, manifestUrl, state); if (declined) window.location.assign(declined); else void navigate('/apps'); }} />
      </div>
    </> : refusal ? null : <p className="dim">Reading the app’s manifest…</p>}
    {refusal ? <p role="alert" className="app-refused">{refusal}</p> : null}
    {blocked && !manifest ? <ul className="app-list" aria-label="Find out which">
      <li>If {appName(manifestUrl)}’s manifest opens, {appName(manifestUrl)} is running but does not let Lys at <span className="mono">{window.location.origin}</span> read it: allow this Lys in {appName(manifestUrl)}, then start again from {appName(manifestUrl)}.</li>
      <li>If it does not open, {appName(manifestUrl)} is not answering at that address: check that it is running, then start again from {appName(manifestUrl)}.</li>
    </ul> : null}
    {refusal && !manifest ? <p className="app-actions">
      {blocked ? <a className="btn" href={manifestUrl} target="_blank" rel="noopener noreferrer">Open {appName(manifestUrl)}’s manifest</a> : null}
      <a className="btn" href="#/apps">Back to Apps</a>
    </p> : null}
  </section></div>;
}
