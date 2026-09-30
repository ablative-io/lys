/** Secrets navigation connects metadata reads; owner mutations remain in their explicit advanced flow. */
import { Link, useParams } from 'react-router';
import { request } from '../../api';
import { Secrets } from './Secrets';
import type { SecretListing } from './Secrets';
import { RevocationLookup, SecretAudit, SecretGrants } from './SecretsDetail';
import { SecretControls } from './SecretControls';
import { secretsApi } from './secretsApi';

const listing = () => request<SecretListing>('/secrets');
const TABS = [['entries', 'Entries'], ['grants', 'Access'], ['audit', 'Activity'], ['revocation', 'Check revoked access'], ['manage', 'Advanced']] as const;
export function SecretsPage() {
  const { section = 'entries' } = useParams();
  return <div className="page fill">
    <div className="head"><div><div className="eyebrow">Secrets</div><h1>Secrets</h1></div></div>
    <nav className="tabs" aria-label="Secrets views">
      {TABS.map(([key, label]) => <Link key={key} className={section === key ? 'on' : ''} aria-current={section === key ? 'page' : undefined} to={'/secrets/' + key}>{label}</Link>)}
    </nav>
    {section === 'entries' ? <Secrets read={listing} />
      : <div className="pane">{section === 'grants' ? <SecretGrants read={secretsApi.grants} />
      : section === 'audit' ? <SecretAudit read={secretsApi.audit} />
      : section === 'manage' ? <SecretControls />
      : section === 'revocation' ? <RevocationLookup check={secretsApi.revocation} />
      : <><h2>Secrets view not found</h2><p>Choose a view above to continue.</p></>}</div>}
  </div>;
}
