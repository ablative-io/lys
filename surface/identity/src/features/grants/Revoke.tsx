import { TAKE_BACK } from '../requests/decision-words';
import './grants.css';
import { useRef, useState } from 'react';
import { Refused, api, operationId } from '../../api';
import type { Grant } from '../../generated/grants';
import { useShell } from '../../shell/ShellContext';
import { nameOf, named, resourceName } from './model';
import { PendingWords, RefusedWords } from './Outcome';
import type { GrantWorld } from './model';
import { Act } from '../../shell/Act';

/** Every grant the caller can see that derives from `g`, at any depth. */
export function derivedFrom(w: GrantWorld, g: Grant): Grant[] {
  const bySource = new Map<string, Grant[]>();
  for (const x of w.list.grants) {
    if (x.source === null) continue;
    const children = bySource.get(x.source);
    if (children) children.push(x);
    else bySource.set(x.source, [x]);
  }
  const out: Grant[] = [];
  const seen = new Set([g.id]);
  const frontier = [g.id];
  for (let id = frontier.pop(); id !== undefined; id = frontier.pop()) {
    for (const x of bySource.get(id) ?? []) {
      if (seen.has(x.id)) continue;
      seen.add(x.id);
      out.push(x);
      frontier.push(x.id);
    }
  }
  return out;
}

type Outcome = { at: 'editing' } | { at: 'sending' } | { at: 'refused'; refused: Refused } | { at: 'pending'; reason: string };

/** Withdraw a grant, and everything derived from it (conformance 2.5). */
export function Revoke({ w, g, done, close }: { w: GrantWorld; g: Grant; done: () => void; close: () => void }) {
  const shell = useShell();
  const [why, setWhy] = useState('No longer needed.');
  const [outcome, setOutcome] = useState<Outcome>({ at: 'editing' });
  const op = useRef(operationId());
  const derived = derivedFrom(w, g).filter((x) => !x.revoked);

  const revoke = async () => {
    setOutcome({ at: 'sending' });
    try {
      await api.revoke(g.id, { operation: op.current, route: 'browser', reason: why });
      close();
      shell.toast(`Taken back from ${nameOf(w, g.holder)}. Everything given on from it goes with it.`);
      done();
    } catch (error) {
      const refused = error instanceof Refused ? error : new Refused(0, { refusal: 'Unanswered', reason: String(error) });
      if (refused.status === 0 || refused.status >= 500) {
        setOutcome({ at: 'pending', reason: refused.refusal.reason });
      } else {
        op.current = operationId();
        setOutcome({ at: 'refused', refused });
      }
    }
  };

  return (
    <>
      <div className="eyebrow">Access</div>
      <h2 className="give-head">Take back {nameOf(w, g.holder)}’s access to {resourceName(w, g.resource)}</h2>
      <p className="sub give-sub">Everything that depends on this grant, before it goes.</p>
      <div className="field">
        <label htmlFor="why">Reason <span className="hint">(kept with the record)</span></label>
        <textarea id="why" className="take-back-why" value={why} disabled={outcome.at === 'sending'} onChange={(e) => setWhy(e.target.value)} />
      </div>
      <div className="card take-back-facts">
        <dl className="facts">
          <dt>Held by</dt>
          <dd><span className="pill">{nameOf(w, g.holder)}</span></dd>
          <dt>Given on from it</dt>
          <dd id="derived">
            {derived.length ? (
              <>
                {derived.map((x) => <span className="pill" key={x.id}>{nameOf(w, x.holder)} · {x.relation}</span>)}
                <span className="note">taken back with it</span>
              </>
            ) : <span className="dim">none</span>}
          </dd>
        </dl>
      </div>
      {outcome.at === 'refused' ? <RefusedWords w={w} id="rAnswer" refused={outcome.refused} /> : null}
      {outcome.at === 'pending' ? (
        <PendingWords id="rAnswer">
          <div className="note">Lys has not confirmed taking it back ({named(w, outcome.reason.replace(/^[A-Za-z_]+:\s*/, ''))}). Take back again sends exactly the same request, so it is never done twice.</div>
        </PendingWords>
      ) : null}
      <div className="give-actions">
        <Act symbol="revoke" name={TAKE_BACK} word={TAKE_BACK} tone="danger" data-act="revokedo" data-g={g.id} disabled={outcome.at === 'sending'} onClick={revoke} />
        <Act symbol="close" name="Cancel" word="Cancel" data-act="close" onClick={close} />
      </div>
    </>
  );
}
