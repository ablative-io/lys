import { ActionPicker, RELATION_CHOICE, appAgentRefusal } from './ActionPicker';
import { delegateChoices } from './delegateChoices';
import { actionWords } from './action-words';
import { pendingGrantKey, readPendingGrant, readPendingRest, restKey } from './pendingGrant';
import { useRef, useState } from 'react';
import { Refused, api, operationId, useLoad } from '../../api';
import { keptSend, keepRecord, releaseRecord } from '../../kept';
import type { DelegateBody, Grant, PassOn } from '../../generated/grants';
import { useShell } from '../../shell/ShellContext';
import { clock, day } from '../file/time';
import { readRoles } from '../roles/AssignedRoles';
import { CannotGiveList } from './CannotGiveList';
import { appKind, appKindProblem, givenOnText, givesTo, grantNo, nameOf, named, passText, relationsOf, sourceText, withinPassOn } from './model';
import type { GrantWorld } from './model';
import type { Role } from '../roles/contract';
import { Act } from '../../shell/Act';
import { AppsFirst, anyAppResource } from './AppsFirst';
import { LevelPicker } from './LevelPicker';
import { legibleRecipients, StuckRecord } from './StuckRecord';
import { PendingWords, RefusedWords } from './Outcome';
import './grants.css';

const DAY = 86400;

/** A recipient that is not active says so in words, after its name. */
const stateWords = (state: string): string => state === 'active' ? '' : state === 'registered' ? ' (not yet activated)' : state === 'suspended' ? ' (suspended)' : ' (' + state + ')';

/** The lease choice that binds the grant's end to one of the agent's role holdings. */
const ASSIGNMENT = 'assignment:';

/** The recipient's role holdings that end on a date, so a grant may end with one (conformance 4.5). */
const roleHoldings = (roles: Role[], holder: string) => roles.flatMap((role) => role.holders.flatMap((h) =>
  h.holder === holder && h.state === 'holding' && h.ends_at !== null ? [{ assignment: h.assignment, role: role.name, ends_at: h.ends_at }] : []));

type Outcome =
  | { at: 'editing' }
  | { at: 'sending' }
  | { at: 'refused'; refused: Refused }
  | { at: 'pending'; reason: string };


/**
 * Give part of a grant to one of your agents, or to a person (conformance 2.1
 * to 2.4). Every decision is the service's: this form only asks, and shows its
 * answer, including what the chosen recipient cannot be given.
 */
export function Delegate({ w, source, to, initialChoices = [], done, close }: { w: GrantWorld; source: Grant; to?: string; initialChoices?: string[]; done: () => void; close: () => void }) {
  const shell = useShell();
  const me = w.me.person.id;
  // Only the kinds of recipient the source may be given to are offered: a name the service would refuse is never in the list.
  const allowed = givesTo(source.pass_on);
  const agents = allowed.includes('agent') ? [...w.who.entries()].filter(([, x]) => x.kind === 'agent' && x.responsible === me && x.state !== 'retired') : [];
  const people = allowed.includes('person') ? [...w.who.entries()].filter(([id, x]) => x.kind === 'person' && id !== me && x.state !== 'retired') : [];
  const firstActive = [...agents, ...people].find(([, x]) => x.state === 'active')?.[0];
  const app = source.resource.kind.includes('.');
  // An app kind the model does not declare (retired, or approved after this read) offers nothing and says why, beside the grant.
  const kindProblem = app ? appKindProblem(w.model, source.resource.kind) : null;
  const declared = app ? appKind(w.model, source.resource.kind) : null;
  const relations = declared ? Object.entries(declared.relations) : relationsOf(w);
  const fits = relations.filter(([, actions]) => withinPassOn(source, actions));
  const [recipient, setRecipient] = useState(to ?? firstActive ?? '');
  const [relation, setRelation] = useState(fits.find(([r]) => r === 'viewer')?.[0] ?? fits.at(-1)?.[0] ?? '');
  const [picked, setPicked] = useState<string[]>(initialChoices);
  const [lasts, setLasts] = useState('7 days');
  const [pass, setPass] = useState('no');
  const [outcome, setOutcome] = useState<Outcome>({ at: 'editing' });
  const key = pendingGrantKey(me, source.id);
  const [pending, setPending] = useState(() => readPendingGrant(key, me, source.id));
  const [rest, setRest] = useState(() => readPendingRest(key, me, source.id));
  const working = useRef(false);
  const unreleased = useRef(false);
  const damaged = pending.kind === 'damaged' || rest.kind === 'damaged';
  const stuckKeys = [...(pending.kind === 'damaged' ? [key] : []), ...(rest.kind === 'damaged' ? [restKey(key)] : [])];
  // A give to someone else is not in the giver's own list of grants, so a kept copy that still names one recipient points at their file.
  const legible = damaged ? legibleRecipients(stuckKeys).filter((id) => w.who.has(id)) : [];
  const gaveTo = legible.length === 1 ? <a href={'#/file/' + encodeURIComponent(legible[0]) + '/access'}>{nameOf(w, legible[0])}’s file</a> : undefined;
  const locked = pending.kind !== 'empty' || outcome.at === 'sending';
  const toAgent = w.who.get(recipient)?.kind !== 'person';
  const { known, offered, carried, choices } = delegateChoices(w.model, source);
  const noActions = kindProblem ?? (declared !== null && declared.agents.length === 0 ? appAgentRefusal
    : offered.length > 0 ? app ? 'The app has no relation carrying only the agent actions you may pass on.' : 'The permission model has no relation carrying these actions alone.'
      : 'You hold nothing on this resource that an agent could be given');
  const heldWords = source.actions.length ? 'You hold: ' + actionWords(w.model, source.resource, source.actions) + '.' : '';
  const actions = relations.find(([r]) => r === relation)?.[1] ?? [];
  const onward = source.pass_on.kind === 'to' && source.pass_on.recipients.includes('agent');
  const ends = source.window.ends_at;
  const now = Math.floor(Date.now() / 1000);
  const roles = useLoad(readRoles, 'delegate-roles:' + recipient);
  const holdings = roles.status === 'ok' ? roleHoldings(roles.data.roles, recipient) : [];
  const chosen = holdings.find((h) => ASSIGNMENT + h.assignment === lasts);
  const leaseKnown = !lasts.startsWith(ASSIGNMENT) || chosen !== undefined;
  const noLater = (end: number) => (ends === null ? end : Math.min(ends, end));
  const endFor = (choice: string): number | null => {
    if (choice === '7 days') return noLater(now + 7 * DAY);
    if (chosen) return noLater(chosen.ends_at);
    return ends;
  };

  // Give is never greyed without its reason beside it.
  const fresh = pending.kind !== 'held' && rest.kind !== 'held';
  const whyNot = damaged ? 'Check what you gave earlier with Lys first (below).'
    : !recipient ? 'Choose who to give it to.'
      : !leaseKnown ? 'Choose how long it lasts.'
        : fresh && toAgent && picked.length === 0 ? 'Tick at least one thing they may do.'
          : fresh && !toAgent && !relation ? 'Choose what they may do.' : '';

  /** The body for one relation, as this form fills it now. */
  const bodyFor = (chosen: string, carried: string[]): DelegateBody => {
    const passOn: PassOn = pass === 'no' ? { kind: 'use_only' } : { kind: 'to', actions: carried, recipients: ['agent'] };
    return {
      operation: operationId(), route: 'browser', source: source.id, recipient, responsible: w.who.get(recipient)?.kind === 'person' ? recipient : me,
      resource: source.resource, relation: chosen, pass_on: passOn, window: { starts_at: now, ends_at: endFor(lasts) },
    };
  };

  /**
   * One grant request, retained under the source's key before it is sent and
   * released only on the service's confirmation. Returns whether it is confirmed;
   * a refusal or an uncertain answer is shown and ends the run.
   */
  const sendOne = async (body: DelegateBody, retry: boolean): Promise<boolean> => {
    setOutcome({ at: 'sending' });
    const kept = await keptSend(key, body, async () => {
      setPending({ kind: 'held', body });
      const answer = await api.delegate(body);
      if (answer.operation !== body.operation || answer.receipt?.caller !== me
        || typeof answer.grant !== 'string' || !/^grant-[0-9a-f]{32}$/.test(answer.grant)) {
        throw new Refused(200, { refusal: 'UnconfirmedAnswer', reason: 'The service did not confirm this grant operation. Its original details remain retained.' });
      }
      return answer;
    }, !retry);
    if (kept.at === 'confirmed') {
      if (kept.unreleased) unreleased.current = true; else setPending({ kind: 'empty' });
      return true;
    }
    const refused = kept.error instanceof Refused ? kept.error : new Refused(0, { refusal: 'Unanswered', reason: String(kept.error) });
    if (kept.at === 'unkept' && !retry) {
      setPending({ kind: 'empty' });
      setOutcome({ at: 'refused', refused: new Refused(0, { refusal: 'RequestNotRetained', reason: 'Nothing was sent because this browser could not retain the grant request: ' + refused.message }) });
    } else if (kept.at === 'refused') {
      if (!kept.unreleased) setPending({ kind: 'empty' });
      setOutcome({ at: 'refused', refused: kept.unreleased ? new Refused(refused.status, { refusal: refused.refusal.refusal, reason: refused.refusal.reason + ' The browser could not release the retained request; Check original grant will answer this same refusal, never a second grant: ' + String(kept.unreleased) }) : refused });
    } else {
      setOutcome({ at: 'pending', reason: refused.refusal.reason });
    }
    return false;
  };

  /** The requests still to send after the retained one, written whole so a reload finishes the run exactly as it was ticked, never twice. */
  const keepRest = (left: DelegateBody[]) => {
    // Storage first, then a copy into state: state says what storage says, never a rest the browser refused to keep.
    if (left.length) keepRecord(restKey(key), left); else releaseRecord(restKey(key));
    setRest(left.length ? { kind: 'held', bodies: [...left] } : { kind: 'empty' });
  };

  /** The words for what a body grants, as the model says them. */
  const grantedWords = (bodies: DelegateBody[]): string =>
    actionWords(w.model, source.resource, bodies.flatMap((body) => relations.find(([name]) => name === body.relation)?.[1] ?? [body.relation]));

  /** Retain one body under the pending key: always before the rest is shortened, so no body is ever both forgotten and unsent. */
  const retain = (body: DelegateBody) => {
    keepRecord(key, body);
    setPending({ kind: 'held', body });
  };
  const notRetained = (error: unknown) => setOutcome({ at: 'refused', refused: new Refused(0, { refusal: 'RequestNotRetained', reason: 'Nothing more was sent because this browser could not retain the grant request: ' + (error instanceof Error ? error.message : String(error)) }) });

  const give = async () => {
    if (working.current || damaged) return;
    working.current = true;
    try {
      // The retained request first, exactly as sent, then the rest exactly as ticked; a rest left without a retained body resumes from the rest; or a fresh run, every body fixed now.
      let first: DelegateBody;
      let queue: DelegateBody[];
      let retry = false;
      if (pending.kind === 'held') {
        const held = pending.body;
        first = held; queue = rest.kind === 'held' ? rest.bodies.filter((body) => body.operation !== held.operation) : []; retry = true;
      } else if (rest.kind === 'held') {
        [first, ...queue] = rest.bodies;
      } else if (toAgent) {
        const bodies: DelegateBody[] = [];
        for (const choice of picked) {
          const carrier = choices.get(choice);
          if (carrier === undefined) {
            setOutcome({ at: 'refused', refused: new Refused(0, { refusal: 'GrantChoiceUnavailable', reason: `The permission model no longer carries the selected choice ${choice.replace(RELATION_CHOICE, '')}, so nothing was sent; choose the actions again.` }) });
            return;
          }
          bodies.push(bodyFor(carrier.relation, carrier.actions));
        }
        if (!bodies.length) return;
        [first, ...queue] = bodies;
      } else {
        first = bodyFor(relation, actions); queue = [];
      }
      const given: DelegateBody[] = [];
      unreleased.current = false;
      try {
        // The whole run is kept before anything else, so a failure at any later write leaves every body somewhere it is offered again by name.
        if (!retry) { keepRest([first, ...queue]); retain(first); }
        keepRest(queue);
      } catch (error) {
        // Nothing has been sent. What is stored is offered again on the next Give, exactly as ticked, never twice.
        notRetained(error);
        return;
      }
      let current: DelegateBody | undefined = first;
      while (current !== undefined) {
        const confirmed = await sendOne(current, retry);
        retry = false;
        if (!confirmed) {
          // An uncertain answer keeps the rest for the retry; a definite refusal ends the run and names what did and did not happen.
          if (outcome.at !== 'pending' && sessionStorage.getItem(key) === null) {
            let cleared = true;
            try { keepRest([]); } catch { cleared = false; }
            if (given.length || queue.length || !cleared) {
              shell.toast(`${given.length ? `Given before the refusal: ${grantedWords(given)} ${givenOnText(source)}. ` : ''}${queue.length ? `Not sent: ${grantedWords(queue)}.` : ''}${cleared ? '' : ' The browser could not clear them; they will be offered again, never sent by themselves.'}`);
            }
          }
          return;
        }
        given.push(current);
        const next = queue.shift();
        if (next !== undefined) {
          try {
            retain(next);
            keepRest(queue);
          } catch (error) {
            // Either the rest still holds next (retain failed) or the key holds it beside the rest (shortening failed): both resume exactly, so stop and say so.
            notRetained(error);
            shell.toast(`Given before the browser stopped retaining: ${grantedWords(given)} ${givenOnText(source)}. Not sent: ${grantedWords([next, ...queue])}.`);
            return;
          }
        }
        current = next;
      }
      close();
      shell.toast(`Given. ${nameOf(w, first.recipient)} can now ${grantedWords(given)} ${givenOnText(source)}, through you.${unreleased.current ? ' The browser could not release the retained request; opening this again checks it, never gives it twice.' : ''}`);
      done();
    } finally { working.current = false; }
  };

  return (
    <>
      <div className="eyebrow">Your access</div>
      <h2 className="give-head">Give part of {source.relation} of {sourceText(source)} to {allowed.includes('person') ? allowed.includes('agent') ? 'someone' : 'a person' : 'an agent'}</h2>
      <p className="sub give-sub">Only what you hold and may pass on. It traces back to you, and ends when yours does.</p>
      <fieldset className="give-fields" disabled={locked}>
      <div className="field">
        <label htmlFor="dTo">To</label>
        {agents.length || people.length ? <select id="dTo" value={recipient} onChange={(e) => setRecipient(e.target.value)}>
          {recipient ? null : <option value="" disabled>Choose who to give it to</option>}
          {agents.length > 0 ? (
            <optgroup label="Your agents">
              {agents.map(([id, x]) => <option key={id} value={id} disabled={x.state !== 'active'}>{x.name}{stateWords(x.state)}</option>)}
            </optgroup>
          ) : null}
          {people.length > 0 ? (
            <optgroup label="People">
              {people.map(([id, x]) => <option key={id} value={id} disabled={x.state !== 'active'}>{x.name}{stateWords(x.state)}</option>)}
            </optgroup>
          ) : null}
        </select> : <p className="note" data-no-recipient>{allowed.includes('person')
          ? <>There is no one to give it to yet. <a href="#/people">Add a person or an agent on People and agents</a> first.</>
          : <>This access may be given only to your agents, and you have none yet. <a href="#/agents/new">Add an agent</a> first.</>}</p>}
        {[...agents, ...people].some(([, x]) => x.state !== 'active') ? <div className="note">Someone not yet activated or suspended is listed but cannot be chosen: Lys gives access only to an active person or agent.</div> : null}
      </div>
      {toAgent ? <div className="field">
        {/* The picker's own legend names the actions; a label is the heading only when no picker is shown. */}
        {known && carried.length ? null : <label>Actions</label>}
        {!known ? <div className="note">The service has not said which actions an agent may hold, so none are offered yet.</div>
          : !carried.length ? <div className="note">{noActions}{heldWords ? <div>{heldWords}</div> : null}</div>
          : <ActionPicker model={w.model} resource={source.resource} actions={carried} value={picked} onChange={setPicked} disabled={locked} />}
        <div className="note">{app ? 'Each choice gives exactly the actions shown.' : 'Each ticked action is given on its own; an agent is never given a wider relation.'}</div>
        {anyAppResource(w.list.grants.map((grant) => grant.resource)) ? null : <AppsFirst />}
      </div> : <div className="field">
        <LevelPicker model={w.model} relations={Object.fromEntries(relations)} value={relation} onChange={setRelation} allowed={(acts) => withinPassOn(source, acts)} disabled={locked} />
        {kindProblem ? <div className="note">{kindProblem}</div> : null}
      </div>}
      <div className="field">
        <label htmlFor="dLease">Lasts</label>
        <select id="dLease" value={lasts} onChange={(e) => setLasts(e.target.value)}>
          <option value="7 days">7 days</option>
          {holdings.length === 0
            ? <option disabled value="">{roles.status === 'loading' ? 'Ends with a role assignment (reading roles…)' : roles.status === 'ok' ? 'Ends with a role assignment (none with an end date)' : 'Ends with a role assignment (roles could not be read)'}</option>
            : holdings.map((h) => <option key={h.assignment} value={ASSIGNMENT + h.assignment}>Ends with the {h.role} assignment ({day(h.ends_at)})</option>)}
          <option value="no end">{ends === null ? 'No end' : 'When your own access ends'}</option>
        </select>
      </div>
      <div className="field">
        <label htmlFor="dPass">May they pass it on</label>
        <select id="dPass" value={pass} onChange={(e) => setPass(e.target.value)}>
          <option value="no">No</option>
          {onward ? <option value="to agents">Yes, to their own agents</option> : <option disabled value="">Yes (your own access does not allow it)</option>}
        </select>
      </div>
      </fieldset>
      <div className="card give-source">
        <h2>Where it comes from</h2>
        <div className="row"><span className="sec">Source grant</span><span className="mono">{grantNo(source.id)} · {source.relation} of {sourceText(source)}</span></div>
        <div className="row"><span className="sec">Actions it allows</span><span className="mono">{actionWords(w.model, source.resource, source.actions)}</span></div>
        <div className="row"><span className="sec">You may pass it on</span><span className="pass">{passText(source.pass_on)}</span></div>
        <div className="row"><span className="sec">Ends no later than</span><span>{source.effective_ends_at !== null ? day(source.effective_ends_at) : 'no end'}</span></div>
      </div>
      <div className="card"><h2>What you can&apos;t give</h2><CannotGiveList w={w} source={source} recipient={recipient} /></div>
      {outcome.at === 'refused' ? <RefusedWords w={w} id="dAnswer" refused={outcome.refused} /> : null}
      {pending.kind === 'held' && outcome.at !== 'sending' ? (
        <PendingWords id="dAnswer">
          <p>{nameOf(w, pending.body.recipient)}: {grantedWords([pending.body])} {givenOnText(source)}, from {clock(pending.body.window.starts_at)} {pending.body.window.ends_at === null ? 'with no end' : 'to ' + clock(pending.body.window.ends_at)}.</p>
          <div className="note">Lys has not confirmed this grant{outcome.at === 'pending' ? ' (' + named(w, outcome.reason.replace(/^[A-Za-z_]+:\s*/, '')) + ')' : ''}. Check sends exactly the same request again, so it can never be given twice{rest.kind === 'held' ? `, then gives the ${rest.bodies.length} remaining` : ''}.</div>
        </PendingWords>
      ) : null}
      {damaged ? <StuckRecord keys={stuckKeys} what="the access you gave" again="give it" where={gaveTo}
        look={<>Open {gaveTo ?? 'the file of the person or agent you gave it to'}, or ask an administrator to look at their access.</>}
        settled={() => { setPending(readPendingGrant(key, me, source.id)); setRest(readPendingRest(key, me, source.id)); }} /> : null}
      <div className="give-actions">
        {pending.kind === 'empty' && rest.kind === 'held' ? <div className="note" data-rest-held>{rest.bodies.length} of an earlier run not yet sent: {grantedWords(rest.bodies)}. Give sends them exactly as ticked, nothing else.</div> : null}
        <Act symbol={pending.kind === 'held' ? 'again' : 'send'} name={pending.kind === 'held' ? 'Check original grant' : 'Give'} word={pending.kind === 'held' ? 'Check' : 'Give'} tone="primary" data-act="delegatedo" disabled={outcome.at === 'sending' || whyNot !== ''} onClick={give} />
        <Act symbol="close" name="Cancel" word="Cancel" data-act="close" onClick={close} />
        {whyNot && outcome.at !== 'sending' ? <span className="why-not" data-why-not>{whyNot}</span> : null}
      </div>
    </>
  );
}
