import { readTogether } from '../../reads';
/** Service-account records are created and retired explicitly; they are not grants or stored credentials. */
import { useState } from 'react';
import { api, operationId, request, useLoad } from '../../api';
import type { MeView, ServiceAccount } from '../../generated';
import { DirectoryGate as Gate, IdentityName } from '../people/Words';
import { DirectoryChangeStatus as ChangeStatus } from '../roles/ChangeStatus';
import { useRoleChange } from '../roles/useRoleChange';
import { clock } from '../file/time';
import { Listing } from '../../shell/Listing';
import type { Column } from '../../shell/Listing';
import { Picker } from '../../shell/Picker';
interface AccountsView { scope: 'personal' | 'directory'; service_accounts: ServiceAccount[] }
export function ServiceAccounts() {
  const load = useLoad(() => readTogether({ me: api.me(), accounts: request<AccountsView>('/service-accounts') }), 'service-accounts');
  return <main className="page fill"><div className="head"><div><div className="eyebrow">Directory</div><h1>Service accounts</h1><p className="sub">Keep a list of accounts used for work. Retiring an account here marks its Lys record retired; it does not close the provider account or revoke credentials.</p></div></div>
    <Gate load={load} title="Service accounts" ok={({ me, accounts }) => <AccountList initial={accounts} me={me} />} />
  </main>;
}
function AccountList({ initial, me }: { initial: AccountsView; me: MeView }) {
  const [accounts, setAccounts] = useState(initial);
  const [revision, setRevision] = useState(0);
  const changed = (answer: ServiceAccount) => {
    setAccounts((view) => ({ ...view, service_accounts: view.service_accounts.some((account) => account.id === answer.id) ? view.service_accounts.map((account) => account.id === answer.id ? answer : account) : [...view.service_accounts, answer] }));
    setRevision((value) => value + 1);
  };
  const [picked, setPicked] = useState<string | null>(null);
  const [adding, setAdding] = useState(() => initial.service_accounts.length === 0 || sessionStorage.getItem('lys.pending.service-create.' + me.person.id) !== null);
  const open = accounts.service_accounts.find((account) => account.id === picked) ?? accounts.service_accounts[0] ?? null;
  const group = [{ id: '', name: 'Accounts', lead: null, depth: 0, items: accounts.service_accounts, within: accounts.service_accounts }];
  const columns: Column<ServiceAccount>[] = [
    { head: 'Account', cell: (account) => account.name },
    { head: 'State', cell: (account) => <span className="sec">{account.state === 'active' ? 'Registered' : account.state}</span> },
    { head: 'Owner', cell: (account) => <IdentityName id={account.owner} /> },
    { head: 'Created', cell: (account) => <span className="sec">{clock(account.created_at)}</span> },
  ];
  return <>
    <div className="tools"><span className="note">{accounts.scope === 'directory' ? 'Showing the directory’s account records.' : 'Showing your own account records.'}</span>
      {!adding ? <button className="btn primary" onClick={() => setAdding(true)}>+ Register an account</button> : null}</div>
    <div className="body">
      <Listing<ServiceAccount> groups={group} columns={columns} id={(account) => account.id} href={(account) => '#/service-accounts?account=' + account.id} words={(account) => account.name + ' ' + account.description}
        noun="accounts" holds={(items) => items.length + ' accounts'} selected={adding ? null : open?.id ?? null} select={() => undefined} open={(account) => { setPicked(account.id); setAdding(false); }} />
      <div className="detail">
        {adding ? <Create key={me.person.id + ':' + revision} person={me.person.id} administrator={accounts.scope === 'directory'} changed={(answer) => { changed(answer); setPicked(answer.id); setAdding(false); }} />
          : open ? <section className="card" key={open.id}><h2>{open.name}</h2><p>{open.description}</p><dl className="facts"><dt>State</dt><dd>{open.state === 'active' ? 'Registered' : open.state}</dd><dt>Owner</dt><dd><IdentityName id={open.owner} /></dd><dt>Created</dt><dd>{clock(open.created_at)}</dd>{open.retired_at !== null ? <><dt>Retired</dt><dd>{clock(open.retired_at)}</dd></> : null}</dl>
            <Retire key={open.id + ':' + revision} account={open} person={me.person.id} changed={changed} />
          </section> : <p>No service-account records were returned.</p>}
      </div>
    </div>
  </>;
}
function Create({ person, administrator, changed }: { person: string; administrator: boolean; changed: (answer: ServiceAccount) => void }) {
  const [owner, setOwner] = useState(person);
  const people = useLoad(api.people, 'service-account-owners');
  const [name, setName] = useState(''); const [description, setDescription] = useState('');
  const change = useRoleChange<ServiceAccount>('lys.pending.service-create.' + person, '/service-accounts', (answer, body) => answer.id === body.operation && answer.owner === (body.owner ?? person) && answer.name === body.name && answer.description === body.description, changed);
  return <form className="card" aria-label="Register service account" onSubmit={(event) => { event.preventDefault(); if (name.trim()) change.submit({ operation: operationId(), name: name.trim(), description: description.trim(), ...(owner === person ? {} : { owner }) }); }}><h2>Register a service account</h2><p>You own this record unless an administrator selects another person.</p>
    {administrator ? <details><summary>Choose a different owner</summary><Gate load={people} title="Account owners" ok={(view) => <div className="field">Responsible person<Picker name="owner" label="Find a person" options={view.people.filter((entry) => entry.state !== 'retired').map((entry) => ({ id: entry.id, name: entry.display_name + (entry.id === person ? ' (you)' : '') }))} onChange={(ids) => setOwner(ids[0] ?? person)} /></div>} /></details> : null}
    <label className="field">Account name<input name="name" required maxLength={100} value={name} disabled={change.blocked} onChange={(event) => setName(event.target.value)} placeholder="For example, invoice processing" /></label>
    <label className="field">What it is for<input maxLength={500} value={description} disabled={change.blocked} onChange={(event) => setDescription(event.target.value)} /></label>
    <button className="btn primary" disabled={change.blocked || !name.trim()} type="submit">Register account</button><ChangeStatus change={change} />
  </form>;
}
function Retire({ account, person, changed }: { account: ServiceAccount; person: string; changed: (answer: ServiceAccount) => void }) {
  const [confirm, setConfirm] = useState(false);
  const change = useRoleChange<ServiceAccount>('lys.pending.service-retire.' + person + '.' + account.id, '/service-accounts/' + encodeURIComponent(account.id) + '/retire', (answer) => answer.id === account.id && answer.state === 'retired' && answer.retired_at !== null, changed);
  return <>{account.state !== 'retired' && !confirm ? <button className="btn danger" disabled={change.blocked} onClick={() => setConfirm(true)}>Retire this service account</button> : null}
    {confirm ? <section aria-label="Confirm service account retirement"><p>Retire this service account, {account.name}, in Lys? Lys keeps its history. This does not close its provider account or revoke credentials.</p><button className="btn danger" disabled={change.blocked} onClick={() => change.submit({ operation: operationId() })}>Yes, retire {account.name}</button><button className="btn" disabled={change.busy} onClick={() => setConfirm(false)}>Cancel</button></section> : null}<ChangeStatus change={change} />
  </>;
}
