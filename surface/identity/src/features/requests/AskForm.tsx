/** A request keeps the same operation and exact body across an uncertain answer or reload. */
import { useRef, useState } from 'react';
import type { FormEvent } from 'react';
import { operationId, Refused, request } from '../../api';
import type { GrantModel, ResourceRef } from '../../generated/grants';
import { field } from '../people/RecordedForm';
import { matchesAsk } from './contract';
import type { AccessRequest, Ask } from './contract';

type Pending = { kind: 'empty' } | { kind: 'damaged' } | { kind: 'held'; asked: Ask };

function readPending(key: string): Pending {
  try {
    const raw = sessionStorage.getItem(key);
    if (raw === null) return { kind: 'empty' };
    const value: unknown = JSON.parse(raw);
    if (value && typeof value === 'object' && 'operation' in value && typeof value.operation === 'string'
      && /^op-[0-9a-f]{32}$/.test(value.operation)
      && 'resource' in value && value.resource && typeof value.resource === 'object'
      && 'kind' in value.resource && typeof value.resource.kind === 'string'
      && 'id' in value.resource && typeof value.resource.id === 'string'
      && 'relation' in value && typeof value.relation === 'string'
      && 'ends_at' in value && (value.ends_at === null || (typeof value.ends_at === 'number' && Number.isSafeInteger(value.ends_at)))
      && 'why' in value && typeof value.why === 'string') {
      return { kind: 'held', asked: { operation: value.operation, resource: { kind: value.resource.kind, id: value.resource.id }, relation: value.relation, ends_at: value.ends_at, why: value.why } };
    }
  } catch {
    // An unreadable pending request cannot establish that a new request is safe.
  }
  return { kind: 'damaged' };
}

export function AskForm({ person, resources, model, changed }: {
  person: string; resources: ResourceRef[]; model: GrantModel; changed: () => void;
}) {
  const key = 'lys.pending.request.' + person;
  const [pending, setPending] = useState<Pending>(() => readPending(key));
  const [busy, setBusy] = useState(false);
  const working = useRef(false);
  const [answer, setAnswer] = useState('');
  const [failure, setFailure] = useState('');
  const [advanced, setAdvanced] = useState(false);
  const [noExpiry, setNoExpiry] = useState(false);
  const [resourceIndex, setResourceIndex] = useState('');
  const finish = () => {
    sessionStorage.removeItem(key);
    setPending({ kind: 'empty' });
    setAnswer('Request recorded. Access is granted only after approval.');
    changed();
  };
  const send = async (asked: Ask, retry: boolean) => {
    if (working.current) return;
    working.current = true;
    setBusy(true); setFailure(''); setAnswer('');
    try {
      sessionStorage.setItem(key, JSON.stringify(asked));
      setPending({ kind: 'held', asked });
      if (retry) {
        const list = await request<{ requests: AccessRequest[] }>('/requests');
        const found = list.requests.find((entry) => entry.id === asked.operation);
        if (found) {
          if (!matchesAsk(found, asked, person)) throw new Error('The recorded request differs from the retained request. It has not been replaced.');
          finish(); return;
        }
      }
      const recorded = await request<AccessRequest>('/requests', asked);
      if (!matchesAsk(recorded, asked, person)) throw new Error('The answer did not confirm the request. Its original details are retained.');
      finish();
    } catch (error) {
      // A later refusal cannot undo an earlier uncertain admission.
      if (!retry && error instanceof Refused && error.status >= 400 && error.status < 500) {
        sessionStorage.removeItem(key); setPending({ kind: 'empty' });
      }
      setFailure(error instanceof Refused ? `${error.refusal.refusal}: ${error.refusal.reason}` : error instanceof Error ? error.message : String(error));
    } finally { working.current = false; setBusy(false); }
  };
  const submit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (pending.kind !== 'empty' || working.current) return;
    try {
      const data = new FormData(event.currentTarget);
      const resource = advanced ? { kind: field(data, 'kind'), id: field(data, 'resource') } : resources[Number(resourceIndex)];
      if ((!advanced && resourceIndex === '') || !resource?.kind || !resource.id) throw new Error('Choose a resource or enter its details under Advanced.');
      const relation = field(data, 'relation');
      if (!model.relations[relation]) throw new Error('Choose the access you need.');
      const why = field(data, 'why');
      if (!why || [...why].length > 500) throw new Error('Explain why you need access in 1 to 500 characters.');
      const ends = noExpiry ? null : Date.parse(field(data, 'expires')) / 1000;
      if (ends !== null && (!Number.isSafeInteger(ends) || ends <= Date.now() / 1000)) throw new Error('Choose an expiry in the future, or explicitly choose no expiry.');
      void send({ operation: operationId(), resource, relation, ends_at: ends, why }, false);
    } catch (error) { setFailure(error instanceof Error ? error.message : String(error)); }
  };
  return <form className="card recorded-form" aria-label="Ask for access" onSubmit={submit}>
    <h2>Ask for access</h2><p>Choose what you need and explain why. Someone who can grant that access will review your request.</p>
    <fieldset disabled={pending.kind !== 'empty' || busy} style={{ border: 0, padding: 0 }}>
      {!advanced ? <label className="field">Resource<select value={resourceIndex} onChange={(event) => setResourceIndex(event.target.value)} required>
        <option value="">Choose a resource</option>{resources.map((resource, index) => <option key={JSON.stringify(resource)} value={index}>{resource.id} · {resource.kind}</option>)}
      </select></label> : null}
      {!resources.length && !advanced ? <p className="note">No resources appear in your visible grants yet. Use Advanced if you have been given a resource's details.</p> : null}
      <details onToggle={(event) => setAdvanced(event.currentTarget.open)}><summary>Advanced: enter another resource</summary>
        {advanced ? <><label className="field">Resource kind<input name="kind" required /></label><label className="field">Resource ID<input name="resource" required /></label></> : null}
      </details>
      <label className="field">Access needed<select name="relation" required defaultValue=""><option value="">Choose access</option>{Object.entries(model.relations).map(([relation, actions]) => <option key={relation} value={relation}>{relation} · {actions.join(', ')}</option>)}</select></label>
      <label className="field">Why do you need this access?<textarea name="why" required maxLength={500} /></label>
      <label className="field">Access until (your local time)<input name="expires" type="datetime-local" required={!noExpiry} disabled={noExpiry} /></label>
      <label><input type="checkbox" checked={noExpiry} onChange={(event) => setNoExpiry(event.target.checked)} /> No expiry requested</label>
      <p><button className="btn primary" type="submit" disabled={busy}>Request access</button></p>
    </fieldset>
    {pending.kind === 'held' ? <div role="status"><p>The result is not yet confirmed. Your original request is retained; checking will not create a duplicate.</p><button type="button" className="btn" disabled={busy} onClick={() => void send(pending.asked, true)}>{busy ? 'Checking…' : 'Check original request'}</button></div> : null}
    {pending.kind === 'damaged' ? <p role="alert">The retained request could not be read. Sending is blocked to prevent a duplicate. Ask an administrator to inspect the pending request for this account.</p> : null}
    {failure ? <p className="why-not" role="alert">{failure}</p> : null}
    {answer ? <p role="status">{answer}</p> : null}
  </form>;
}
