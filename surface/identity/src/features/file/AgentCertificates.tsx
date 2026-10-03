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
export function AgentCertificates({ id }: { id: string }) {
  const [revision, setRevision] = useState(0);
  const caller = useLoad(api.me, 'certificate-caller');
  const refresh = () => setRevision((value) => value + 1);
  const load = useLoad(async () => {
    const answer = await request<CertificatesAnswer>('/agents/' + encodeURIComponent(id) + '/certificates');
    if (answer.agent !== id || answer.claims_are_live !== false || !Array.isArray(answer.certificates)) throw new Error('The certificate answer did not identify this agent and its issuance-only claims.');
    return answer;
  }, 'agent-certificates:' + id + ':' + revision);
  return <section><h2>Capability certificates</h2><p>A certificate records what held when it was issued. Current permissions are checked in <a href={'#/access/reach/' + id}>Access</a>.</p>
    <Gate load={load} title="Certificates" ok={(answer) => answer.certificates.length ? answer.certificates.map((certificate) => <section className="card" key={certificate.serial}>
      <h3>Certificate <span className="mono">{certificate.serial}</span></h3><dl className="facts"><dt>Issued for</dt><dd><IdentityName id={certificate.person} /></dd><dt>Issued at</dt><dd>{clock(certificate.issued_at)}</dd><dt>Withdrawal record</dt><dd>{certificate.withdrawn ? <>Withdrawn at {clock(certificate.withdrawn.withdrawn_at)} by {certificate.withdrawn.by}: {certificate.withdrawn.reason}</> : 'No withdrawal recorded'}</dd><dt>Log entry</dt><dd>{certificate.entry.leaf} of {certificate.entry.tree_size} leaves</dd></dl>
      <h4>Claims at issuance</h4><pre>{JSON.stringify(certificate.claims, null, 2)}</pre>
      <h4>Certificate and inclusion proof</h4><p>These are the bytes returned by the certificate log. This screen has not independently verified their signatures or your trust in the issuer.</p><p>Root: <span className="mono">{certificate.entry.root}</span></p>
        <a className="btn" download="certificate.der" href={'data:application/pkix-cert;base64,' + certificate.der}>Download certificate</a>{' '}
        <a className="btn" download="certificate-evidence.json" href={'data:application/json;charset=utf-8,' + encodeURIComponent(JSON.stringify({ agent: id, claims_are_live: false, certificate }, null, 2))}>Download evidence</a>
      {caller.status === 'ok' ? <WithdrawCertificate key={certificate.serial + ':' + revision} agent={id} serial={certificate.serial} withdrawn={certificate.withdrawn !== null} person={caller.data.person.id} changed={refresh} /> : null}
    </section>) : <section className="card"><p>No certificate has been entered for this agent.</p></section>} />
    {caller.status === 'ok' && load.status === 'ok' ? <IssueCertificate key={id + ':' + revision} agent={id} person={caller.data.person.id} changed={refresh} /> : null}
  </section>;
}
