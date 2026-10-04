import { useState } from 'react';
import { resourceText } from '../../generated/grants';
import type { ResourceRef } from '../../generated/grants';
import { calledBy } from '../people/directory';
import { ErrorWords } from '../people/Words';
import { actionSentence } from './action-words';
import { AnswerView } from './Answer';
import { ask } from './check';
import type { Answer } from './check';
import { nameOf, resourceLabel } from './model';
import type { GrantWorld } from './model';
import './grants.css';

/** Every resource a grant the caller can see is on, and the actions granted on it. */
export function resourcesSeen(w: GrantWorld): Map<string, { resource: ResourceRef; actions: string[] }> {
  const out = new Map<string, { resource: ResourceRef; actions: string[] }>();
  for (const g of w.list.grants) {
    const key = resourceText(g.resource);
    const at = out.get(key) ?? { resource: g.resource, actions: [] };
    for (const a of g.actions) if (!at.actions.includes(a)) at.actions.push(a);
    out.set(key, at);
  }
  for (const v of out.values()) v.actions.sort();
  return out;
}

/** Everyone the caller may see, as a person picks them: by name, an agent with whom it answers to, never a raw id. Yourself first. */
export function whoChoices(w: GrantWorld): { id: string; name: string }[] {
  const out = [...w.who].map(([id, x]) => ({
    id, name: x.kind === 'agent' && x.responsible ? `${x.name} (agent of ${nameOf(w, x.responsible)})` : x.kind === 'service_account' ? `${x.name} (service)` : x.name,
  }));
  out.sort((a, b) => Number(b.id === w.me.person.id) - Number(a.id === w.me.person.id) || a.name.localeCompare(b.name));
  return out;
}

/** The "who" of a question, part of its sentence: a choice of names whose value is the identity asked about. */
export function WhoChoice({ w, value, change, id }: { w: GrantWorld; value: string; change: (id: string) => void; id: string }) {
  return <label className="ask-field"><span className="note">Who</span><select id={id} value={value} onChange={(event) => change(event.target.value)}>
    {whoChoices(w).map((each) => <option key={each.id} value={each.id}>{each.name}</option>)}
  </select></label>;
}

/**
 * "Can X do this?" (conformance 8.1), one sentence on one line:
 * Can [who] [do what] [on what]? [Check]. With `who` fixed it is the file's box
 * and the who is the agent's name; without, the Access screen's, and who is the first field.
 */
export function CheckBox({ w, who }: { w: GrantWorld; who?: string }) {
  const resources = resourcesSeen(w);
  const keys = [...resources.keys()];
  const [res, setRes] = useState(keys[0] ?? '');
  const picked = resources.get(res);
  const actions = picked?.actions ?? [];
  const [action, setAction] = useState(actions[0] ?? '');
  const [chosen, setChosen] = useState(who ?? w.me.person.id);
  const [answer, setAnswer] = useState<Answer | null>(null);
  const [asking, setAsking] = useState(false);
  const [refused, setRefused] = useState<unknown>(null);
  const subject = who ?? chosen;

  const check = async () => {
    const at = resources.get(res);
    if (!at || !action) return;
    setAsking(true); setRefused(null);
    try {
      setAnswer(await ask(w.me.person.id, subject, nameOf(w, subject), at.resource, action));
    } catch (error) {
      // A read that was refused answers nothing: the answer to the question before is taken down, never left standing.
      setAnswer(null); setRefused(error);
    } finally {
      setAsking(false);
    }
  };
  const pickRes = (key: string) => {
    setRes(key);
    setAction(resources.get(key)?.actions[0] ?? '');
  };

  const shown = refused ? <ErrorWords problem={refused} /> : answer && answer.who === subject ? <AnswerView w={w} a={answer} land /> : null;
  const question = (
    <div className="q ask-line" role="group" aria-label="Question">
      <span className="sec">Can</span>
      {who ? <span className="ask-field"><span className="note">Who</span><strong>{calledBy(who, nameOf(w, who))}</strong></span> : <WhoChoice w={w} id="cWho" value={chosen} change={(id) => { setChosen(id); setAnswer(null); setRefused(null); }} />}
      <label className="ask-field"><span className="note">Do what</span><select id="cPerm" value={action} onChange={(e) => setAction(e.target.value)}>
        {picked ? actions.map((a) => <option key={a} value={a}>{actionSentence(w.model, picked.resource, a)}</option>) : null}
      </select></label>
      <label className="ask-field"><span className="note">On what</span><select id="cRes" value={res} onChange={(e) => pickRes(e.target.value)}>
        {[...resources].map(([k, v]) => <option key={k} value={k}>{resourceLabel(v.resource, w)}</option>)}
      </select></label>
      <span className="sec">?</span>
      <button className="btn" data-act="check" onClick={check} disabled={asking || !keys.length}>
        Check <span className="kbd">c</span>
      </button>
    </div>
  );

  if (who) {
    return (
      <div className="check">
        {question}
        <div className="answer-box" id="answer">
          {shown ?? <span className="note">The answer shows the path to a person, or the reason it is refused, and which version of the model it used.</span>}
        </div>
        {keys.length ? null : <div className="note" style={{ marginTop: 8 }}>No grant you can see is on anything yet, so there is nothing to ask about.</div>}
      </div>
    );
  }
  return (
    <>
      {question}
      <div className="answer-box" id="answer">{shown}</div>
    </>
  );
}
