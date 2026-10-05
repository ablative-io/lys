/** An agent's or a team's budgets with where each stands, and the form that sets one. */
import { useRef, useState } from 'react';
import type { ReactNode } from 'react';
import { Refused, request } from '../../api';
import { ACTS, kept, shown, standing } from './contract';
import type { BudgetsView, BudgetBody, BudgetAct, Length, Measure, Receipt } from './contract';
import { PERIODS, confirmsBudget, validAmount } from './budgetForm';
import { PERIOD_WORDS, UNITS, limitWords, usedAgainst } from './budgetWords';
import { answeredNo } from '../../kept';
import { plain, reasons } from './reasons';
import type { Named } from './reasons';
import { Act } from '../../shell/Act';

/** `name` is the holder's name, said wherever the service's words carry its identifier. */
type Props = { budgets: BudgetsView; receipts: Receipt[]; changed: (words: string) => void; name?: string };

export function UsageBudgets({ budgets, receipts, changed, name }: Props) {
  const holder: Named = { id: budgets.holder.id, name: name ?? 'this ' + budgets.holder.kind };
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
      if (!answeredNo(error)) {
        locked.current = true; setUncertain(true);
      }
      setFailure(error instanceof Refused ? error.refusal.refusal + ': ' + error.message : String(error));
    } finally { working.current = false; setBusy(false); }
  };
  const blocked = busy || done || uncertain;
  const missing = budgets.used.flatMap((used) => (used && used.figure === null ? plain(used.unavailable, holder) : []));
  const add = useSetBudget({ budgets, busy: blocked, save });
  return <section className="card usage-budgets" aria-label="Budgets"><h3>Budget</h3>
    <EditBudgets budgets={budgets} receipts={receipts} busy={blocked} save={save} holder={holder} add={add.row} />
    {add.form}
    {reasons(budgets.unavailable, holder, missing).map((words) => <p key={words} className="usage-reason">{words}</p>)}
    {busy ? <p role="status">Sending; not yet confirmed.</p> : null}
    {failure ? <p role="alert" className="why-not">{failure}</p> : null}
    {uncertain ? <p role="status">The saved budget must be reloaded before another change; this answer is unconfirmed.</p> : null}
  </section>;
}

const COLUMNS = ['38%', '17%', '18%', '15%', '12%'];

type FormProps = { budgets: BudgetsView; busy: boolean; save: (body: BudgetBody) => Promise<void> };

/** The limits as one table: each row's amount and act are changed where they stand, one button saves them all, and the last row adds one. */
function EditBudgets({ budgets, receipts, busy, save, holder, add }: FormProps & { receipts: Receipt[]; holder: Named; add: ReactNode }) {
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
      {/* The rows say each limit and its act; that the first reached acts first is said nowhere else, so once, here. */}
      {budgets.limits.length > 1 ? <caption className="usage-caption">Each limit acts on its own: whichever is reached first acts first.</caption> : null}
      <colgroup>{COLUMNS.map((width, index) => <col key={index} style={{ width }} />)}</colgroup>
      <thead><tr><th>Spend at most</th><th>Used</th><th>When it's hit</th><th>Where it stands</th><th>Change</th></tr></thead>
      <tbody>
        {budgets.limits.map((limit, index) => {
          const stands = standing(budgets, index, receipts);
          const used = budgets.used[index];
          const words = used && used.figure === null ? plain(used.unavailable, holder).join(' ') : stands.words;
          return <tr key={index} data-reached={stands.reached}>
            <td><div className="usage-amount">
              <input name={'amount-' + index} aria-label={'Amount of ' + limitWords(limit)} type="number" min={0} max={limit.unit.includes('percent') ? 100 : undefined} step="any" required disabled={busy} value={rows[index].amount} onChange={(event) => edit(index, { amount: event.target.value })} />
              <span>{UNITS[limit.unit]}{limit.period ? ' a ' + PERIOD_WORDS[limit.period] : ', at any moment'}</span>
            </div></td>
            <td className="usage-used">{usedAgainst(budgets, index) ?? 'Nothing reported yet'}</td>
            <td><select name={'act-' + index} aria-label={'When ' + limitWords(limit) + ' is hit'} disabled={busy} value={rows[index].act} onChange={(event) => edit(index, { act: event.target.value as BudgetAct })}>{Object.entries(ACTS).map(([value, words]) => <option key={value} value={value}>{words}</option>)}</select></td>
            <td>{words}</td>
            <td><Act symbol="remove" name={'Remove ' + limitWords(limit)} disabled={busy} onClick={() => remove(index)} /></td>
          </tr>;
        })}
        {budgets.limits.length ? null : <tr><td colSpan={5} className="dim">No budget is set for this {budgets.holder.kind}.</td></tr>}
      </tbody>
      <tfoot>{budgets.limits.length ? <tr>
        <td colSpan={3}><div className="usage-amount"><span id="warn-at-words">Warn me at this % of a limit (leave blank for no warning)</span>
          <input name="warn_at" aria-labelledby="warn-at-words" type="number" min={0} max={100} step="any" value={warning} disabled={busy} onChange={(event) => setWarning(event.target.value)} /></div></td>
        <td colSpan={2}><Act symbol="save" name="Save these limits" word="Save" tone="primary" type="submit" disabled={busy || !valid} /></td>
      </tr> : null}{add}</tfoot>
    </table>
  </form>;
}

/** The table's last row, which adds a limit in the table's own columns: Spend at most, Used (empty), When it's hit, Where it stands (empty), Change.
 * A form cannot hold a table row, so the row's controls belong to a form kept beside the table. */
function useSetBudget({ budgets, busy, save }: FormProps): { row: ReactNode; form: ReactNode } {
  const [measure, setMeasure] = useState<Measure>('tokens');
  const [limit, setLimit] = useState('');
  const [length, setLength] = useState<Length>('day');
  const [act, setAct] = useState<BudgetAct>('tell');
  const id = 'set-budget-' + budgets.holder.kind + '-' + budgets.holder.id.replace(/[^A-Za-z0-9_-]/g, '_');
  const valid = validAmount(measure, limit)
    && !budgets.unavailable.some((each) => each.unit === measure);
  const submit = () => {
    if (busy || !valid) return;
    const next = { unit: measure, amount: kept(measure, Number(limit)), period: measure === 'context_percent' ? null : length, act };
    const body: BudgetBody = { limits: [...budgets.limits, next], warn_at: budgets.warn_at, version: budgets.version };
    void save(body);
  };
  const form = <form id={id} aria-label="Set a budget" className="usage-add-form" onSubmit={(event) => { event.preventDefault(); if (limit !== '') submit(); }} />;
  const row = <tr className="usage-add-line" data-add="budget">
    <td><div className="usage-amount">
      <input form={id} name="limit" aria-label="Spend at most" type="number" min={0} step="any" required value={limit} disabled={busy} placeholder="Amount" onChange={(event) => setLimit(event.target.value)} />
      <select form={id} aria-label="Unit" value={measure} disabled={busy} onChange={(event) => {
        const next = event.target.value as Measure; setMeasure(next);
        if (next !== 'context_percent' && !PERIODS[next].includes(length)) setLength(PERIODS[next][0]);
      }}>{Object.entries(UNITS).map(([value, words]) => <option key={value} value={value} disabled={budgets.unavailable.some((each) => each.unit === value)}>{words}</option>)}</select>
      {measure === 'context_percent' ? <span>at any moment</span> : <select form={id} aria-label="Period" value={length} disabled={busy} onChange={(event) => setLength(event.target.value as Length)}>{PERIODS[measure].map((period) => <option key={period} value={period}>{'a ' + PERIOD_WORDS[period]}</option>)}</select>}
    </div></td>
    <td />
    <td><select form={id} aria-label="When it's hit" value={act} disabled={busy} onChange={(event) => setAct(event.target.value as BudgetAct)}>{Object.entries(ACTS).map(([value, words]) => <option key={value} value={value}>{words}</option>)}</select></td>
    <td />
    <td><Act symbol="add" name="Add this limit" tone="primary" type="submit" form={id} disabled={busy || !valid} /></td>
  </tr>;
  return { row, form };
}
