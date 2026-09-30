/** An agent's budget: limits a person sets in their own terms, what is used against each, and what happens at the first one reached. */
import { useState } from 'react';
import { Refused, request } from '../../api';
import { ACT_WORDS, PERIODS, PERIOD_WORDS, UNITS, summary, usedWords } from './limits';
import type { Limit, LimitAct, LimitPeriod, LimitsBudget, LimitsView, Unit, Used } from './limits';

type Props = { agent: string; budget: LimitsView; used: Used[]; changed: (words: string) => void };

/** A limit as the form holds it: running time is typed in minutes. */
type Row = { unit: Unit; amount: string; period: LimitPeriod | null };

const toRow = (limit: Limit): Row => ({ unit: limit.unit, period: limit.period, amount: String(limit.unit === 'running_ms' ? limit.amount / 60000 : limit.amount) });
const toLimit = (row: Row): Limit => ({ unit: row.unit, period: row.period, amount: row.unit === 'running_ms' ? Math.round(Number(row.amount) * 60000) : Number(row.amount) });
const firstPeriod = (unit: Unit): LimitPeriod | null => PERIODS[unit][0] ?? null;

export function UsageBudgets({ agent, budget, used, changed }: Props) {
  const offered = (Object.keys(UNITS) as Unit[]).filter((unit) => !budget.unavailable.some((each) => each.unit === unit));
  const [rows, setRows] = useState<Row[]>(budget.limits.map(toRow));
  const [warnAt, setWarnAt] = useState(budget.warn_at === null ? '' : String(budget.warn_at));
  const [act, setAct] = useState<LimitAct>(budget.act);
  const [busy, setBusy] = useState(false);
  const [failure, setFailure] = useState('');
  const complete = rows.every((row) => row.amount !== '' && Number(row.amount) > 0);
  const limits = complete ? rows.map(toLimit) : budget.limits;
  const warn = warnAt === '' ? null : Number(warnAt);
  const edit = (index: number, change: Partial<Row>) => setRows((all) => all.map((row, at) => at === index ? { ...row, ...change } : row));
  const save = async () => {
    setBusy(true); setFailure('');
    try {
      const answer = await request<LimitsBudget>('/budgets/agent/' + encodeURIComponent(agent), { limits: rows.map(toLimit), warn_at: warn, act, version: budget.version }, 'PUT');
      changed('Budget saved as version ' + answer.version + '.');
    } catch (error) {
      setFailure(error instanceof Refused ? error.refusal.refusal + ': ' + error.message : String(error));
    } finally { setBusy(false); }
  };
  return <section className="card usage-budgets" aria-label="Budgets"><h3>Budget</h3>
    <form aria-label="Budget" onSubmit={(event) => { event.preventDefault(); if (complete) void save(); }}>
      <ul className="usage-lines">{rows.map((row, index) => <li key={index} className="usage-limit">
        <span>Spend at most</span>
        <input name="amount" aria-label="Amount" type="number" min={0} step="any" required value={row.amount} disabled={busy} onChange={(event) => edit(index, { amount: event.target.value })} />
        <select aria-label="Unit" value={row.unit} disabled={busy} onChange={(event) => { const unit = event.target.value as Unit; edit(index, { unit, period: firstPeriod(unit) }); }}>
          {offered.map((unit) => <option key={unit} value={unit}>{UNITS[unit]}</option>)}
        </select>
        {row.period === null ? <span>at any moment</span> : <><span>per</span><select aria-label="Period" value={row.period} disabled={busy} onChange={(event) => edit(index, { period: event.target.value as LimitPeriod })}>
          {PERIODS[row.unit].map((period) => <option key={period} value={period}>{PERIOD_WORDS[period]}</option>)}
        </select></>}
        {index < budget.limits.length ? <span className="usage-used">{usedWords(budget.limits[index], used) ?? 'Nothing used yet'}</span> : null}
        <button className="btn" type="button" aria-label="Remove this limit" disabled={busy} onClick={() => setRows((all) => all.slice(0, index).concat(all.slice(index + 1)))}>Remove</button>
      </li>)}</ul>
      <button className="btn" type="button" disabled={busy || !offered.length} onClick={() => setRows((all) => [...all, { unit: offered[0], amount: '', period: firstPeriod(offered[0]) }])}>{rows.length ? '+ Add another limit' : '+ Add a limit'}</button>
      <p className="usage-summary">{summary(limits, act, warn)}</p>
      <div className="usage-row">
        <label>Warn me at <input name="warn_at" aria-label="Warn me at" type="number" min={1} max={99} value={warnAt} disabled={busy} onChange={(event) => setWarnAt(event.target.value)} />%</label>
        <label>When a limit is hit <select aria-label="When a limit is hit" value={act} disabled={busy} onChange={(event) => setAct(event.target.value as LimitAct)}>
          {(Object.keys(ACT_WORDS) as LimitAct[]).map((each) => <option key={each} value={each}>{ACT_WORDS[each]}</option>)}
        </select></label>
        <button className="btn primary" type="submit" disabled={busy || !complete}>Save budget</button>
      </div>
      {budget.unavailable.map((each) => <p key={each.unit} className="usage-note">{UNITS[each.unit]}: {each.reason}</p>)}
      {busy ? <p role="status">Saving; not yet confirmed.</p> : null}
      {failure ? <p role="alert" className="why-not">{failure}</p> : null}
    </form>
  </section>;
}
