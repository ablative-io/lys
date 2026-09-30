/** Personal budgets keep the requested change beside the limits still enforced until an administrator confirms. */
import { useRef, useState } from 'react';
import { api, request, useLoad } from '../../api';
import { DirectoryGate as Gate, ErrorWords } from '../people/Words';
import { ACTS, MEASURES, shown, asLimit } from '../usage/contract';
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

function Details({ limit, version, zone }: { limit: Limit; version: number; zone: string }) {
  return <dl className="facts">
    <dt>Limit</dt><dd>{shown(limit.unit, limit.amount)}</dd>
    <dt>Period</dt><dd>{limit.period ? 'Each ' + limit.period + ', ' + (limit.zone ?? zone) : 'No period'}</dd>
    <dt>When reached</dt><dd>{ACTS[limit.act]}</dd>
    <dt>Version</dt><dd>{version}</dd>
  </dl>;
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
    {!view.limits.length && !view.unconfirmed.length ? <p>No personal budget is set.</p> : null}
    {view.limits.map((limit, index) => ({ limit, index })).filter(({ limit }) => !waiting.has(limit.unit)).map(({ limit, index }) => <article key={index} aria-label={MEASURES[limit.unit]}>
      <h3>{MEASURES[limit.unit]}</h3><p>Enforced budget</p><Details limit={limit} version={view.version} zone={view.zone} />
    </article>)}
    {view.unconfirmed.map(({ requested, effective, reason }) => <article key={requested.measure} aria-label={'Pending ' + requested.measure}>
      <h3>{MEASURES[requested.measure]}</h3>
      <p>{reason}</p>
      <div className="grid2">
        <section aria-label="Currently enforced"><h4>Currently enforced</h4><Details limit={asLimit(effective)} version={effective.version} zone={view.zone} /></section>
        <section aria-label="Requested change"><h4>Requested change</h4><Details limit={asLimit(requested)} version={requested.version} zone={view.zone} /></section>
      </div>
      {administrator ? <button className="btn primary" disabled={busy} onClick={() => void confirm(requested)}>Apply the new limit of {shown(requested.measure, requested.limit)} {requested.measure === 'tokens' ? 'tokens' : requested.measure === 'running_ms' ? 'minutes' : 'percent'}</button>
        : <p>An administrator must confirm this change. The currently enforced budget remains in place.</p>}
    </article>)}
    {busy ? <p role="status">Sending the confirmation; the stored result is not yet known.</p> : null}
  </>;
}
