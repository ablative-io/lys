/**
 * The test bench beside the builder: example people and agents placed on
 * example resources, and the question "may X do Y to Z" asked of the draft.
 *
 * The draft is opened as a bench of its own at the service, a scratch copy no
 * standing check can see, and asked there; the answer names the path that
 * allowed it or the refusal the real check would give. A changed draft is a
 * new bench, and every bench is closed when the draft moves on and when the
 * builder closes, so nothing of a draft outlives it.
 */
import { useEffect, useRef, useState } from 'react';
import { Refused } from '../../api';
import { send } from './SchemaBuilder';
import type { Draft, SchemaJson } from './SchemaBuilder';

/** An example holding: X holds a relation on a resource. */
interface Holding { subject: string; relation: string; kind: string; id: string }
/** An example placement: a resource in its parent. */
interface Placement { childKind: string; childId: string; parentKind: string; parentId: string }
/** The bench's answer. */
export interface BenchAnswer { allowed: boolean; path: string[]; refusal?: string }

const blankHolding: Holding = { subject: '', relation: '', kind: '', id: '' };
const blankPlacement: Placement = { childKind: '', childId: '', parentKind: '', parentId: '' };

export function SchemaBench({ app, schema, draft }: { app: string; schema: SchemaJson; draft: Draft }) {
  const [holdings, setHoldings] = useState<Holding[]>([{ ...blankHolding }]);
  const [placements, setPlacements] = useState<Placement[]>([]);
  const [question, setQuestion] = useState({ subject: '', action: '', kind: '', id: '' });
  const [answer, setAnswer] = useState<BenchAnswer | null>(null);
  const [refusal, setRefusal] = useState('');
  const bench = useRef<{ id: string; schema: string } | null>(null);
  const drafted = JSON.stringify(schema);
  const close = (id: string) => send('POST', '/apps/bench/' + encodeURIComponent(id) + '/close');
  // The builder is closing: nothing is left on screen to say a failed close
  // on, so it is reported to the console by name rather than dropped.
  useEffect(() => () => {
    if (bench.current) close(bench.current.id).catch((error: unknown) => { console.error('The test bench could not be closed:', error instanceof Refused ? error.refusal.refusal : String(error)); });
  }, []);
  const kinds = draft.kinds.map((kind) => kind.name);
  const relationsOf = (kind: string) => draft.kinds.find((entry) => entry.name === kind)?.relations.map((relation) => relation.name) ?? [];
  const actionsOf = (kind: string) => draft.kinds.find((entry) => entry.name === kind)?.actions ?? [];
  const full = (kind: string) => app + '.' + kind;
  const ask = async () => {
    setRefusal(''); setAnswer(null);
    try {
      if (bench.current && bench.current.schema !== drafted) { const earlier = bench.current.id; bench.current = null; await close(earlier); }
      if (!bench.current) {
        const opened = await send<{ bench: string }>('POST', '/apps/bench', { app, schema });
        bench.current = { id: opened.bench, schema: drafted };
      }
      const asked = await send<BenchAnswer>('POST', '/apps/bench/' + encodeURIComponent(bench.current.id) + '/ask', {
        holdings: holdings.filter((holding) => holding.subject && holding.relation && holding.kind && holding.id).map((holding) => ({ subject: holding.subject, relation: holding.relation, resource: { kind: full(holding.kind), id: holding.id } })),
        placements: placements.filter((placement) => placement.childKind && placement.childId && placement.parentKind && placement.parentId).map((placement) => ({ child: { kind: full(placement.childKind), id: placement.childId }, parent: { kind: full(placement.parentKind), id: placement.parentId } })),
        question: { subject: question.subject, action: question.action, resource: { kind: full(question.kind), id: question.id } },
      });
      setAnswer(asked);
    } catch (error) {
      setRefusal(error instanceof Refused ? error.refusal.refusal + ': ' + error.refusal.reason : String(error));
    }
  };
  const pick = (label: string, value: string, options: string[], set: (value: string) => void) => <select className="sb-name" aria-label={label} value={value} onChange={(event) => set(event.target.value)}><option value="">{label}</option>{options.map((option) => <option key={option} value={option}>{option}</option>)}</select>;
  const typed = (label: string, value: string, set: (value: string) => void) => <input className="sb-name" aria-label={label} placeholder={label} value={value} onChange={(event) => set(event.target.value)} />;
  return <aside className="sb-bench" aria-label="Test bench">
    <h3>Test bench</h3>
    <p className="note">Place example people and agents on example resources, then ask whether one may act. The draft is asked, never the saved schema.</p>
    <div className="sb-examples" aria-label="Example holdings">
      {holdings.map((holding, index) => {
        const set = (edit: Partial<Holding>) => setHoldings(holdings.map((entry, at) => (at === index ? { ...entry, ...edit } : entry)));
        return <div className="sb-row" key={index}>
          {typed('Example ' + (index + 1) + ' person or agent', holding.subject, (subject) => set({ subject }))}
          {pick('Example ' + (index + 1) + ' relation', holding.relation, relationsOf(holding.kind), (relation) => set({ relation }))}
          {pick('Example ' + (index + 1) + ' kind', holding.kind, kinds, (kind) => set({ kind, relation: '' }))}
          {typed('Example ' + (index + 1) + ' resource id', holding.id, (id) => set({ id }))}
        </div>;
      })}
      <button type="button" className="sb-chip" onClick={() => setHoldings([...holdings, { ...blankHolding }])}>Add an example holding</button>
    </div>
    <div className="sb-examples" aria-label="Example placements">
      {placements.map((placement, index) => {
        const set = (edit: Partial<Placement>) => setPlacements(placements.map((entry, at) => (at === index ? { ...entry, ...edit } : entry)));
        return <div className="sb-row" key={index}>
          {pick('Placement ' + (index + 1) + ' child kind', placement.childKind, kinds, (childKind) => set({ childKind }))}
          {typed('Placement ' + (index + 1) + ' child id', placement.childId, (childId) => set({ childId }))}
          <span className="note">is in</span>
          {pick('Placement ' + (index + 1) + ' parent kind', placement.parentKind, kinds, (parentKind) => set({ parentKind }))}
          {typed('Placement ' + (index + 1) + ' parent id', placement.parentId, (parentId) => set({ parentId }))}
        </div>;
      })}
      <button type="button" className="sb-chip" onClick={() => setPlacements([...placements, { ...blankPlacement }])}>Add an example placement</button>
    </div>
    <div className="sb-row sb-question" aria-label="Question">
      <span className="note">May</span>{typed('Who asks', question.subject, (subject) => setQuestion({ ...question, subject }))}
      {pick('Action', question.action, actionsOf(question.kind), (action) => setQuestion({ ...question, action }))}
      {pick('Kind asked about', question.kind, kinds, (kind) => setQuestion({ ...question, kind, action: '' }))}
      {typed('Resource id asked about', question.id, (id) => setQuestion({ ...question, id }))}
      <button type="button" className="btn primary" disabled={!question.subject || !question.action || !question.kind || !question.id} onClick={() => { void ask(); }}>Ask the draft</button>
    </div>
    {answer ? <div className={'sb-answer ' + (answer.allowed ? 'sb-allowed' : 'sb-refused')} role="status">
      <b>{answer.allowed ? 'Allowed' : 'Refused' + (answer.refusal ? ' (' + answer.refusal + ')' : '')}</b>
      <ol>{answer.path.map((step, index) => <li key={index}>{step}</li>)}</ol>
    </div> : null}
    {refusal ? <p role="alert" className="sb-refused">{refusal}</p> : null}
  </aside>;
}
