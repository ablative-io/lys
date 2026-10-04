import { useState } from 'react';
import { IssueCertificate, WithdrawCertificate } from './CertificateChanges';
/** Certificate records expose issuance claims and offline proof material without claiming current permission or verified trust. */
import { api, request, useLoad } from '../../api';
import { DirectoryGate as Gate } from '../people/Words';
import { clock } from './time';
import { IdentityName } from '../people/Words';
export interface AgentCertificate {
  serial: string; person: string; claims: unknown; der: string; issued_at: number;
  withdrawn: null | { serial: string; by: string; reason: string; withdrawn_at: number };
  entry: { leaf: number; leaf_bytes: string; tree_size: number; root: string; proof: string };
}
export interface CertificatesAnswer { agent: string; recorded?: string | null; certificates: AgentCertificate[]; claims_are_live: false }
const count = (n: number, one: string, many: string) => n + ' ' + (n === 1 ? one : many);
const record = (value: unknown): value is Record<string, unknown> => Boolean(value) && typeof value === 'object' && !Array.isArray(value);

/**
 * The claims a certificate carries (crates/lys-identity-server/src/certificates_issue.rs, `claims`), as a short
 * facts list. The agent and the person are named only where they differ from this page's agent and the person
 * the heading already names, so nothing is said twice. Claims of another shape are said as such, never dumped.
 */
export function Claims({ claims, agent, person }: { claims: unknown; agent: string; person: string }) {
  if (!record(claims)) return <p className="dim">The claims are not in a form this screen can read; they are in the evidence download.</p>;
  const { grants, roles, profile_version: version, held_at: held } = claims;
  return <dl className="facts">
    {typeof held === 'number' ? <><dt>Held at</dt><dd>{clock(held)}</dd></> : null}
    {Array.isArray(grants) ? <><dt>Grants</dt><dd>Holds {count(grants.length, 'grant', 'grants')}</dd></> : null}
    {Array.isArray(roles) ? <><dt>Roles</dt><dd>{roles.length ? 'Holds ' + count(roles.length, 'role', 'roles') : 'No roles'}</dd></> : null}
    <dt>Settings</dt><dd>{typeof version === 'number' ? 'Version ' + version : 'No settings saved'}</dd>
    {typeof claims.agent === 'string' && claims.agent !== agent ? <><dt>Agent</dt><dd><IdentityName id={claims.agent} /></dd></> : null}
    {typeof claims.person === 'string' && claims.person !== person ? <><dt>Person</dt><dd><IdentityName id={claims.person} /></dd></> : null}
  </dl>;
}

export function AgentCertificates({ id }: { id: string }) {
  const [revision, setRevision] = useState(0);
  const caller = useLoad(api.me, 'certificate-caller');
  const refresh = () => setRevision((value) => value + 1);
  const load = useLoad(async () => {
    const answer = await request<CertificatesAnswer>('/agents/' + encodeURIComponent(id) + '/certificates');
    if (answer.agent !== id || answer.claims_are_live !== false || !Array.isArray(answer.certificates)) throw new Error('The certificate answer did not identify this agent and its issuance-only claims.');
    return answer;
  }, 'agent-certificates:' + id + ':' + revision);
  return <section className="card certificates"><h2>Capability certificates</h2><p>A certificate records what held when it was issued. Current permissions are checked in <a href={'#/access/reach/' + id}>Access</a>.</p>
    <Gate load={load} title="Certificates" ok={(answer) => answer.certificates.length ? answer.certificates.map((certificate) => <section className="certificate" key={certificate.serial}>
      {/* Titled by when it was issued and to whom; the serial is a technical detail, small. */}
      <h3>Issued {clock(certificate.issued_at)} for <IdentityName id={certificate.person} /> <small className="refusal-name">{certificate.serial}</small></h3>
      <dl className="facts"><dt>Withdrawal record</dt><dd>{certificate.withdrawn ? <>Withdrawn at {clock(certificate.withdrawn.withdrawn_at)} by {/^(person|agent)-[0-9a-f]{32}$/.test(certificate.withdrawn.by) ? <IdentityName id={certificate.withdrawn.by} /> : certificate.withdrawn.by}: {certificate.withdrawn.reason}</> : 'No withdrawal recorded'}</dd><dt>Log entry</dt><dd>{certificate.entry.leaf} of {certificate.entry.tree_size} leaves</dd></dl>
      <h4>Claims at issuance</h4><Claims claims={certificate.claims} agent={id} person={certificate.person} />
      <h4>Certificate and inclusion proof</h4><p>These are the bytes returned by the certificate log. This screen has not independently verified their signatures or your trust in the issuer.</p><p>Root: <span className="mono">{certificate.entry.root}</span></p>
        <a className="btn" download="certificate.der" href={'data:application/pkix-cert;base64,' + certificate.der}>Download certificate</a>{' '}
        <a className="btn" download="certificate-evidence.json" href={'data:application/json;charset=utf-8,' + encodeURIComponent(JSON.stringify({ agent: id, claims_are_live: false, certificate }, null, 2))}>Download evidence</a>
      {caller.status === 'ok' ? <WithdrawCertificate key={certificate.serial + ':' + revision} agent={id} serial={certificate.serial} withdrawn={certificate.withdrawn !== null} person={caller.data.person.id} changed={refresh} /> : null}
    </section>) : <p>No certificate has been entered for this agent.</p>} />
    {caller.status === 'ok' && load.status === 'ok' ? <IssueCertificate key={id + ':' + revision} agent={id} person={caller.data.person.id} changed={refresh} /> : null}
  </section>;
}
