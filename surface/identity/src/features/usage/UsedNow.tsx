/**
 * What an agent used, first on its Limits and goals tab, before anything that changes it: this week and today, each as
 * money, tokens and running time, from `figures` of `GET /agents/{id}/usage`; then each account's windows as its
 * provider last reported them; then when use was last reported. A figure the service could not give is said to be
 * missing, never shown as zero. Per model is not in the service's answer, and is not pretended to.
 */
import { count } from '../../shell/count';
import { clockMs, duration } from '../file/time';
import { money } from '../../shell/count';
import { tracking } from './contract';
import type { AccountWindows, Length, Measure, Used, UsageView } from './contract';

/** One figure in words: "US$4.20", "1,204,330 tokens", "3 hours 10 minutes running"; "at least" when some records gave none. */
function said(unit: Measure, value: number): string {
  if (unit === 'dollars') return money(value);
  if (unit === 'running_ms') return (value < 60000 ? 'under a minute' : duration(Math.floor(value / 60000) * 60)) + ' running';
  return count(Math.round(value)) + ' tokens';
}

export function figureWords(figures: Used[] | undefined, unit: Measure, period: Length): string | null {
  const used = Array.isArray(figures) ? figures.find((each) => each?.unit === unit && each.period === period) : undefined;
  if (!used) return null;
  if (used.figure !== null) return said(unit, used.figure);
  if (used.reported) return 'at least ' + said(unit, used.reported.figure);
  return null;
}

const MEASURES: Measure[] = ['dollars', 'tokens', 'running_ms'];
const MISSING: Record<string, string> = { dollars: 'no price', tokens: 'no tokens', running_ms: 'no running time' };

/** "US$4.20, 1,204,330 tokens, 3 hours running", with what is missing said by name; null when nothing at all was reported. */
export function periodWords(figures: Used[] | undefined, period: Length): string | null {
  const parts = MEASURES.map((unit) => figureWords(figures, unit, period));
  if (parts.every((part) => part === null)) return null;
  return parts.map((part, index) => part ?? MISSING[MEASURES[index]] + ' reported').join(', ');
}

/** A window's length as a person says it: "5-hour window", "weekly window", "2-hour window". */
export const windowName = (minutes: number): string =>
  minutes === 10080 ? 'Weekly window' : minutes % 60 === 0 ? minutes / 60 + '-hour window' : minutes + '-minute window';

function Windows({ account }: { account: AccountWindows }) {
  return <>{account.windows.map((each) => <tr key={account.account + '/' + each.duration_minutes} data-window={each.duration_minutes}>
    <th scope="row">{windowName(each.duration_minutes)}<span className="note usage-used-account">{account.account}</span></th>
    <td>{Math.round(each.used_percent) + '% used'}<span className="note usage-used-account">{each.resets_at_ms <= Date.now() ? 'reset ' + clockMs(each.resets_at_ms) + '; read again on its next use' : 'resets ' + clockMs(each.resets_at_ms)}</span></td>
  </tr>)}</>;
}

export function UsedNow({ usage }: { usage: UsageView }) {
  const tracked = tracking(usage);
  const week = periodWords(usage.figures, 'week');
  const today = periodWords(usage.figures, 'day');
  const accounts = Array.isArray(usage.accounts) ? usage.accounts.filter((account) => Array.isArray(account?.windows) && account.windows.length) : [];
  return <section className="card usage-used" aria-label="Used">
    <h2>Used</h2>
    <table className="usage-list usage-used-list"><tbody>
      <tr data-used="week"><th scope="row">This week</th><td>{week ?? 'Nothing reported yet'}</td></tr>
      <tr data-used="day"><th scope="row">Today</th><td>{today ?? 'Nothing reported yet'}</td></tr>
      {accounts.map((account) => <Windows key={account.account} account={account} />)}
    </tbody></table>
    {accounts.length ? <p className="note">The windows are the plan account&rsquo;s own, as its provider last reported them. Money is in US dollars at API prices; on a plan, the plan is what is paid.</p>
      : <p className="note">Money is in US dollars, at the prices the agent&rsquo;s program reports.</p>}
    <p className="usage-tracking" data-complete={tracked.complete} role="status">{tracked.words}</p>
  </section>;
}
