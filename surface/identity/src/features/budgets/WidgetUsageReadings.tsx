import { useState } from 'react';
import type { ReactNode } from 'react';
import type { DashboardAgent } from '../dashboard/contract';
import { refusedPart } from '../dashboard/contract';
import { clockMs } from '../file/time';
import { THIS, amount } from '../usage/budgetWords';
import type { Length } from '../usage/contract';
import { ContextReadingNote, contextPresentation } from './widgetReadings';
import { tokenPresentations, tokenSummary } from './tokenReadings';
import './widgetReadings.css';
import { Refused } from '../../api';
import { ErrorWords } from '../people/Words';

export interface SessionSelection {
  selected?: Record<string, string>;
  select?: (agent: string, session: string) => void;
}
interface LevelRenderer { renderLevel?: (percent: number, display: string) => ReactNode }

/** The picker chooses a named live session; the service's current session remains the initial choice. */
export function ContextReadings({ rows, selected, select, renderLevel }: { rows: DashboardAgent[] } & SessionSelection & LevelRenderer) {
  const [choices, setChoices] = useState<Record<string, string>>({});
  return <>{rows.map((row) => {
    if (refusedPart(row.usage)) return <div key={row.agent.id}><p className="why-not">The use of {row.agent.display_name} could not be read.</p><ErrorWords problem={new Refused(0, row.usage)} /></div>;
    const chosen = selected !== undefined ? selected[row.agent.id] : choices[row.agent.id];
    const reading = contextPresentation([row], chosen);
    if (!reading) return <section className="canvas-stat none" key={row.agent.id} data-context-agent={row.agent.id}><b>Not reported</b><span className="sec">Context of {row.agent.display_name}</span></section>;
    const current = contextPresentation([row]);
    const listed = reading.live_sessions.some((each) => each.session === chosen);
    return <section className="canvas-stat" key={row.agent.id} data-context-agent={row.agent.id} data-context-session={reading.session ?? undefined}
      aria-label={'Context of ' + row.agent.display_name}>
      <span data-context-percent={reading.figure !== null ? 'true' : undefined}>
        {reading.figure !== null && renderLevel ? renderLevel(reading.figure, reading.display) : <b>{reading.display}</b>}
      </span>
      <span className="sec">Context of {row.agent.display_name}{reading.model ? ', ' + reading.model : ''}</span>
      <ContextReadingNote reading={reading} />
      {reading.context_tokens !== null ? <span className="sec" data-context-tokens>{amount('tokens', reading.context_tokens)} reported
        {reading.window_tokens !== null ? ' of the declared ' + amount('tokens', reading.window_tokens) + ' window' : ''}</span> : null}
      {reading.live_sessions.length > 1 || chosen !== undefined ? <label className="sec" onPointerDown={(event) => event.stopPropagation()}>
        Showing session <select aria-label={'Context session for ' + row.agent.display_name} value={chosen ?? current?.session ?? ''}
          onChange={(event) => { const session = event.target.value; if (select) select(row.agent.id, session); else setChoices((now) => ({ ...now, [row.agent.id]: session })); }}>
          {chosen !== undefined && !listed ? <option value={chosen} title={chosen}>A session that has ended</option> : null}
          {chosen === undefined && current?.session === null ? <option value="">Current session not reported</option> : null}
          {reading.live_sessions.map((each, index) => <option key={each.session} value={each.session} title={each.session}>{'Session ' + (index + 1)}{each.session === current?.session ? ' (current)' : ''}{each.model ? ', ' + each.model : ''}</option>)}
        </select></label> : null}
      {reading.unavailable.length ? <span className="dim" data-context-unavailable>{reading.unavailable.join('; ')}</span> : null}
    </section>;
  })}</>;
}

/** Token classes are shown separately even when an older answer also carries a combined scalar. */
export function TokenReadings({ rows, period = 'day' }: { rows: DashboardAgent[]; period?: Length }) {
  return <>{tokenPresentations(rows, period).map((reading) => <section className="canvas-stat" key={reading.agent} data-token-agent={reading.agent}
    data-token-period={period} aria-label={'Tokens of ' + reading.name + ' ' + THIS[period]}>
    {reading.empty ? <span>{period === 'five_hour' ? 'This 5-hour interval' : THIS[period][0].toUpperCase() + THIS[period].slice(1)}: {reading.empty}</span> : <>
    <span className="sec">Tokens {period === 'five_hour' ? 'in this 5-hour interval' : THIS[period]} of {reading.name}</span>
    {reading.unavailable ? <span className="why-not" data-token-unavailable>{reading.unavailable}</span> : <table className="dash-table"><tbody>
      {reading.components.map((each) => <tr key={each.key} data-token-component={each.key}><th scope="row">{each.label}</th><td>{each.words}</td></tr>)}
    </tbody></table>}
    <span className="sec" data-token-counts>{reading.counts}</span>
    <span className="dim">{reading.since_ms === null ? 'Period start not reported' : <>Since <time dateTime={new Date(reading.since_ms).toISOString()}>{clockMs(reading.since_ms)}</time></>}</span>
    </>}
  </section>)}</>;
}

const tokenPeriod = (key: string): Length | null => {
  const period = key.startsWith('tokens/') ? key.slice('tokens/'.length) : '';
  return period === 'day' || period === 'week' || period === 'month' || period === 'five_hour' ? period : null;
};
export const supportsUsageReading = (key: string): boolean => key === 'context_percent/' || tokenPeriod(key) !== null || key === 'window/300';

/** The service writes a reset as `reset at <milliseconds>`; a person reads it as a time, never a count of milliseconds. */
export const readableResets = (text: string): string =>
  text.replace(/\breset at (\d{10,})\b/g, (_, ms: string) => 'reset at ' + clockMs(Number(ms)));

/** A provider window whose reset has passed is no longer its current level: what is left is when it was last read. */
const expired = (window: { resets_at_ms: number }, now: number): boolean => window.resets_at_ms <= now;
const notReadSince = (at_ms: number, resets_at_ms: number): string =>
  'not read since ' + clockMs(at_ms) + '; it reset ' + clockMs(resets_at_ms) + ', and the next call through this account reads it again';

function providerGaps(rows: DashboardAgent[]): string[] {
  return rows.flatMap((row) => {
    if (refusedPart(row.usage)) return [];
    const figures = row.usage.figures ?? [];
    if (!Array.isArray(figures) || figures.some((used) => !used || typeof used !== 'object')) return [row.agent.display_name + ': the 5-hour window could not be read: the reported figures are not a list of usage records'];
    return figures.filter((used) => used?.unit === 'plan_percent' && used.period === 'five_hour' && used.figure === null)
      .map((used) => row.agent.display_name + ': ' + (used.unavailable ? readableResets(used.unavailable) : 'the 5-hour window could not be read, and no reason was given'));
  });
}

/** These faces replace scalar context and token faces while other widget figures keep their existing renderer. */
export function WidgetUsageReadings({ rows, keys, selected, select, renderLevel }: { rows: DashboardAgent[]; keys: string[] } & SessionSelection & LevelRenderer) {
  return <>{keys.map((key) => {
    if (key === 'context_percent/') return <ContextReadings key={key} rows={rows} selected={selected} select={select} renderLevel={renderLevel} />;
    const period = tokenPeriod(key);
    if (period) return <TokenReadings key={key} rows={rows} period={period} />;
    if (key === 'window/300') {
      const gaps = providerGaps(rows);
      const windows = new Map<string, { account: string; at_ms: number; used_percent: number; resets_at_ms: number }>();
      for (const row of rows) if (!refusedPart(row.usage)) {
        const accounts = row.usage.accounts ?? [];
        if (!Array.isArray(accounts)) return <p className="why-not" key={key} role="alert">The 5-hour window could not be read: the reported accounts are not a list</p>;
        for (const account of accounts) {
          if (!account || !Array.isArray(account.windows)) return <p className="why-not" key={key} role="alert">The 5-hour window could not be read: the reported windows are not a list</p>;
          for (const each of account.windows) {
            if (!each) return <p className="why-not" key={key} role="alert">The 5-hour window could not be read: a reported window is absent</p>;
            if (each.duration_minutes !== 300) continue;
            if (typeof account.account !== 'string' || !account.account.length || !Number.isFinite(each.used_percent) || each.used_percent < 0 || each.used_percent > 100
              || !Number.isSafeInteger(account.at_ms) || !Number.isFinite(new Date(account.at_ms).getTime())
              || !Number.isSafeInteger(each.resets_at_ms) || !Number.isFinite(new Date(each.resets_at_ms).getTime())) {
              return <p className="why-not" key={key} role="alert">The 5-hour window could not be read: the reported account, level or time is invalid</p>;
            }
            const held = windows.get(account.account);
            if (!held || held.at_ms < account.at_ms) windows.set(account.account, { account: account.account, at_ms: account.at_ms, ...each });
          }
        }
      }
      const now = Date.now();
      return <div key={key}>{windows.size ? [...windows.values()].map((each) => expired(each, now)
        ? <section className="canvas-stat none" key={each.account} data-provider-window={each.account} data-provider-window-expired>
          <b>Not read since {clockMs(each.at_ms)}</b>
          <span className="sec">5-hour provider window of {each.account}: {notReadSince(each.at_ms, each.resets_at_ms)}.</span>
        </section>
        : <section className="canvas-stat" key={each.account} data-provider-window={each.account}>
        {renderLevel ? renderLevel(each.used_percent, amount('plan_percent', each.used_percent)) : <b>{amount('plan_percent', each.used_percent)}</b>}
        <span className="sec">5-hour provider window of {each.account}, resets {clockMs(each.resets_at_ms)}</span>
      </section>) : <><p className="dim" data-provider-window-unavailable>5-hour provider window not reported</p><TokenReadings rows={rows} period="five_hour" /></>}
        {gaps.length ? <ul className="dim" data-provider-window-gaps>{gaps.map((gap, index) => <li key={index}>{gap}</li>)}</ul> : null}
      </div>;
    }
    return null;
  })}</>;
}

/** Scalar token totals and context maxima never become a widget's small figure or a formula's input. */
export function usageSummary(rows: DashboardAgent[], key = 'context_percent/', selected?: Record<string, string>): { figure: string; percent?: number; value?: { amount: number; unit: 'context_percent' } } | null {
  if (key === 'context_percent/') {
    const chosen = rows.length === 1 ? selected?.[rows[0].agent.id] : undefined;
    const reading = contextPresentation(rows, chosen);
    if (!reading) return { figure: 'Not reported' };
    return { figure: reading.display + (reading.figure !== null ? ' context' : '') + ' · ' + reading.words,
      ...(reading.figure !== null ? { percent: reading.figure, value: { amount: reading.figure, unit: 'context_percent' as const } } : {}) };
  }
  const period = tokenPeriod(key);
  if (period) return { figure: tokenSummary(rows, period) };
  if (key === 'window/300' && providerGaps(rows).length) {
    const now = Date.now();
    const levels = rows.flatMap((row) => refusedPart(row.usage) || !Array.isArray(row.usage.accounts) ? [] : row.usage.accounts
      .flatMap((account) => (Array.isArray(account?.windows) ? account.windows : []).filter((window) => window?.duration_minutes === 300)
        .map((window) => account.account + ': ' + (expired(window, now) ? notReadSince(account.at_ms, window.resets_at_ms)
          : amount('plan_percent', window.used_percent) + ' reported'))));
    return { figure: (levels.length ? levels.join(' · ') : '5-hour provider window not reported') + ' · ' + providerGaps(rows).join('; ') };
  }
  if (key === 'window/300' && !rows.some((row) => !refusedPart(row.usage)
    && Array.isArray(row.usage.accounts) && row.usage.accounts.some((account) => Array.isArray(account?.windows) && account.windows.some((window) => window?.duration_minutes === 300)))) {
    return { figure: '5-hour provider window not reported · ' + tokenSummary(rows, 'five_hour') };
  }
  return null;
}
