/** An agent's spending limit: one amount in dollars over one period, and what happens when it is reached, saved with
 * PUT /budgets/agent/{id}. It is the one editor of that limit: the Budget table below leaves its row out. An
 * administrator's or the responsible person's save is the budget at once; there is no request to approve first. The
 * sentence under it is the chosen act's own, never a promise the act does not keep. */
import { useRef, useState } from 'react';
import { Refused, request } from '../../api';
import { answeredNo } from '../../kept';
import { Act } from '../../shell/Act';
import { ACTS, standing } from './contract';
import type { BudgetAct, BudgetBody, BudgetsView, Length, Limit, Receipt } from './contract';
import { PERIODS, confirmsBudget, validAmount } from './budgetForm';
import { PERIOD_WORDS, money, usedAgainst } from './budgetWords';
import { plain } from './reasons';
import { ErrorWords } from '../people/Words';

/** Where the spending limit sits among the limits: the first dollar limit over a period. */
export const spendingIndex = (limits: Limit[]): number => limits.findIndex((limit) => limit.unit === 'dollars' && limit.period !== null);

/** What happens when the spending limit is reached, as the card says it under the amount. */
export const REACHED: Record<BudgetAct, string> = {
  stop: 'the running session ends.',
  compact: 'the running session is compacted and keeps running.',
  notice: 'a notice is typed into the running session, which keeps running.',
  tell: 'the responsible person is told; the session keeps running.',
};

/** The limits with the spending limit set to `amount` a `period` with `act`, or taken away when `amount` is null; every other limit kept as it was. */
export function withSpending(limits: Limit[], amount: number | null, period: Length, act: BudgetAct = 'stop'): Limit[] {
  const at = spendingIndex(limits);
  if (amount === null) return at < 0 ? limits : limits.filter((_, index) => index !== at);
  // An existing limit keeps any explicit zone; the amount, period and act are this form's.
  const next: Limit = at < 0 ? { unit: 'dollars', amount, period, act } : { ...limits[at], amount, period, act };
  return at < 0 ? [...limits, next] : limits.map((limit, index) => (index === at ? next : limit));
}

/**
 * What the limit has used and where it stands, as the Budget table says it of every other limit: a reached limit says
 * what its act did, so a limit that only told someone is never read as one that stopped the agent.
 */
function Stands({ budgets, at, receipts, name }: { budgets: BudgetsView; at: number; receipts: Receipt[]; name?: string }) {
  const used = budgets.used[at];
  const stands = standing(budgets, at, receipts);
  const words = used && used.figure === null ? plain(used.unavailable, { id: budgets.holder.id, name: name ?? 'this ' + budgets.holder.kind }).join(' ') : stands.words;
  return <div className="usage-stands" data-limit="spending">
    <p className="usage-used">{usedAgainst(budgets, at) ?? 'Nothing reported yet'}</p>
    <p data-reached={stands.reached}>{words}</p>
  </div>;
}

export function SpendingLimit({ budgets, receipts = [], name, changed }: { budgets: BudgetsView; receipts?: Receipt[]; name?: string; changed: (words: string) => void }) {
  const at = spendingIndex(budgets.limits);
  const current = at < 0 ? null : budgets.limits[at];
  const [amount, setAmount] = useState(current ? String(current.amount) : '');
  const [period, setPeriod] = useState<Length>(current?.period ?? 'month');
  const [act, setAct] = useState<BudgetAct>(current?.act ?? 'stop');
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
    const body: BudgetBody = { limits: withSpending(budgets.limits, cleared ? null : Number(amount), period, act), warn_at: budgets.warn_at, version: budgets.version };
    try {
      const answer = await request<unknown>('/budgets/' + budgets.holder.kind + '/' + encodeURIComponent(budgets.holder.id), body, 'PUT');
      if (!confirmsBudget(answer, body, budgets.holder)) throw new Refused(200, { refusal: 'UnconfirmedReceipt', reason: 'The answer did not confirm the spending limit.' });
      changed(cleared ? 'Spending limit removed.' : 'Spending limit set: ' + money(Number(amount)) + ' a ' + PERIOD_WORDS[period] + '. It applies now.');
    } catch (error) {
      if (!answeredNo(error)) setUnsure(true);
      setFailure(error);
    } finally { sending.current = false; setBusy(false); }
  };
  return <form aria-label="Spending limit" className="card usage-spending" onSubmit={(event) => { event.preventDefault(); void save(); }}>
    <h3>Spending limit</h3>
    {current ? <Stands budgets={budgets} at={at} receipts={receipts} name={name} /> : null}
    {unavailable ? <p className="usage-reason">Lys cannot measure this agent’s spending yet, so a spending limit cannot be set. Start it once through Lys so its model calls go through the Lys proxy, then come back.</p> : <>
      <div className="usage-amount">
        <span>US$</span>
        <input name="spending_amount" aria-label="Spending limit in US dollars" type="number" min={0} step="any" placeholder="No limit" value={amount} disabled={busy || unsure} onChange={(event) => setAmount(event.target.value)} />
        <select name="spending_period" aria-label="Spending limit period" value={period} disabled={busy || unsure} onChange={(event) => setPeriod(event.target.value as Length)}>
          {PERIODS.dollars.map((each) => <option key={each} value={each}>{'a ' + PERIOD_WORDS[each]}</option>)}
        </select>
      </div>
      <div className="usage-amount usage-reached">
        <label htmlFor={'spending-act-' + budgets.holder.id}>When it is reached</label>
        <select id={'spending-act-' + budgets.holder.id} name="spending_act" aria-label="When the spending limit is reached" value={act} disabled={busy || unsure} onChange={(event) => setAct(event.target.value as BudgetAct)}>
          {Object.entries(ACTS).map(([value, words]) => <option key={value} value={value}>{words}</option>)}
        </select>
        <Act symbol="save" name="Save the spending limit" word="Save" tone="primary" type="submit" disabled={busy || unsure || !valid} />
      </div>
      <p className="note" data-reached-act={act}>{'Applies as soon as it is saved. When it is reached, ' + REACHED[act]}{current ? ' Leave the amount empty and save to remove it.' : ''}</p>
    </>}
    {busy ? <p role="status">Saving…</p> : null}
    {failure ? <><p role="alert" className="why-not">The spending limit was not saved.</p><ErrorWords problem={failure} /></> : null}
    {unsure ? <p role="status">Whether this was saved is not confirmed. Reload the page before changing it again.</p> : null}
  </form>;
}
