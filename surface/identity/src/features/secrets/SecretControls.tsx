/** The controls of one secret, shown on Entries above the list when its row is opened: what it is set to now, its visibility, and who may receive it. Only its owner can save a change. */
import { useState } from 'react';
import { api, useLoad } from '../../api';
import { readTogether } from '../../reads';
import { ErrorWords } from '../people/Words';
import { readTeams } from '../teams/Teams';
import { useSearchParams } from 'react-router';
import type { SecretListing } from './Secrets';
import { RecipientsChange, ScopeChange } from './SecretsDetail';
import { secretsApi } from './secretsApi';
import { CurrentSettings } from './CurrentSettings';

export function SecretControls({ listing }: { listing: SecretListing }) {
  const [params, setParams] = useSearchParams();
  const selected = params.get('secret') ?? '';
  const [revision, setRevision] = useState(0);
  const changed = () => setRevision((value) => value + 1);
  const names = useLoad(() => readTogether({ people: api.people(), teams: readTeams() }), 'secret-scope-names');
  if (!selected) return null;
  if (!listing.secrets.some((entry) => entry.name === selected)) return <p className="why-not" role="alert">No secret named {selected} is visible to this account.</p>;
  return <section className="secrets-view secret-controls" aria-label={'Controls of ' + selected} key={selected}>
    <div className="row"><h2>{selected}</h2><button type="button" className="btn" onClick={() => setParams({})}>Close</button></div>
    <div className="form-grid">
      <CurrentSettings secret={selected} revision={revision} />
      <section className="card"><ScopeChange secret={selected} change={secretsApi.scope} changed={changed}
        people={names.status === 'ok' ? names.data.people.people.map((person) => ({ id: person.id, name: person.display_name })) : undefined}
        teams={names.status === 'ok' ? names.data.teams.filter((team) => team.state === 'active').map((team) => team.name) : undefined} />
        {names.status === 'refused' ? <><p className="note">People and teams could not be read, so the name is typed.</p><ErrorWords problem={names.refused} /></> : null}</section>
      <section className="card"><RecipientsChange secret={selected} change={secretsApi.recipients} changed={changed} /></section>
    </div>
  </section>;
}
