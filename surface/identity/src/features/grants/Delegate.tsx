import { ActionPicker, singleActionCarriers } from './ActionPicker';
import { agentOffer } from './agent-actions';
import { actionWords } from './action-words';
import { pendingGrantKey, readPendingGrant, readPendingRest, restKey } from './pendingGrant';
import { useRef, useState } from 'react';
import { Refused, api, operationId, useLoad } from '../../api';
import type { DelegateBody, Grant, PassOn } from '../../generated/grants';
import { keyable } from '../../shell/keyable';
import { useShell } from '../../shell/ShellContext';
import { day } from '../file/time';
import { readRoles } from '../roles/AssignedRoles';
import { CannotGiveList } from './CannotGiveList';
import { grantNo, nameOf, onText, passText, relationsOf, withinPassOn } from './model';
import type { GrantWorld } from './model';
import type { Role } from '../roles/contract';

const DAY = 86400;

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

/** Whether a failure leaves the change uncertain: kept pending under its operation, never retried as new. */
const uncertain = (r: Refused) => r.status < 400 || r.status >= 500;

/**
 * Give part of a grant to one of your agents, or to a person (conformance 2.1
 * to 2.4). Every decision is the service's: this form only asks, and shows its
 * answer, including what the chosen recipient cannot be given.
 */
export function Delegate({ w, source, to, done }: { w: GrantWorld; source: Grant; to?: string; done: () => void }) {
  const shell = useShell();
  const me = w.me.person.id;
  const agents = [...w.who.entries()].filter(([, x]) => x.kind === 'agent' && x.responsible === me && x.state !== 'retired');
  const people = [...w.who.entries()].filter(([id, x]) => x.kind === 'person' && id !== me && x.state !== 'retired');
  const relations = relationsOf(w);
  const fits = relations.filter(([, actions]) => withinPassOn(source, actions));
  const [recipient, setRecipient] = useState(to ?? agents[0]?.[0] ?? '');
  const [relation, setRelation] = useState(fits.find(([r]) => r === 'viewer')?.[0] ?? fits.at(-1)?.[0] ?? '');
  const [picked, setPicked] = useState<string[]>([]);
  const [lasts, setLasts] = useState('7 days');
  const [pass, setPass] = useState('no');
  const [outcome, setOutcome] = useState<Outcome>({ at: 'editing' });
  const key = pendingGrantKey(me, source.id);
  const [pending, setPending] = useState(() => readPendingGrant(key, me, source.id));
  const [rest, setRest] = useState(() => readPendingRest(key, me, source.id));
  const working = useRef(false);
  const damaged = pending.kind === 'damaged' || rest.kind === 'damaged';
  const locked = pending.kind !== 'empty' || outcome.at === 'sending';
  const toAgent = w.who.get(recipient)?.kind !== 'person';
  // An agent is offered one action at a time, never a wider relation: what the source may pass on, less what the service withholds from agents.
  const passable = source.pass_on.kind === 'to' ? source.actions.filter((action) => (source.pass_on as { actions: string[] }).actions.includes(action)) : [];
  const offer = agentOffer(w.model, source.resource.kind, passable);
  // The relation that carries one action alone, as the picker itself names it; an action with no such carrier is never offered.
  const carriers = singleActionCarriers(w.model);
  const carried = offer.offered.filter((action) => carriers.has(action));
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
    let submitted = false;
    try {
      sessionStorage.setItem(key, JSON.stringify(body));
      setPending({ kind: 'held', body });
      submitted = true;
      const answer = await api.delegate(body);
      if (answer.operation !== body.operation || answer.receipt?.caller !== me
        || typeof answer.grant !== 'string' || !/^grant-[0-9a-f]{32}$/.test(answer.grant)) {
        throw new Refused(200, { refusal: 'UnconfirmedAnswer', reason: 'The service did not confirm this grant operation. Its original details remain retained.' });
      }
      sessionStorage.removeItem(key);
      setPending({ kind: 'empty' });
      return true;
    } catch (error) {
      const refused = error instanceof Refused ? error : new Refused(0, { refusal: 'Unanswered', reason: String(error) });
      if (!submitted && !retry) {
        setPending({ kind: 'empty' });
        setOutcome({ at: 'refused', refused: new Refused(0, { refusal: 'RequestNotRetained', reason: 'Nothing was sent because this browser could not retain the grant request: ' + refused.message }) });
      } else if (retry || uncertain(refused)) {
        setOutcome({ at: 'pending', reason: refused.refusal.reason });
      } else {
        sessionStorage.removeItem(key);
        setPending({ kind: 'empty' });
        setOutcome({ at: 'refused', refused });
      }
      return false;
    }
  };

  /** The requests still to send after the retained one, written whole so a reload finishes the run exactly as it was ticked, never twice. */
  const keepRest = (left: DelegateBody[]) => {
    setRest(left.length ? { kind: 'held', bodies: left } : { kind: 'empty' });
    if (left.length) sessionStorage.setItem(restKey(key), JSON.stringify(left)); else sessionStorage.removeItem(restKey(key));
  };

  /** The words for what a body grants, as the model says them. */
  const grantedWords = (bodies: DelegateBody[]): string =>
    actionWords(w.model, source.resource, bodies.flatMap((body) => relations.find(([name]) => name === body.relation)?.[1] ?? [body.relation]));

  const give = async () => {
    if (working.current || damaged) return;
    working.current = true;
    try {
      // The retained request first, exactly as sent, then the rest exactly as ticked; or a fresh run, every body fixed now.
      let first: DelegateBody;
      let queue: DelegateBody[];
      let retry = false;
      if (pending.kind === 'held') {
        first = pending.body; queue = rest.kind === 'held' ? rest.bodies : []; retry = true;
      } else if (toAgent) {
        const bodies: DelegateBody[] = [];
        for (const action of picked) {
          const carrier = carriers.get(action);
          if (carrier === undefined) {
            setOutcome({ at: 'refused', refused: new Refused(0, { refusal: 'NoSingleActionRelation', reason: `The permission model has no relation carrying ${action} alone; nothing was sent.` }) });
            return;
          }
          bodies.push(bodyFor(carrier, [action]));
        }
        if (!bodies.length) return;
        [first, ...queue] = bodies;
      } else {
        first = bodyFor(relation, actions); queue = [];
      }
      const given: DelegateBody[] = [];
      keepRest(queue);
      let current: DelegateBody | undefined = first;
      while (current !== undefined) {
        const confirmed = await sendOne(current, retry);
        retry = false;
        if (!confirmed) {
          // An uncertain answer keeps the rest for the retry; a definite refusal ends the run and names what did and did not happen.
          if (outcome.at !== 'pending' && sessionStorage.getItem(key) === null) {
            keepRest([]);
            if (given.length || queue.length) {
              shell.toast(`${given.length ? `Given before the refusal: ${grantedWords(given)} ${onText(source)}. ` : ''}${queue.length ? `Not sent: ${grantedWords(queue)}.` : ''}`);
            }
          }
          return;
        }
        given.push(current);
        current = queue.shift();
        keepRest(queue);
      }
      shell.closeAll();
      shell.toast(`Given. ${nameOf(w, first.recipient)} can now ${grantedWords(given)} ${onText(source)}, through you.`);
      done();
    } finally { working.current = false; }
  };

  return (
    <>
      <div className="eyebrow">Your access</div>
      <h2 style={{ marginTop: 6, fontSize: 16 }}>Give part of {source.relation} of {onText(source)} to an agent</h2>
      <p className="sub" style={{ marginTop: 6 }}>Only what you hold and may pass on. It traces back to you, and ends when yours does.</p>
      <fieldset disabled={locked} style={{ border: 0, padding: 0 }}>
      <div className="field">
        <label htmlFor="dTo">To</label>
        <select id="dTo" value={recipient} onChange={(e) => setRecipient(e.target.value)}>
          {agents.map(([id, x]) => (
            <option key={id} value={id}>{x.name}{x.state !== 'active' ? ' (' + x.state + ')' : ''}</option>
          ))}
          {people.length > 0 ? (
            <optgroup label="People">
              {people.map(([id, x]) => (
                <option key={id} value={id}>{x.name}{x.state !== 'active' ? ' (' + x.state + ')' : ''}</option>
              ))}
            </optgroup>
          ) : null}
        </select>
      </div>
      {toAgent ? <div className="field">
        <label>Actions</label>
        {!offer.known ? <div className="note">The service has not said which actions an agent may hold, so none are offered yet.</div>
          : !carried.length ? <div className="note">{source.resource.kind.includes('.') ? "Lys can't give an agent this app's actions until the app allows it." : 'Nothing you may pass on here can be held by an agent.'}</div>
          : <ActionPicker model={w.model} resource={source.resource} actions={carried} value={picked} onChange={setPicked} disabled={locked} />}
        <div className="note">Each ticked action is given on its own; an agent is never given a wider relation.</div>
      </div> : <div className="field">
        <label>Relation</label>
        <div>
          {relations.map(([r, acts]) =>
            withinPassOn(source, acts) ? (
              <span key={r} className={'chk' + (r === relation ? ' on' : '')} data-pickrel={r} title={acts.join(', ')} aria-disabled={locked} {...keyable(() => { if (!locked) setRelation(r); })}>
                {r}
              </span>
            ) : (
              <span key={r} className="chk" style={{ opacity: 0.4 }} title="More than you hold">{r}</span>
            ),
          )}
        </div>
        <div className="note">Greyed: more than you hold.</div>
      </div>}
      <div className="field">
        <label htmlFor="dLease">Lasts</label>
        <select id="dLease" value={lasts} onChange={(e) => setLasts(e.target.value)}>
          <option>7 days</option>
          {holdings.length === 0
            ? <option disabled title={roles.status === 'ok' ? 'the agent holds no role with an end date' : roles.status === 'loading' ? 'reading the agent\'s roles' : roles.refused.refusal.reason}>ends with assignment</option>
            : holdings.map((h) => <option key={h.assignment} value={ASSIGNMENT + h.assignment}>ends with {h.role} assignment ({day(h.ends_at)})</option>)}
          <option>no end</option>
        </select>
      </div>
      <div className="field">
        <label htmlFor="dPass">May the agent pass it on</label>
        <select id="dPass" value={pass} onChange={(e) => setPass(e.target.value)}>
          <option>no</option>
          {onward ? <option value="to agents">to agents</option> : <option disabled>to agents (you may not allow this for {source.relation})</option>}
        </select>
      </div>
      </fieldset>
      <div className="card" style={{ marginTop: 4 }}>
        <h2>Where it comes from</h2>
        <div className="row"><span className="sec">Source grant</span><span className="mono">{grantNo(source.id)} · {source.relation} of {onText(source)}</span></div>
        <div className="row"><span className="sec">Actions it allows</span><span className="mono">{actionWords(w.model, source.resource, source.actions)}</span></div>
        <div className="row"><span className="sec">You may pass it on</span><span className="pass">{passText(source.pass_on)}</span></div>
        <div className="row"><span className="sec">Ends no later than</span><span>{source.effective_ends_at !== null ? day(source.effective_ends_at) : 'no end'}</span></div>
      </div>
      <div className="card"><h2>What you can&apos;t give</h2><CannotGiveList w={w} source={source} recipient={recipient} /></div>
      {outcome.at === 'refused' ? (
        <div className="why-not" id="dAnswer"><b>{outcome.refused.refusal.refusal}</b><div className="note">{outcome.refused.refusal.reason}</div></div>
      ) : null}
      {pending.kind === 'held' && outcome.at !== 'sending' ? (
        <div className="why-not" id="dAnswer" style={{ color: 'var(--warn)' }}>
          <b>pending</b>
          {pending.kind === 'held' ? <p>{nameOf(w, pending.body.recipient)} · {pending.body.relation} of {pending.body.resource.kind}:{pending.body.resource.id}. Original start {new Date(pending.body.window.starts_at * 1000).toLocaleString('en-AU', { timeZone: 'Australia/Melbourne' })}; end {pending.body.window.ends_at === null ? 'none' : new Date(pending.body.window.ends_at * 1000).toLocaleString('en-AU', { timeZone: 'Australia/Melbourne' })}.</p> : null}
          <div className="note">Sent as {pending.kind === 'held' ? pending.body.operation : ''}; the service has not confirmed it{outcome.at === 'pending' ? ' (' + outcome.reason + ')' : ''}. Check original grant sends exactly the same request and operation{rest.kind === 'held' ? `, then gives the ${rest.bodies.length} remaining` : ''}.</div>
        </div>
      ) : null}
      {damaged ? <p role="alert">The retained grant request could not be read. Sending is blocked until its original outcome is established.</p> : null}
      <div style={{ marginTop: 14, display: 'flex', gap: 8, alignItems: 'center' }}>
        <button className="btn primary" data-act="delegatedo" disabled={outcome.at === 'sending' || damaged || !recipient || !leaseKnown || (pending.kind !== 'held' && (toAgent ? picked.length === 0 : !relation))} onClick={give}>
          {pending.kind === 'held' ? 'Check original grant' : 'Give'}
        </button>
        <button className="btn" data-act="close" onClick={shell.closeAll}>Cancel</button>
      </div>
    </>
  );
}
