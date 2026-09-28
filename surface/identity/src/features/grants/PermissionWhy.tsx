import { useState } from 'react';
import { useParams } from 'react-router';
import { Refused, api, request, useLoad } from '../../api';
import { resourceText } from '../../generated/grants';
import type { GrantList, ResourceRef } from '../../generated/grants';

/** One identity on the path of a permitted answer, as the server names it. */
export interface PathStep {
  identity: string;
  responsible: boolean;
}

/** POST /grants/explain: the server's answer to why an identity can or cannot do a thing. */
export interface Explanation {
  permitted: boolean;
  identity: string;
  resource: ResourceRef;
  action: string;
  grant: string | null;
  path: PathStep[];
  responsible: string | null;
  reason: string | null;
  policy: { schema_sha256: string; revision: string };
}

type Asked =
  | { state: 'idle' }
  | { state: 'asking' }
  | { state: 'answered'; answer: Explanation }
  | { state: 'refused'; refused: Refused };

/**
 * What may be asked about: every resource the loaded grants are on and the
 * actions they grant there. It offers questions only; it never answers one.
 */
function askable(list: GrantList): Map<string, { resource: ResourceRef; actions: string[] }> {
  const out = new Map<string, { resource: ResourceRef; actions: string[] }>();
  for (const g of list.grants) {
    const key = resourceText(g.resource);
    const at = out.get(key) ?? { resource: g.resource, actions: [] };
    for (const a of g.actions) if (!at.actions.includes(a)) at.actions.push(a);
    out.set(key, at);
  }
  for (const v of out.values()) v.actions.sort();
  return out;
}

/** The policy an answer was read under: the schema's digest and the revision. */
function Policy({ answer }: { answer: Explanation }) {
  return (
    <div className="meta-line">
      <span>schema <span className="mono">{answer.policy.schema_sha256.slice(0, 12)}</span></span>
      <span>revision <span className="mono" data-revision>{answer.policy.revision}</span></span>
      <span>{answer.action} on {resourceText(answer.resource)}</span>
    </div>
  );
}

/** The server's answer as it was given: a verdict with its path or reason, or the server's refusal by name and no verdict. */
function Shown({ asked }: { asked: Asked }) {
  if (asked.state === 'idle') {
    return <span className="note">The server answers: the path to the responsible person, or the named reason it refuses.</span>;
  }
  if (asked.state === 'asking') return <span className="note" data-state="asking">Asking the server…</span>;
  if (asked.state === 'refused') {
    const { refusal, reason } = asked.refused.refusal;
    return (
      <div className="answer" data-state="refusal">
        <span className="tag" data-refusal>{refusal}</span>
        <span className="why">{reason}</span>
      </div>
    );
  }
  const { answer } = asked;
  if (answer.permitted) {
    return (
      <>
        <div className="answer" data-verdict="yes">
          <span className="verdict-mark yes land">Permitted</span>
          <span className="why">{answer.identity} may {answer.action} {resourceText(answer.resource)}{answer.grant ? <> through <span className="mono">{answer.grant}</span></> : null}.</span>
        </div>
        <ol className="why-path" style={{ marginTop: 8 }}>
          {answer.path.map((step) => (
            <li key={step.identity} className="mono" data-responsible={step.responsible ? 'true' : 'false'}>
              {step.identity}
              {step.responsible ? <> <span className="tag">responsible</span></> : null}
            </li>
          ))}
        </ol>
        <Policy answer={answer} />
      </>
    );
  }
  return (
    <>
      <div className="answer" data-verdict="no">
        <span className="verdict-mark no land">Refused</span>
        <span className="tag" data-reason>{answer.reason}</span>
        {answer.grant ? <span className="mono" data-grant>{answer.grant}</span> : null}
      </div>
      <Policy answer={answer} />
    </>
  );
}

/**
 * Why can this identity do a thing, or why not (R7): the person picks an
 * action and a resource, and the screen shows the server's answer from
 * POST /grants/explain. The browser computes, infers and keeps no verdict:
 * nothing is shown until the server answers, and each question asks it again.
 */
export function PermissionWhy() {
  const { id = '' } = useParams();
  const grants = useLoad(() => api.grants(), 'why-grants');
  const options = grants.status === 'ok' ? askable(grants.data) : new Map<string, { resource: ResourceRef; actions: string[] }>();
  const keys = [...options.keys()];
  const [picked, setPicked] = useState('');
  const [pickedAction, setPickedAction] = useState('');
  const [asked, setAsked] = useState<Asked>({ state: 'idle' });
  const res = options.has(picked) ? picked : (keys[0] ?? '');
  const actions = options.get(res)?.actions ?? [];
  const action = actions.includes(pickedAction) ? pickedAction : (actions[0] ?? '');

  const ask = async () => {
    const at = options.get(res);
    if (!at || !action) return;
    setAsked({ state: 'asking' });
    try {
      const answer = await request<Explanation>('/grants/explain', { identity: id, resource: at.resource, action });
      setAsked({ state: 'answered', answer });
    } catch (error) {
      const refused = error instanceof Refused ? error : new Refused(0, { refusal: 'Unanswered', reason: String(error) });
      setAsked({ state: 'refused', refused });
    }
  };

  return (
    <div className="check" id="why">
      <h2>Why can <span className="mono">{id}</span> do this, or not?</h2>
      <div className="q" style={{ marginTop: 10 }}>
        <select id="whyRes" value={res} onChange={(e) => { setPicked(e.target.value); setAsked({ state: 'idle' }); }} aria-label="Resource">
          {keys.map((k) => <option key={k} value={k}>{k}</option>)}
        </select>
        <select id="whyAction" value={action} onChange={(e) => { setPickedAction(e.target.value); setAsked({ state: 'idle' }); }} aria-label="Action">
          {actions.map((a) => <option key={a}>{a}</option>)}
        </select>
        <button className="btn" data-act="why" onClick={ask} disabled={asked.state === 'asking' || !keys.length}>
          Ask the server
        </button>
      </div>
      <div className="answer-box" id="why-answer">
        <Shown asked={asked} />
      </div>
      {grants.status === 'refused' ? <div className="note" style={{ marginTop: 8 }}>The grants could not be read: {grants.refused.refusal.refusal}.</div> : null}
      {grants.status === 'ok' && !keys.length ? <div className="note" style={{ marginTop: 8 }}>No grant you can see is on anything yet, so there is nothing to ask about.</div> : null}
    </div>
  );
}
