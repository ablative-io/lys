/** An agent's or a team's budgets with where each stands, and the form that sets one. */
import { useRef, useState } from 'react';
import { Refused, request } from '../../api';
import { ACTS, kept, shown, standing } from './contract';
import type { BudgetsView, BudgetBody, BudgetAct, Length, Measure, Receipt } from './contract';
import { PERIODS, confirmsBudget, validAmount } from './budgetForm';
import { PERIOD_WORDS, UNITS, limitWords, summary, usedWords } from './budgetWords';

type Props = { budgets: BudgetsView; receipts: Receipt[]; changed: (words: string) => void };

export function UsageBudgets({ budgets, receipts, changed }: Props) {
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
      const answer = await request<unknown>('/budgets/' + budgets.holder.kind + '/' + encodeURIComponent(budgets.holder.id), body, 'PUT');
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
    <EditBudgets budgets={budgets} receipts={receipts} busy={blocked} save={save} />
    <SetBudget budgets={budgets} busy={blocked} save={save} />
    <p className="usage-summary">{summary(budgets.limits, budgets.warn_at)}</p>
    {budgets.unavailable.map((each) => <p key={each.unit}>{UNITS[each.unit]}: {each.reason}</p>)}
    {busy ? <p role="status">Sending; not yet confirmed.</p> : null}
    {failure ? <p role="alert" className="why-not">{failure}</p> : null}
    {uncertain ? <p role="status">The saved budget must be reloaded before another change; this answer is unconfirmed.</p> : null}
  </section>;
}

const COLUMNS = ['34%', '20%', '18%', '18%', '10%'];

type FormProps = { budgets: BudgetsView; busy: boolean; save: (body: BudgetBody) => Promise<void> };

/** The limits as one table: each row's amount and act are changed where they stand, and one button saves them all. */
function EditBudgets({ budgets, receipts, busy, save }: FormProps & { receipts: Receipt[] }) {
  const [rows, setRows] = useState(() => budgets.limits.map((limit) => ({ amount: String(shown(limit.unit, limit.amount)), act: limit.act })));
  const [warning, setWarning] = useState(budgets.warn_at === null ? '' : String(budgets.warn_at));
  const valid = rows.every((row, index) => validAmount(budgets.limits[index].unit, row.amount))
    && (warning === '' || validAmount('context_percent', warning));
  const edit = (index: number, next: Partial<(typeof rows)[number]>) => setRows((all) => all.map((row, at) => at === index ? { ...row, ...next } : row));
  const remove = (index: number) => {
    if (busy) return;
    void save({ limits: budgets.limits.slice(0, index).concat(budgets.limits.slice(index + 1)), warn_at: budgets.warn_at, version: budgets.version });
  };
  const submit = () => {
    if (busy || !valid) return;
    const limits = budgets.limits.map((limit, index) => ({ ...limit,
      amount: rows[index].amount === String(shown(limit.unit, limit.amount)) ? limit.amount : kept(limit.unit, Number(rows[index].amount)), act: rows[index].act }));
    void save({ limits, warn_at: warning === '' ? null : Number(warning), version: budgets.version });
  };
  return <form aria-label="Edit budget limits" className="usage-edit" onSubmit={(event) => { event.preventDefault(); submit(); }}>
    <table className="usage-list usage-table">
      <colgroup>{COLUMNS.map((width, index) => <col key={index} style={{ width }} />)}</colgroup>
      <thead><tr><th>Spend at most</th><th>Used</th><th>When it's hit</th><th>Where it stands</th><th>Change</th></tr></thead>
      <tbody>
        {budgets.limits.map((limit, index) => {
          const stands = standing(budgets, index, receipts);
          return <tr key={index} data-reached={stands.reached}>
            <td><div className="usage-amount">
              <input name={'amount-' + index} aria-label={'Amount of ' + limitWords(limit)} type="number" min={0} max={limit.unit.includes('percent') ? 100 : undefined} step="any" required disabled={busy} value={rows[index].amount} onChange={(event) => edit(index, { amount: event.target.value })} />
              <span>{UNITS[limit.unit]}{limit.period ? ' a ' + PERIOD_WORDS[limit.period] : ', at any moment'}</span>
            </div></td>
            <td className="usage-used">{usedWords(limit, budgets.used[index]) ?? 'Nothing reported yet'}</td>
            <td><select name={'act-' + index} aria-label={'When ' + limitWords(limit) + ' is hit'} disabled={busy} value={rows[index].act} onChange={(event) => edit(index, { act: event.target.value as BudgetAct })}>{Object.entries(ACTS).map(([value, words]) => <option key={value} value={value}>{words}</option>)}</select></td>
            <td>{stands.words}</td>
            <td><button className="btn" type="button" disabled={busy} aria-label={'Remove ' + limitWords(limit)} onClick={() => remove(index)}>Remove</button></td>
          </tr>;
        })}
        {budgets.limits.length ? null : <tr><td colSpan={5} className="dim">No budget is set for this {budgets.holder.kind}.</td></tr>}
      </tbody>
      {budgets.limits.length ? <tfoot><tr>
        <td colSpan={3}><div className="usage-amount"><span id="warn-at-words">Warn me at this % of a limit (leave blank for no warning)</span>
          <input name="warn_at" aria-labelledby="warn-at-words" type="number" min={0} max={100} step="any" value={warning} disabled={busy} onChange={(event) => setWarning(event.target.value)} /></div></td>
        <td colSpan={2}><button className="btn primary" type="submit" disabled={busy || !valid}>Save these limits</button></td>
      </tr></tfoot> : null}
    </table>
  </form>;
}

/** The row under the table that adds a limit, in the table's own columns. */
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
  return <form aria-label="Set a budget" className="usage-add-row" style={{ gridTemplateColumns: COLUMNS.join(' ') }} onSubmit={(event) => { event.preventDefault(); if (limit !== '') void submit(); }}>
    <div className="usage-amount">
      <input name="limit" aria-label="Spend at most" type="number" min={0} step="any" required value={limit} disabled={busy} placeholder="Amount" onChange={(event) => setLimit(event.target.value)} />
      <select aria-label="Unit" value={measure} disabled={busy} onChange={(event) => {
        const next = event.target.value as Measure; setMeasure(next);
        if (next !== 'context_percent' && !PERIODS[next].includes(length)) setLength(PERIODS[next][0]);
      }}>{Object.entries(UNITS).map(([value, words]) => <option key={value} value={value} disabled={budgets.unavailable.some((each) => each.unit === value)}>{words}</option>)}</select>
      {measure === 'context_percent' ? <span>at any moment</span> : <select aria-label="Period" value={length} disabled={busy} onChange={(event) => setLength(event.target.value as Length)}>{PERIODS[measure].map((period) => <option key={period} value={period}>{'a ' + PERIOD_WORDS[period]}</option>)}</select>}
    </div>
    <span className="dim">New limit</span>
    <select aria-label="When it's hit" value={act} disabled={busy} onChange={(event) => setAct(event.target.value as BudgetAct)}>{Object.entries(ACTS).map(([value, words]) => <option key={value} value={value}>{words}</option>)}</select>
    <span />
    <span><button className="btn primary" type="submit" disabled={busy || !valid}>Add this limit</button></span>
    {budgets.limits.length >= 64 ? <p>A budget can hold at most 64 limits.</p> : null}
  </form>;
}
