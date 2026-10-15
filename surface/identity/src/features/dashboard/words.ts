/** An agent's budget and goals in one compact line each, as the Dashboard's agents table says them. */
import { day } from '../file/time';
import { contextPresentation } from '../budgets/widgetReadings';
import { amount, limitWords, usedAgainst } from '../usage/budgetWords';
import type { BudgetsView, GoalsView, Limit } from '../usage/contract';
import { counted } from '../../shell/count';
import { refusedPart } from './contract';
import type { DashboardAgent, PartRefused } from './contract';

/** How near the limit at `index` is to being reached; a limit with no reported figure is placed after every measured one. */
const nearness = (budget: BudgetsView, index: number): number => {
  const used = budget.used[index];
  const amount = budget.limits[index].amount;
  return used && used.figure !== null && amount > 0 ? used.figure / amount : -1;
};

/** How near the agent's nearest limit is to being reached, 1 being reached; null when no limit has a reported figure. */
export const tightest = (budget: BudgetsView): number | null => {
  const near = Math.max(-1, ...budget.limits.map((_, index) => nearness(budget, index)));
  return near < 0 ? null : near;
};

const CONTEXT_ACTS: Record<Limit['act'], string> = { compact: 'compacts', notice: 'sends a notice', stop: 'stops', tell: 'tells you' };

/** A context limit describes its act; the current reading comes from the same session evidence as the usage widget. */
function contextWords(row: DashboardAgent, limit: Limit): string {
  const reading = contextPresentation([row]);
  const current = reading && reading.figure !== null ? reading.display + ' now'
    : !reading || reading.display === 'Not reported' ? 'not reported yet'
    : reading.display.toLowerCase() + (reading.unavailable.length ? ': ' + reading.unavailable.join('; ') : '');
  return CONTEXT_ACTS[limit.act] + ' at ' + amount(limit.unit, limit.amount) + ' · ' + current;
}

/**
 * A line of the agents table or a widget: its words, each limit or goal on a line of its own, or the part the service
 * could not read. A refused part carries its refusal for the Error details fold; its name is never the words.
 */
export interface Line { words: string; lines: string[]; refused: PartRefused | null }

const said = (lines: string[]): Line => ({ words: lines.join(' · '), lines, refused: null });
const unread = (part: PartRefused, what: string): Line => ({ words: what + ' could not be read', lines: [what + ' could not be read'], refused: part });

/** Each limit with its reading, tightest first; context names the budget's act separately from current usage. */
export function budgetLine(row: DashboardAgent): Line {
  const { budget, usage } = row;
  if (refusedPart(budget)) return unread(budget, 'Its limits');
  if (refusedPart(usage)) return unread(usage, 'Its use');
  if (!budget.limits.length) return said(['No limit']);
  const order = budget.limits.map((_, index) => index).sort((left, right) => nearness(budget, right) - nearness(budget, left));
  return said(order.map((index) => budget.limits[index].unit === 'context_percent' ? contextWords(row, budget.limits[index])
    : usedAgainst(budget, index) ?? limitWords(budget.limits[index]) + ', nothing reported yet'));
}

/** Open goals with the next deadline, missed ones, or that there is none: "2 open, next due 6 Oct", "1 missed", "No goal". */
export function goalsLine(goals: GoalsView | PartRefused): Line {
  if (refusedPart(goals)) return unread(goals, 'Its goals');
  const items = goals.goals.filter((item) => item.goal.active);
  if (!items.length) return said(['No goal']);
  const open = items.filter((item) => item.standing === 'open');
  const missed = items.filter((item) => item.standing === 'missed').length;
  const due = open.flatMap((item) => item.goal.deadline === null ? [] : [item.goal.deadline]).sort((a, b) => a - b)[0];
  const parts = [
    ...(open.length ? [open.length + ' open' + (due === undefined ? '' : ', next due ' + day(due))] : []),
    ...(missed ? [missed + ' missed'] : []),
  ];
  if (parts.length) return said([parts.join(', ')]);
  const met = items.filter((item) => item.standing === 'met').length;
  return said([met ? counted(met, 'met') : 'None open']);
}
