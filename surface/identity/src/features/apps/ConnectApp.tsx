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
  return <div className="row" aria-label={'Known app ' + known.id}>
    <b>{known.name}</b> <span className={'app-state app-' + (state ?? 'none')}>{state ?? 'not connected'}</span>
    {asking ? <>
      <label className="field">{known.name}’s web address<input value={address} onChange={(event) => setAddress(event.target.value)} placeholder="https://cambium.example.com" aria-label={known.name + ' web address'} /></label>
      <Act symbol="approve" name={'Go to ' + known.name + ' to connect'} word="Go" tone="primary" disabled={!origin} onClick={() => { if (origin) window.location.assign(origin + '/identity/lys/connect?lys=' + encodeURIComponent(window.location.origin)); }} />
    </> : <Act symbol="add" name={'Connect ' + known.name} word={state === 'approved' ? 'Connect again' : 'Connect'} tone="primary" onClick={() => setAsking(true)} />}
  </div>;
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
    if (!appOrigin(manifestUrl) || !back || !state) { setRefusal('This link does not name an app’s manifest, its return address and its state. Start the connection from the app again.'); return; }
    let live = true;
    fetch(manifestUrl).then((answer) => { if (!answer.ok) throw new Error('status ' + answer.status); return answer.json() as Promise<unknown>; }).then((value: unknown) => {
      if (!live) return;
      if (validManifest(value)) setManifest(value); else setRefusal('The app at ' + appOrigin(manifestUrl) + ' answered, but not with a manifest Lys reads.');
    }).catch(() => { if (live) setRefusal('The app’s manifest at ' + manifestUrl + ' could not be read.'); });
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
      <p className="note">At <span className="mono">{appOrigin(manifestUrl)}</span>. It will send people back to <span className="mono">{back}</span>{manifest.profile ? ' and is given their name' : ''}.</p>
      <div><b>The permissions it asks for</b><SchemaTable app={manifest.id} schema={manifest.schema} label={'Permissions of ' + manifest.id} /></div>
      <div className="app-actions">
        <Act symbol="approve" name={'Approve ' + manifest.name} word="Approve" tone="primary" disabled={busy} onClick={() => { void approve(); }} />
        <Act symbol="decline" name={'Decline ' + manifest.name} word="Decline" tone="danger" disabled={busy} onClick={() => { void navigate('/apps'); }} />
      </div>
    </> : refusal ? null : <p className="dim">Reading the app’s manifest…</p>}
    {refusal ? <p role="alert" className="app-refused">{refusal}</p> : null}
  </section></div>;
}
