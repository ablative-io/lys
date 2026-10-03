/** The emergency stop of an agent: suspend it, withdraw its certificates, end its credential handles and ask every runtime to end its sessions. Nothing here claims a session ended. */
import { useState } from 'react';
import { operationId } from '../../api';
import { DirectoryChangeStatus as ChangeStatus } from '../roles/ChangeStatus';
import { useRoleChange } from '../roles/useRoleChange';

/** The answer of `POST /agents/{id}/stop`. */
export interface StopAnswer {
  agent: string; operation: string; state: string; by: string; at: number; certificates_withdrawn: string[];
  credentials_ended: string[] | null; credentials_refused: string | null; sessions_asked: string[]; reason: string; sessions_confirmed?: { session: string; at: number; confirmation: string }[]; sessions_refused?: string[];
}

/** Shown while the agent is active (the button) and while it is suspended (a retained stop). `stopped` receives the service's answer. */
export function EmergencyStop({ id, active, stopped }: { id: string; active: boolean; stopped: (answer: StopAnswer) => void }) {
  const [open, setOpen] = useState(false);
  const [reason, setReason] = useState('');
  const change = useRoleChange<StopAnswer>('lys.pending.stop.' + id, '/agents/' + encodeURIComponent(id) + '/stop',
    (result, body) => { if (result.agent !== id || result.operation !== body.operation || result.state !== 'suspended') return false; stopped(result); return true; }, () => undefined);
  return <>
    {active && !open ? <button className="btn danger" data-act="stop" title="Suspend access, withdraw certificates and ask the credential service and runtimes to end their use" disabled={change.blocked} onClick={() => setOpen(true)}>Emergency stop</button> : null}
    {open && active ? <form className="card" aria-label="Confirm emergency stop" onSubmit={(event) => { event.preventDefault(); if (reason.trim()) change.submit({ operation: operationId(), reason: reason.trim() }); }}>
      <p>This suspends the agent at once, withdraws its certificates, asks the credential service to end every credential it holds, and asks the runtime of each of its sessions to end it. A session shows unconfirmed until its runtime reports it stopped.</p>
      <label className="field">Why<input required maxLength={500} value={reason} disabled={change.blocked} onChange={(event) => setReason(event.target.value)} placeholder="What happened" /></label>
      <button className="btn danger" type="submit" disabled={change.blocked || !reason.trim()}>Stop this agent now</button>{' '}
      <button className="btn" type="button" disabled={change.busy} onClick={() => setOpen(false)}>Cancel</button>
    </form> : null}
    <ChangeStatus change={change} />
  </>;
}

/** What the stop just did, as the service answered it. */
export function StopReceipt({ answer }: { answer: StopAnswer }) {
  const confirmed = answer.sessions_confirmed ?? [];
  const refused = answer.sessions_refused ?? [];
  return <div className="card" role="status" aria-label="Emergency stop recorded">
    <p>The agent’s access is suspended.</p>
    <p>Certificates withdrawn: {answer.certificates_withdrawn.length}.</p>
    {answer.credentials_ended !== null ? <p>Credentials ended at the credential service: {answer.credentials_ended.length}.</p> : <div className="why-not" role="alert"><p>The credential service did not confirm that credentials ended. Open the Credentials tab and end them after the service answers.</p><p><small className="refusal-name">{answer.credentials_refused}</small></p></div>}
    <p>Sessions asked to end: {answer.sessions_asked.length}. Confirmed stopped by their runtime: {confirmed.length}. Other sessions remain unconfirmed; open the Sessions tab to read their reports.</p>
    {refused.length ? <p role="alert">A runtime could not complete {refused.length} end request(s). Ask the administrator to check the session error details.</p> : null}
    <p className="refusal-name">Request {answer.operation}</p>
  </div>;
}
