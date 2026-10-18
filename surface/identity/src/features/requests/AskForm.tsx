/** A request keeps the same operation and exact body across an uncertain answer or reload. */
import { useRef, useState } from 'react';
import type { FormEvent, ReactNode } from 'react';
import { operationId, request } from '../../api';
import type { GrantModel, ResourceRef } from '../../generated/grants';
import { ActionPicker, singleActionCarriers } from '../grants/ActionPicker';
import { actionWords } from '../grants/action-words';
import { kindLabel } from '../grants/IssueRoot';
import { appKind } from '../grants/model';
import { LevelPicker } from '../grants/LevelPicker';
import { StuckRecord } from '../grants/StuckRecord';
import { field } from '../people/RecordedForm';
import { failureWords } from '../signin/words';
import { matchesAsk } from './contract';
import type { AccessRequest, Ask } from './contract';
import { keptSend, keepRecord, releaseRecord } from '../../kept';
import { Act } from '../../shell/Act';
import './requests.css';

type Pending = { kind: 'empty' } | { kind: 'damaged' } | { kind: 'held'; asked: Ask };

/** What a refusal's link names as needed: the thing, by its kind and its id in its app, and the one action that was not allowed. */
export interface Wanted { kind: string; id: string; action: string }

/** The relations a kind may be asked for under: an app's kind under its own, Lys's kinds under the model's. */
const relationsOf = (model: GrantModel, kind: string): Record<string, string[]> => kind.includes('.') ? appKind(model, kind).relations : model.relations;

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

/** What a refusal's link asks for, as one sentence: the model's words for the action with the thing named in place of "this resource". */
const wantedWords = (model: GrantModel, wanted: Wanted): string => {
  const words = actionWords(model, wanted, [wanted.action]);
  const app = wanted.kind.includes('.') ? wanted.kind.slice(0, wanted.kind.indexOf('.')) : '';
  const thing = kindLabel(wanted.kind) + ' ' + wanted.id + (app ? ' in ' + app.charAt(0).toUpperCase() + app.slice(1) : '');
  const said = words.charAt(0).toLowerCase() + words.slice(1);
  return said.includes('this resource') ? said.replace('this resource', thing) : said + ' on ' + thing;
};

/** The fields naming what is asked for: open as the form itself, or folded under one line when a refusal's link already filled them. */
function What({ folded, children }: { folded: boolean; children: ReactNode }) {
  return folded ? <details className="ask-what"><summary>Change what I am asking for</summary>{children}</details> : <>{children}</>;
}

export function AskForm({ person, resources, unlisted = '', wanted, model, changed, cancel }: {
  person: string; resources: ResourceRef[]; model: GrantModel; changed: (answer: AccessRequest) => void;
  /** Why the asker's own access is missing, as its cause, so an empty list is not read as holding nothing; empty when it was read. */
  unlisted?: string;
  /** What a refusal named as needed, from its link: the form opens asking for exactly that. */
  wanted?: Wanted;
  /** Closes the form; left out where the form is the only thing the page has to offer. */
  cancel?: () => void;
}) {
  const key = 'lys.pending.request.' + person;
  const [pending, setPending] = useState<Pending>(() => readPending(key));
  const [busy, setBusy] = useState(false);
  const working = useRef(false);
  const [answer, setAnswer] = useState('');
  const [failure, setFailure] = useState('');
  const wantedIndex = wanted ? resources.findIndex((resource) => resource.kind === wanted.kind && resource.id === wanted.id) : -1;
  // With nothing to choose from, or a wanted thing not in the list, the fields for naming the thing are the form, not a box to tick first.
  const [advanced, setAdvanced] = useState(resources.length === 0 || (wanted !== undefined && wantedIndex < 0));
  const [typedKind, setTypedKind] = useState(wanted && wantedIndex < 0 ? wanted.kind : '');
  const [noExpiry, setNoExpiry] = useState(false);
  const [resourceIndex, setResourceIndex] = useState(wantedIndex < 0 ? '' : String(wantedIndex));
  const [picked, setPicked] = useState<string[]>(() => wanted && askable(model, wanted).includes(wanted.action) ? [wanted.action] : []);
  // A wanted ask names an exact action; its relation is the one carrying that action alone, when the kind has one.
  const wantedRelation = wanted ? singleActionCarriers(model, wanted.kind).get(wanted.action) ?? '' : '';
  // A level (or an app's relation) is one ask; ticked actions are one ask each. Choosing either clears the other.
  const [level, setLevel] = useState(() => picked.length ? '' : wantedRelation);
  // Folded only when every field the link filled is complete: a required field left empty under a closed fold would stop the form without a word.
  const [folded] = useState(() => wanted !== undefined && (picked.length > 0 || level !== ''));
  // Nothing is chosen until the person chooses it: an empty choice is no resource, never the first in the list.
  const chosenResource: ResourceRef | undefined = resourceIndex === '' ? undefined : resources[Number(resourceIndex)];
  const [rest, setRest] = useState<Rest>(() => readRest(key));
  const damaged = pending.kind === 'damaged' || rest.kind === 'damaged';
  const keepRest = (left: Ask[]) => {
    // Storage first, then a copy into state: state says what storage says, never a rest the browser refused to keep.
    if (left.length) keepRecord(restKey(key), left); else releaseRecord(restKey(key));
    setRest(left.length ? { kind: 'held', asks: [...left] } : { kind: 'empty' });
  };
  /** A queued ask found already recorded is released without sending. */
  const finish = (recorded: AccessRequest) => {
    try { releaseRecord(key); setPending({ kind: 'empty' }); } catch {
      setFailure('Recorded, but the browser could not release the retained request; Check original request will confirm it again, never record it twice.');
    }
    changed(recorded);
  };
  /** The queue's place: the next ask goes under the pending key before the rest is shortened, so no ask is ever both forgotten and unsent. */
  const retain = (asked: Ask) => {
    keepRecord(key, asked);
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
      const resource = advanced ? { kind: field(data, 'kind'), id: field(data, 'resource') } : chosenResource;
      if (!resource?.kind || !resource.id) throw new Error(advanced ? 'Enter both the type of thing and its name.' : 'Choose what you need access to, or tick the box to enter something not in the list.');
      const offered = askable(model, resource);
      const chosen = offered.length ? picked.filter((action) => offered.includes(action)) : [];
      const carriers = singleActionCarriers(model);
      const relations = level ? [level] : chosen.map((action) => carriers.get(action) ?? '');
      if (!relations.length || relations.some((relation) => !relation || !relationsOf(model, resource.kind)[relation])) throw new Error('Choose the access you need.');
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
    <h2>Ask for access</h2>
    {wanted ? <p className="ask-wanted">You are asking to <strong>{wantedWords(model, wanted)}</strong>. Say why, and someone who can give it will decide.</p>
      : <p>Choose what you need and explain why. Someone who can grant that access will review your request.</p>}
    <fieldset className="ask-fields" disabled={pending.kind !== 'empty' || rest.kind === 'held' || damaged || busy}>
      <What folded={folded}>
      {!advanced ? <label className="field">What do you need access to?<select value={resourceIndex} onChange={(event) => setResourceIndex(event.target.value)} required>
        <option value="">Choose what you need</option>{resources.map((resource, index) => <option key={JSON.stringify(resource)} value={index}>{resource.id} · {resource.kind}</option>)}
      </select></label> : null}
      {!resources.length ? <p className="note">{(unlisted ? 'There is no list of your access to choose from. ' + unlisted : 'You hold no access yet, so there is no list to choose from.') + ' Type what you need below; whoever approves sees exactly what you typed.'}</p> : null}
      {resources.length ? <label className="tick"><input type="checkbox" name="another" checked={advanced} onChange={(event) => setAdvanced(event.target.checked)} />What I need is not in the list</label> : null}
        {advanced ? <div className="form-grid"><label className="field">Type of thing<input name="kind" required autoComplete="off" value={typedKind} onChange={(event) => setTypedKind(event.target.value)} /><span className="hint">For example, project or workspace.</span></label><label className="field">Its name<input name="resource" required autoComplete="off" defaultValue={wanted && wantedIndex < 0 ? wanted.id : undefined} /><span className="hint">Exactly as the person or app that told you about it wrote it.</span></label></div> : null}
      {(() => {
        const resource = advanced ? undefined : chosenResource;
        const offered = askable(model, resource);
        return <LevelPicker model={model} relations={relationsOf(model, advanced ? typedKind : chosenResource?.kind ?? '')} value={level} legend="You need to" moreLabel="More: only some specific actions"
          onChange={(relation) => { setLevel(relation); setPicked([]); }} moreOpen={picked.length > 0}
          more={offered.length ? <><ActionPicker model={model} resource={resource as ResourceRef} actions={offered} value={picked} onChange={(actions) => { setPicked(actions); setLevel(''); }} agents={false} title="Specific actions" />
            <p className="note">Each ticked action is asked for on its own and approved on its own.</p></> : undefined} />;
      })()}
      </What>
      <label className="field">Why do you need this access?<textarea name="why" required /></label>
      <label className="field">Access until (your local time)<input name="expires" type="datetime-local" required={!noExpiry} disabled={noExpiry} /></label>
      <label className="tick"><input type="checkbox" name="no-expiry" checked={noExpiry} onChange={(event) => setNoExpiry(event.target.checked)} /> No expiry requested</label>
      <p><Act symbol="send" name="Request access" word="Request" tone="primary" type="submit" disabled={busy} />{' '}
        {cancel ? <Act symbol="close" name="Cancel asking" word="Cancel" disabled={busy} onClick={cancel} /> : null}</p>
    </fieldset>
    {pending.kind === 'held' ? <div role="status"><p>The result is not yet confirmed. Your original request is retained; checking will not create a duplicate.</p><Act symbol="again" name={busy ? 'Checking…' : rest.kind === 'held' ? `Check original request, then ask for the ${rest.asks.length} remaining` : 'Check original request'} word={busy ? 'Checking…' : rest.kind === 'held' ? 'Check, then ask' : 'Check'} disabled={busy} onClick={() => void send(pending.asked, true, rest.kind === 'held' ? rest.asks.filter((each) => each.operation !== pending.asked.operation) : [])} /></div> : null}
    {pending.kind === 'empty' && rest.kind === 'held' ? <div role="status"><p>{rest.asks.length} of an earlier run not yet sent: {rest.asks.map((each) => each.relation).join(', ')}.</p><Act symbol="send" name={busy ? 'Sending…' : `Ask for the ${rest.asks.length} remaining`} word={busy ? 'Sending…' : 'Ask'} data-act="ask-rest" disabled={busy} onClick={() => { const [head, ...tail] = rest.asks; if (head) void send(head, false, tail); }} /></div> : null}
    {damaged ? <StuckRecord keys={[...(pending.kind === 'damaged' ? [key] : []), ...(rest.kind === 'damaged' ? [restKey(key)] : [])]} what="your request"
      again="ask for it" look="Look for it under All waiting or Decided on this page; if it is not there, ask an administrator to look."
      settled={() => { setPending(readPending(key)); setRest(readRest(key)); }} /> : null}
    {failure ? <p className="why-not" role="alert">{failure}</p> : null}
    {answer ? <p role="status">{answer}</p> : null}
  </form>;
}
