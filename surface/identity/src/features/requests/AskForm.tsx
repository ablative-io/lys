/** A request keeps the same operation and exact body across an uncertain answer or reload. */
import { useRef, useState } from 'react';
import type { FormEvent } from 'react';
import { operationId, request } from '../../api';
import type { GrantModel, ResourceRef } from '../../generated/grants';
import { ActionPicker, singleActionCarriers } from '../grants/ActionPicker';
import { field } from '../people/RecordedForm';
import { failureWords } from '../signin/words';
import { matchesAsk } from './contract';
import type { AccessRequest, Ask } from './contract';
import { keptSend } from '../../kept';

type Pending = { kind: 'empty' } | { kind: 'damaged' } | { kind: 'held'; asked: Ask };

/** The shipped actions a person may ask for on a shipped resource; an app's actions keep their own names and are asked as relations. */
const askable = (model: GrantModel, resource: ResourceRef | undefined): string[] =>
  resource && !resource.kind.includes('.') ? Object.keys(model.action_sentences).filter((action) => singleActionCarriers(model).has(action)).sort() : [];

/** One retained ask, exactly as sent, or null when the record does not describe one. */
function parseAsk(value: unknown): Ask | null {
    if (value && typeof value === 'object' && 'operation' in value && typeof value.operation === 'string'
      && /^op-[0-9a-f]{32}$/.test(value.operation)
      && 'resource' in value && value.resource && typeof value.resource === 'object'
      && 'kind' in value.resource && typeof value.resource.kind === 'string'
      && 'id' in value.resource && typeof value.resource.id === 'string'
      && 'relation' in value && typeof value.relation === 'string'
      && 'ends_at' in value && (value.ends_at === null || (typeof value.ends_at === 'number' && Number.isSafeInteger(value.ends_at)))
      && 'why' in value && typeof value.why === 'string') {
      return { operation: value.operation, resource: { kind: value.resource.kind, id: value.resource.id }, relation: value.relation, ends_at: value.ends_at, why: value.why };
    }
  return null;
}

function readPending(key: string): Pending {
  try {
    const raw = sessionStorage.getItem(key);
    if (raw === null) return { kind: 'empty' };
    const asked = parseAsk(JSON.parse(raw));
    return asked === null ? { kind: 'damaged' } : { kind: 'held', asked };
  } catch {
    // An unreadable pending request cannot establish that a new request is safe.
    return { kind: 'damaged' };
  }
}

/** The asks still to send after the retained one, whole, so a reload finishes the run exactly as it was ticked and never twice. */
type Rest = { kind: 'empty' } | { kind: 'damaged' } | { kind: 'held'; asks: Ask[] };
const restKey = (key: string): string => key + '.rest';
function readRest(key: string): Rest {
  try {
    const raw = sessionStorage.getItem(restKey(key));
    if (raw === null) return { kind: 'empty' };
    const value: unknown = JSON.parse(raw);
    if (!Array.isArray(value)) return { kind: 'damaged' };
    const asks: Ask[] = [];
    for (const each of value) {
      const asked = parseAsk(each);
      if (asked === null) return { kind: 'damaged' };
      asks.push(asked);
    }
    return asks.length ? { kind: 'held', asks } : { kind: 'empty' };
  } catch {
    return { kind: 'damaged' };
  }
}

export function AskForm({ person, resources, model, changed }: {
  person: string; resources: ResourceRef[]; model: GrantModel; changed: (answer: AccessRequest) => void;
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
  const [picked, setPicked] = useState<string[]>([]);
  const [rest, setRest] = useState<Rest>(() => readRest(key));
  const damaged = pending.kind === 'damaged' || rest.kind === 'damaged';
  const keepRest = (left: Ask[]) => {
    // Storage first, then a copy into state: state says what storage says, never a rest the browser refused to keep.
    if (left.length) sessionStorage.setItem(restKey(key), JSON.stringify(left)); else sessionStorage.removeItem(restKey(key));
    setRest(left.length ? { kind: 'held', asks: [...left] } : { kind: 'empty' });
  };
  /** A queued ask found already recorded is released without sending. */
  const finish = (recorded: AccessRequest) => {
    try { sessionStorage.removeItem(key); setPending({ kind: 'empty' }); } catch {
      setFailure('Recorded, but the browser could not release the retained request; Check original request will confirm it again, never record it twice.');
    }
    changed(recorded);
  };
  /** The queue's place: the next ask goes under the pending key before the rest is shortened, so no ask is ever both forgotten and unsent. */
  const retain = (asked: Ask) => {
    sessionStorage.setItem(key, JSON.stringify(asked));
    setPending({ kind: 'held', asked });
  };
  /** One ask, kept before it is sent and released only on the service's confirmation; true when confirmed. */
  const sendOne = async (asked: Ask, retry: boolean): Promise<boolean> => {
    const kept = await keptSend(key, asked, async () => {
      setPending({ kind: 'held', asked });
      const recorded = await request<AccessRequest>('/requests', asked);
      if (!matchesAsk(recorded, asked, person)) throw new Error('The answer did not confirm the request. Its original details are retained.');
      return recorded;
    }, !retry);
    if (kept.at === 'confirmed') {
      if (kept.unreleased) setFailure('Recorded, but the browser could not release the retained request; Check original request will confirm it again, never record it twice.');
      else setPending({ kind: 'empty' });
      changed(kept.answer);
      return true;
    }
    // A later refusal cannot undo an earlier uncertain admission: only a first sending's no releases the request.
    const unreleased = kept.at === 'refused' ? kept.unreleased : null;
    if (kept.at === 'refused' && !unreleased) setPending({ kind: 'empty' });
    setFailure(failureWords(kept.error, 'Check the request details. If its result is unconfirmed, choose Check original request; do not make a second request.')
      + (unreleased ? ' The browser could not release the retained request; Check original request will answer this same refusal, never record a second: ' + String(unreleased) : ''));
    return false;
  };
  /** The retained ask first, exactly as sent, then the rest exactly as ticked; stops at the first unconfirmed answer and keeps the rest for the retry. */
  const send = async (asked: Ask, retry: boolean, more: Ask[]) => {
    if (working.current || damaged) return;
    working.current = true;
    setBusy(true); setFailure(''); setAnswer('');
    try {
      const queue = [...more];
      try {
        // The whole run is kept before anything else, so a failure at any later write leaves every ask somewhere it is offered again by name.
        if (!retry) { keepRest([asked, ...queue]); retain(asked); }
        keepRest(queue);
      } catch (error) {
        // Nothing has been sent. What is stored is offered again, exactly as asked, never twice.
        throw new Error('Nothing was sent because this browser could not retain the request: ' + (error instanceof Error ? error.message : String(error)));
      }
      let recorded = 0;
      if (retry) {
        const list = await request<{ requests: AccessRequest[] }>('/requests');
        const found = list.requests.find((entry) => entry.id === asked.operation);
        if (found) {
          if (!matchesAsk(found, asked, person)) throw new Error('The recorded request differs from the retained request. It has not been replaced.');
          finish(found);
        } else if (!(await sendOne(asked, true))) return;
      } else if (!(await sendOne(asked, false))) return;
      recorded += 1;
      while (queue.length) {
        const next = queue.shift() as Ask;
        try {
          retain(next);
          keepRest(queue);
        } catch (error) {
          // Either the rest still holds next (retain failed) or the key holds it beside the rest (shortening failed): both resume exactly, so stop and say so.
          setAnswer(`${recorded} recorded before the browser stopped retaining; not yet sent: ${[next, ...queue].map((each) => each.relation).join(', ')}.`);
          throw new Error('The browser could not retain the next request: ' + (error instanceof Error ? error.message : String(error)));
        }
        if (!(await sendOne(next, false))) {
          // A definite refusal ends the run; what was recorded stays recorded and the rest is named, never re-sent by itself.
          if (sessionStorage.getItem(key) === null) {
            let cleared = true;
            try { keepRest([]); } catch { cleared = false; }
            setAnswer(`${recorded} recorded before the refusal; not sent: ${[next, ...queue].map((each) => each.relation).join(', ')}.${cleared ? '' : ' The browser could not clear them; they will be offered again, never sent by themselves.'}`);
          }
          return;
        }
        recorded += 1;
      }
      setAnswer(recorded > 1 ? `${recorded} requests recorded, one per action. Access is granted only after approval.` : 'Request recorded. Access is granted only after approval.');
    } catch (error) {
      setFailure(failureWords(error, 'Check the request details. If its result is unconfirmed, choose Check original request; do not make a second request.'));
    } finally { working.current = false; setBusy(false); }
  };
  const submit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (pending.kind !== 'empty' || rest.kind === 'held' || damaged || working.current) return;
    try {
      const data = new FormData(event.currentTarget);
      const resource = advanced ? { kind: field(data, 'kind'), id: field(data, 'resource') } : resources[Number(resourceIndex)];
      if ((!advanced && resourceIndex === '') || !resource?.kind || !resource.id) throw new Error('Choose what you need access to, or enter its details under Advanced.');
      const offered = askable(model, resource);
      const chosen = offered.length ? picked.filter((action) => offered.includes(action)) : [];
      const carriers = singleActionCarriers(model);
      const relations = offered.length ? chosen.map((action) => carriers.get(action) ?? '') : [field(data, 'relation')];
      if (!relations.length || relations.some((relation) => !relation || !model.relations[relation])) throw new Error('Choose the access you need.');
      const why = field(data, 'why');
      if (!why) throw new Error('Explain why you need access.');
      const ends = noExpiry ? null : Date.parse(field(data, 'expires')) / 1000;
      if (ends !== null && (!Number.isSafeInteger(ends) || ends <= Date.now() / 1000)) throw new Error('Choose an expiry in the future, or explicitly choose no expiry.');
      const [relation, ...others] = relations;
      void send({ operation: operationId(), resource, relation, ends_at: ends, why }, false,
        others.map((each) => ({ operation: operationId(), resource, relation: each, ends_at: ends, why })));
    } catch (error) { setFailure(error instanceof Error ? error.message : String(error)); }
  };
  return <form className="card recorded-form" aria-label="Ask for access" onSubmit={submit}>
    <h2>Ask for access</h2><p>Choose what you need and explain why. Someone who can grant that access will review your request.</p>
    <fieldset disabled={pending.kind !== 'empty' || rest.kind === 'held' || damaged || busy} style={{ border: 0, padding: 0 }}>
      {!advanced ? <label className="field">What do you need access to?<select value={resourceIndex} onChange={(event) => setResourceIndex(event.target.value)} required>
        <option value="">Choose what you need</option>{resources.map((resource, index) => <option key={JSON.stringify(resource)} value={index}>{resource.id} · {resource.kind}</option>)}
      </select></label> : null}
      {!resources.length && !advanced ? <p className="note">No resources appear in your visible grants yet. Tick the box below if you have been given a resource's details.</p> : null}
      <label className="tick"><input type="checkbox" name="another" checked={advanced} onChange={(event) => setAdvanced(event.target.checked)} />Enter another resource by its kind and ID</label>
        {advanced ? <><label className="field">Resource kind<input name="kind" required /></label><label className="field">Resource ID<input name="resource" required /></label></> : null}
      {(() => {
        const resource = advanced ? undefined : resources[Number(resourceIndex)];
        const offered = askable(model, resource);
        return offered.length
          ? <><ActionPicker model={model} resource={resource as ResourceRef} actions={offered} value={picked} onChange={setPicked} agents={false} title="Access needed" />
            <p className="note">Each ticked action is asked for on its own and approved on its own.</p></>
          : <label className="field">Access needed<select name="relation" required defaultValue=""><option value="">Choose access</option>{Object.entries(model.relations).map(([relation, actions]) => <option key={relation} value={relation}>{relation} · {actions.join(', ')}</option>)}</select></label>;
      })()}
      <label className="field">Why do you need this access?<textarea name="why" required /></label>
      <label className="field">Access until (your local time)<input name="expires" type="datetime-local" required={!noExpiry} disabled={noExpiry} /></label>
      <label><input type="checkbox" name="no-expiry" checked={noExpiry} onChange={(event) => setNoExpiry(event.target.checked)} /> No expiry requested</label>
      <p><button className="btn primary" type="submit" disabled={busy}>Request access</button></p>
    </fieldset>
    {pending.kind === 'held' ? <div role="status"><p>The result is not yet confirmed. Your original request is retained; checking will not create a duplicate.</p><button type="button" className="btn" disabled={busy} onClick={() => void send(pending.asked, true, rest.kind === 'held' ? rest.asks.filter((each) => each.operation !== pending.asked.operation) : [])}>{busy ? 'Checking…' : rest.kind === 'held' ? `Check original request, then ask for the ${rest.asks.length} remaining` : 'Check original request'}</button></div> : null}
    {pending.kind === 'empty' && rest.kind === 'held' ? <div role="status"><p>{rest.asks.length} of an earlier run not yet sent: {rest.asks.map((each) => each.relation).join(', ')}.</p><button type="button" className="btn" data-act="ask-rest" disabled={busy} onClick={() => { const [head, ...tail] = rest.asks; if (head) void send(head, false, tail); }}>{busy ? 'Sending…' : `Ask for the ${rest.asks.length} remaining`}</button></div> : null}
    {damaged ? <p role="alert">The retained request could not be read. Sending is blocked to prevent a duplicate. Ask an administrator to inspect the pending request for this account.</p> : null}
    {failure ? <p className="why-not" role="alert">{failure}</p> : null}
    {answer ? <p role="status">{answer}</p> : null}
  </form>;
}
