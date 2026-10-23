/** Decisions retain their exact operation through unknown outcomes; server choices govern approval. */
import { useRef, useState } from 'react';
import type { FormEvent } from 'react';
import { operationId, request } from '../../api';
import { answeredNo, sendKept } from '../../kept';
import { field } from '../people/RecordedForm';
import { failureWords } from '../signin/words';
import type { AccessRequest } from './contract';
import { Act } from '../../shell/Act';
import { StuckRecord } from '../grants/StuckRecord';
import { APPROVE, DECLINE } from './decision-words';

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

export function DecisionForm({ entry, person, canIssueRoot, declineOnly = false, changed, describe }: {
  entry: AccessRequest; person: string; canIssueRoot: boolean; changed: (answer: AccessRequest) => void;
  /** The model no longer defines what is asked (ACCESS-011 R3): only Decline is offered. */
  declineOnly?: boolean;
  /** A source grant in words, by what it is on; never its identifier. */
  describe: (source: string, index: number) => string;
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
      if (!note) throw new Error('Give a reason for your decision.');
      if (action === 'decline') { void send({ kind: action, note }, false); return; }
      const source = field(data, 'source');
      if (!(canIssueRoot && source === 'root') && !entry.sources.includes(source)) throw new Error('Choose an available source of access.');
      void send({ kind: action, operation: operationId(), route: 'browser', source: source === 'root' ? null : source, note }, false);
    } catch (error) { setFailure(error instanceof Error ? error.message : String(error)); }
  };
  // A decision moves its request out of waiting, so a request still waiting has no recorded decision, whatever the copy said.
  // Who may decide is worked out from the grants that stand now, so a request missing from the list says nothing either way.
  if (pending.kind === 'damaged') return <StuckRecord keys={[key]} what="your decision" byOperation={false}
    recorded={async () => {
      const found = (await request<{ requests: AccessRequest[] }>('/requests')).requests.find((each) => each.id === entry.id);
      return found ? found.state !== 'waiting' : null;
    }}
    looked="the requests you may decide" again="decide it" look={`Ask ${entry.asked_by_name ?? 'whoever asked'} or an administrator whether this request was decided.`}
    settled={() => setPending(load(key))} />;
  return <div>
    {pending.kind === 'held' ? <div role="status"><p>This decision has no confirmed answer. Checking sends the same decision again and cannot create a second one.</p>
      <Act symbol="again" name={busy ? 'Checking…' : 'Check original decision'} word={busy ? 'Checking…' : 'Check'} disabled={busy} onClick={() => void send(pending.decision, true)} /></div>
      : done ? <p role="status">{done}</p> : action ? <form onSubmit={submit} aria-label="Decide request">
        <h4>{action === 'approve' ? 'Approve this access?' : 'Decline this request?'}</h4>
        {action === 'approve' ? <label className="field">Grant access from<select name="source" required defaultValue="">
          <option value="">Choose how you can give this access</option>{canIssueRoot ? <option value="root">Give access directly as the administrator</option> : null}
          {entry.sources.map((source, index) => <option key={source} value={source}>{describe(source, index)}</option>)}
        </select><span className="hint">The access they get traces back to what you choose here, and ends when it does.</span></label> : null}
        <label className="field">Reason for your decision<textarea name="note" required /></label>
        <Act symbol="approve" name={'Confirm ' + (action === 'approve' ? 'approval' : 'decline')} word="Confirm" tone="primary" type="submit" disabled={busy} />{' '}
        <Act symbol="close" name="Cancel" word="Cancel" disabled={busy} onClick={() => setAction(null)} />
      </form> : <>{declineOnly ? null : <><Act symbol="approve" name="Approve access" word={APPROVE} tone="primary" disabled={!canIssueRoot && !entry.sources.length} onClick={() => setAction('approve')} />{' '}</>}
        <Act symbol="decline" name="Decline request" word={DECLINE} onClick={() => setAction('decline')} />
        {!declineOnly && !canIssueRoot && !entry.sources.length ? <p className="why-not">You cannot approve this: you hold no access you may pass on that covers it. You can still decline it.</p> : null}</>}
    {failure ? <p className="why-not" role="alert">{failure}</p> : null}
  </div>;
}
