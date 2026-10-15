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
import { operationId } from '../../api';
import { Act } from '../../shell/Act';
import { send } from './SchemaBuilder';
import { SchemaTable, refusalWords } from './Apps';
import type { AppRecord } from './Apps';

/** An app Lys knows how to connect, built into the surface. */
export interface KnownApp { id: string; name: string; manifest: string }
export const KNOWN_APPS: KnownApp[] = [{ id: 'cambium', name: 'Cambium', manifest: '/identity/lys/manifest' }];

/** The app's origin from what a person typed, or null when it is not a web address. */
export function appOrigin(typed: string): string | null {
  try {
    const url = new URL(typed.trim());
    return url.protocol === 'https:' || url.protocol === 'http:' ? url.origin : null;
  } catch { return null; }
}

/** The known apps, each with its state on this Lys and a Connect button. */
export function KnownApps({ apps }: { apps: AppRecord[] }) {
  return <section className="card" aria-label="Known apps">
    <h3>Known apps</h3>
    {KNOWN_APPS.map((known) => <KnownAppRow key={known.id} known={known} state={apps.find((app) => app.id === known.id)?.state ?? null} />)}
  </section>;
}

function KnownAppRow({ known, state }: { known: KnownApp; state: AppRecord['state'] | null }) {
  const [asking, setAsking] = useState(false);
  const [address, setAddress] = useState('');
  const origin = appOrigin(address);
  const whyNot = origin ? null : address.trim()
    ? 'That is not a whole web address. Type it as it appears in the browser, starting with https://.'
    : 'Type ' + known.name + '’s web address to continue.';
  const cancel = () => { setAsking(false); setAddress(''); };
  // The address takes the card's whole width on its own line, its label above it and the acts under it.
  return <div aria-label={'Known app ' + known.id}>
    <div className="row">
      <span><b>{known.name}</b> <span className={'app-state app-' + (state ?? 'none')}>{state ?? 'not connected'}</span></span>
      {asking ? null : <Act symbol="add" name={'Connect ' + known.name} word={state === 'approved' ? 'Connect again' : 'Connect'} tone="primary" onClick={() => setAsking(true)} />}
    </div>
    {asking ? <div className="app-connect">
      <p className="note">Lys sends your browser to {known.name}, which asks to connect and brings you back here to approve what it asks for.</p>
      <label className="field">{known.name}’s web address<input type="url" inputMode="url" autoComplete="url" autoFocus value={address} onChange={(event) => setAddress(event.target.value)} placeholder="https://cambium.example.com" aria-label={known.name + ' web address'} /></label>
      {whyNot ? <p className="note" aria-live="polite">{whyNot}</p> : null}
      <div className="app-actions">
        <Act symbol="approve" name={'Continue to ' + known.name} word={'Continue to ' + known.name} tone="primary" disabled={!origin} hint={whyNot ?? undefined} onClick={() => { if (origin) window.location.assign(origin + '/identity/lys/connect?lys=' + encodeURIComponent(window.location.origin)); }} />
        <Act symbol="close" name={'Cancel connecting ' + known.name} word="Cancel" onClick={cancel} />
      </div>
    </div> : null}
  </div>;
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
  useEffect(() => {
    const name = appName(manifestUrl);
    const from = name === 'the app' ? 'the app’s' : name + '’s';
    if (!appOrigin(manifestUrl) || !back || !state) { setRefusal('This link is incomplete. Start again from ' + from + ' Connect to Lys.'); return; }
    let live = true;
    const unreachable = 'Lys could not reach ' + name + ' at ' + appOrigin(manifestUrl) + '. Check that ' + name + ' is running and the address is right, then start again from ' + name + '.';
    const notManifest = (said: string) => 'Lys reached ' + appOrigin(manifestUrl) + ', but ' + said + ', so it is not ' + name + ' as Lys knows it. Check that the address is ' + from + ', then start again from ' + name + '.';
    // Not answering at all, answering with a refusal, and answering with something else are three different faults, each said as itself.
    fetch(manifestUrl).then(async (answer) => {
      if (!live) return;
      if (!answer.ok) { setRefusal(notManifest('it answered ' + answer.status + ' for the manifest')); return; }
      const value: unknown = await answer.json().catch(() => undefined);
      if (!live) return;
      if (validManifest(value)) setManifest(value); else setRefusal(notManifest('its answer is not a manifest Lys reads'));
    }, () => { if (live) setRefusal(unreachable); });
    return () => { live = false; };
  }, [manifestUrl, back, state]);
  const approve = async () => {
    setBusy(true); setRefusal('');
    try {
      const answer = await send<{ redirect: string }>('POST', '/apps/connect', { operation: operationId(), manifest, return: back, state });
      window.location.assign(answer.redirect);
    } catch (error) { setRefusal(refusalWords(error)); setBusy(false); }
  };
  return <div className="page apps"><section className="card" aria-label="Connect an app">
    <h1>Connect an app</h1>
    {manifest ? <>
      <h3>{manifest.name}</h3>
      <p className="note app-addresses">At <span className="mono">{appOrigin(manifestUrl)}</span>. It will send people back to <span className="mono">{back}</span>{manifest.profile ? ' and is given their name' : ''}.</p>
      <div><b>The permissions it asks for</b><SchemaTable app={manifest.id} schema={manifest.schema} label={'Permissions of ' + manifest.id} /></div>
      <div className="app-actions">
        <Act symbol="approve" name={'Approve ' + manifest.name} word="Approve" tone="primary" disabled={busy} onClick={() => { void approve(); }} />
        <Act symbol="decline" name={'Decline ' + manifest.name} word="Decline" tone="danger" disabled={busy} onClick={() => { const declined = declinedAddress(back, manifestUrl, state); if (declined) window.location.assign(declined); else void navigate('/apps'); }} />
      </div>
    </> : refusal ? null : <p className="dim">Reading the app’s manifest…</p>}
    {refusal ? <p role="alert" className="app-refused">{refusal}</p> : null}
  </section></div>;
}
