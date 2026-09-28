import { pendingGrantKey, readPendingGrant } from './pendingGrant';
import { useRef, useState } from 'react';
import { Refused, api, operationId } from '../../api';
import type { DelegateBody, Grant, PassOn } from '../../generated/grants';
import { keyable } from '../../shell/keyable';
import { useShell } from '../../shell/ShellContext';
import { day } from '../file/time';
import { CannotGive } from './CannotGive';
import { grantNo, nameOf, onText, passText, relationsOf, withinPassOn } from './model';
import type { GrantWorld } from './model';

const DAY = 86400;

type Outcome =
  | { at: 'editing' }
  | { at: 'sending' }
  | { at: 'refused'; refused: Refused }
  | { at: 'pending'; reason: string };

/** Whether a failure leaves the change uncertain: kept pending under its operation, never retried as new. */
const uncertain = (r: Refused) => r.status < 400 || r.status >= 500;

/**
 * Give part of a grant to one of your agents (conformance 2.1 to 2.4). Every
 * decision is the service's: this form only asks, and shows its answer.
 */
export function Delegate({ w, source, to, done }: { w: GrantWorld; source: Grant; to?: string; done: () => void }) {
  const shell = useShell();
  const me = w.me.person.id;
  const agents = [...w.who.entries()].filter(([, x]) => x.kind === 'agent' && x.responsible === me && x.state !== 'retired');
  const relations = relationsOf(w);
  const fits = relations.filter(([, actions]) => withinPassOn(source, actions));
  const [recipient, setRecipient] = useState(to ?? agents[0]?.[0] ?? '');
  const [relation, setRelation] = useState(fits.find(([r]) => r === 'viewer')?.[0] ?? fits.at(-1)?.[0] ?? '');
  const [lasts, setLasts] = useState('7 days');
  const [pass, setPass] = useState('no');
  const [outcome, setOutcome] = useState<Outcome>({ at: 'editing' });
  const key = pendingGrantKey(me, source.id);
  const [pending, setPending] = useState(() => readPendingGrant(key, me, source.id));
  const working = useRef(false);
  const locked = pending.kind !== 'empty' || outcome.at === 'sending';
  const actions = relations.find(([r]) => r === relation)?.[1] ?? [];
  const onward = source.pass_on.kind === 'to' && source.pass_on.recipients.includes('agent');
  const ends = source.window.ends_at;
  const now = Math.floor(Date.now() / 1000);
  const endFor = (choice: string): number | null => {
    if (choice === '7 days') return ends === null ? now + 7 * DAY : Math.min(ends, now + 7 * DAY);
    return ends;
  };

  const give = async () => {
    if (working.current || pending.kind === 'damaged') return;
    working.current = true;
    const retry = pending.kind === 'held';
    const passOn: PassOn = pass === 'no' ? { kind: 'use_only' } : { kind: 'to', actions, recipients: ['agent'] };
    const body: DelegateBody = pending.kind === 'held' ? pending.body : {
      operation: operationId(), route: 'browser', source: source.id, recipient, responsible: me,
      resource: source.resource, relation, pass_on: passOn, window: { starts_at: now, ends_at: endFor(lasts) },
    };
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
      shell.closeAll();
      const grantedActions = relations.find(([name]) => name === body.relation)?.[1] ?? [];
      shell.toast(`Given. ${nameOf(w, body.recipient)} can now ${grantedActions.join(', ')} ${onText(source)}, through you.`);
      done();
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
        </select>
      </div>
      <div className="field">
        <label>Relation</label>
        <div>
          {relations.map(([r, acts]) =>
            withinPassOn(source, acts) ? (
              <span key={r} className={'chk' + (r === relation ? ' on' : '')} data-pickrel={r} title={acts.join(', ')} aria-disabled={locked} onClick={() => { if (!locked) setRelation(r); }} {...keyable(() => { if (!locked) setRelation(r); })}>
                {r}
              </span>
            ) : (
              <span key={r} className="chk" style={{ opacity: 0.4 }} title="More than you hold">{r}</span>
            ),
          )}
        </div>
        <div className="note">Greyed: more than you hold.</div>
      </div>
      <div className="field">
        <label htmlFor="dLease">Lasts</label>
        <select id="dLease" value={lasts} onChange={(e) => setLasts(e.target.value)}>
          <option>7 days</option>
          <option disabled title="not built yet">ends with assignment</option>
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
        <div className="row"><span className="sec">Actions it allows</span><span className="mono">{source.actions.join(', ')}</span></div>
        <div className="row"><span className="sec">You may pass it on</span><span className="pass">{passText(source.pass_on)}</span></div>
        <div className="row"><span className="sec">Ends no later than <span className="open-q">illustrative</span></span><span>{ends !== null ? day(ends) + ', when yours does' : 'when yours ends or is revoked'}</span></div>
      </div>
      <div className="card"><h2>What you can&apos;t give</h2><CannotGive w={w} source={source} /></div>
      {outcome.at === 'refused' ? (
        <div className="why-not" id="dAnswer"><b>{outcome.refused.refusal.refusal}</b><div className="note">{outcome.refused.refusal.reason}</div></div>
      ) : null}
      {pending.kind === 'held' && outcome.at !== 'sending' ? (
        <div className="why-not" id="dAnswer" style={{ color: 'var(--warn)' }}>
          <b>pending</b>
          {pending.kind === 'held' ? <p>{nameOf(w, pending.body.recipient)} · {pending.body.relation} of {pending.body.resource.kind}:{pending.body.resource.id}. Original start {new Date(pending.body.window.starts_at * 1000).toLocaleString('en-AU', { timeZone: 'Australia/Melbourne' })}; end {pending.body.window.ends_at === null ? 'none' : new Date(pending.body.window.ends_at * 1000).toLocaleString('en-AU', { timeZone: 'Australia/Melbourne' })}.</p> : null}
          <div className="note">Sent as {pending.kind === 'held' ? pending.body.operation : ''}; the service has not confirmed it{outcome.at === 'pending' ? ' (' + outcome.reason + ')' : ''}. Check original grant sends exactly the same request and operation.</div>
        </div>
      ) : null}
      {pending.kind === 'damaged' ? <p role="alert">The retained grant request could not be read. Sending is blocked until its original outcome is established.</p> : null}
      <div style={{ marginTop: 14, display: 'flex', gap: 8, alignItems: 'center' }}>
        <button className="btn primary" data-act="delegatedo" disabled={outcome.at === 'sending' || pending.kind === 'damaged' || !recipient || !relation} onClick={give}>
          {pending.kind === 'held' ? 'Check original grant' : 'Give'}
        </button>
        <button className="btn" data-act="close" onClick={shell.closeAll}>Cancel</button>
      </div>
    </>
  );
}
