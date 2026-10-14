/** Secrets navigation connects metadata reads; owner mutations remain in their explicit advanced flow. */
import { Link, useParams } from 'react-router';
import { request } from '../../api';
import { Secrets } from './Secrets';
import type { SecretListing } from './Secrets';
import { SecretAudit, SecretGrants } from './SecretsDetail';
import { secretsApi } from './secretsApi';
import { PageHeader } from '../../shell/PageHeader';
import { NotFound } from '../../shell/NotFound';

const listing = () => request<SecretListing>('/secrets');
const TABS = [['entries', 'Entries'], ['grants', 'Access'], ['audit', 'Activity']] as const;
/** Views that were tabs of their own and are now part of another: a secret's controls open from its row on Entries; checking a withdrawn handle is on Activity. */
const MERGED: Record<string, string> = { manage: 'entries', revocation: 'audit' };
export function SecretsPage() {
  const { section: asked = 'entries' } = useParams();
  const section = MERGED[asked] ?? asked;
  if (!TABS.some(([key]) => key === section)) return <NotFound />;
  return <div className="page fill">
    <PageHeader title="Secrets" subtitle="A secret's value is never shown, and seeing it here does not give permission to use it." />
    <nav className="tabs" aria-label="Secrets views">
      {TABS.map(([key, label]) => <Link key={key} className={section === key ? 'on' : ''} aria-current={section === key ? 'page' : undefined} to={'/secrets/' + key}>{label}</Link>)}
    </nav>
    {section === 'entries' ? <Secrets read={listing} />
      : <div className="pane">{section === 'grants' ? <SecretGrants read={secretsApi.grants} />
      : section === 'audit' ? <SecretAudit read={secretsApi.audit} check={secretsApi.revocation} />
      : <><h2>Secrets view not found</h2><p>Choose a view above to continue.</p></>}</div>}
  </div>;
}
