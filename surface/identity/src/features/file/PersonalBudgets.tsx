/** Personal budgets keep the requested change beside the limits still enforced until an administrator confirms. */
import { useRef, useState } from 'react';
import { api, request, useLoad } from '../../api';
import { DirectoryGate as Gate, ErrorWords } from '../people/Words';
import { ACTS, MEASURES, shown } from '../usage/contract';
import type { Budget, BudgetsView } from '../usage/contract';

type Unconfirmed = { requested: Budget; effective: Budget; reason: string };
type PersonalView = BudgetsView & { unconfirmed: Unconfirmed[] };

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

function Details({ budget }: { budget: Budget }) {
  return <dl className="facts">
    <dt>Limit</dt><dd>{shown(budget.measure, budget.limit)}</dd>
    <dt>Period</dt><dd>{budget.period ? 'Each ' + budget.period.length + ', ' + budget.period.zone : 'No period'}</dd>
    <dt>When reached</dt><dd>{ACTS[budget.act]}</dd>
    <dt>Version</dt><dd>{budget.version}</dd>
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
      setView((current) => ({ ...current, budgets: [...current.budgets.filter((entry) => entry.measure !== answer.measure), answer], unconfirmed: current.unconfirmed.filter((entry) => entry.requested.measure !== answer.measure) }));
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
    {!view.budgets.length && !view.unconfirmed.length ? <p>No personal budget is set.</p> : null}
    {view.budgets.filter((budget) => !waiting.has(budget.measure)).map((budget) => <article key={budget.measure} aria-label={MEASURES[budget.measure]}>
      <h3>{MEASURES[budget.measure]}</h3><p>Enforced budget</p><Details budget={budget} />
    </article>)}
    {view.unconfirmed.map(({ requested, effective, reason }) => <article key={requested.measure} aria-label={'Pending ' + requested.measure}>
      <h3>{MEASURES[requested.measure]}</h3>
      <p>{reason}</p>
      <div className="grid2">
        <section aria-label="Currently enforced"><h4>Currently enforced</h4><Details budget={effective} /></section>
        <section aria-label="Requested change"><h4>Requested change</h4><Details budget={requested} /></section>
      </div>
      {administrator ? <button className="btn primary" disabled={busy} onClick={() => void confirm(requested)}>Apply the new limit of {shown(requested.measure, requested.limit)} {requested.measure === 'tokens' ? 'tokens' : requested.measure === 'running_ms' ? 'minutes' : 'percent'}</button>
        : <p>An administrator must confirm this change. The currently enforced budget remains in place.</p>}
    </article>)}
    {busy ? <p role="status">Sending the confirmation; the stored result is not yet known.</p> : null}
  </>;
}
