import { readTogether } from '../../reads';
/** Plain controls for an agent's budgets and goals on its own file, and a team's on the Teams page; graphs and analytics stay with the monitoring app. */
import { useRef, useState } from 'react';
import { controls, operationId, Refused, request, useLive, useLoad } from '../../api';
import type { ControlReceiptsPage, ControlSessionsPage, CurrentControl, PublicControlReceipt, ResendLookup, ResendOccurrence } from '../../api';
import { refreshLive } from '../../live';
import { Gate } from '../signin/Gate';
import { UsageBudgets } from './UsageBudgets';
import { UsageGoals } from './UsageGoals';
import { tracking } from './contract';
import type { ProvisioningAnswer } from '../provisioning/Provisioning';
import type { BudgetsView, GoalsView, UsageView } from './contract';
import './usage.css';

/** `name` is the agent's name, said in place of its identifier wherever the service's words carry it. */
export function AgentUsage({ agent, name }: { agent: string; name?: string }) {
  const [revision, setRevision] = useState(0);
  const [notice, setNotice] = useState('');
  const path = encodeURIComponent(agent);
  const load = useLoad(() => readTogether({
    budgets: request<BudgetsView>('/budgets/agent/' + path),
    usage: request<UsageView>('/agents/' + path + '/usage'),
    goals: request<GoalsView>('/agents/' + path + '/goals'),
  }), 'usage:' + agent + ':' + revision);
  const changed = (words: string) => { setNotice(words); setRevision((value) => value + 1); };
  return <>{notice ? <p role="status" className="usage-notice">{notice}</p> : null}<Gate load={load} title="Usage" ok={({ budgets, usage, goals }) => {
    const tracked = tracking(usage);
    return <>
      <ControlSetup agent={agent} />
      <ManagedControls agent={agent} />
      <p className="usage-tracking" data-complete={tracked.complete} role="status">{tracked.words}</p>
      <UsageBudgets budgets={budgets} receipts={usage.receipts} changed={changed} name={name} />
      <UsageGoals agent={agent} goals={goals.goals} changed={changed} />
    </>;
  }} /></>;
}

/** A team's limits and goals: the same two tables as an agent's. A team's use is read with its limits. */
export function TeamUsage({ team, name }: { team: string; name?: string }) {
  const [revision, setRevision] = useState(0);
  const [notice, setNotice] = useState('');
  const path = encodeURIComponent(team);
  const load = useLoad(() => readTogether({
    budgets: request<BudgetsView>('/budgets/team/' + path),
    goals: request<GoalsView>('/teams/' + path + '/goals'),
  }), 'team-usage:' + team + ':' + revision);
  const changed = (words: string) => { setNotice(words); setRevision((value) => value + 1); };
  return <div className="usage">{notice ? <p role="status" className="usage-notice">{notice}</p> : null}<Gate load={load} title="Team limits and goals" ok={({ budgets, goals }) => <>
    <UsageBudgets budgets={budgets} receipts={[]} changed={changed} name={name} />
    <UsageGoals agent={team} kind="team" goals={goals.goals} changed={changed} />
  </>} /></div>;
}

/** The saved setup chooses the next start; a running session is never moved by a read. */
function ControlSetup({ agent }: { agent: string }) {
  const load = useLoad(async () => {
    const answer = await request<ProvisioningAnswer>('/agents/' + encodeURIComponent(agent) + '/provisioning');
    const required = answer.profile?.session?.requires_controls;
    if (answer.agent !== agent || (required !== undefined && typeof required !== 'boolean')) throw new Error('ControlSetupUnreadable: the saved setup did not name this agent and its control requirement.');
    return required === true;
  }, 'control-setup:' + agent);
  return <Gate load={load} title="Control setup" renderError={(failure) => <p role="status">Control setup unavailable: {failure.refusal.refusal}: {failure.message}</p>} ok={(required) => <p role="status">{required ? 'Managed controls are required by the saved setup. No managed adapter is qualified; starts are refused (control_adapter_unqualified).' : 'Notices use the terminal. No managed adapter is qualified.'}</p>} />;
}

function controlFailure(failure: Refused) { return <p role="status">{failure.refusal.refusal}: {failure.message}</p>; }

/** Session and operation pages follow the existing change signal and advance only on a person's action. */
export function ManagedControls({ agent }: { agent: string }) {
  const [after, setAfter] = useState<string | null>(null);
  const load = useLive(() => controls.sessions(agent, after), 'control-sessions:' + agent + ':' + after);
  return <section aria-label="Managed controls" className="managed-controls">
    <h3>Managed controls</h3>
    <Gate load={load} title="Control sessions" renderError={controlFailure} ok={(page) => <>
      <ControlSessionPage key={after ?? 'first'} page={page} />
      {after ? <button type="button" onClick={() => setAfter(null)}>First sessions</button> : null}
      {page.after ? <button type="button" onClick={() => setAfter(page.after)}>More sessions</button> : null}
    </>} />
  </section>;
}

function ControlSessionPage({ page }: { page: ControlSessionsPage }) {
  const [selected, setSelected] = useState(page.sessions[0] ?? '');
  const session = page.sessions.includes(selected) ? selected : page.sessions[0];
  if (!session) return <p>No session has been reported for this agent.</p>;
  return <>
    <label>Control session <select aria-label="Control session" value={session} onChange={(event) => setSelected(event.target.value)}>{page.sessions.map((id) => <option key={id} value={id}>{id}</option>)}</select></label>
    <SessionControls key={session} session={session} />
  </>;
}

/** A direct current-status query carries no terminal scrollback or account-move history. */
export function SessionControls({ session }: { session: string }) {
  const [after, setAfter] = useState<string | null>(null);
  const load = useLive(() => readTogether({ control: controls.status(session), page: controls.receipts(session, after) }), 'session-controls:' + session + ':' + after);
  return <Gate load={load} title="Current controls" renderError={controlFailure} ok={({ control, page }) => <>
    <CurrentControlState control={control} />
    <ControlOperations page={page} />
    {after ? <button type="button" onClick={() => setAfter(null)}>First operations</button> : null}
    {page.after ? <button type="button" onClick={() => setAfter(page.after)}>More operations</button> : null}
  </>} />;
}

function CurrentControlState({ control }: { control: CurrentControl | null }) {
  if (control === null) return <p role="status">Managed control channel unavailable. No boundary is assumed.</p>;
  const phase = { unknown: 'Boundary unknown; no idle boundary is proved', idle: 'Idle boundary', reserved: 'Next turn reserved', active: 'Active turn', closed: 'Managed channel closed' }[control.phase];
  const context = control.context;
  const words = context === null ? 'Context decision not yet received' : context.state === 'released' ? 'Normal input permitted' : context.state === 'compact' ? 'Compaction authorised; completion not yet proved' : context.state === 'held' ? 'Held: ' + context.reason.replaceAll('_', ' ') : 'Authority unavailable: ' + context.reason.replaceAll('_', ' ');
  return <div className="current-control" role="status">
    <p>{phase}{control.active ? ' · ' + control.active : ''}</p>
    <p>{words}</p>
    <p>Queued reminders: {control.queued.length}</p>
    {control.queued.length ? <ul>{control.queued.map((queued) => <li key={queued.operation}>{queued.operation} · aim {queued.reference.goal} · occurrence {queued.reference.occurrence}{queued.reference.prior ? ' · possible prior delivery ' + queued.reference.prior : ''}</li>)}</ul> : null}
  </div>;
}

function controlOutcome(receipt: PublicControlReceipt): string {
  if (receipt.state === 'uncertain') return 'Unconfirmed';
  if (receipt.state === 'accepted') return 'Accepted; waiting for its boundary';
  if (receipt.state === 'delivering' || receipt.state === 'delivered') return 'Accepted; completion unconfirmed';
  if (receipt.state === 'refused') return 'Refused';
  if (receipt.request === 'compact' || receipt.request === 'context_compact') return 'Compaction confirmed';
  if (receipt.request === 'stop') return 'Process exit confirmed';
  return 'Harness admission confirmed; goal completion unconfirmed';
}

function ControlOperations({ page }: { page: ControlReceiptsPage }) {
  return <div className="control-operations">
    <h4>Control operations</h4>
    <p>A recorded admission confirms the harness received the request. It does not prove a goal was completed.</p>
    {page.receipts.length ? <ul>{page.receipts.map((receipt) => <li key={receipt.operation}><ControlOperation receipt={receipt} /></li>)}</ul> : <p>No control operation is recorded on this page.</p>}
  </div>;
}

function ControlOperation({ receipt }: { receipt: PublicControlReceipt }) {
  const [recorded, setRecorded] = useState<PublicControlReceipt | null>(null);
  const [failure, setFailure] = useState('');
  const [busy, setBusy] = useState(false);
  const attempt = useRef<{ operation: string; choice: 'seen' | 'not_seen' } | null>(null);
  const sending = useRef(false);
  const current = receipt.reconciled ? receipt : recorded ?? receipt;
  const decide = async (choice: 'seen' | 'not_seen') => {
    if (sending.current || (attempt.current && attempt.current.choice !== choice)) return;
    attempt.current ??= { operation: operationId(), choice };
    sending.current = true; setBusy(true); setFailure('');
    try {
      setRecorded(await controls.decide(current, attempt.current.operation, choice));
      refreshLive();
    } catch (error) {
      setFailure(error instanceof Refused ? error.refusal.refusal + ': ' + error.message : String(error));
    } finally { sending.current = false; setBusy(false); }
  };
  const personal = current.reconciled?.decision;
  return <article>
    <p><b>{controlOutcome(current)}</b> · {current.operation} · {current.request.replaceAll('_', ' ')}</p>
    {current.state === 'uncertain' || current.reference?.prior ? <p>Possible prior delivery{current.reference?.prior ? ': ' + current.reference.prior : ''}. It will not be resent automatically.</p> : null}
    <p>{current.admitted ? 'Matching harness admission recorded.' : 'Matching harness admission not recorded.'}</p>
    {personal ? <p>Person recorded {personal.choice.replaceAll('_', ' ')}{personal.choice === 'resent' ? ' under occurrence ' + personal.occurrence : ''}. The harness outcome remains {controlOutcome(current)}.</p> : null}
    {failure ? <p role="status">{failure} Keep this decision when retrying; its response is unconfirmed.</p> : null}
    {current.state === 'uncertain' && !personal ? <div className="control-actions">
      <button type="button" disabled={busy || (attempt.current !== null && attempt.current.choice !== 'seen')} onClick={() => void decide('seen')}>Record seen</button>
      <button type="button" disabled={busy || (attempt.current !== null && attempt.current.choice !== 'not_seen')} onClick={() => void decide('not_seen')}>Record not seen</button>
      {current.reference ? <ResendOptions receipt={current} disabled={busy || attempt.current !== null} recorded={setRecorded} /> : null}
    </div> : null}
  </article>;
}


function ResendOptions({ receipt, disabled, recorded }: { receipt: PublicControlReceipt; disabled: boolean; recorded: (receipt: PublicControlReceipt) => void }) {
  const [lookup, setLookup] = useState<ResendLookup | null>(null);
  const [failure, setFailure] = useState('');
  const [busy, setBusy] = useState(false);
  const [session, setSession] = useState(receipt.session);
  const [attempted, setAttempted] = useState(false);
  const request = useRef<ResendOccurrence | null>(null);
  const sending = useRef(false);
  const open = async () => {
    if (sending.current || disabled) return;
    sending.current = true; setBusy(true); setFailure('');
    try { setLookup(await controls.resendLookup(receipt)); }
    catch (error) { setFailure(error instanceof Refused ? error.refusal.refusal + ': ' + error.message : String(error)); }
    finally { sending.current = false; setBusy(false); }
  };
  const send = async () => {
    if (sending.current || disabled || !lookup || !session.trim()) return;
    request.current ??= lookup.occurrence ?? { operation: operationId(), session: session.trim() };
    sending.current = true; setBusy(true); setAttempted(true); setFailure('');
    try { recorded(await controls.resend(receipt, request.current)); refreshLive(); }
    catch (error) { setFailure(error instanceof Refused ? error.refusal.refusal + ': ' + error.message : String(error)); }
    finally { sending.current = false; setBusy(false); }
  };
  return <div className="control-resend">
    {!lookup ? <button type="button" disabled={disabled || busy} onClick={() => void open()}>Resend options</button> : lookup.occurrence ? <>
      <p>A resend is already recorded for session {lookup.occurrence.session}. Finish its person decision under occurrence {lookup.occurrence.operation}; this does not ask a second delivery.</p>
      <button type="button" disabled={disabled || busy} onClick={() => void send()}>Finish the recorded resend</button>
    </> : <>
      <p>Sending again creates a distinct occurrence labelled possible prior delivery. The current authority and turn boundary are checked before delivery.</p>
      <label>Intended session <input aria-label="Intended resend session" value={session} disabled={busy || attempted} onChange={(event) => setSession(event.target.value)} /></label>
      <button type="button" disabled={disabled || busy || !session.trim()} onClick={() => void send()}>{attempted ? 'Retry the same resend request' : 'Send a new occurrence'}</button>
    </>}
    {failure ? <p role="status">{failure}{attempted ? ' Its answer is unconfirmed. Keep the same occurrence when retrying.' : ''}</p> : null}
  </div>;
}
