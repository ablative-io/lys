/**
 * The controls of one secret, shown on Entries above the list when its row is opened: what it is set to now, its visibility,
 * and who may receive it. Only its owner can save a change. A secret Lys keeps for an approved app is the app's: it is shown
 * here, with its settings, and changed only with the app on Apps, because replacing or retiring it here cuts the app off.
 */
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
import { ReplaceValue, RetireSecret } from './ValueChange';
import { Act } from '../../shell/Act';
import { entries } from '../people/directory';
import { appOfSecret, secretName } from './secret-name';

/** The people and teams a visibility is chosen among, read together. */
const useScopeNames = () => useLoad(() => readTogether({ people: api.people(), teams: readTeams() }), 'secret-scope-names');

export function SecretControls({ listing, changed: listed }: { listing: SecretListing; changed?: (message: string) => void }) {
  const [params, setParams] = useSearchParams();
  const selected = params.get('secret') ?? '';
  const [revision, setRevision] = useState(0);
  const changed = () => setRevision((value) => value + 1);
  const names = useScopeNames();
  if (!selected) return null;
  if (!listing.secrets.some((entry) => entry.name === selected)) return <p className="why-not" role="alert">No secret named {selected} is visible to this account.</p>;
  const named = new Map(names.status === 'ok' ? entries(names.data.people).map((entry) => [entry.id, entry.display_name]) : []);
  const app = appOfSecret(selected);
  return <section className="secrets-view secret-controls" aria-label={'Controls of ' + selected} key={selected}>
    <div className="row"><h2 title={selected}>{secretName(selected, named)}</h2><Act symbol="close" name="Close" word="Close" onClick={() => setParams({})} /></div>
    {app ? <AppsSecret app={app} /> : null}
    <div className="form-grid">
      <CurrentSettings secret={selected} revision={revision} changeable={!app} />
      {app ? null : <Changes selected={selected} names={names} changed={changed} listed={listed} close={() => setParams({})} />}
    </div>
  </section>;
}

/** Said on a secret Lys keeps for an approved app: whose it is, how it changes, and where. */
function AppsSecret({ app }: { app: string }) {
  const words = app.replace(/[._-]/g, ' ');
  const name = words.charAt(0).toUpperCase() + words.slice(1);
  return <p className="app-owned" role="note">This secret is {name}’s own, kept for it by Lys when {name} was approved. It changes only with {name}: connecting {name} again replaces it, and retiring {name} ends it, both on <a href="#/apps">Apps</a>. Changing or retiring it here would cut {name} off from Lys, so it cannot be done here.</p>;
}

/** The four changes an owner makes to a secret of their own: who sees it, who it is handed to, its value, and retiring it. */
function Changes({ selected, names, changed, listed, close }: { selected: string; names: ReturnType<typeof useScopeNames>; changed: () => void; listed?: (message: string) => void; close: () => void }) {
  const current = useLoad(() => secretsApi.settings(selected), 'secret-recipients:' + selected);
  const held = current.status === 'ok' && current.data.secret === selected ? current.data.recipients : null;
  return <>
      <section className="card"><ScopeChange secret={selected} change={secretsApi.scope} changed={changed}
        people={names.status === 'ok' ? names.data.people.people.map((person) => ({ id: person.id, name: person.display_name })) : undefined}
        teams={names.status === 'ok' ? names.data.teams.filter((team) => team.state === 'active').map((team) => team.name) : undefined} />
        {names.status === 'refused' ? <><p className="note">People and teams could not be read, so the name is typed.</p><ErrorWords problem={names.refused} /></> : null}</section>
      <section className="card"><RecipientsChange secret={selected} current={held} unread={current.status === 'refused'} change={secretsApi.recipients} changed={changed} /></section>
      <section className="card"><ReplaceValue secret={selected} changed={() => { changed(); listed?.(selected + '’s value was changed.'); }} /></section>
      <section className="card"><RetireSecret secret={selected} retired={(message) => { close(); listed?.(message); }} /></section>
  </>;
}
