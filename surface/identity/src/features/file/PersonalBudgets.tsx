/** Personal budgets keep the requested change beside the limits still enforced until an administrator confirms. */
import { useRef, useState } from 'react';
import { api, Refused, request, useLoad } from '../../api';
import { Gate } from '../signin/Gate';
import { ACTS, MEASURES, shown } from '../usage/contract';
import type { Budget, BudgetsView } from '../usage/contract';

type Unconfirmed = { requested: Budget; effective: Budget; reason: string };
type PersonalView = BudgetsView & { unconfirmed: Unconfirmed[] };

export function PersonalBudgets({ id, name }: { id: string; name: string }) {
  const [revision, setRevision] = useState(0);
  const path = '/budgets/person/' + encodeURIComponent(id);
  const load = useLoad(async () => {
    const [view, people] = await Promise.all([request<PersonalView>(path), api.people()]);
    return { view, administrator: people.scope === 'directory' };
  }, 'personal-budgets:' + id + ':' + revision);
  return <section className="card" aria-label="Personal budgets">
    <h2>Budgets for {name}</h2>
    <p>These limits apply across this person’s agents. A requested change does not replace the enforced budget until an administrator confirms it.</p>
    <Gate load={load} title="Personal budgets" ok={({ view, administrator }) =>
      <BudgetReview key={id + ':' + revision} path={path} view={view} administrator={administrator} refresh={() => setRevision((value) => value + 1)} />
    } />
    {load.status === 'refused' ? <button className="btn" onClick={() => setRevision((value) => value + 1)}>Read budgets again</button> : null}
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

function BudgetReview({ path, view, administrator, refresh }: { path: string; view: PersonalView; administrator: boolean; refresh: () => void }) {
  const sending = useRef(false);
  const [busy, setBusy] = useState(false);
  const [failure, setFailure] = useState('');
  const confirm = async (budget: Budget) => {
    if (sending.current || failure) return;
    sending.current = true;
    setBusy(true);
    try {
      await request<Budget>(path + '/confirm', { measure: budget.measure, version: budget.version });
      refresh();
    } catch (error) {
      setFailure(error instanceof Refused ? error.refusal.refusal + ': ' + error.message : String(error));
    } finally {
      sending.current = false;
      setBusy(false);
    }
  };
  const waiting = new Set(view.unconfirmed.map((entry) => entry.requested.measure));
  return <>
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
      {administrator ? <button className="btn primary" disabled={busy || Boolean(failure)} onClick={() => void confirm(requested)}>Confirm requested {MEASURES[requested.measure].toLowerCase()}</button>
        : <p>An administrator must confirm this change. The currently enforced budget remains in place.</p>}
    </article>)}
    {busy ? <p role="status">Sending the confirmation; the stored result is not yet known.</p> : null}
    {failure ? <div role="alert"><p>{failure}</p><p>Read the budgets again to check what was recorded before making another change.</p><button className="btn" onClick={refresh}>Read budgets again</button></div> : null}
  </>;
}
