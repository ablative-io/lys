/** A budget's limits as a person says them: "US$500.00 a week", "50% of the weekly plan", "US$212.00 of US$500.00 this week". */
import type { BudgetsView, Length, Limit, Measure, Used } from './contract';

export const UNITS: Record<Measure, string> = { dollars: 'US dollars', plan_percent: '% of its plan', tokens: 'tokens', running_ms: 'minutes running', context_percent: '% of its context' };

export const PERIOD_WORDS: Record<Length, string> = { five_hour: '5-hour window', day: 'day', week: 'week', month: 'month' };

export const THIS: Record<Length, string> = { five_hour: 'this 5-hour window', day: 'today', week: 'this week', month: 'this month' };

const ADJECTIVE: Record<Length, string> = { five_hour: '5-hour', day: 'daily', week: 'weekly', month: 'monthly' };

/**
 * Money as every page writes it: US dollars, two decimals, Australian English grouping, never the browser's locale:
 * US$1,234.50. The figure is the harness's own `total_cost_usd`, so it is always US dollars.
 */
export const money = (value: number): string =>
  'US$' + value.toLocaleString('en-AU', { minimumFractionDigits: 2, maximumFractionDigits: 2 });

/**
 * An amount as a person reads it, in Australian English whatever the browser's locale: US$500.00, 50%, 2,000,000 tokens,
 * 90 minutes running. Every figure of a budget on every page goes through here, money through `money`.
 */
export function amount(unit: Measure, value: number): string {
  if (unit === 'dollars') return money(value);
  if (unit === 'running_ms') return (value / 60000).toLocaleString('en-AU', { maximumFractionDigits: 0 }) + ' minutes running';
  const figure = value.toLocaleString('en-AU', { maximumFractionDigits: 0 });
  if (unit === 'plan_percent' || unit === 'context_percent') return figure + '%';
  return figure + ' tokens';
}

/** One limit in words. */
export function limitWords(limit: Pick<Limit, 'unit' | 'amount' | 'period'>): string {
  if (limit.unit === 'context_percent' || limit.period === null) return amount(limit.unit, limit.amount) + ' of its context';
  if (limit.unit === 'plan_percent') return amount(limit.unit, limit.amount) + ' of the ' + ADJECTIVE[limit.period] + ' plan';
  return amount(limit.unit, limit.amount) + ' a ' + (limit.period === 'five_hour' ? '5-hour window' : limit.period);
}

/** What is used against a limit, beside it; null when nothing is reported for it. */
export function usedWords(limit: Limit, used: Used | undefined): string | null {
  if (!used || used.figure === null) return null;
  return amount(limit.unit, used.figure) + ' of ' + amount(limit.unit, limit.amount) + (limit.period ? ' ' + THIS[limit.period] : ' now');
}

/**
 * What is used against the budget's limit at `index`, as the agent's Limits and goals page and the Dashboard both say it:
 * each limit is paired with the figure the same budget answer gives for it. Null when nothing is reported for it.
 */
export function usedAgainst(budgets: Pick<BudgetsView, 'limits' | 'used'>, index: number): string | null {
  return usedWords(budgets.limits[index], budgets.used[index]);
}
