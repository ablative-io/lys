/** Metadata-only secrets listing; authentication and visibility belong to the broker adapter. */
import { useState } from 'react';
import { useLoad } from '../../api';
import { Gate } from '../signin/Gate';

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
  return <div className="page">
    <div className="head"><div><div className="eyebrow">Runtime</div><h1>Secrets</h1>
      <p className="sub">Entries the broker permits you to discover. Credentials stay in the broker.</p></div>
    </div>
    <Gate load={load} title="Secrets" ok={(listing) => <SecretRows listing={listing} />} />
  </div>;
}

export function SecretRows({ listing }: { listing: SecretListing }) {
  const [filter, setFilter] = useState('');
  const visible = listing.secrets.filter((entry) => (entry.name + ' ' + entry.class + ' ' + entry.owner).toLowerCase().includes(filter.toLowerCase()));
  return <>
    <label className="field">Find a secret<input type="search" value={filter} onChange={(event) => setFilter(event.target.value)} placeholder="Name, kind or owner" /></label>
    <table><thead><tr><th>Name</th><th>Kind</th><th>Owner</th><th>Recorded sequence</th></tr></thead>
      <tbody>{visible.map((entry) => <tr key={entry.name}>
        <td>{entry.name}</td><td>{entry.class}</td><td>{entry.owner}</td><td>{entry.sequence}</td>
      </tr>)}</tbody>
    </table>
    {!visible.length ? <p className="note">{listing.secrets.length ? 'No visible entry matches this search.' : 'No entries were returned within your discovery scope.'}</p> : null}
    <p className="note">Being able to discover an entry does not grant permission to use or delegate it. This screen never requests a credential value or a bearer handle.</p>
  </>;
}
