/** An agent's budget and goals in one compact line each, as the Dashboard's agents table says them. */
import { day } from '../file/time';
import { limitWords, usedAgainst } from '../usage/budgetWords';
import type { BudgetsView, GoalsView, UsageView } from '../usage/contract';
import { counted } from '../../shell/count';
import { refusedPart } from './contract';
import type { PartRefused } from './contract';

/** How near the limit at `index` is to being reached; a limit with no reported figure is placed after every measured one. */
const nearness = (budget: BudgetsView, index: number): number => {
  const used = budget.used[index];
  const amount = budget.limits[index].amount;
  return used && used.figure !== null && amount > 0 ? used.figure / amount : -1;
};

/**
 * Each limit as used of limit with its unit and period, the tightest first, paired exactly as the agent's Limits and goals
 * page pairs them; "No limit" when there is none; the refusal name when the budget or the usage could not be read.
 */
export function budgetLine(budget: BudgetsView | PartRefused, usage: UsageView | PartRefused): { words: string; refused: boolean } {
  if (refusedPart(budget)) return { words: budget.refusal, refused: true };
  if (refusedPart(usage)) return { words: usage.refusal, refused: true };
  if (!budget.limits.length) return { words: 'No limit', refused: false };
  const order = budget.limits.map((_, index) => index).sort((left, right) => nearness(budget, right) - nearness(budget, left));
  return { words: order.map((index) => usedAgainst(budget, index) ?? limitWords(budget.limits[index]) + ', nothing reported yet').join(' · '), refused: false };
}

/** Open goals with the next deadline, missed ones, or that there is none: "2 open, next due 6 Oct", "1 missed", "No goal". */
export function goalsLine(goals: GoalsView | PartRefused): { words: string; refused: boolean } {
  if (refusedPart(goals)) return { words: goals.refusal, refused: true };
  const items = goals.goals.filter((item) => item.goal.active);
  if (!items.length) return { words: 'No goal', refused: false };
  const open = items.filter((item) => item.standing === 'open');
  const missed = items.filter((item) => item.standing === 'missed').length;
  const due = open.flatMap((item) => item.goal.deadline === null ? [] : [item.goal.deadline]).sort((a, b) => a - b)[0];
  const parts = [
    ...(open.length ? [open.length + ' open' + (due === undefined ? '' : ', next due ' + day(due))] : []),
    ...(missed ? [missed + ' missed'] : []),
  ];
  if (parts.length) return { words: parts.join(', '), refused: false };
  const met = items.filter((item) => item.standing === 'met').length;
  return { words: met ? counted(met, 'met') : 'None open', refused: false };
}
