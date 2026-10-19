/**
 * Connect an app (the app-manifest pattern). The Apps page offers any app by its web address, and each app that connected
 * before offers Connect again on its own card with the address Lys says it connected from (connect_origin); Lys holds no
 * app's name (ADR-116). Connect hands the browser to the app with Lys's own public address, which sends it back to
 * #/apps/connect with its manifest's address, its return address and its state. There Lys's server reads the manifest, and
 * the administrator reads the app's name, address and permissions in words and approves or declines; approval is one act
 * on Lys, and the browser goes back to the app with a one-time code its server exchanges for its credentials. The running
 * app keeps its credentials until it collects the new ones.
 */
import { useEffect, useState } from 'react';
import { useNavigate, useSearchParams } from 'react-router';
import { API, Refused, operationId, useLoad } from '../../api';
import { rememberFileReturn } from '../signin/fileReturn';
import { Act } from '../../shell/Act';
import { send } from './SchemaBuilder';
import { SchemaTable, refusalWords } from './Apps';
import type { AppRecord } from './Apps';

/** Where every app that connects to Lys starts the connect on its own side; Lys sends the browser there with its address. */
export const CONNECT_PATH = '/identity/lys/connect';
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

/** Lys's own address as GET /apps answers it: the public address an app calls back, or none and Lys's reason. */
export interface LysAddress { address: string | null; reason: string | null }
/** Lys's address from GET /apps's answer; an answer that names none is said as none, with Lys's reason when it gave one. */
export const lysAddress = (answer: { public_address?: string | null; public_address_reason?: string | null }): LysAddress =>
  typeof answer.public_address === 'string' ? { address: answer.public_address, reason: null }
    : { address: null, reason: answer.public_address_reason ?? 'Lys did not say which address an app can call it back at.' };

/** The Connect card: any app by its web address alone; its name arrives from its manifest after the hand-off. */
export function ConnectAnother({ lys }: { lys: LysAddress }) {
  return <section className="card" aria-label="Connect an app">
    <h3>Connect an app</h3>
    <ConnectRow name="the app" lys={lys} label="Another app" />
  </section>;
}

/** Connect again on an app's own card, with the address Lys says it last connected from filled in. */
export function ConnectAgain({ app, lys }: { app: AppRecord; lys: LysAddress }) {
  return <ConnectRow name={app.name} lys={lys} label={'Connect ' + app.id + ' again'} again={app.connect_origin ?? ''} />;
}

/**
 * Ask for the app's web address once and hand the browser to it with Lys's public address. With no public address the page
 * says Lys's reason and hands the browser's own address instead: said, never enforced, since a tunnel may still carry it.
 */
function ConnectRow({ name, lys, label, again }: { name: string; lys: LysAddress; label: string; again?: string }) {
  const [asking, setAsking] = useState(false);
  const [address, setAddress] = useState(again ?? '');
  const origin = appOrigin(address);
  const handed = lys.address ?? window.location.origin;
  const another = again === undefined;
  const whyNot = origin ? null : address.trim()
    ? 'That is not a whole web address. Type it as it appears in the browser, starting with https://.'
    : 'Type ' + name + '’s web address to continue.';
  const cancel = () => { setAsking(false); setAddress(again ?? ''); };
  // The address takes the card's whole width on its own line, its label above it and the acts under it.
  return <div aria-label={label}>
    <div className="row">
      {another ? <span><b>Another app</b> <span className="note">any app that connects to Lys, by its web address</span></span> : null}
      {asking ? null : <Act symbol="add" name={another ? 'Connect another app' : 'Connect ' + name + ' again'} word={another ? 'Connect' : 'Connect again'} tone="plain" onClick={() => setAsking(true)} />}
    </div>
    {asking ? <div className="app-connect">
      {another ? null : <p className="note">The running {name} keeps its credentials until it collects the new ones.</p>}
      <p className="note">Lys sends your browser to {name}, which asks to connect and brings you back here to approve what it asks for.</p>
      <label className="field">{opening(name)}’s web address<input type="url" inputMode="url" autoComplete="url" autoFocus value={address} onChange={(event) => setAddress(event.target.value)} placeholder="https://app.example.com" aria-label={opening(name) + ' web address'} /></label>
      {whyNot ? <p className="note" aria-live="polite">{whyNot}</p>
        : lys.address ? <p className="note">{opening(name)} will call Lys back at <span className="mono">{lys.address}</span>.</p>
        : <p className="app-replace" role="note">{lys.reason} {opening(name)} will be handed this browser’s address, {handed}, instead. To fix it, change Lys’s registered address to an https address {name}’s computer can reach, then connect again.</p>}
      <div className="app-actions">
        <Act symbol="approve" name={'Continue to ' + name} word={'Continue to ' + name} tone="primary" disabled={!origin} hint={whyNot ?? undefined} onClick={() => { if (origin) window.location.assign(origin + CONNECT_PATH + '?lys=' + encodeURIComponent(handed)); }} />
        <Act symbol="close" name={'Cancel connecting ' + name} word="Cancel" onClick={cancel} />
      </div>
    </div> : null}
  </div>;
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

/** The line a reason Lys gave for not reading the manifest is said as, with the next act; any other refusal in its own words. */
export function unreadWords(error: unknown, at: string): string {
  if (!(error instanceof Refused)) return refusalWords(error);
  const more = (error.answer ?? {}) as { error?: unknown; status?: unknown; found?: unknown };
  switch (error.refusal.refusal) {
    case 'manifest_not_answering':
      return 'Lys could not reach the app at ' + at + ': ' + (typeof more.error === 'string' ? more.error : error.refusal.reason) + '. Check that the app is running and answers at that address, then start again from the app.';
    case 'manifest_answered_status':
      return 'The app at ' + at + ' answered ' + (typeof more.status === 'number' ? String(more.status) : 'an error') + ' for its manifest, so Lys could not read it. Check that the address is the app’s and that the app is running fully, then start again from the app.';
    case 'manifest_not_a_manifest':
      return 'The app at ' + at + ' answered with ' + (typeof more.found === 'string' ? more.found : 'something else') + ', not a manifest Lys reads. Check that the address is the app’s own, then start again from the app.';
    case 'manifest_address_refused':
      return error.refusal.reason + '. Start again from the app.';
    case 'NotAdmitted':
      return 'Lys did not read the app’s manifest for you: ' + error.refusal.reason + '. Send this page’s address to the administrator, who connects apps on this Lys.';
    default:
      return refusalWords(error);
  }
}

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
  // Whether the app is already connected, so approving says the running app keeps its credentials; unread, the words say "if".
  const apps = useLoad(() => send<{ apps: AppRecord[] }>('GET', '/apps'), 'apps');
  const held = apps.status === 'ok' ? apps.data.apps.find((app) => app.id === manifest?.id)?.state ?? null : undefined;
  const at = appOrigin(manifestUrl);
  useEffect(() => {
    if (!at || !back || !state) { setRefusal('This link is incomplete. Start again from the app’s Connect to Lys.'); return; }
    let live = true;
    // Lys's server reads the manifest, so not answering, answering with an error and answering with something else are
    // each said as themselves, and no browser rule stands between this page and the app.
    send<{ manifest: unknown }>('GET', '/apps/connect/manifest?address=' + encodeURIComponent(manifestUrl)).then((answer) => {
      if (!live) return;
      if (validManifest(answer.manifest)) setManifest(answer.manifest);
      else setRefusal('Lys read the app at ' + at + ', but did not answer a manifest it reads. Start again from the app.');
    }, (error: unknown) => { if (live) setRefusal(unreadWords(error, at)); });
    return () => { live = false; };
  }, [manifestUrl, at, back, state]);
  const approve = async () => {
    setBusy(true); setRefusal('');
    try {
      const answer = await send<{ redirect: string }>('POST', '/apps/connect', { operation: operationId(), manifest, return: back, state });
      window.location.assign(answer.redirect);
    } catch (error) {
      setRefusal(error instanceof Refused && error.status === 401
        ? 'Your sign-in to Lys ended before Approve reached it, so nothing was approved and ' + (manifest?.name ?? 'the app') + ' is still waiting. Sign in again, and Lys brings you back to this page.'
        : refusalWords(error));
      setSignedOut(error instanceof Refused && error.status === 401);
      setBusy(false);
    }
  };
  return <div className="page apps"><section className="card" aria-label="Connect an app">
    <h1>Connect an app</h1>
    {manifest ? <>
      <h3>{manifest.name}</h3>
      <p className="note app-addresses">At <span className="mono">{at}</span>. It will send people back to <span className="mono">{back}</span>{manifest.profile ? ' and is given their name' : ''}.</p>
      <div><b>The permissions it asks for</b><SchemaTable app={manifest.id} schema={manifest.schema} label={'Permissions of ' + manifest.id} /></div>
      {held === 'approved' ? <p className="note">The running app keeps its credentials until it collects the new ones.</p>
        : held === undefined ? <p className="note">If {manifest.name} is already connected, the running app keeps its credentials until it collects the new ones.</p> : null}
      <div className="app-actions">
        {signedOut ? <a className="btn primary" href={API + '/login'} onClick={(event) => {
          try { rememberFileReturn(); } catch (error) { event.preventDefault(); setRefusal('The browser could not keep this link for sign-in: ' + String(error) + '. Copy the address from the address bar, sign in, then open it again.'); }
        }}>Sign in again</a>
          : <Act symbol="approve" name={'Approve ' + manifest.name} word="Approve" tone="primary" disabled={busy} onClick={() => { void approve(); }} />}
        <Act symbol="decline" name={'Decline ' + manifest.name} word="Decline" tone="danger" disabled={busy} onClick={() => { const declined = declinedAddress(back, manifestUrl, state); if (declined) window.location.assign(declined); else void navigate('/apps'); }} />
      </div>
    </> : refusal ? null : <p className="dim">Reading the app’s manifest…</p>}
    {refusal ? <p role="alert" className="app-refused">{refusal}</p> : null}
    {refusal && !manifest ? <p className="app-actions"><a className="btn" href="#/apps">Back to Apps</a></p> : null}
  </section></div>;
}
