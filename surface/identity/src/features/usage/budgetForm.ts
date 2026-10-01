/** Form amounts use business units; confirmation matches the whole versioned collection. */
import type { BudgetBody, BudgetsView, Holder, Length, Measure } from './contract';
import { kept } from './contract';

export const PERIODS: Record<Measure, Length[]> = {
  context_percent: [], plan_percent: ['five_hour', 'week'],
  dollars: ['five_hour', 'day', 'week', 'month'], tokens: ['five_hour', 'day', 'week', 'month'], running_ms: ['five_hour', 'day', 'week', 'month'],
};

export function validAmount(unit: Measure, text: string): boolean {
  if (!text.trim()) return false;
  const given = Number(text);
  if (!Number.isFinite(given) || given < 0) return false;
  const amount = kept(unit, given);
  if (unit === 'tokens' || unit === 'running_ms') return Number.isSafeInteger(amount);
  return Number(amount.toFixed(6)) === amount && amount * 1000000 <= Number.MAX_SAFE_INTEGER
    && (unit === 'dollars' || amount <= 100);
}

export function confirmsBudget(answer: unknown, body: BudgetBody, holder: Holder): boolean {
  if (!answer || typeof answer !== 'object') return false;
  const value = answer as Partial<BudgetsView>;
  return value.holder?.kind === holder.kind && value.holder.id === holder.id
    && value.version === body.version + 1 && value.warn_at === body.warn_at
    && Array.isArray(value.limits) && value.limits.length === body.limits.length
    && value.limits.every((limit, index) => {
      const sent = body.limits[index];
      return limit?.unit === sent.unit && limit.amount === sent.amount && limit.period === sent.period
        && limit.act === sent.act && limit.zone === sent.zone;
    });
}
