/**
 * What the person's agents cost this week: one total in US dollars, then every agent, the dearest first, whether or not
 * it has a limit. Each figure is the service's own (`usage.figures`, dollars over the week); the page only adds them
 * up for the total. A figure the service could not give is never counted as nothing: the total then says "at least"
 * and the line says which agent is missing and why. The price is what the harness reports, at API rates, so an agent on
 * a plan account is said to be paid by its plan, in words on the face, never in a tooltip.
 */
import { day } from '../file/time';
import { money } from '../usage/budgetWords';
import type { Used } from '../usage/contract';
import { refusedPart } from './contract';
import type { DashboardAgent, PartRefused } from './contract';
import { Unread } from './Unread';

/** One agent's week: its dollars, a part of them when some records reported nothing, nothing reported, or unread. */
export type Week =
  | { kind: 'spent'; figure: number }
  | { kind: 'partial'; figure: number; missing: number }
  | { kind: 'none' }
  | { kind: 'unread'; refused: PartRefused };

const weekOf = (figures: Used[] | undefined): Used | undefined =>
  Array.isArray(figures) ? figures.find((used) => used?.unit === 'dollars' && used.period === 'week') : undefined;

export function weekSpend(row: DashboardAgent): Week {
  if (refusedPart(row.usage)) return { kind: 'unread', refused: row.usage };
  const used = weekOf(row.usage.figures);
  if (!used) return { kind: 'none' };
  if (used.figure !== null) return { kind: 'spent', figure: used.figure };
  if (used.reported) return { kind: 'partial', figure: used.reported.figure, missing: used.reported.missing };
  return { kind: 'none' };
}

/** Whether an agent's use is paid by a plan: its account reported plan windows. */
const onPlan = (row: DashboardAgent): boolean =>
  !refusedPart(row.usage) && Array.isArray(row.usage.accounts) && row.usage.accounts.some((account) => Array.isArray(account?.windows) && account.windows.length > 0);

/** The week's start as the service counted it, from the first figure that says. */
function since(rows: DashboardAgent[]): number | null {
  for (const row of rows) {
    if (refusedPart(row.usage)) continue;
    const used = weekOf(row.usage.figures);
    if (used?.since_ms) return used.since_ms;
  }
  return null;
}

const known = (week: Week): number => week.kind === 'spent' || week.kind === 'partial' ? week.figure : 0;

/** The total of what was reported, and how many agents gave less than a whole figure. */
export function weekTotal(rows: DashboardAgent[]): { total: number; short: number; priced: number } {
  const weeks = rows.map(weekSpend);
  return { total: weeks.reduce((sum, week) => sum + known(week), 0), short: weeks.filter((week) => week.kind !== 'spent').length,
    priced: weeks.filter((week) => week.kind === 'spent' || week.kind === 'partial').length };
}

/** The total as said: the sum, "at least" the sum when some agent gave less than a whole figure, or that nothing was priced. */
function totalWords(rows: DashboardAgent[]): string {
  const { total, short, priced } = weekTotal(rows);
  if (!priced && short) return 'Nothing reported yet';
  return (short ? 'At least ' : '') + money(total);
}

function Figure({ week }: { week: Week }) {
  switch (week.kind) {
    case 'spent': return <span className="dash-money">{money(week.figure)}</span>;
    case 'partial': return <span className="dash-money">at least {money(week.figure)}<span className="note dash-money-why">some of its use gave no price</span></span>;
    case 'none': return <span className="dim">Nothing reported</span>;
    case 'unread': return <Unread part={week.refused} words="Could not be read" />;
  }
}

/** The panel: the week's total, each agent's line, and what the price means. */
export function Spend({ rows }: { rows: DashboardAgent[] }) {
  const lines = rows.map((row) => ({ row, week: weekSpend(row) })).sort((left, right) => known(right.week) - known(left.week));
  const start = since(rows);
  const plan = rows.some(onPlan);
  return <section className="you-panel dash-spend" aria-label="This week">
    <div className="section-h"><span>This week</span></div>
    <p className="dash-spend-total">
      <b data-spend="total">{totalWords(rows)}</b>
      <span className="sec">{start === null ? ' so far this week' : ' since ' + day(Math.floor(start / 1000))}</span>
    </p>
    <div className="you-scroll">
      {rows.length ? <table className="dash-table"><tbody>
        {lines.map(({ row, week }) => <tr key={row.agent.id} data-spend={row.agent.id}>
          <td className="dash-name"><a href={'#/file/' + encodeURIComponent(row.agent.id) + '/budgets'}>{row.agent.display_name}</a></td>
          <td className="dash-num"><Figure week={week} /></td>
        </tr>)}
      </tbody></table> : <p className="dim">You have no agents yet, so nothing has been spent. <a href="#/agents/new">Add an agent</a></p>}
    </div>
    <p className="note dash-spend-note">
      {plan ? 'On a plan, you pay the plan’s price. These figures are what the same use would cost at API prices, in US dollars.'
        : 'In US dollars, at the prices the agents’ programs report.'}
    </p>
  </section>;
}
