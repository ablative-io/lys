/**
 * Withdraw, the emergency act on an agent (AGENTS-017 R2; the route is `POST /agents/{id}/stop`, unchanged): suspend
 * it, withdraw its certificates, end its credential handles and ask every runtime to end its sessions. It is final:
 * the agent that registered this agent cannot certify it again, and a person issues its next certificate. The
 * ordinary Stop of one run is elsewhere (run/SessionStop). Nothing here claims a session ended.
 */
import { useState } from 'react';
import { operationId } from '../../api';
import { ChangeStatus } from '../roles/ChangeStatus';
import { useRoleChange } from '../roles/useRoleChange';
import { Act } from '../../shell/Act';
import { ErrorWords } from '../people/Words';

/** The act's one name, in its button, its heading and its confirmation. */
export const WITHDRAW = 'Withdraw';

/** What Withdraw leaves final, when the service's answer does not say it in its own words. */
export const WITHDRAW_FINAL = 'Its certificates are withdrawn for good: the agent that registered it cannot certify it again, and a person issues its next certificate.';

/** The answer of `POST /agents/{id}/stop`. */
export interface StopAnswer {
  agent: string; operation: string; state: string; by: string; at: number; certificates_withdrawn: string[];
  credentials_ended: string[] | null; credentials_refused: string | null; sessions_asked: string[]; reason: string; sessions_confirmed?: { session: string; at: number; confirmation: string }[]; sessions_refused?: string[];
  /** The sentence that says what the act leaves final (AGENTS-017 R2); absent from a service older than it. */
  final_words?: string;
}

/** The named withdrawal form and any retained request stay together, including after the identity is suspended. */
export function EmergencyStop({ id, active, stopped, open = false, cancel }: { id: string; active: boolean; stopped: (answer: StopAnswer) => void; open?: boolean; cancel?: () => void }) {
  const [reason, setReason] = useState('');
  const change = useRoleChange<StopAnswer>('lys.pending.stop.' + id, '/agents/' + encodeURIComponent(id) + '/stop',
    (result, body) => { if (result.agent !== id || result.operation !== body.operation || result.state !== 'suspended') return false; stopped(result); return true; }, () => undefined);
  return <>
    {open && active ? <form className="card" aria-label="Confirm withdraw" onSubmit={(event) => { event.preventDefault(); if (reason.trim()) change.submit({ operation: operationId(), reason: reason.trim() }); }}>
      <h3>{WITHDRAW}</h3>
      <p>This pauses the agent at once, withdraws its certificates, asks the credential service to end every credential it holds, and asks the runtime of each of its sessions to end it. A session shows unconfirmed until its runtime reports it stopped.</p>
      <p className="why-not">{WITHDRAW_FINAL} To end one run and keep the agent, use Stop beside that run instead.</p>
      <label className="field">Why<input required value={reason} disabled={change.blocked} onChange={(event) => setReason(event.target.value)} placeholder="What happened" /></label>
      <Act symbol="suspend" name="Withdraw this agent now" word={WITHDRAW} tone="danger" type="submit" disabled={change.blocked || !reason.trim()} />{' '}
      <Act symbol="close" name="Cancel" word="Cancel" disabled={change.busy} onClick={cancel} />
    </form> : null}
    <ChangeStatus change={change} />
  </>;
}

/** What the withdrawal just did, as the service answered it. */
export function StopReceipt({ answer }: { answer: StopAnswer }) {
  const confirmed = answer.sessions_confirmed ?? [];
  const refused = answer.sessions_refused ?? [];
  return <div className="card" role="status" aria-label="Agent suspension recorded">
    <p>The agent’s access is suspended.</p>
    <p>Certificates withdrawn: {answer.certificates_withdrawn.length}.</p>
    <p data-withdraw-final>{answer.final_words ?? WITHDRAW_FINAL}</p>
    {answer.credentials_ended !== null ? <p>Credentials ended at the credential service: {answer.credentials_ended.length}.</p> : <div role="alert"><p className="why-not">The credential service did not confirm that credentials ended. Open the Credentials tab and end them after the service answers.</p>{answer.credentials_refused ? <ErrorWords problem={answer.credentials_refused} /> : null}</div>}
    <p>Sessions asked to end: {answer.sessions_asked.length}. Confirmed stopped by their runtime: {confirmed.length}. Other sessions remain unconfirmed; open the Sessions tab to read their reports.</p>
    {refused.length ? <p role="alert">{refused.length === 1 ? 'One session could not be asked to end.' : refused.length + ' sessions could not be asked to end.'} Open the Sessions tab to see them, and ask the administrator to check those computers.</p> : null}
  </div>;
}
