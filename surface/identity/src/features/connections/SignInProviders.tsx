/**
 * The sign-in providers set from here: Google, Microsoft and GitHub, every
 * step shown. The redirect address to paste is the one the service answers,
 * Lys's own; saving proves the client id at the provider, and a provider
 * that refuses it is named with its own words.
 */
import { useState } from 'react';
import type { FormEvent } from 'react';
import { Refused, request, useLoad } from '../../api';
import { Gate } from '../signin/Gate';
import { ReadFailure, failureWords } from '../signin/words';

import { GUIDES } from './provider-guides';
import type { Provider } from './provider-guides';
import './providers.css';
interface ProviderView { id: string; provider: Provider | null; name: string; enabled: boolean; client_id: string }
export interface ProvidersView { providers: ProviderView[]; offered: Provider[]; redirect_address: string }
interface SetBody { provider: Provider; client_id: string; client_secret: string; tenant?: string }

const PATH = '/sign-in-providers';
const readProviders = () => request<ProvidersView>(PATH);
const setProvider = (body: SetBody) => request<ProvidersView>(PATH, body);
const NAMES: Record<Provider, string> = { google: GUIDES.google.name, microsoft: GUIDES.microsoft.name, github: GUIDES.github.name };

function RedirectAddress({ address }: { address: string }) {
  const [copied, setCopied] = useState('');
  async function copy() {
    try {
      await navigator.clipboard.writeText(address);
      setCopied('Copied.');
    } catch {
      setCopied('Copy did not work here; select the address and copy it.');
    }
  }
  return <div className="field">
    <label htmlFor="sign-in-redirect">Redirect address</label>
    <input id="sign-in-redirect" className="mono" value={address} readOnly onFocus={(event) => event.currentTarget.select()} />
    <div><button className="btn" type="button" onClick={copy}>Copy address</button> <span role="status">{copied}</span></div>
  </div>;
}

export function SignInProviders() {
  const load = useLoad(readProviders, 'sign-in-providers');
  const [confirmed, setConfirmed] = useState<ProvidersView | null>(null);
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
      const answer = await setProvider(body);
      setConfirmed(answer);
      setSecret('');
      setOutcome({ kind: 'set', name: NAMES[provider] });
    } catch (error) {
      setOutcome({ kind: 'refused', refused: error instanceof Refused ? error : new Refused(0, { refusal: 'Unanswered', reason: String(error) }) });
    } finally {
      setBusy(false);
    }
  }

  const guide = GUIDES[provider];
  const ready = !busy && clientId.trim() !== '' && secret !== '' && (provider !== 'microsoft' || tenant.trim() !== '');
  return <section className="card providers" aria-label="Sign-in providers">
    <h2>Sign-in providers</h2>
    <p className="sub">The accounts people can sign in to Lys with. Set each one up once here; its client secret is sent to the sign-in service and never shown again.</p>
    <Gate load={load} title="the sign-in settings" renderError={(error) => <ReadFailure error={error} subject="sign-in settings" administrator />} ok={(loaded) => {
      const data = confirmed ?? loaded;
      const held = (id: Provider) => data.providers.find((entry) => entry.provider === id);
      return <>
        <div className="provider-picker" role="radiogroup" aria-label="Provider">
          {data.offered.map((offered) => {
            const set = held(offered);
            return <button type="button" role="radio" aria-checked={offered === provider} key={offered} data-provider={offered}
              className={'provider-card' + (offered === provider ? ' on' : '')} disabled={busy}
              onClick={() => setProvider_(offered)}>
              <span className="provider-name">{GUIDES[offered].name}</span>
              <span className="provider-blurb">{GUIDES[offered].blurb}</span>
              <span className={'pill' + (set?.enabled ? ' ok' : '')}>{set ? (set.enabled ? 'Enabled' : 'Disabled') : 'Not set up'}</span>
            </button>;
          })}
        </div>
        {data.providers.length === 0 ? <p className="note">No sign-in provider is set yet. People sign in with their Lys email and password.</p> : <table className="table">
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
          <li><h3>Give {guide.name} this redirect address</h3>
            <RedirectAddress address={data.redirect_address} />
            <p className="note">A provider already registered with the old address needs the new one added.</p>
          </li>
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
                {busy ? 'Checking with ' + NAMES[provider] + '…' : 'Set ' + NAMES[provider]}
              </button>
            </form>
          </li>
        </ol>
        {outcome?.kind === 'set' ? <p className="notice ok" role="status">{outcome.name} is set. It is a button on Lys's sign-in page now.</p> : null}
        {outcome?.kind === 'refused' ? <p className="notice bad" role="alert">{failureWords(outcome.refused, 'Check the settings issued by the provider before trying again. Nothing is sent again on its own.', [secret])}</p> : null}
      </>;
    }} />
  </section>;
}
