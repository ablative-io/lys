/**
 * Needs a look: only what is wrong with the person's agents, each in one sentence, and nothing at all when nothing is.
 * A limit reached, a limit past where its warning begins, a goal missed. Context limits are left out: compacting at
 * a level is the limit working, not something to look at. Everything else about an agent is its row in the table.
 */
import { limitWords } from '../usage/budgetWords';
import { refusedPart } from './contract';
import type { DashboardAgent } from './contract';
import { nearness, warnFraction } from './words';

/** One exception: the agent, the sentence, and how bad, a reached limit and a missed goal before a near one. */
export interface Exception { agent: DashboardAgent['agent']; words: string; rank: number; kind: 'reached' | 'near' | 'missed' }

export function exceptionsOf(rows: DashboardAgent[]): Exception[] {
  const found = rows.flatMap((row): Exception[] => {
    const name = row.agent.display_name;
    const limits = refusedPart(row.budget) ? [] : row.budget.limits.flatMap((limit, index): Exception[] => {
      if (refusedPart(row.budget) || limit.unit === 'context_percent') return [];
      const near = nearness(row.budget, index);
      if (near >= 1) return [{ agent: row.agent, kind: 'reached', rank: 0, words: name + ' reached its limit of ' + limitWords(limit) }];
      if (near >= warnFraction(row.budget)) return [{ agent: row.agent, kind: 'near', rank: 2, words: name + ' has used ' + Math.floor(near * 100) + '% of its ' + limitWords(limit) }];
      return [];
    });
    const goals = refusedPart(row.goals) ? [] : row.goals.goals.filter((item) => item.goal.active && item.standing === 'missed')
      .map((item): Exception => ({ agent: row.agent, kind: 'missed', rank: 1, words: name + ' missed a goal: ' + item.goal.words }));
    return [...limits, ...goals];
  });
  return found.sort((left, right) => left.rank - right.rank);
}

export function Exceptions({ rows }: { rows: DashboardAgent[] }) {
  const found = exceptionsOf(rows);
  if (!found.length) return null;
  return <section className="you-panel dash-exceptions" aria-label="Needs a look">
    <div className="section-h"><span>Needs a look</span><b className="dash-count">{found.length}</b></div>
    <div className="you-scroll"><table className="dash-table"><tbody>
      {found.map((each, index) => <tr key={each.agent.id + '/' + index} data-exception={each.kind}>
        <td><span role="img" className={'dot ' + (each.kind === 'near' ? 'dash-dot-warn' : 'dash-dot-danger')} aria-label={each.kind === 'near' ? 'near' : 'reached'} />{each.words}</td>
        <td className="you-act"><a className="btn" href={'#/file/' + encodeURIComponent(each.agent.id) + '/budgets'}>Open</a></td>
      </tr>)}
    </tbody></table></div>
  </section>;
}
