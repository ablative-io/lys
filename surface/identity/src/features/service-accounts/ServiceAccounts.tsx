import { readTogether } from '../../reads';
/** Service-account records are created and retired explicitly; they are not grants or stored credentials. */
import { useState } from 'react';
import { api, operationId, request, useLoad } from '../../api';
import type { MeView, ServiceAccount } from '../../generated';
import { DirectoryGate as Gate, IdentityName, STATUS } from '../people/Words';
import { ChangeStatus } from '../roles/ChangeStatus';
import { useRoleChange } from '../roles/useRoleChange';
import { clock } from '../file/time';
import { Listing } from '../../shell/Listing';
import type { Column } from '../../shell/Listing';
import { Picker } from '../../shell/Picker';
import { Act } from '../../shell/Act';
interface AccountsView { scope: 'personal' | 'directory'; service_accounts: ServiceAccount[] }
/** The service-account records, as a page of Settings. */
export function ServiceAccountsList() {
  const load = useLoad(() => readTogether({ me: api.me(), accounts: request<AccountsView>('/service-accounts') }), 'service-accounts');
  return <>
    <p className="note">Keep a list of accounts used for work. Retiring an account here marks its Lys record retired; it does not close the provider account or revoke credentials.</p>
    <Gate load={load} title="Service accounts" ok={({ me, accounts }) => <AccountList initial={accounts} me={me} />} />
  </>;
}
function AccountList({ initial, me }: { initial: AccountsView; me: MeView }) {
  const [accounts, setAccounts] = useState(initial);
  const [revision, setRevision] = useState(0);
  const changed = (answer: ServiceAccount) => {
    setAccounts((view) => ({ ...view, service_accounts: view.service_accounts.some((account) => account.id === answer.id) ? view.service_accounts.map((account) => account.id === answer.id ? answer : account) : [...view.service_accounts, answer] }));
    setRevision((value) => value + 1);
  };
  const [adding, setAdding] = useState(() => initial.service_accounts.length === 0 || sessionStorage.getItem('lys.pending.service-create.' + me.person.id) !== null);
  // A retirement whose answer was lost is opened again, so its Check is in front of the person, never hidden in a row.
  const [retiring, setRetiring] = useState<string | null>(() => initial.service_accounts.find((account) => sessionStorage.getItem(retireKey(me.person.id, account.id)) !== null)?.id ?? null);
  const [notice, setNotice] = useState('');
  const chosen = accounts.service_accounts.find((account) => account.id === retiring) ?? null;
  const group = [{ id: '', name: 'Accounts', lead: null, depth: 0, items: accounts.service_accounts, within: accounts.service_accounts }];
  const columns: Column<ServiceAccount>[] = [
    { head: 'Account', cell: (account) => account.name },
    { head: 'State', cell: (account) => <span className="sec">{STATUS[account.state] ?? 'State not recognised'}</span> },
    { head: 'Owner', cell: (account) => <IdentityName id={account.owner} /> },
    { head: 'What it is for', cell: (account) => account.description },
    { head: 'Created', cell: (account) => <span className="sec">{clock(account.created_at)}</span> },
    { head: 'Retired', cell: (account) => account.retired_at !== null ? <span className="sec">{clock(account.retired_at)}</span>
      : account.state === 'retired' ? <span className="sec">Yes</span>
        : <Act symbol="retire" name="Retire this service account" hint={'Retire ' + account.name} word="Retire" tone="danger" disabled={retiring !== null} onClick={() => { setNotice(''); setRetiring(account.id); }} /> },
  ];
  return <>
    <div className="tools account-tools"><span className="note">{accounts.scope === 'directory' ? 'Showing the directory’s account records.' : 'Showing your own account records.'}</span>
      {!adding ? <Act symbol="add" name="Register an account" word="Register" tone="primary" onClick={() => setAdding(true)} /> : null}</div>
    {notice ? <p role="status">{notice}</p> : null}
    {adding ? <Create key={me.person.id + ':' + revision} person={me.person.id} administrator={accounts.scope === 'directory'} changed={(answer) => { changed(answer); setAdding(false); setNotice(answer.name + ' is registered.'); }}
      cancel={accounts.service_accounts.length ? () => setAdding(false) : undefined} /> : null}
    {chosen ? <Retire key={chosen.id} account={chosen} person={me.person.id} changed={(answer) => { changed(answer); setRetiring(null); setNotice(answer.name + ' is retired in Lys. Its provider account and credentials are unchanged.'); }} cancel={() => setRetiring(null)} /> : null}
    {!adding && !accounts.service_accounts.length ? <p>No service accounts are registered yet.</p> : null}
    <div className="body one">
      <Listing<ServiceAccount> groups={group} columns={columns} id={(account) => account.id} href={(account) => '#/settings/service-accounts?account=' + account.id} words={(account) => account.name + ' ' + account.description}
        noun="accounts" holds={(items) => items.length + ' accounts'} selected={null} select={() => undefined} />
    </div>
  </>;
}
function Create({ person, administrator, changed, cancel }: { person: string; administrator: boolean; changed: (answer: ServiceAccount) => void; cancel?: () => void }) {
  const [owner, setOwner] = useState(person);
  const people = useLoad(api.people, 'service-account-owners');
  const [name, setName] = useState(''); const [description, setDescription] = useState('');
  const change = useRoleChange<ServiceAccount>('lys.pending.service-create.' + person, '/service-accounts', (answer, body) => answer.id === body.operation && answer.owner === (body.owner ?? person) && answer.name === body.name && answer.description === body.description, changed);
  return <form className="card form-grid" aria-label="Register service account" onSubmit={(event) => { event.preventDefault(); if (name.trim()) change.submit({ operation: operationId(), name: name.trim(), description: description.trim(), ...(owner === person ? {} : { owner }) }); }}><h2>Register a service account</h2><p>You own this record unless an administrator selects another person.</p>
    {administrator ? <Gate load={people} title="Account owners" ok={(view) => <div className="field">Responsible person, if not you<Picker name="owner" label="Find a person" options={view.people.filter((entry) => entry.state !== 'retired').map((entry) => ({ id: entry.id, name: entry.display_name + (entry.id === person ? ' (you)' : '') }))} onChange={(ids) => setOwner(ids[0] ?? person)} /></div>} /> : null}
    <label className="field">Account name (required)<input name="name" required value={name} disabled={change.blocked} onChange={(event) => setName(event.target.value)} /><span className="hint">For example, Invoice processing.</span></label>
    <label className="field wide">What it is for<textarea name="description" rows={3} value={description} disabled={change.blocked} onChange={(event) => setDescription(event.target.value)} /><span className="hint">Which provider it is with and what it is used for, so others know whether it is still needed.</span></label>
    <p className="actions"><Act symbol="add" name="Register account" word="Register" tone="primary" type="submit" disabled={change.blocked || !name.trim()} />{' '}
      {cancel ? <Act symbol="close" name="Cancel registering" word="Cancel" disabled={change.busy} onClick={cancel} /> : null}
      {!name.trim() && !change.blocked ? <span className="why-not"> Give the account a name to register it.</span> : null}</p>
    <ChangeStatus change={change} />
  </form>;
}
const retireKey = (person: string, account: string): string => 'lys.pending.service-retire.' + person + '.' + account;
/** An account's state in words; the service's own name never shows. */
/** The retirement of one account, asked above the list where its words have room, never squeezed into a table cell. */
function Retire({ account, person, changed, cancel }: { account: ServiceAccount; person: string; changed: (answer: ServiceAccount) => void; cancel: () => void }) {
  const change = useRoleChange<ServiceAccount>(retireKey(person, account.id), '/service-accounts/' + encodeURIComponent(account.id) + '/retire', (answer) => answer.id === account.id && answer.state === 'retired' && answer.retired_at !== null, changed);
  return <section className="card" aria-label="Confirm service account retirement" style={{ marginBottom: 14 }}><h2>Retire {account.name}?</h2>
    <p>Lys marks its record retired and keeps its history. This does not close its provider account or revoke credentials; do that with the provider.</p>
    {change.pending ? null : <p className="actions"><Act symbol="retire" name={'Yes, retire ' + account.name} word="Retire" tone="danger" disabled={change.blocked} onClick={() => change.submit({ operation: operationId() })} />{' '}<Act symbol="close" name="Cancel" word="Cancel" disabled={change.busy} onClick={cancel} /></p>}
    <ChangeStatus change={change} />
  </section>;
}
