/** Certificate acts take an agent-made public request and retain uncertain submissions; no private key is generated here. */
import { useState } from 'react';
import type { FormEvent } from 'react';
import { operationId } from '../../api';
import { useRoleChange } from '../roles/useRoleChange';
import { ChangeStatus } from '../roles/ChangeStatus';
import type { CertificatesAnswer } from './AgentCertificates';
import { Act } from '../../shell/Act';

export function IssueCertificate({ agent, person, changed }: { agent: string; person: string; changed: () => void }) {
  const [csr, setCsr] = useState('');
  const [problem, setProblem] = useState('');
  const path = '/agents/' + encodeURIComponent(agent) + '/certificates';
  const change = useRoleChange<CertificatesAnswer>('lys.pending.certificate-issue.' + person + '.' + agent, path,
    (answer, body) => answer.agent === agent && answer.recorded === body.operation && answer.certificates.some((certificate) => certificate.serial === body.operation), changed);
  const submit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const request = csr.replace(/\s/g, '');
    if (!request || !/^[A-Za-z0-9+/]+={0,2}$/.test(request) || request.length % 4 !== 0) { setProblem('Paste the PKCS#10 request as standard base64, without PEM header lines. Never paste a private key.'); return; }
    setProblem(''); change.submit({ operation: operationId(), request });
  };
  return <form className="card" aria-label="Issue capability certificate" onSubmit={submit}><h3>Issue a certificate</h3><p>The agent must create a public PKCS#10 request using its own key. Its common name must be <span className="mono">{agent}</span>. Lys checks possession and gathers the claims.</p>
    <label className="field">Public certificate request (DER encoded as base64)<textarea value={csr} onChange={(event) => setCsr(event.target.value)} required disabled={change.blocked} rows={5} autoComplete="off" spellCheck={false} /></label>
    <p className="note">Do not paste a private key. This page does not create or take custody of the agent’s key.</p>
    <Act symbol="add" name="Issue certificate" word="Issue" tone="primary" type="submit" disabled={change.blocked || !csr.trim()} />
    {problem ? <p role="alert">{problem}</p> : null}<ChangeStatus change={change} />
  </form>;
}

export function WithdrawCertificate({ agent, serial, person, withdrawn, changed }: { agent: string; serial: string; person: string; withdrawn: boolean; changed: () => void }) {
  const [confirm, setConfirm] = useState(false);
  const [reason, setReason] = useState('');
  const path = '/agents/' + encodeURIComponent(agent) + '/certificates/' + encodeURIComponent(serial) + '/withdrawal';
  const change = useRoleChange<CertificatesAnswer>('lys.pending.certificate-withdraw.' + person + '.' + serial, path, (answer, body) => answer.agent === agent && answer.recorded === serial && answer.certificates.some((certificate) => certificate.serial === serial && certificate.withdrawn?.by === person && certificate.withdrawn.reason === body.reason), changed);
  return <>{!withdrawn && !confirm && !change.done ? <Act symbol="revoke" name="Withdraw certificate" word="Withdraw" tone="danger" disabled={change.blocked} onClick={() => setConfirm(true)} /> : null}
    {confirm ? <form aria-label="Withdraw capability certificate" onSubmit={(event) => { event.preventDefault(); change.submit({ reason: reason.trim() }); }}><p>Record withdrawal of this certificate? Its issuance remains in the log. This does not separately revoke grants.</p><label className="field">Reason<input value={reason} onChange={(event) => setReason(event.target.value)} required disabled={change.blocked} /></label><Act symbol="approve" name="Confirm withdrawal" word="Confirm" tone="danger" type="submit" disabled={change.blocked || !reason.trim()} /><Act symbol="close" name="Cancel" word="Cancel" disabled={change.busy} onClick={() => setConfirm(false)} /></form> : null}
    <ChangeStatus change={change} />
  </>;
}
