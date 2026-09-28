/** The emergency stop of an agent: suspend it, withdraw its certificates, end its credential handles and ask every runtime to end its sessions. Nothing here claims a session ended. */
import { useState } from 'react';
import { operationId } from '../../api';
import { ChangeStatus } from '../roles/ChangeStatus';
import { useRoleChange } from '../roles/useRoleChange';

/** The answer of `POST /agents/{id}/stop`. */
export interface StopAnswer {
  agent: string; operation: string; state: string; certificates_withdrawn: string[];
  credentials_ended: string[] | null; credentials_refused: string | null; sessions_asked: string[]; reason: string;
}

export function EmergencyStop({ id, stopped }: { id: string; stopped: () => void }) {
  const [open, setOpen] = useState(false);
  const [reason, setReason] = useState('');
  const [answer, setAnswer] = useState<StopAnswer | null>(null);
  const change = useRoleChange<StopAnswer>('lys.pending.stop.' + id, '/agents/' + encodeURIComponent(id) + '/stop',
    (result, body) => { if (result.agent !== id || result.operation !== body.operation || result.state !== 'suspended') return false; setAnswer(result); return true; }, stopped);
  return <>
    {!open && !answer ? <button className="btn danger" data-act="stop" title="Suspend it, withdraw its certificates, end its credentials and ask every runtime to end its sessions" disabled={change.blocked} onClick={() => setOpen(true)}>Emergency stop</button> : null}
    {open && !answer ? <form className="card" aria-label="Confirm emergency stop" onSubmit={(event) => { event.preventDefault(); if (reason.trim()) change.submit({ operation: operationId(), reason: reason.trim() }); }}>
      <p>This suspends the agent at once, withdraws its certificates, ends every credential handle it holds, and asks the runtime of each of its sessions to end it. A session shows unconfirmed until its runtime reports it stopped.</p>
      <label className="field">Why<input required maxLength={500} value={reason} disabled={change.blocked} onChange={(event) => setReason(event.target.value)} placeholder="What happened" /></label>
      <button className="btn danger" type="submit" disabled={change.blocked || !reason.trim()}>Stop this agent now</button>{' '}
      <button className="btn" type="button" disabled={change.busy} onClick={() => setOpen(false)}>Cancel</button>
    </form> : null}
    {answer ? <div className="card" role="status" aria-label="Emergency stop recorded">
      <p>Stopped: the agent is suspended.</p>
      <p>Certificates withdrawn: {answer.certificates_withdrawn.length ? answer.certificates_withdrawn.join(', ') : 'none held'}.</p>
      {answer.credentials_ended !== null ? <p>Credential handles ended: {answer.credentials_ended.length ? answer.credentials_ended.join(', ') : 'none held'}.</p> : <p className="why-not" role="alert">Credential handles were not ended: {answer.credentials_refused}. End them from the Credentials tab once the broker answers.</p>}
      <p>Sessions asked to end: {answer.sessions_asked.length ? answer.sessions_asked.join(', ') : 'none open'}. Each stays unconfirmed until its runtime reports it stopped.</p>
    </div> : null}
    <ChangeStatus change={change} />
  </>;
}
