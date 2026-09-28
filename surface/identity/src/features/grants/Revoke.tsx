import { useRef, useState } from 'react';
import { Refused, api, operationId } from '../../api';
import type { Grant } from '../../generated/grants';
import { useShell } from '../../shell/ShellContext';
import { grantNo, nameOf, onText } from './model';
import type { GrantWorld } from './model';

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
export function Revoke({ w, g, done }: { w: GrantWorld; g: Grant; done: () => void }) {
  const shell = useShell();
  const [why, setWhy] = useState('No longer needed.');
  const [outcome, setOutcome] = useState<Outcome>({ at: 'editing' });
  const op = useRef(operationId());
  const derived = derivedFrom(w, g).filter((x) => !x.revoked);

  const revoke = async () => {
    setOutcome({ at: 'sending' });
    try {
      await api.revoke(g.id, { operation: op.current, route: 'browser', reason: why });
      shell.closeAll();
      shell.toast(`Revoked ${grantNo(g.id)}. Everything derived from it goes with it.`);
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
      <h2 style={{ marginTop: 6, fontSize: 16 }}>Revoke {g.relation} of {onText(g)}</h2>
      <p className="sub" style={{ marginTop: 6 }}>Everything that depends on this grant, before it goes.</p>
      <div className="field">
        <label htmlFor="why">Reason</label>
        <textarea id="why" style={{ minHeight: 60 }} value={why} onChange={(e) => setWhy(e.target.value)} />
      </div>
      <div className="card" style={{ marginTop: 12 }}>
        <dl className="facts">
          <dt>Held by</dt>
          <dd><span className="pill">{nameOf(w, g.holder)}</span></dd>
          <dt>Grants derived from it</dt>
          <dd id="derived">
            {derived.length ? (
              <>
                {derived.map((x) => <span className="pill" key={x.id}>{nameOf(w, x.holder)} · {x.relation}</span>)}
                <span className="note">revoked with it</span>
              </>
            ) : <span className="dim">none</span>}
          </dd>
        </dl>
      </div>
      {outcome.at === 'refused' ? (
        <div className="why-not" id="rAnswer"><b>{outcome.refused.refusal.refusal}</b><div className="note">{outcome.refused.refusal.reason}</div></div>
      ) : null}
      {outcome.at === 'pending' ? (
        <div className="why-not" id="rAnswer" style={{ color: 'var(--warn)' }}>
          <b>pending</b>
          <div className="note">Sent as {op.current}; the service has not confirmed it ({outcome.reason}). Revoke again sends the same operation.</div>
        </div>
      ) : null}
      <div style={{ marginTop: 14, display: 'flex', gap: 8, alignItems: 'center' }}>
        <button className="btn primary" data-act="revokedo" data-g={g.id} disabled={outcome.at === 'sending'} onClick={revoke}>Revoke</button>
        <button className="btn" data-act="close" onClick={shell.closeAll}>Cancel</button>
      </div>
    </>
  );
}
