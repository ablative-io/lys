/** An agent's budgets with where each stands, and the form that sets one. */
import { useRef, useState } from 'react';
import { Refused, request } from '../../api';
import { kept, shown, standing } from './contract';
import type { BudgetsView, BudgetBody, BudgetAct, Length, Measure, Receipt } from './contract';
import { PERIODS, confirmsBudget, validAmount } from './budgetForm';
import { ACT_WORDS, PERIOD_WORDS, UNITS, limitWords, summary, usedWords } from './budgetWords';

type Props = { agent: string; budgets: BudgetsView; receipts: Receipt[]; changed: (words: string) => void };

export function UsageBudgets({ agent, budgets, receipts, changed }: Props) {
  const [busy, setBusy] = useState(false);
  const [failure, setFailure] = useState('');
  const [done, setDone] = useState(false);
  const [uncertain, setUncertain] = useState(false);
  const working = useRef(false);
  const locked = useRef(false);
  const save = async (body: BudgetBody) => {
    if (working.current || locked.current) return;
    working.current = true; setBusy(true); setFailure('');
    try {
      const answer = await request<unknown>('/budgets/agent/' + encodeURIComponent(agent), body, 'PUT');
      if (!confirmsBudget(answer, body, budgets.holder)) throw new Error('BudgetAnswerUnconfirmed: the answer did not confirm every limit and its version. Reload the saved budget before another change.');
      locked.current = true; setDone(true); changed('Budget kept as version ' + (body.version + 1) + '.');
    } catch (error) {
      if (!(error instanceof Refused && error.status >= 400 && error.status < 500)) {
        locked.current = true; setUncertain(true);
      }
      setFailure(error instanceof Refused ? error.refusal.refusal + ': ' + error.message : String(error));
    } finally { working.current = false; setBusy(false); }
  };
  const blocked = busy || done || uncertain;
  return <section className="card usage-budgets" aria-label="Budgets"><h3>Budget</h3>
    {budgets.limits.length ? <table className="usage-list"><thead><tr><th>Spend at most</th><th>Used</th><th>When it's hit</th><th>Where it stands</th></tr></thead>
      <tbody>{budgets.limits.map((limit, index) => {
        const stands = standing(budgets, index, receipts);
        return <tr key={index} data-reached={stands.reached}><td>{limitWords(limit)}</td><td className="usage-used">{usedWords(limit, budgets.used[index]) ?? 'Nothing reported yet'}</td>
          <td>{ACT_WORDS[limit.act]}</td><td>{stands.words}</td></tr>;
      })}</tbody></table> : <p>No budget is set for this agent.</p>}
    <p className="usage-summary">{summary(budgets.limits, budgets.warn_at)}</p>
    {budgets.limits.length ? <EditBudgets budgets={budgets} busy={blocked} save={save} /> : null}
    <SetBudget budgets={budgets} busy={blocked} save={save} />
    {budgets.unavailable.map((each) => <p key={each.unit}>{UNITS[each.unit]}: {each.reason}</p>)}
    {busy ? <p role="status">Sending; not yet confirmed.</p> : null}
    {failure ? <p role="alert" className="why-not">{failure}</p> : null}
    {uncertain ? <p role="status">The saved budget must be reloaded before another change; this answer is unconfirmed.</p> : null}
  </section>;
}

type FormProps = { budgets: BudgetsView; busy: boolean; save: (body: BudgetBody) => Promise<void> };

function EditBudgets({ budgets, busy, save }: FormProps) {
  const [rows, setRows] = useState(() => budgets.limits.map((limit) => ({ amount: String(shown(limit.unit, limit.amount)), act: limit.act })));
  const [warning, setWarning] = useState(budgets.warn_at === null ? '' : String(budgets.warn_at));
  const valid = rows.every((row, index) => validAmount(budgets.limits[index].unit, row.amount))
    && (warning === '' || validAmount('context_percent', warning));
  const edit = (index: number, next: Partial<(typeof rows)[number]>) => setRows((all) => all.map((row, at) => at === index ? { ...row, ...next } : row));
  const submit = () => {
    if (busy || !valid) return;
    const limits = budgets.limits.map((limit, index) => ({ ...limit,
      amount: rows[index].amount === String(shown(limit.unit, limit.amount)) ? limit.amount : kept(limit.unit, Number(rows[index].amount)), act: rows[index].act }));
    void save({ limits, warn_at: warning === '' ? null : Number(warning), version: budgets.version });
  };
  return <form aria-label="Edit budget limits" onSubmit={(event) => { event.preventDefault(); submit(); }}><h4>Change the limits</h4>
    {budgets.limits.map((limit, index) => <fieldset key={index} disabled={busy}><legend>{limitWords(limit)}</legend>
      <label className="field">Spend at most, in {UNITS[limit.unit]}{limit.period ? ' per ' + PERIOD_WORDS[limit.period] : ''}
        <input name={'amount-' + index} type="number" min={0} max={limit.unit.includes('percent') ? 100 : undefined} step="any" required value={rows[index].amount} onChange={(event) => edit(index, { amount: event.target.value })} /></label>
      <label className="field">When it's hit<select name={'act-' + index} value={rows[index].act} onChange={(event) => edit(index, { act: event.target.value as BudgetAct })}>{Object.entries(ACT_WORDS).map(([value, words]) => <option key={value} value={value}>{words}</option>)}</select></label>
    </fieldset>)}
    <label className="field">Warn me at this % of a limit (leave blank for no warning)<input name="warn_at" type="number" min={0} max={100} step="any" value={warning} disabled={busy} onChange={(event) => setWarning(event.target.value)} /></label>
    <button className="btn primary" type="submit" disabled={busy || !valid}>Save these limits</button>
  </form>;
}

function SetBudget({ budgets, busy, save }: FormProps) {
  const [measure, setMeasure] = useState<Measure>('tokens');
  const [limit, setLimit] = useState('');
  const [length, setLength] = useState<Length>('day');
  const [act, setAct] = useState<BudgetAct>('tell');
  const valid = validAmount(measure, limit) && budgets.limits.length < 64
    && !budgets.unavailable.some((each) => each.unit === measure);
  const submit = () => {
    if (busy || !valid) return;
    const next = { unit: measure, amount: kept(measure, Number(limit)), period: measure === 'context_percent' ? null : length, act };
    const body: BudgetBody = { limits: [...budgets.limits, next], warn_at: budgets.warn_at, version: budgets.version };
    void save(body);
  };
  return <form aria-label="Set a budget" onSubmit={(event) => { event.preventDefault(); if (limit !== '') void submit(); }}><h4>Add a limit</h4>
    <label className="field">Spend at most<input name="limit" type="number" min={0} step="any" required value={limit} disabled={busy} onChange={(event) => setLimit(event.target.value)} /></label>
    <label className="field">In<select aria-label="Unit" value={measure} disabled={busy} onChange={(event) => {
      const next = event.target.value as Measure; setMeasure(next);
      if (next !== 'context_percent' && !PERIODS[next].includes(length)) setLength(PERIODS[next][0]);
    }}>{Object.entries(UNITS).map(([value, words]) => <option key={value} value={value} disabled={budgets.unavailable.some((each) => each.unit === value)}>{words}</option>)}</select></label>
    {measure === 'context_percent' ? <p>at any moment</p> : <label className="field">Per<select aria-label="Period" value={length} disabled={busy} onChange={(event) => setLength(event.target.value as Length)}>{PERIODS[measure].map((period) => <option key={period} value={period}>{PERIOD_WORDS[period]}</option>)}</select></label>}
    <label className="field">When it's hit<select aria-label="When it's hit" value={act} disabled={busy} onChange={(event) => setAct(event.target.value as BudgetAct)}>{Object.entries(ACT_WORDS).map(([value, words]) => <option key={value} value={value}>{words}</option>)}</select></label>
    <button className="btn primary" type="submit" disabled={busy || !valid}>Add this limit</button>
    {budgets.limits.length >= 64 ? <p>A budget can hold at most 64 limits.</p> : null}
  </form>;
}
