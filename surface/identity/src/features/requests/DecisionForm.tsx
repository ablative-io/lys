/** Decisions retain their exact operation through unknown outcomes; server choices govern approval. */
import { useRef, useState } from 'react';
import type { FormEvent } from 'react';
import { operationId, request } from '../../api';
import { answeredNo, sendKept } from '../../kept';
import { field } from '../people/RecordedForm';
import { failureWords } from '../signin/words';
import type { AccessRequest } from './contract';

type Decision = { kind: 'approve'; operation: string; route: 'browser'; source: string | null; note: string }
  | { kind: 'decline'; note: string };
type Pending = { kind: 'empty' } | { kind: 'damaged' } | { kind: 'held'; decision: Decision };

function load(key: string): Pending {
  try {
    const raw = sessionStorage.getItem(key);
    if (raw === null) return { kind: 'empty' };
    const value: unknown = JSON.parse(raw);
    if (value && typeof value === 'object' && 'note' in value && typeof value.note === 'string' && 'kind' in value) {
      if (value.kind === 'decline') return { kind: 'held', decision: { kind: 'decline', note: value.note } };
      if (value.kind === 'approve' && 'operation' in value && typeof value.operation === 'string' && /^op-[0-9a-f]{32}$/.test(value.operation)
        && 'route' in value && value.route === 'browser' && 'source' in value && (value.source === null || typeof value.source === 'string')) {
        return { kind: 'held', decision: { kind: 'approve', operation: value.operation, route: 'browser', source: value.source, note: value.note } };
      }
    }
  } catch { /* A damaged decision stays held rather than acquiring a new operation. */ }
  return { kind: 'damaged' };
}

export function DecisionForm({ entry, person, canIssueRoot, changed }: {
  entry: AccessRequest; person: string; canIssueRoot: boolean; changed: (answer: AccessRequest) => void;
}) {
  const key = 'lys.pending.decision.' + person + '.' + entry.id;
  const [pending, setPending] = useState(() => load(key));
  const [action, setAction] = useState<'approve' | 'decline' | null>(null);
  const [busy, setBusy] = useState(false);
  const [failure, setFailure] = useState('');
  const [done, setDone] = useState('');
  const working = useRef(false);
  const send = async (decision: Decision, retry: boolean) => {
    if (working.current) return;
    working.current = true; setBusy(true); setFailure('');
    try {
      const { kind, ...body } = decision;
      const answer = await sendKept(key, decision, async () => {
        setPending({ kind: 'held', decision });
        const given = await request<AccessRequest>('/requests/' + encodeURIComponent(entry.id) + '/' + kind, body);
        if (given.id !== entry.id || given.state !== (kind === 'approve' ? 'approved' : 'declined')
          || given.decision?.by !== person || given.decision.note !== decision.note
          || (kind === 'approve' ? typeof given.decision.grant !== 'string' : given.decision.grant !== null)) {
          throw new Error('The answer did not confirm this decision. Its original details remain held.');
        }
        return given;
      }, !retry);
      setPending({ kind: 'empty' }); setAction(null);
      setDone(kind === 'approve' ? 'Access approved.' : 'Request declined.'); changed(answer);
    } catch (error) {
      if (!retry && answeredNo(error)) setPending({ kind: 'empty' });
      setFailure(failureWords(error, 'Check the decision details. If the result is unconfirmed, choose Check original decision.'));
    } finally { working.current = false; setBusy(false); }
  };
  const submit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (action === null || pending.kind !== 'empty' || working.current || done) return;
    try {
      const data = new FormData(event.currentTarget);
      const note = field(data, 'note');
      if (!note || [...note].length > 500) throw new Error('Give a reason in 1 to 500 characters.');
      if (action === 'decline') { void send({ kind: action, note }, false); return; }
      const source = field(data, 'source');
      if (!(canIssueRoot && source === 'root') && !entry.sources.includes(source)) throw new Error('Choose an available source of access.');
      void send({ kind: action, operation: operationId(), route: 'browser', source: source === 'root' ? null : source, note }, false);
    } catch (error) { setFailure(error instanceof Error ? error.message : String(error)); }
  };
  if (pending.kind === 'damaged') return <p role="alert">A retained decision could not be read. New decisions are blocked until its outcome can be established.</p>;
  return <div>
    {pending.kind === 'held' ? <div role="status"><p>This decision has no confirmed answer. Checking sends the same decision again and cannot create a second one.</p>
      <button type="button" className="btn" disabled={busy} onClick={() => void send(pending.decision, true)}>{busy ? 'Checking…' : 'Check original decision'}</button></div>
      : done ? <p role="status">{done}</p> : action ? <form onSubmit={submit} aria-label="Decide request">
        <h4>{action === 'approve' ? 'Approve this access?' : 'Decline this request?'}</h4>
        {action === 'approve' ? <label className="field">Grant access from<select name="source" required defaultValue="">
          <option value="">Choose how you can give this access</option>{canIssueRoot ? <option value="root">Give access directly as the administrator</option> : null}
          {entry.sources.map((source, index) => <option key={source} value={source}>Your permission {index + 1}</option>)}
        </select></label> : null}
        {action === 'approve' && entry.sources.length ? <><p className="note">Permissions this approval draws on</p><ul>{entry.sources.map((source, index) => <li key={source}>Permission {index + 1}: {source}</li>)}</ul></> : null}
        <label className="field">Reason for your decision<textarea name="note" required maxLength={500} /></label>
        <button type="submit" className="btn primary" disabled={busy}>Confirm {action === 'approve' ? 'approval' : 'decline'}</button>{' '}
        <button type="button" className="btn" disabled={busy} onClick={() => setAction(null)}>Cancel</button>
      </form> : <><button type="button" className="btn primary" disabled={!canIssueRoot && !entry.sources.length} onClick={() => setAction('approve')}>Approve access</button>{' '}
        <button type="button" className="btn" onClick={() => setAction('decline')}>Decline request</button></>}
    {failure ? <p className="why-not" role="alert">{failure}</p> : null}
  </div>;
}
