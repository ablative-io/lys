/** The sign-in providers set at the issuer from here: Google, Microsoft and GitHub. */
import { useState } from 'react';
import type { FormEvent } from 'react';
import { Refused, request, useLoad } from '../../api';
import { Gate } from '../signin/Gate';

type Provider = 'google' | 'microsoft' | 'github';
interface ProviderView { id: string; provider: Provider | null; name: string; enabled: boolean; client_id: string }
export interface ProvidersView { providers: ProviderView[]; offered: Provider[] }
interface SetBody { provider: Provider; client_id: string; client_secret: string; tenant?: string }

const PATH = '/sign-in-providers';
const readProviders = () => request<ProvidersView>(PATH);
const setProvider = (body: SetBody) => request<ProvidersView>(PATH, body);
const NAMES: Record<Provider, string> = { google: 'Google', microsoft: 'Microsoft', github: 'GitHub' };

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
  return <section className="card" aria-label="Sign-in providers">
    <h2>Sign-in providers</h2>
    <p>The accounts people can sign in to Lys with. Each is registered at the provider and set here; the client secret goes to the sign-in provider and is never shown again.</p>
    <Gate load={load} title="Sign-in providers" ok={(data) => <>
      {data.providers.length === 0 ? <p>No sign-in provider is set yet. People can only sign in with the accounts the sign-in provider holds itself.</p> : <table className="table">
        <thead><tr><th>Provider</th><th>State</th><th>Client id</th></tr></thead>
        <tbody>{data.providers.map((entry) => <tr key={entry.id}>
          <td>{entry.name}</td>
          <td>{entry.enabled ? 'Enabled' : 'Disabled'}</td>
          <td className="mono">{entry.client_id}</td>
        </tr>)}</tbody>
      </table>}
      <form aria-label="Set a sign-in provider" onSubmit={submit}>
        <label>Provider{' '}
          <select value={provider} onChange={(event) => setProvider_(event.target.value as Provider)} disabled={busy}>
            {data.offered.map((offered) => <option key={offered} value={offered}>{NAMES[offered]}</option>)}
          </select>
        </label>
        {callback ? <p>Register the address below with {NAMES[provider]} as the redirect (callback) address, then enter what it issued.</p> : null}
        {callback ? <p className="mono">{callback}</p> : null}
        <label>Client id{' '}
          <input value={clientId} onChange={(event) => setClientId(event.target.value)} disabled={busy} autoComplete="off" required />
        </label>
        <label>Client secret{' '}
          <input type="password" value={secret} onChange={(event) => setSecret(event.target.value)} disabled={busy} autoComplete="off" required />
        </label>
        {provider === 'microsoft' ? <label>Tenant{' '}
          <input value={tenant} onChange={(event) => setTenant(event.target.value)} disabled={busy} autoComplete="off" placeholder="the directory (tenant) id or its domain" required />
        </label> : null}
        <button className="btn" type="submit" disabled={busy || clientId.trim() === '' || secret === '' || (provider === 'microsoft' && tenant.trim() === '')}>
          {busy ? 'Setting…' : 'Set ' + NAMES[provider]}
        </button>
      </form>
      {outcome?.kind === 'set' ? <p role="status">{outcome.name} is set. People can sign in with it now.</p> : null}
      {outcome?.kind === 'refused' ? <p role="alert">{outcome.refused.refusal.refusal}: {outcome.refused.refusal.reason}</p> : null}
    </>} />
  </section>;
}
