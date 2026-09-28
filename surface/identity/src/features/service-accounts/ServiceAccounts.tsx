/** Service-account records are created and retired explicitly; they are not grants or stored credentials. */
import { useState } from 'react';
import { api, operationId, request, useLoad } from '../../api';
import type { ServiceAccount } from '../../generated';
import { Gate } from '../signin/Gate';
import { ChangeStatus } from '../roles/ChangeStatus';
import { useRoleChange } from '../roles/useRoleChange';
import { clock } from '../file/time';
interface AccountsView { scope: 'personal' | 'directory'; service_accounts: ServiceAccount[] }
export function ServiceAccounts() {
  const [revision, setRevision] = useState(0);
  const refresh = () => setRevision((value) => value + 1);
  const load = useLoad(async () => ({ me: await api.me(), accounts: await request<AccountsView>('/service-accounts') }), 'service-accounts:' + revision);
  return <main className="page"><div className="head"><div><div className="eyebrow">Directory</div><h1>Service accounts</h1><p>Record accounts used for work and retire records no longer needed. This does not store a password, grant access or close the account at its provider.</p></div><button className="btn" onClick={refresh}>Refresh accounts</button></div>
    <Gate load={load} title="Service accounts" ok={({ me, accounts }) => <>
      <p>{accounts.scope === 'directory' ? 'Showing the directory’s account records.' : 'Showing your own account records.'}</p>
      {accounts.service_accounts.length ? accounts.service_accounts.map((account) => <section className="card" key={account.id}><h2>{account.name}</h2><p>{account.description}</p><dl className="facts"><dt>State</dt><dd>{account.state === 'active' ? 'Registered' : account.state}</dd><dt>Owner</dt><dd><a href={'#/file/' + account.owner}>{account.owner}</a></dd><dt>Created</dt><dd>{clock(account.created_at)}</dd>{account.retired_at !== null ? <><dt>Retired</dt><dd>{clock(account.retired_at)}</dd></> : null}</dl>
        <Retire key={account.id + ':' + revision} account={account} person={me.person.id} changed={refresh} />
      </section>) : <p>No service-account records were returned.</p>}
      <Create key={me.person.id + ':' + revision} person={me.person.id} changed={refresh} />
    </>} />
  </main>;
}
function Create({ person, changed }: { person: string; changed: () => void }) {
  const [name, setName] = useState(''); const [description, setDescription] = useState('');
  const change = useRoleChange<ServiceAccount>('lys.pending.service-create.' + person, '/service-accounts', (answer, body) => answer.id === body.operation && answer.owner === person && answer.name === body.name && answer.description === body.description, changed);
  return <form className="card" aria-label="Register service account" onSubmit={(event) => { event.preventDefault(); if (name.trim()) change.submit({ operation: operationId(), name: name.trim(), description: description.trim() }); }}><h2>Register an account you own</h2>
    <label className="field">Account name<input required maxLength={100} value={name} disabled={change.blocked} onChange={(event) => setName(event.target.value)} placeholder="For example, invoice processing" /></label>
    <label className="field">What it is for<input maxLength={500} value={description} disabled={change.blocked} onChange={(event) => setDescription(event.target.value)} /></label>
    <button className="btn primary" disabled={change.blocked || !name.trim()} type="submit">Register account</button><ChangeStatus change={change} />
  </form>;
}
function Retire({ account, person, changed }: { account: ServiceAccount; person: string; changed: () => void }) {
  const [confirm, setConfirm] = useState(false);
  const change = useRoleChange<ServiceAccount>('lys.pending.service-retire.' + person + '.' + account.id, '/service-accounts/' + encodeURIComponent(account.id) + '/retire', (answer) => answer.id === account.id && answer.state === 'retired' && answer.retired_at !== null, changed);
  return <>{account.state !== 'retired' && !confirm ? <button className="btn danger" disabled={change.blocked} onClick={() => setConfirm(true)}>Retire record</button> : null}
    {confirm ? <section aria-label="Confirm service account retirement"><p>Retire the record for {account.name}? This does not close its provider account or revoke credentials.</p><button className="btn danger" disabled={change.blocked} onClick={() => change.submit({ operation: operationId() })}>Confirm retirement</button><button className="btn" disabled={change.busy} onClick={() => setConfirm(false)}>Cancel</button></section> : null}<ChangeStatus change={change} />
  </>;
}
