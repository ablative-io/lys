/** Personal budgets keep the requested change beside the limits still enforced until an administrator confirms. */
import { useRef, useState } from 'react';
import type { ReactNode } from 'react';
import { api, request, useLoad } from '../../api';
import { DirectoryGate as Gate, ErrorWords } from '../people/Words';
import { ACTS, MEASURES, asLimit } from '../usage/contract';
import { amount } from '../usage/budgetWords';
import type { Budget, BudgetsView, Limit } from '../usage/contract';

type PersonalView = BudgetsView;

export function PersonalBudgets({ id, name }: { id: string; name: string }) {
  const path = '/budgets/person/' + encodeURIComponent(id);
  const load = useLoad(async () => {
    const [view, people] = await Promise.all([request<PersonalView>(path), api.people()]);
    return { view, administrator: people.scope === 'directory' };
  }, path);
  return <section className="card" aria-label="Personal budgets">
    <h2>Limits across {name}’s agents</h2>
    <p>A requested change does not replace the enforced budget until an administrator applies it.</p>
    <Gate load={load} title="Personal budgets" ok={({ view, administrator }) => <BudgetReview key={id} path={path} initial={view} administrator={administrator} />} />
  </section>;
}

/** One limit as a row of the table: what it is on, how much, over what period, what happens, its version, and where it stands. */
function Row({ label, limit, version, zone, state, children }: { label: string; limit: Limit; version: number; zone: string; state: string; children?: ReactNode }) {
  return <tr aria-label={label}>
    <td>{MEASURES[limit.unit]}</td>
    <td>{amount(limit.unit, limit.amount)}</td>
    <td>{limit.period ? 'Each ' + limit.period + ', ' + (limit.zone ?? zone) : 'No period'}</td>
    <td>{ACTS[limit.act]}</td>
    <td>{version}</td>
    <td>{state}</td>
    <td>{children}</td>
  </tr>;
}

function BudgetReview({ path, initial, administrator }: { path: string; initial: PersonalView; administrator: boolean }) {
  const [view, setView] = useState(initial);
  const [failure, setFailure] = useState<unknown>(null);
  const sending = useRef(false);
  const [busy, setBusy] = useState(false);
  const confirm = async (budget: Budget) => {
    if (sending.current) return;
    sending.current = true;
    setBusy(true);
    try {
      setFailure(null);
      const answer = await request<Budget>(path + '/confirm', { measure: budget.measure, version: budget.version });
      if (answer.holder.kind !== budget.holder.kind || answer.holder.id !== budget.holder.id || answer.measure !== budget.measure || answer.version <= budget.version) throw new Error('The answer did not confirm the requested budget. Open this tab again to check its outcome.');
      setView((current) => {
        const limits = current.limits.map((limit) => limit.unit === answer.measure ? asLimit(answer) : limit);
        const unconfirmed = current.unconfirmed.filter((entry) => entry.requested.measure !== answer.measure);
        return { ...current, limits, version: current.version + answer.version - budget.version, by: answer.by, at: answer.at, unconfirmed,
          effective_limits: unconfirmed.length ? limits.map((limit) => { const pending = unconfirmed.find((entry) => entry.requested.measure === limit.unit); return pending ? asLimit(pending.effective) : limit; }) : undefined,
          used: current.used.map((used) => used.unit === answer.measure ? { ...used, period: answer.period?.length ?? null, figure: null, since_ms: null, unavailable: 'Usage for the confirmed limit must be read again' } : used),
        };
      });
    } catch (error) {
      setFailure(error);
    } finally {
      sending.current = false;
      setBusy(false);
    }
  };
  const waiting = new Set(view.unconfirmed.map((entry) => entry.requested.measure));
  return <>
    {failure ? <ErrorWords problem={failure} /> : null}
    <table className="usage-list usage-table">
      <thead><tr><th>Limit on</th><th>Amount</th><th>Period</th><th>When reached</th><th>Version</th><th>Where it stands</th><th>Change</th></tr></thead>
      <tbody>
        {!view.limits.length && !view.unconfirmed.length ? <tr><td colSpan={7} className="dim">No personal budget is set.</td></tr> : null}
        {view.limits.map((limit, index) => ({ limit, index })).filter(({ limit }) => !waiting.has(limit.unit)).map(({ limit, index }) =>
          <Row key={index} label={MEASURES[limit.unit]} limit={limit} version={view.version} zone={view.zone} state="Enforced budget" />)}
        {view.unconfirmed.flatMap(({ requested, effective, reason }) => [
          <Row key={requested.measure + '.enforced'} label={'Enforced ' + requested.measure} limit={asLimit(effective)} version={effective.version} zone={view.zone} state="Currently enforced" />,
          <Row key={requested.measure + '.requested'} label={'Pending ' + requested.measure} limit={asLimit(requested)} version={requested.version} zone={view.zone} state={'Requested change. ' + reason}>
            {administrator ? <button className="btn primary" disabled={busy} onClick={() => void confirm(requested)}>Apply the new limit of {amount(requested.measure, requested.limit)}</button>
              : <p>An administrator must confirm this change. The currently enforced budget remains in place.</p>}
          </Row>,
        ])}
      </tbody>
    </table>
    {busy ? <p role="status">Sending the confirmation; the stored result is not yet known.</p> : null}
  </>;
}
