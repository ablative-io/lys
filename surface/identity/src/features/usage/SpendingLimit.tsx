/** An agent's spending limit: one amount in dollars over one period, saved with PUT /budgets/agent/{id}. An administrator's
 * or the responsible person's save is the budget at once; there is no request to approve first. When it is reached the
 * session ends, so a limit set here holds. */
import { useRef, useState } from 'react';
import { Refused, request } from '../../api';
import { answeredNo } from '../../kept';
import { Act } from '../../shell/Act';
import type { BudgetBody, BudgetsView, Length, Limit } from './contract';
import { PERIODS, confirmsBudget, validAmount } from './budgetForm';
import { PERIOD_WORDS } from './budgetWords';
import { ErrorWords } from '../people/Words';

/** Where the spending limit sits among the limits: the first dollar limit over a period. */
export const spendingIndex = (limits: Limit[]): number => limits.findIndex((limit) => limit.unit === 'dollars' && limit.period !== null);

/** The limits with the spending limit set to `amount` a `period`, or taken away when `amount` is null; every other limit kept as it was. */
export function withSpending(limits: Limit[], amount: number | null, period: Length): Limit[] {
  const at = spendingIndex(limits);
  if (amount === null) return at < 0 ? limits : limits.filter((_, index) => index !== at);
  // An existing limit keeps its own act and any explicit zone; only the amount and period are this form's.
  const next: Limit = at < 0 ? { unit: 'dollars', amount, period, act: 'stop' } : { ...limits[at], amount, period };
  return at < 0 ? [...limits, next] : limits.map((limit, index) => (index === at ? next : limit));
}

export function SpendingLimit({ budgets, changed }: { budgets: BudgetsView; changed: (words: string) => void }) {
  const at = spendingIndex(budgets.limits);
  const current = at < 0 ? null : budgets.limits[at];
  const [amount, setAmount] = useState(current ? String(current.amount) : '');
  const [period, setPeriod] = useState<Length>(current?.period ?? 'month');
  const [busy, setBusy] = useState(false);
  const [failure, setFailure] = useState<unknown>(null);
  const [unsure, setUnsure] = useState(false);
  const sending = useRef(false);
  const unavailable = budgets.unavailable.find((each) => each.unit === 'dollars');
  const cleared = amount.trim() === '';
  const valid = cleared ? current !== null : validAmount('dollars', amount);
  const save = async () => {
    if (sending.current || unsure || !valid) return;
    sending.current = true; setBusy(true); setFailure(null);
    const body: BudgetBody = { limits: withSpending(budgets.limits, cleared ? null : Number(amount), period), warn_at: budgets.warn_at, version: budgets.version };
    try {
      const answer = await request<unknown>('/budgets/' + budgets.holder.kind + '/' + encodeURIComponent(budgets.holder.id), body, 'PUT');
      if (!confirmsBudget(answer, body, budgets.holder)) throw new Refused(200, { refusal: 'UnconfirmedReceipt', reason: 'The answer did not confirm the spending limit.' });
      changed(cleared ? 'Spending limit removed.' : 'Spending limit set: $' + Number(amount) + ' a ' + PERIOD_WORDS[period] + '. It applies now.');
    } catch (error) {
      if (!answeredNo(error)) setUnsure(true);
      setFailure(error);
    } finally { sending.current = false; setBusy(false); }
  };
  return <form aria-label="Spending limit" className="card usage-spending" onSubmit={(event) => { event.preventDefault(); void save(); }}>
    <h3>Spending limit</h3>
    {unavailable ? <p className="usage-reason">Lys cannot measure this agent’s spending yet, so a spending limit cannot be set. Start it once through Lys so its model calls go through the Lys proxy, then come back.</p> : <>
      <div className="usage-amount">
        <span>$</span>
        <input name="spending_amount" aria-label="Spending limit in dollars" type="number" min={0} step="any" placeholder="No limit" value={amount} disabled={busy || unsure} onChange={(event) => setAmount(event.target.value)} />
        <select name="spending_period" aria-label="Spending limit period" value={period} disabled={busy || unsure} onChange={(event) => setPeriod(event.target.value as Length)}>
          {PERIODS.dollars.map((each) => <option key={each} value={each}>{'a ' + PERIOD_WORDS[each]}</option>)}
        </select>
        <Act symbol="save" name="Save the spending limit" word="Save" tone="primary" type="submit" disabled={busy || unsure || !valid} />
      </div>
      <p className="note">{current ? 'Applies as soon as it is saved. When it is reached, the session ends. Leave it empty and save to remove it.' : 'Applies as soon as it is saved. When it is reached, the session ends.'}</p>
    </>}
    {busy ? <p role="status">Saving…</p> : null}
    {failure ? <><p role="alert" className="why-not">The spending limit was not saved.</p><ErrorWords problem={failure} /></> : null}
    {unsure ? <p role="status">Whether this was saved is not confirmed. Reload the page before changing it again.</p> : null}
  </form>;
}
