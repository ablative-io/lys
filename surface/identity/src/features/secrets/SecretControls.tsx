/** Owners choose a discovered secret by name before entering the detailed access controls. */
import { useState } from 'react';
import { useSearchParams } from 'react-router';
import { request, useLoad } from '../../api';
import { Gate } from '../signin/Gate';
import type { SecretListing } from './Secrets';
import { RecipientsChange, ScopeChange } from './SecretsDetail';
import { secretsApi } from './secretsApi';
import { CurrentSettings } from './CurrentSettings';

export function SecretControls() {
  const [params, setParams] = useSearchParams();
  const selected = params.get('secret') ?? '';
  const [revision, setRevision] = useState(0);
  const changed = () => setRevision((value) => value + 1);
  const load = useLoad(() => request<SecretListing>('/secrets'), 'secret-controls');
  return <div className="page">
    <div className="head"><div><div className="eyebrow">Advanced · Secrets</div><h1>Manage secret access</h1>
      <p className="sub">Choose a secret, then change its visibility or who may receive it. Only its owner can save these changes.</p></div></div>
    <Gate load={load} title="Secret controls" ok={(listing) => <>
      <label className="field">Secret<select value={selected} onChange={(event) => setParams(event.target.value ? { secret: event.target.value } : {})}>
        <option value="">Choose a secret</option>{listing.secrets.map((entry) => <option key={entry.name} value={entry.name}>{entry.name}</option>)}
      </select></label>
      {listing.secrets.some((entry) => entry.name === selected) ? <div className="grid2" key={selected}>
        <CurrentSettings secret={selected} revision={revision} />
        <section className="card"><ScopeChange secret={selected} change={secretsApi.scope} changed={changed} /></section>
        <section className="card"><RecipientsChange secret={selected} change={secretsApi.recipients} changed={changed} /></section>
      </div> : <p className="note">{listing.secrets.length ? 'Select an entry to see its controls.' : 'No secrets are visible to this account.'}</p>}
    </>} />
  </div>;
}
