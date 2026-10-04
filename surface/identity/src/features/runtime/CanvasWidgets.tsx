/**
 * Widgets as they stand on the surface: a small window a person places, moves, sizes and draws lines to, holding one
 * kind of what Lys holds about the agents it counts. What a widget holds is the same part the Dashboard shows in a card.
 */
import { useLayoutEffect, useRef } from 'react';
import type { PointerEvent, ReactNode } from 'react';
import { Refused } from '../../api';
import type { Load } from '../../api';
import type { Board } from '../dashboard/board';
import { refusedPart } from '../dashboard/contract';
import { budgetPart, draftsPart, goalsPart, requestsPart } from '../dashboard/Widgets';
import { tightest } from '../dashboard/words';
import type { Part } from '../dashboard/Widgets';
import { clockMs } from '../file/time';
import { THIS, amount } from '../usage/budgetWords';
import { tracking } from '../usage/contract';
import type { Measure } from '../usage/contract';
import { KindSymbol } from './CanvasDock';
import { Anchors } from './CanvasMarks';
import { COLOURS, nextView, standing, tint } from './canvas-marks';
import type { Side, Widget } from './canvas-marks';
import { KINDS, accountsOf, summed } from './canvas-widgets';
import type { Scope } from './canvas-widgets';
import '../dashboard/dashboard.css';

const FIGURE: Record<Measure, string> = { context_percent: 'Context', tokens: 'Tokens', running_ms: 'Running', dollars: 'Dollars', plan_percent: 'Plan' };

/** A window of an account in words: 300 minutes is its 5-hour window, 10,080 its 7-day one. */
export function windowWords(minutes: number): string {
  if (minutes % 1440 === 0) return minutes / 1440 + '-day';
  return minutes % 60 === 0 ? minutes / 60 + '-hour' : minutes + '-minute';
}

const bar = (percent: number, says: string) => <span className="dash-bar" role="img" aria-label={says}>
  <span className={percent >= 100 ? 'full' : ''} style={{ width: Math.max(0, Math.min(100, percent)) + '%' }} /></span>;

type Look = Widget['look'];
const full = (percent: number): number => Math.max(0, Math.min(100, percent));

/** A level as a dial: a ring filled as far round as the level is high. Small it sits in a pill; large it holds the figure. */
export function Dial({ percent, large, children }: { percent: number; large?: boolean; children?: ReactNode }) {
  const [size, r, stroke] = large ? [132, 56, 10] : [18, 7, 3];
  const round = 2 * Math.PI * r;
  return <span className={'canvas-dial' + (large ? ' large' : '') + (percent >= 100 ? ' full' : '')} role="img" aria-label={Math.round(percent) + '%'}>
    <svg viewBox={`0 0 ${size} ${size}`} width={size} height={size} aria-hidden="true">
      <circle cx={size / 2} cy={size / 2} r={r} strokeWidth={stroke} className="canvas-dial-track" />
      <circle cx={size / 2} cy={size / 2} r={r} strokeWidth={stroke} className="canvas-dial-level" strokeDasharray={`${round * full(percent) / 100} ${round}`} transform={`rotate(-90 ${size / 2} ${size / 2})`} />
    </svg>{children}</span>;
}

/** One figure alone, large: what a widget set to show a single figure holds. A level is drawn the way the person chose: its number, a bar, or a dial. */
const stat = (key: string, figure: string, says: string, percent: number | null, look: Look, more?: ReactNode) => <div className={'canvas-stat' + (percent !== null && look ? ' ' + look : '')} key={key} data-stat={key}>
  {percent !== null && look === 'dial' ? <Dial percent={percent} large><b>{figure}</b></Dial> : <b>{figure}</b>}
  <span className="sec">{says}{more}</span>{percent !== null && look === 'bar' ? bar(percent, figure) : null}</div>;

/**
 * What the agents used: each figure once, added over the agents or at its highest, and each account's windows. Nothing
 * unreported is shown as zero. Set to show one figure, it shows that figure alone, large.
 */
function usagePart(scope: Scope, shows: string | undefined, look: Look): Part {
  const [lines, accounts] = [summed(scope.rows), accountsOf(scope.rows)];
  const unread = scope.rows.filter((row) => refusedPart(row.usage));
  const silent = scope.rows.filter((row) => !refusedPart(row.usage) && !tracking(row.usage).complete);
  const context = lines.find((line) => line.unit === 'context_percent' && line.figure !== null);
  const several = scope.rows.length !== 1;
  const level = (unit: Measure) => unit === 'context_percent' || unit === 'plan_percent';
  const reporting = scope.rows.length - silent.length - unread.length;
  const tokens = lines.find((line) => line.unit === 'tokens' && line.period === 'day' && line.figure !== null);
  const count = context?.figure != null ? amount('context_percent', context.figure) + ' context' : tokens?.figure != null ? amount('tokens', tokens.figure) + ' today'
    : !reporting ? 'No usage yet' : reporting + ' of ' + scope.rows.length + ' reporting';
  const nothing = !scope.rows.length ? 'No agent is counted here.' : several ? 'Not reported by these agents yet.' : 'Not reported by this agent yet.';
  if (shows?.startsWith('window/')) {
    const minutes = Number(shows.slice('window/'.length));
    const windows = accounts.flatMap((account) => account.windows.filter((each) => each.duration_minutes === minutes).map((each) => ({ account, each })));
    return { label: 'Usage', href: '#/people', count: '', percent: windows.length ? Math.max(...windows.map(({ each }) => each.used_percent)) : undefined,
      figure: windows.length ? windows.map(({ each }) => amount('plan_percent', each.used_percent)).join(', ') + ' of ' + windowWords(minutes) : 'Not reported', body: windows.length
      ? <>{windows.map(({ account, each }) => stat(account.account, amount('plan_percent', each.used_percent), windowWords(minutes) + ' window of ' + account.account + ', resets ' + clockMs(each.resets_at_ms), each.used_percent, look))}</>
      : <p className="dim canvas-stat-none">{nothing}</p> };
  }
  if (shows) {
    const line = lines.find((each) => each.unit + '/' + (each.period ?? '') === shows);
    return { label: 'Usage', href: '#/people', count: '', percent: line && line.figure !== null && level(line.unit) ? line.figure : undefined,
      figure: line && line.figure !== null ? amount(line.unit, line.figure) + (line.unit === 'context_percent' ? ' context' : line.period ? ' ' + THIS[line.period] : '') : 'Not reported', body: line && line.figure !== null
      ? stat(shows, amount(line.unit, line.figure), FIGURE[line.unit] + ' ' + (line.period ? THIS[line.period] : 'now') + (several && level(line.unit) ? ', highest' : ''), level(line.unit) ? line.figure : null, look,
        several && line.missing.length ? <span className="dim" title={line.missing.join('\n')}> ({line.reported} of {line.reported + line.missing.length} reporting)</span> : null)
      : <p className="dim canvas-stat-none" title={line?.missing.join('\n')}>{nothing}</p> };
  }
  return { label: 'Usage', href: '#/people', count, percent: context?.figure ?? undefined,
    body: <table className="dash-table"><tbody>
      {lines.map((line) => <tr key={line.unit + '/' + line.period} data-figure={line.unit} data-period={line.period ?? 'now'}>
        <td>{FIGURE[line.unit]} {line.period ? THIS[line.period] : 'now'}{several && level(line.unit) ? ', highest' : ''}</td>
        <td className="sec">{line.figure === null ? <span className="dim" title={line.missing.join('\n')}>Not reported</span> : <>
          {level(line.unit) ? bar(line.figure, amount(line.unit, line.figure)) : null}{amount(line.unit, line.figure)}</>}
          {several && line.figure !== null && line.missing.length ? <span className="dim" title={line.missing.join('\n')}> ({line.reported} of {line.reported + line.missing.length} reporting)</span> : null}</td>
      </tr>)}
      {accounts.flatMap((account) => account.windows.map((each) => <tr key={account.account + '/' + each.duration_minutes} data-account={account.account}>
        <td>{account.account}, {windowWords(each.duration_minutes)}</td>
        <td className="sec">{bar(each.used_percent, amount('plan_percent', each.used_percent) + ' used')}{amount('plan_percent', each.used_percent)} used, resets {clockMs(each.resets_at_ms)}</td>
      </tr>))}
      {unread.map((row) => <tr key={row.agent.id} data-usage="unread"><td>{row.agent.display_name}</td>
        <td className="why-not">Its usage could not be read. {refusedPart(row.usage) ? <small className="refusal-name" title={row.usage.reason}>{row.usage.refusal}</small> : null}</td></tr>)}
      {silent.length ? <tr className="empty" data-usage="silent"><td className="dim" colSpan={2}>{several
        ? 'No usage has been reported for ' + silent.map((row) => row.agent.display_name).join(', ') + '.'
        : 'No usage has been reported for this agent yet.'}</td></tr> : null}
      {scope.rows.length ? null : <tr className="empty"><td className="dim" colSpan={2}>No agent is counted here.</td></tr>}
    </tbody></table> };
}

/** What a widget of `kind` holds for the agents it counts; null for a kind this page does not know. */
export function partOf(kind: string, scope: Scope, board: Board, shows?: string, look?: Look): Part | null {
  const theirs = new Set(scope.rows.map((row) => row.agent.id));
  if (kind === 'usage') return usagePart(scope, shows, look);
  // A pill says there is none in words, not as a nought.
  const none = (part: Part, words: string, any: boolean): Part => any ? part : { ...part, figure: words };
  if (kind === 'budget') {
    const near = scope.rows.flatMap((row) => refusedPart(row.budget) ? [] : [tightest(row.budget)]).filter((each): each is number => each !== null);
    return { ...none(budgetPart({ rows: scope.rows }), 'No limit', scope.rows.some((row) => refusedPart(row.budget) || row.budget.limits.length > 0)), ...(near.length ? { percent: Math.max(...near) * 100 } : {}) };
  }
  if (kind === 'goals') return none(goalsPart({ rows: scope.rows }), 'No goal', scope.rows.some((row) => refusedPart(row.goals) || row.goals.goals.some((item) => item.goal.active)));
  // Of every agent, a list of what waits is the whole list; of one agent or a box, only what those agents asked for or prepared.
  if (kind === 'requests') {
    const requests = board.requests instanceof Refused || scope.everyone ? board.requests : board.requests.filter((each) => theirs.has(each.asked_by));
    return none(requestsPart({ me: board.me, requests }), 'None waiting', requests instanceof Refused || requests.some((each) => each.state === 'waiting'));
  }
  if (kind === 'drafts') {
    const drafts = board.drafts instanceof Refused || scope.everyone ? board.drafts : board.drafts.filter((each) => each.agent !== null && theirs.has(each.agent.id));
    return none(draftsPart({ drafts }), 'None waiting', drafts instanceof Refused || drafts.length > 0);
  }
  return null;
}

type Press = (event: PointerEvent<HTMLElement>) => void;

/**
 * A widget on the surface. It is a pill: its kind's symbol, whose it is, and its one figure. Its own button takes it
 * through its views: opened out to everything it holds, then its settings (which figure it shows, and taking it away),
 * then a pill again. Pressing it anywhere else only chooses it and moves it.
 */
export function WidgetCard({ widget, scope, board, morph, chosen, pick, move, size, link, remove, set, fit }: {
  widget: Widget; scope: Scope | null; board: Load<Board>;
  /** It has just changed view, and moves between the two sizes. */
  morph: boolean;
  /** It was the last thing pressed: it holds out the small cross that takes it away. */
  chosen: boolean;
  pick: Press | undefined; move: Press; size: Press; link: (side: Side) => Press; remove: () => void;
  /** Changes what the person chose for it: which figure it shows, how a level is drawn, the view it is in. */
  set: (change: Pick<Widget, 'shows'> | Pick<Widget, 'look'> | Pick<Widget, 'view'> | Pick<Widget, 'colour'>) => void;
  /** Opened out, it is as tall as what it holds, so nothing in it has to be scrolled to; this says how tall that came to. */
  fit: (height: number) => void;
}) {
  const card = useRef<HTMLElement>(null);
  useLayoutEffect(() => {
    const height = card.current?.offsetHeight ?? 0;
    if (widget.view && height > 0 && height !== widget.h) fit(height);
  });
  const label = KINDS[widget.kind]?.label ?? 'Widget';
  const part = scope && board.status === 'ok' ? partOf(widget.kind, scope, board.data, widget.shows, widget.look) : null;
  const choices = KINDS[widget.kind]?.choices;
  const whose = scope?.whose ?? '';
  const now = standing(widget);
  let body: ReactNode;
  if (board.status === 'loading') body = <p role="status">Reading…</p>;
  else if (board.status === 'refused') body = <p className="why-not" role="alert">This could not be read. <small className="refusal-name" title={board.refused.refusal.reason}>{board.refused.refusal.refusal}</small></p>;
  else body = part ? part.body : <p className="why-not" role="alert">This page does not know a widget of the kind “{widget.kind}”.</p>;
  const figure = board.status === 'refused' ? board.refused.refusal.refusal : part ? part.figure ?? part.count : board.status === 'loading' ? '…' : 'Unknown kind';
  const next = widget.view === undefined ? 'Open it out' : widget.view === 'detail' ? 'Its settings' : 'Back to a pill';
  return <article className={'canvas-widget' + (widget.view ? ' wide' : '') + (morph ? ' morph' : '') + (pick ? ' picking' : '')} data-widget={widget.id} data-kind={widget.kind} data-view={widget.view ?? 'pill'}
    ref={card} aria-label={label + (whose ? ' of ' + whose : '')} style={{ left: now.x, top: now.y, width: now.w, height: widget.view ? undefined : now.h, ...tint(widget.colour) }} onPointerDownCapture={pick}
    onContextMenu={(event) => { event.preventDefault(); set({ view: 'settings' }); }}>
    <header className="canvas-widget-bar" onPointerDown={move}>
      <KindSymbol kind={widget.kind} /><h3>{whose || label}</h3>{part?.percent !== undefined && widget.look === 'dial' ? <Dial percent={part.percent} /> : null}<b className="canvas-widget-figure">{figure}</b>
      <button type="button" className="canvas-widget-turn" data-act="widget-view" aria-label={next + ': ' + label + (whose ? ' of ' + whose : '')} title={next} onClick={() => set({ view: nextView(widget.view) })}>
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d={widget.view === 'settings' ? 'M7 14l5-5 5 5' : 'M7 10l5 5 5-5'} /></svg></button>
      {part?.percent !== undefined && widget.look === 'bar' ? <span className={'canvas-widget-level' + (part.percent >= 100 ? ' full' : '')} aria-hidden="true"><span style={{ width: full(part.percent) + '%' }} /></span> : null}
    </header>
    {widget.view === 'detail' ? <div className="canvas-widget-body">{body}</div> : null}
    {widget.view === 'settings' ? <div className="canvas-widget-body canvas-widget-settings">
      <table className="dash-table"><tbody>
        <tr><td>Widget</td><td className="sec">{label}</td></tr>
        <tr><td>Counts</td><td className="sec">{whose || 'Nothing yet'}</td></tr>
        {choices ? <tr><td>Shows</td><td><div className="canvas-widget-variants" role="toolbar" aria-label={'What this ' + label + ' widget shows'}>
          {[['', 'Everything'], ...Object.entries(choices)].map(([key, name]) => <button key={key} type="button" data-variant={key} aria-pressed={(widget.shows ?? '') === key} onClick={() => set({ shows: key || undefined })}>{name}</button>)}
        </div></td></tr> : null}
        {part?.percent !== undefined ? <tr><td>Drawn as</td><td><div className="canvas-widget-variants" role="toolbar" aria-label="How its level is drawn">
          {([[undefined, 'Number'], ['bar', 'Bar'], ['dial', 'Dial']] as const).map(([key, name]) => <button key={name} type="button" data-look={key ?? 'number'} aria-pressed={widget.look === key} onClick={() => set({ look: key })}>{name}</button>)}
        </div></td></tr> : null}
        {part ? <tr><td>Whole page</td><td><a href={part.href}>Open {label.toLowerCase()}</a></td></tr> : null}
        <tr><td>Colour</td><td><div className="canvas-swatches" role="toolbar" aria-label="Its colour">
          {[undefined, ...Object.keys(COLOURS)].map((key) => <button key={key ?? 'own'} type="button" className="canvas-swatch" data-colour={key ?? 'own'} aria-label={key ?? 'Lys’s own'} title={key ?? 'Lys’s own'} aria-pressed={widget.colour === key} style={tint(key)} onClick={() => set({ colour: key })} />)}
        </div></td></tr>
      </tbody></table>
    </div> : null}
    {widget.view ? <span className="session-canvas-grip" aria-hidden="true" onPointerDown={size} /> : null}
    <Anchors from={link} />
    {chosen || widget.view === 'settings' ? <button type="button" className="canvas-widget-remove" data-act="remove-widget" aria-label={'Remove the ' + label + ' widget' + (whose ? ' of ' + whose : '')} title="Remove" onClick={remove}>×</button> : null}
  </article>;
}
