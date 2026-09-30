import { useState } from 'react';
import { resourceText } from '../../generated/grants';
import type { ResourceRef } from '../../generated/grants';
import { firstName } from '../people/directory';
import { Picker } from '../../shell/Picker';
import { AnswerView } from './Answer';
import { ask } from './check';
import type { Answer } from './check';
import { nameOf, resourceLabel } from './model';
import type { GrantWorld } from './model';

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

/**
 * "Can X do this?" (conformance 8.1). With `who` fixed it is the file's box;
 * without, the Access screen's, which asks about anyone.
 */
export function CheckBox({ w, who }: { w: GrantWorld; who?: string }) {
  const resources = resourcesSeen(w);
  const keys = [...resources.keys()];
  const [res, setRes] = useState(keys[0] ?? '');
  const actions = resources.get(res)?.actions ?? [];
  const [action, setAction] = useState(actions[0] ?? '');
  const people = [...w.who.entries()];
  const [chosen, setChosen] = useState(who ?? w.me.person.id);
  const [answer, setAnswer] = useState<Answer | null>(null);
  const [asking, setAsking] = useState(false);
  const subject = who ?? chosen;

  const check = async () => {
    const at = resources.get(res);
    if (!at || !action) return;
    setAsking(true);
    try {
      setAnswer(await ask(w.me.person.id, subject, nameOf(w, subject), at.resource, action));
    } finally {
      setAsking(false);
    }
  };
  const pickRes = (key: string) => {
    setRes(key);
    setAction(resources.get(key)?.actions[0] ?? '');
  };

  const resSelect = (
    <select id="cRes" value={res} onChange={(e) => pickRes(e.target.value)} aria-label="Resource">
      {[...resources].map(([k, v]) => <option key={k} value={k}>{resourceLabel(v.resource)}</option>)}
    </select>
  );
  const permSelect = (
    <select id="cPerm" value={action} onChange={(e) => setAction(e.target.value)} aria-label="Action">
      {actions.map((a) => <option key={a}>{a}</option>)}
    </select>
  );
  const button = (
    <button className="btn" data-act="check" onClick={check} disabled={asking || !keys.length}>
      Check <span className="kbd">c</span>
    </button>
  );
  const shown = answer && answer.who === subject ? <AnswerView w={w} a={answer} land /> : null;

  if (who) {
    return (
      <div className="check">
        <h2>Can {firstName(nameOf(w, who))} do this?</h2>
        <div className="q" style={{ marginTop: 10 }}>{resSelect}{permSelect}{button}</div>
        <div className="answer-box" id="answer">
          {shown ?? <span className="note">The answer shows the path to a person, or the reason it is refused, and which version of the model it used.</span>}
        </div>
        {keys.length ? null : <div className="note" style={{ marginTop: 8 }}>No grant you can see is on anything yet, so there is nothing to ask about.</div>}
      </div>
    );
  }
  return (
    <>
      <div className="q">
        <span className="sec">Can <strong>{nameOf(w, chosen)}</strong></span>
        {permSelect}
        {resSelect}
        {button}
      </div>
      <Picker key={chosen} name="who" label="Ask about someone else" options={people.map(([id, x]) => ({ id, name: x.name }))} onChange={(ids) => { if (ids[0]) { setChosen(ids[0]); setAnswer(null); } }} />
      <div className="answer-box" id="answer">{shown}</div>
    </>
  );
}
