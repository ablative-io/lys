/** Metadata-only secrets listing; authentication and visibility belong to the broker adapter. */
import { useState } from 'react';
import { api, useLoad } from '../../api';
import type { PeopleView } from '../../generated';
import { entries } from '../people/directory';
import { Gate } from '../signin/Gate';
import { IdentityName, ReadFailure } from '../signin/words';

/** The existing /_lys/secrets response in lys-secrets/bin/lys-secrets/view.rs. */
export interface SecretListing {
  secrets: {
    name: string;
    class: 'credential' | 'key' | 'memory' | 'oauth';
    owner: string;
    sequence: number;
    upstream: string | null;
    header: string | null;
  }[];
}

export function Secrets({ read }: { read: () => Promise<SecretListing> }) {
  const load = useLoad(read, 'secrets');
  const people = useLoad(api.people, 'secret-people');
  return <div className="page">
    <div className="head"><div>
      <p className="sub">Find the secrets you are allowed to see. Their values stay in Lys secret storage.</p></div>
    </div>
    {people.status === 'refused' ? <ReadFailure error={people.refused} subject="secret owners’ names" /> : null}
    <Gate load={load} title="your secrets" renderError={(error) => <ReadFailure error={error} subject="your secrets" administrator={people.status === 'ok' && people.data.scope === 'directory'} />} ok={(listing) => <SecretRows listing={listing} people={people.status === 'ok' ? people.data : undefined} />} />
  </div>;
}

export function SecretRows({ listing, people }: { listing: SecretListing; people?: PeopleView }) {
  const [filter, setFilter] = useState('');
  const names = new Map(people ? entries(people).map((entry) => [entry.id, entry.display_name]) : []);
  const visible = listing.secrets.filter((entry) => (entry.name + ' ' + entry.class + ' ' + (names.get(entry.owner) ?? '')).toLowerCase().includes(filter.toLowerCase()));
  return <>
    <label className="field">Find a secret<input type="search" value={filter} onChange={(event) => setFilter(event.target.value)} placeholder="Name, kind or owner" /></label>
    <table><thead><tr><th>Name</th><th>Kind</th><th>Owner</th><th>Recorded sequence</th></tr></thead>
      <tbody>{visible.map((entry) => <tr key={entry.name}>
        <td>{entry.name}</td><td>{entry.class}</td><td><IdentityName id={entry.owner} people={people} /></td><td><details><summary>Storage details</summary>Recorded change: {entry.sequence}</details></td>
      </tr>)}</tbody>
    </table>
    {!visible.length ? <p className="note">{listing.secrets.length ? 'No visible entry matches this search.' : 'No secrets were returned for this account.'}</p> : null}
    <p className="note">Seeing a secret here does not give permission to use it or share that permission. Its value is never shown.</p>
  </>;
}
