/** An agent's budgets with where each stands, and the form that sets one. */
import { useState } from 'react';
import { Refused, request } from '../../api';
import { ACTS, MEASURES, kept, shown, standing } from './contract';
import type { BudgetsView, BudgetBody, BudgetAct, Measure, Receipt } from './contract';

type Props = { agent: string; budgets: BudgetsView; receipts: Receipt[]; changed: (words: string) => void };

export function UsageBudgets({ agent, budgets, receipts, changed }: Props) {
  return <section className="card usage-budgets" aria-label="Budgets"><h3>Budgets</h3>
    {budgets.limits.length ? <table className="usage-list"><thead><tr><th>Measure</th><th>Limit</th><th>Period</th><th>When reached</th><th>Where it stands</th></tr></thead>
      <tbody>{budgets.limits.map((limit, index) => {
        const stands = standing(budgets, index, receipts);
        return <tr key={index} data-reached={stands.reached}><td>{MEASURES[limit.unit]}</td><td>{shown(limit.unit, limit.amount)}</td>
          <td>{limit.period ? 'Each ' + limit.period + ', ' + (limit.zone ?? budgets.zone) : 'None'}</td><td>{ACTS[limit.act]}</td><td>{stands.words}</td></tr>;
      })}</tbody></table> : <p>No budget is set for this agent.</p>}
    <SetBudget agent={agent} budgets={budgets} changed={changed} />
  </section>;
}

function SetBudget({ agent, budgets, changed }: { agent: string; budgets: BudgetsView; changed: (words: string) => void }) {
  const [measure, setMeasure] = useState<Measure>('tokens');
  const [limit, setLimit] = useState('');
  const [length, setLength] = useState<'day' | 'week'>('day');
  const [act, setAct] = useState<BudgetAct>('tell');
  const [busy, setBusy] = useState(false);
  const [failure, setFailure] = useState('');
  const submit = async () => {
    const next = { unit: measure, amount: kept(measure, Number(limit)), period: measure === 'context_percent' ? null : length, act };
    const body: BudgetBody = { limits: [...budgets.limits, next], warn_at: budgets.warn_at, version: budgets.version };
    setBusy(true); setFailure('');
    try {
      const answer = await request<BudgetsView>('/budgets/agent/' + encodeURIComponent(agent), body, 'PUT');
      changed('Budget kept as version ' + answer.version + '.');
    } catch (error) {
      setFailure(error instanceof Refused ? error.refusal.refusal + ': ' + error.message : String(error));
    } finally { setBusy(false); }
  };
  return <form aria-label="Set a budget" onSubmit={(event) => { event.preventDefault(); if (limit !== '') void submit(); }}><h4>Set a budget</h4>
    <label className="field">Measure<select value={measure} disabled={busy} onChange={(event) => setMeasure(event.target.value as Measure)}>{Object.entries(MEASURES).map(([value, words]) => <option key={value} value={value}>{words}</option>)}</select></label>
    <label className="field">Limit<input name="limit" type="number" min={0} step="any" required value={limit} disabled={busy} onChange={(event) => setLimit(event.target.value)} /></label>
    {measure === 'context_percent' ? null : <><label className="field">Counted each<select value={length} disabled={busy} onChange={(event) => setLength(event.target.value as 'day' | 'week')}><option value="day">day</option><option value="week">week</option></select></label>
      <label className="field">In the time zone<input name="zone" value={budgets.zone} readOnly /></label></>}
    <label className="field">When reached<select value={act} disabled={busy} onChange={(event) => setAct(event.target.value as BudgetAct)}>{Object.entries(ACTS).map(([value, words]) => <option key={value} value={value}>{words}</option>)}</select></label>
    <button className="btn primary" type="submit" disabled={busy || limit === ''}>Set budget</button>
    {busy ? <p role="status">Sending; not yet confirmed.</p> : null}
    {failure ? <p role="alert" className="why-not">{failure}</p> : null}
  </form>;
}
