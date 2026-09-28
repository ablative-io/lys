/** The sign-in providers set at the issuer from here: Google, Microsoft and GitHub. */
import { useState } from 'react';
import type { FormEvent } from 'react';
import { Refused, request, useLoad } from '../../api';
import { Gate } from '../signin/Gate';

import { GUIDES } from './provider-guides';
import type { Provider } from './provider-guides';
import './providers.css';
interface ProviderView { id: string; provider: Provider | null; name: string; enabled: boolean; client_id: string }
export interface ProvidersView { providers: ProviderView[]; offered: Provider[] }
interface SetBody { provider: Provider; client_id: string; client_secret: string; tenant?: string }

const PATH = '/sign-in-providers';
const readProviders = () => request<ProvidersView>(PATH);
const setProvider = (body: SetBody) => request<ProvidersView>(PATH, body);
const NAMES: Record<Provider, string> = { google: GUIDES.google.name, microsoft: GUIDES.microsoft.name, github: GUIDES.github.name };

/** Where the provider's console asks for the address it sends people back to. */
export function callbackOf(issuer: string | null): string | null {
  return issuer ? issuer.replace(/\/$/, '') + '/auth/v1/providers/callback' : null;
}

export function SignInProviders({ issuer }: { issuer: string | null }) {
  const [revision, setRevision] = useState(0);
  const load = useLoad(readProviders, 'sign-in-providers-' + revision);
  const [provider, setProvider_] = useState<Provider>('google');
  const [clientId, setClientId] = useState('');
  const [secret, setSecret] = useState('');
  const [tenant, setTenant] = useState('');
  const [busy, setBusy] = useState(false);
  const [outcome, setOutcome] = useState<{ kind: 'set'; name: string } | { kind: 'refused'; refused: Refused } | null>(null);

  async function submit(event: FormEvent) {
    event.preventDefault();
    setBusy(true);
    setOutcome(null);
    const body: SetBody = { provider, client_id: clientId.trim(), client_secret: secret };
    if (provider === 'microsoft') body.tenant = tenant.trim();
    try {
      await setProvider(body);
      setSecret('');
      setOutcome({ kind: 'set', name: NAMES[provider] });
      setRevision((value) => value + 1);
    } catch (error) {
      setOutcome({ kind: 'refused', refused: error instanceof Refused ? error : new Refused(0, { refusal: 'Unanswered', reason: String(error) }) });
    } finally {
      setBusy(false);
    }
  }

  const callback = callbackOf(issuer);
  const guide = GUIDES[provider];
  const [copied, setCopied] = useState(false);
  async function copy(value: string) {
    await navigator.clipboard.writeText(value);
    setCopied(true);
  }
  const ready = !busy && clientId.trim() !== '' && secret !== '' && (provider !== 'microsoft' || tenant.trim() !== '');
  return <section className="card providers" aria-label="Sign-in providers">
    <h2>Sign-in providers</h2>
    <p className="sub">The accounts people can sign in to Lys with. Set each one up once here; its client secret is sent to the sign-in service and never shown again.</p>
    <Gate load={load} title="Sign-in providers" ok={(data) => {
      const held = (id: Provider) => data.providers.find((entry) => entry.provider === id);
      return <>
        <div className="provider-picker" role="radiogroup" aria-label="Provider">
          {data.offered.map((offered) => {
            const set = held(offered);
            return <button type="button" role="radio" aria-checked={offered === provider} key={offered} data-provider={offered}
              className={'provider-card' + (offered === provider ? ' on' : '')} disabled={busy}
              onClick={() => { setProvider_(offered); setCopied(false); }}>
              <span className="provider-name">{GUIDES[offered].name}</span>
              <span className="provider-blurb">{GUIDES[offered].blurb}</span>
              <span className={'pill' + (set?.enabled ? ' ok' : '')}>{set ? (set.enabled ? 'Enabled' : 'Disabled') : 'Not set up'}</span>
            </button>;
          })}
        </div>
        {data.providers.length === 0 ? <p className="note">No sign-in provider is set yet. People can only sign in with the accounts the sign-in provider holds itself.</p> : <table className="table">
          <thead><tr><th>Provider</th><th>State</th><th>Client id</th></tr></thead>
          <tbody>{data.providers.map((entry) => <tr key={entry.id}>
            <td>{entry.name}</td>
            <td><span className={'pill' + (entry.enabled ? ' ok' : '')}>{entry.enabled ? 'Enabled' : 'Disabled'}</span></td>
            <td className="mono">{entry.client_id}</td>
          </tr>)}</tbody>
        </table>}
        <ol className="steps">
          <li><h3>Prepare {guide.name}</h3>
            <ul className="prepare">{guide.prepare.map((step) => <li key={step.text}>{step.text}{step.link ? <> <a className="btn" href={step.link.href} target="_blank" rel="noreferrer">{step.link.label} ↗</a></> : null}</li>)}</ul>
          </li>
          {callback ? <li><h3>Give {guide.name} this redirect address</h3>
            <div className="copy-row"><code className="mono">{callback}</code>
              <button type="button" className="btn" onClick={() => void copy(callback)}>{copied ? 'Copied' : 'Copy'}</button></div>
          </li> : null}
          <li><h3>Enter what {guide.name} issued</h3>
            <form aria-label="Set a sign-in provider" onSubmit={submit}>
              <label className="field">{guide.idLabel}
                <input value={clientId} onChange={(event) => setClientId(event.target.value)} disabled={busy} autoComplete="off" spellCheck={false} required />
              </label>
              <label className="field">{guide.secretLabel}
                <input type="password" value={secret} onChange={(event) => setSecret(event.target.value)} disabled={busy} autoComplete="off" required />
              </label>
              {provider === 'microsoft' ? <label className="field">Tenant <span className="hint">the directory (tenant) ID, or its domain</span>
                <input value={tenant} onChange={(event) => setTenant(event.target.value)} disabled={busy} autoComplete="off" placeholder="contoso.onmicrosoft.com" required />
              </label> : null}
              <button className="btn primary lg" type="submit" disabled={!ready}>
                {busy ? 'Setting…' : 'Set ' + NAMES[provider]}
              </button>
            </form>
          </li>
        </ol>
        {outcome?.kind === 'set' ? <p className="notice ok" role="status">{outcome.name} is set. People can sign in with it now.</p> : null}
        {outcome?.kind === 'refused' ? <p className="notice bad" role="alert">{outcome.refused.refusal.refusal}: {outcome.refused.refusal.reason}</p> : null}
      </>;
    }} />
  </section>;
}
