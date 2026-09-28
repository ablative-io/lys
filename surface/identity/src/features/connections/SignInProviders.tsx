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
import '../people/recorded-form.css';

type Provider = 'google' | 'microsoft' | 'github';
interface ProviderView { id: string; provider: Provider | null; name: string; enabled: boolean; client_id: string }
export interface ProvidersView { providers: ProviderView[]; offered: Provider[]; redirect_address: string }
interface SetBody { provider: Provider; client_id: string; client_secret: string; tenant?: string }

const PATH = '/sign-in-providers';
const readProviders = () => request<ProvidersView>(PATH);
const setProvider = (body: SetBody) => request<ProvidersView>(PATH, body);
const NAMES: Record<Provider, string> = { google: 'Google', microsoft: 'Microsoft', github: 'GitHub' };

/** Where each provider makes the client a person registers Lys as. */
export const CONSOLES: Record<Provider, { href: string; label: string; client: string }> = {
  google: { href: 'https://console.cloud.google.com/apis/credentials', label: 'Google Cloud credentials', client: 'Create an OAuth client ID of type Web application.' },
  microsoft: { href: 'https://entra.microsoft.com/#view/Microsoft_AAD_RegisteredApps/ApplicationsListBlade', label: 'Microsoft Entra app registrations', client: 'Register an application with a Web redirect address, then add a client secret.' },
  github: { href: 'https://github.com/settings/applications/new', label: 'GitHub OAuth apps', client: 'Register a new OAuth app, then generate a client secret.' },
};

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

  const console_ = CONSOLES[provider];
  return <section className="card" aria-label="Sign-in providers">
    <h2>Sign-in providers</h2>
    <p>The accounts people can sign in to Lys with, as a button on Lys's sign-in page. The client secret goes to Lys's sign-in service and is never shown again.</p>
    <Gate load={load} title="Sign-in providers" ok={(data) => <>
      {data.providers.length === 0 ? <p>No sign-in provider is set yet. People sign in with their Lys email and password.</p> : <table className="table">
        <thead><tr><th>Provider</th><th>State</th><th>Client id</th></tr></thead>
        <tbody>{data.providers.map((entry) => <tr key={entry.id}>
          <td>{entry.name}</td>
          <td>{entry.enabled ? 'Enabled' : 'Disabled'}</td>
          <td className="mono">{entry.client_id}</td>
        </tr>)}</tbody>
      </table>}
      <div className="field">
        <label htmlFor="sign-in-provider">Provider</label>
        <select id="sign-in-provider" value={provider} onChange={(event) => setProvider_(event.target.value as Provider)} disabled={busy}>
          {data.offered.map((offered) => <option key={offered} value={offered}>{NAMES[offered]}</option>)}
        </select>
      </div>
      <ol className="steps">
        <li>Open <a href={console_.href} target="_blank" rel="noreferrer">{console_.label}</a>. {console_.client}</li>
        <li>Paste this redirect address where {NAMES[provider]} asks for the redirect (callback) address.</li>
        <li>Enter the client id and client secret {NAMES[provider]} shows you{provider === 'microsoft' ? ', and your tenant' : ''}, then save. Lys checks them with {NAMES[provider]} before saving.</li>
      </ol>
      <RedirectAddress address={data.redirect_address} />
      <p className="note">A provider already registered with the old address needs the new one added.</p>
      <form className="recorded-form" aria-label="Set a sign-in provider" onSubmit={submit}>
        <div className="field">
          <label htmlFor="sign-in-client-id">Client id</label>
          <input id="sign-in-client-id" value={clientId} onChange={(event) => setClientId(event.target.value)} disabled={busy} autoComplete="off" required />
        </div>
        <div className="field">
          <label htmlFor="sign-in-client-secret">Client secret</label>
          <input id="sign-in-client-secret" type="password" value={secret} onChange={(event) => setSecret(event.target.value)} disabled={busy} autoComplete="off" required />
        </div>
        {provider === 'microsoft' ? <div className="field">
          <label htmlFor="sign-in-tenant">Tenant</label>
          <input id="sign-in-tenant" value={tenant} onChange={(event) => setTenant(event.target.value)} disabled={busy} autoComplete="off" placeholder="the directory (tenant) id or its domain" required />
        </div> : null}
        <button className="btn primary" type="submit" disabled={busy || clientId.trim() === '' || secret === '' || (provider === 'microsoft' && tenant.trim() === '')}>
          {busy ? 'Checking with ' + NAMES[provider] + '…' : 'Set ' + NAMES[provider]}
        </button>
      </form>
      {outcome?.kind === 'set' ? <p role="status">{outcome.name} is set. It is a button on Lys's sign-in page now.</p> : null}
      {outcome?.kind === 'refused' ? <p role="alert">{outcome.refused.refusal.refusal}: {outcome.refused.refusal.reason}</p> : null}
    </>} />
  </section>;
}
