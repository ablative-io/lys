/** Complete collection responses keep the fixtures aligned with the wire shape. */
import { asLimit } from '../src/features/usage/contract';
import type { Budget, BudgetsView, Holder } from '../src/features/usage/contract';

export function budgetsView(holder: Holder, budgets: Budget[] = [], extra: Partial<BudgetsView> = {}): BudgetsView {
  const limits = budgets.map(asLimit);
  return { holder, limits, warn_at: null, zone: 'UTC', version: budgets.reduce((sum, budget) => sum + budget.version, 0), by: budgets[0]?.by ?? null, at: budgets[0]?.at ?? null,
    used: limits.map((limit) => ({ unit: limit.unit, period: limit.period, figure: 0, since_ms: limit.period ? 0 : null, unavailable: null })), unavailable: [], within: [], unconfirmed: [], ...extra };
}
