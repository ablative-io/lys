/** Certificate records expose issuance claims and offline proof material without claiming current permission or verified trust. */
import { request, useLoad } from '../../api';
import { Gate } from '../signin/Gate';
import { clock } from './time';
export interface AgentCertificate {
  serial: string; person: string; claims: unknown; der: string; issued_at: number;
  withdrawn: null | { serial: string; by: string; reason: string; withdrawn_at: number };
  entry: { leaf: number; leaf_bytes: string; tree_size: number; root: string; proof: string };
}
export interface CertificatesAnswer { agent: string; certificates: AgentCertificate[]; claims_are_live: false }
export function AgentCertificates({ id }: { id: string }) {
  const load = useLoad(async () => {
    const answer = await request<CertificatesAnswer>('/agents/' + encodeURIComponent(id) + '/certificates');
    if (answer.agent !== id || answer.claims_are_live !== false || !Array.isArray(answer.certificates)) throw new Error('The certificate answer did not identify this agent and its issuance-only claims.');
    return answer;
  }, 'agent-certificates:' + id);
  return <section><h2>Capability certificates</h2><p>A certificate records what held when it was issued. Current permissions are checked in <a href={'#/access/reach/' + id}>Access</a>.</p>
    <Gate load={load} title="Certificates" ok={(answer) => answer.certificates.length ? answer.certificates.map((certificate) => <section className="card" key={certificate.serial}>
      <h3>Certificate <span className="mono">{certificate.serial}</span></h3><dl className="facts"><dt>Issued for</dt><dd><a href={'#/file/' + certificate.person}>{certificate.person}</a></dd><dt>Issued at</dt><dd>{clock(certificate.issued_at)}</dd><dt>Withdrawal record</dt><dd>{certificate.withdrawn ? <>Withdrawn at {clock(certificate.withdrawn.withdrawn_at)} by {certificate.withdrawn.by}: {certificate.withdrawn.reason}</> : 'No withdrawal recorded'}</dd><dt>Log entry</dt><dd>{certificate.entry.leaf} of {certificate.entry.tree_size} leaves</dd></dl>
      <details><summary>Claims at issuance</summary><pre>{JSON.stringify(certificate.claims, null, 2)}</pre></details>
      <details><summary>Certificate and inclusion proof</summary><p>These are the bytes returned by the certificate log. This screen has not independently verified their signatures or your trust in the issuer.</p><p>Root: <span className="mono">{certificate.entry.root}</span></p>
        <a className="btn" download="certificate.der" href={'data:application/pkix-cert;base64,' + certificate.der}>Download certificate</a>{' '}
        <a className="btn" download="certificate-evidence.json" href={'data:application/json;charset=utf-8,' + encodeURIComponent(JSON.stringify({ agent: id, claims_are_live: false, certificate }, null, 2))}>Download evidence</a>
      </details>
    </section>) : <section className="card"><p>No certificate has been entered for this agent.</p></section>} />
  </section>;
}
