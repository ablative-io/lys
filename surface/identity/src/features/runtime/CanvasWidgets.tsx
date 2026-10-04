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
import type { AccountWindows, Measure } from '../usage/contract';
import { KindSymbol } from './CanvasDock';
import { Anchors } from './CanvasMarks';
import { COLOURS, nextView, standing, tint } from './canvas-marks';
import type { Group, Link, Side, Widget } from './canvas-marks';
import { counted } from '../../shell/count';
import { COMBINES, KINDS, accountsOf, combined, scopeOf, summed } from './canvas-widgets';
import type { AgentWindow, Scope, Summed, Value } from './canvas-widgets';
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

/** A level as a dial: a ring filled as far round as the level is high. Small it sits in a pill; in a face it holds the figure. */
export function Dial({ percent, face, children }: { percent: number; face?: 'mid' | 'large'; children?: ReactNode }) {
  const [size, r, stroke] = face === 'large' ? [132, 56, 10] : face === 'mid' ? [84, 35, 7] : [18, 7, 3];
  const round = 2 * Math.PI * r;
  return <span className={'canvas-dial' + (face ? ' ' + face : '') + (percent >= 100 ? ' full' : '')} role="img" aria-label={Math.round(percent) + '%'}>
    <svg viewBox={`0 0 ${size} ${size}`} width={size} height={size} aria-hidden="true">
      <circle cx={size / 2} cy={size / 2} r={r} strokeWidth={stroke} className="canvas-dial-track" />
      <circle cx={size / 2} cy={size / 2} r={r} strokeWidth={stroke} className="canvas-dial-level" strokeDasharray={`${round * full(percent) / 100} ${round}`} transform={`rotate(-90 ${size / 2} ${size / 2})`} />
    </svg>{children}</span>;
}

/** A figure as a face holds it: its number, what it is in words, its level when it is one, and how a pill ends it. */
interface Figure { key: string; figure: string; says: string; percent: number | null; tail: string; value: Value; more?: ReactNode }

const level = (unit: Measure) => unit === 'context_percent' || unit === 'plan_percent';

/**
 * One of the figures a usage widget can show, by its name, as the agents reported it: a window of an account once for
 * each account that reported it, any other figure once. Nothing when nobody reported it; never a nought.
 */
function figuresOf(key: string, lines: Summed[], accounts: AccountWindows[], several: boolean): Figure[] {
  if (key.startsWith('window/')) {
    const minutes = Number(key.slice('window/'.length));
    return accounts.flatMap((account) => account.windows.filter((each) => each.duration_minutes === minutes).map((each) => ({ key: key + '/' + account.account,
      figure: amount('plan_percent', each.used_percent), says: windowWords(minutes) + ' window of ' + account.account + ', resets ' + clockMs(each.resets_at_ms), percent: each.used_percent, tail: ' of ' + windowWords(minutes), value: { amount: each.used_percent, unit: 'plan_percent' } })));
  }
  const line = lines.find((each) => each.unit + '/' + (each.period ?? '') === key);
  if (!line || line.figure === null) return [];
  return [{ key, figure: amount(line.unit, line.figure), says: FIGURE[line.unit] + ' ' + (line.period ? THIS[line.period] : 'now') + (several && level(line.unit) ? ', highest' : ''),
    percent: level(line.unit) ? line.figure : null, tail: line.unit === 'context_percent' ? ' context' : line.period ? ' ' + THIS[line.period] : '', value: { amount: line.figure, unit: line.unit },
    more: several && line.missing.length ? <span className="dim" title={line.missing.join('\n')}> ({line.reported} of {line.reported + line.missing.length} reporting)</span> : undefined }];
}

/** One figure on a face. A level is drawn the way the person chose: its number, a bar, or a dial. */
const stat = (each: Figure, look: Look, face: 'mid' | 'large') => <div className={'canvas-stat ' + face + (each.percent !== null && look ? ' ' + look : '')} key={each.key} data-stat={each.key}>
  {each.percent !== null && look === 'dial' ? <Dial percent={each.percent} face={face}><b>{each.figure}</b></Dial> : <b>{each.figure}</b>}
  <span className="sec">{each.says}{each.more}</span>{each.percent !== null && look === 'bar' ? bar(each.percent, each.figure) : null}</div>;

/** What a widget holds: the part the Dashboard shows, and for a kind with figures to choose among, its medium face. */
export interface Held extends Part {
  /** The medium face: the chosen figures side by side. */
  faces?: ReactNode;
  /** One of the figures it shows is a level, so how a level is drawn can be chosen. */
  levelled?: boolean;
  /** Its one figure as a number, when it has one: what a formula it feeds works on. Several accounts' windows feed their highest. */
  value?: Value;
}
type Chosen = Pick<Widget, 'shows' | 'faces' | 'look'>;

/**
 * What the agents used. Small, it is one figure: the one the person chose, or the first reported. Medium, it is the
 * chosen figures side by side. Large, it is every figure once, added over the agents or at its highest, and each
 * account's windows. Nothing unreported is shown as zero.
 */
function usagePart(scope: Scope, { shows, faces, look }: Chosen): Held {
  const [lines, accounts] = [summed(scope.rows), accountsOf(scope.rows)];
  const unread = scope.rows.filter((row) => refusedPart(row.usage));
  const silent = scope.rows.filter((row) => !refusedPart(row.usage) && !tracking(row.usage).complete);
  const context = lines.find((line) => line.unit === 'context_percent' && line.figure !== null);
  const several = scope.rows.length !== 1;
  const reporting = scope.rows.length - silent.length - unread.length;
  const tokens = lines.find((line) => line.unit === 'tokens' && line.period === 'day' && line.figure !== null);
  const count = context?.figure != null ? amount('context_percent', context.figure) + ' context' : tokens?.figure != null ? amount('tokens', tokens.figure) + ' today'
    : !reporting ? 'No usage yet' : reporting + ' of ' + scope.rows.length + ' reporting';
  const read = (key: string) => figuresOf(key, lines, accounts, several);
  const small = shows ? read(shows) : null;
  const levels = small?.flatMap((each) => each.percent ?? []) ?? [];
  const names = KINDS.usage.choices ?? {};
  const medium = (faces ?? KINDS.usage.faces ?? []).map((key) => ({ key, held: read(key) }));
  const face = medium.reduce((sum, each) => sum + Math.max(1, each.held.length), 0) === 1 ? 'large' : 'mid';
  const why = (key: string) => lines.find((each) => each.unit + '/' + (each.period ?? '') === key)?.missing.join('\n');
  return { label: 'Usage', href: '#/people', count, ...(small ? { figure: small.length ? small.map((each) => each.figure).join(', ') + small[0].tail : 'Not reported' } : {}),
    percent: small ? (levels.length ? Math.max(...levels) : undefined) : context?.figure ?? undefined,
    value: small ? (small.length ? { amount: Math.max(...small.map((each) => each.value.amount)), unit: small[0].value.unit } : undefined)
      : context?.figure != null ? { amount: context.figure, unit: 'context_percent' } : tokens?.figure != null ? { amount: tokens.figure, unit: 'tokens' } : undefined,
    levelled: medium.some((each) => each.held.some((held) => held.percent !== null)),
    faces: !scope.rows.length ? <p className="dim canvas-stat-none">No agent is counted here.</p>
      : !medium.length ? <p className="dim canvas-stat-none">No figure is chosen for this face.</p>
      : <div className="canvas-faces">{medium.flatMap(({ key, held }) => held.length ? held.map((each) => stat(each, look, face))
        : [<div className={'canvas-stat none ' + face} key={key} data-stat={key} title={why(key)}><b>Not reported</b><span className="sec">{names[key] ?? key}</span></div>])}</div>,
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
export function partOf(kind: string, scope: Scope, board: Board, chosen: Chosen = {}): Held | null {
  const theirs = new Set(scope.rows.map((row) => row.agent.id));
  if (kind === 'usage') return usagePart(scope, chosen);
  // A pill says there is none in words, not as a nought.
  const none = (part: Part, words: string, any: boolean): Part => any ? part : { ...part, figure: words };
  if (kind === 'budget') {
    const near = scope.rows.flatMap((row) => refusedPart(row.budget) ? [] : [tightest(row.budget)]).filter((each): each is number => each !== null);
    return { ...none(budgetPart({ rows: scope.rows }), 'No limit', scope.rows.some((row) => refusedPart(row.budget) || row.budget.limits.length > 0)), ...(near.length ? { percent: Math.max(...near) * 100, value: { amount: Math.max(...near) * 100, unit: 'percent' } } : {}) };
  }
  const many = (amount: number): Value => ({ amount, unit: 'count' });
  if (kind === 'goals') {
    const active = scope.rows.flatMap((row) => refusedPart(row.goals) ? [] : row.goals.goals.filter((item) => item.goal.active));
    return { ...none(goalsPart({ rows: scope.rows }), 'No goal', scope.rows.some((row) => refusedPart(row.goals)) || active.length > 0), value: many(active.length) };
  }
  // Of every agent, a list of what waits is the whole list; of one agent or a box, only what those agents asked for or prepared.
  if (kind === 'requests') {
    const requests = board.requests instanceof Refused || scope.everyone ? board.requests : board.requests.filter((each) => theirs.has(each.asked_by));
    return { ...none(requestsPart({ me: board.me, requests }), 'None waiting', requests instanceof Refused || requests.some((each) => each.state === 'waiting')), ...(requests instanceof Refused ? {} : { value: many(requests.filter((each) => each.state === 'waiting').length) }) };
  }
  if (kind === 'drafts') {
    const drafts = board.drafts instanceof Refused || scope.everyone ? board.drafts : board.drafts.filter((each) => each.agent !== null && theirs.has(each.agent.id));
    return { ...none(draftsPart({ drafts }), 'None waiting', drafts instanceof Refused || drafts.length > 0), ...(drafts instanceof Refused ? {} : { value: many(drafts.length) }) };
  }
  return null;
}

const LEVELS: ReadonlySet<string> = new Set(['context_percent', 'plan_percent', 'percent']);
/** A combined figure in words: an amount in its own unit's words, a count as its number. */
const said = (value: Value): string => value.unit === 'count' ? value.amount.toLocaleString('en-AU', { maximumFractionDigits: 1 })
  : value.unit === 'percent' ? Math.round(value.amount) + '%' : amount(value.unit as Measure, value.amount);

/** A widget that feeds a formula: what it is, whose it is, its figure in words and as a number. */
interface Input { id: string; label: string; whose: string; figure: ReactNode; value?: Value }

/** What a formula holds: the figures of the widgets fed to it, combined the way the person chose. */
function formulaPart(by: string, inputs: Input[]): Held {
  const result = combined(by, inputs.flatMap((each) => each.value ?? []));
  const figure = result === 'none' ? (inputs.length ? 'Nothing reported' : 'No input') : result === 'mixed' ? 'Mixed units' : said(result);
  const value = typeof result === 'object' ? result : undefined;
  return { label: 'Formula', href: '', count: '', figure, value, ...(value && LEVELS.has(value.unit) ? { percent: value.amount } : {}),
    body: <table className="dash-table"><tbody>
      {inputs.map((each) => <tr key={each.id} data-input={each.id}><td>{each.label}{each.whose ? ' of ' + each.whose : ''}</td><td className="sec">{each.figure}</td></tr>)}
      {inputs.length ? null : <tr className="empty"><td className="dim" colSpan={2}>Draw a line from a widget to this one to feed it.</td></tr>}
      <tr data-result={by}><td>{COMBINES[by] ?? COMBINES.sum}</td><td className="sec"><b>{figure}</b>{result === 'mixed' ? <span className="dim"> Figures of different kinds are not combined.</span> : null}</td></tr>
    </tbody></table> };
}

/** Everything on the surface a widget's figure can come from. */
export interface Surface { widgets: Widget[]; links: Link[]; groups: Group[]; windows: AgentWindow[]; board: Board }
/** A widget as it stands: whose it is in words, and what it holds; nothing held for a kind this page does not know. */
export interface Standing { whose: string; part: Held | null }

/**
 * What a widget holds. A formula holds the figures of the widgets a line joins it to: any widget feeds it whichever way
 * the line was drawn, and another formula feeds it only by a line drawn from that formula to this one, so two formulas
 * never feed each other round in a ring.
 */
export function heldOf(widget: Widget, on: Surface, within: ReadonlySet<string> = new Set()): Standing {
  if (KINDS[widget.kind]?.takes !== 'widgets') {
    const scope = scopeOf(widget, on.links, on.groups, on.windows, on.board.rows);
    return { whose: scope.whose, part: partOf(widget.kind, scope, on.board, widget) };
  }
  const inside = new Set([...within, widget.id]);
  const inputs = on.links.flatMap((link): Input[] => {
    const fed = on.widgets.find((each) => each.id === (link.to === widget.id ? link.from : link.from === widget.id ? link.to : null));
    if (!fed || inside.has(fed.id) || (KINDS[fed.kind]?.takes === 'widgets' && link.to !== widget.id)) return [];
    const held = heldOf(fed, on, inside);
    return [{ id: fed.id, label: KINDS[fed.kind]?.label ?? fed.kind, whose: held.whose, value: held.part?.value, figure: held.part ? held.part.figure ?? held.part.count : 'Unknown kind' }];
  });
  return { whose: counted(inputs.length, 'inputs'), part: formulaPart(widget.shows ?? 'sum', inputs) };
}

type Press = (event: PointerEvent<HTMLElement>) => void;

/**
 * A widget on the surface. Small, it is a pill: its kind's symbol, whose it is, and its one figure. Its own button takes
 * it through its faces: medium (a few figures side by side, for a kind that has figures to choose among), large
 * (everything it holds), then its settings, then a pill again. Pressing it anywhere else only chooses it and moves it.
 * It is as large as what it holds; it is not dragged to a size.
 */
export function WidgetCard({ widget, held, board, morph, chosen, pick, move, link, remove, set, fit }: {
  widget: Widget;
  /** Whose it is and what it holds, once the board is read. */
  held: Standing | null; board: Load<Board>;
  /** It has just changed view, and moves between the two sizes. */
  morph: boolean;
  /** It was the last thing pressed: it holds out the small cross that takes it away. */
  chosen: boolean;
  pick: Press | undefined; move: Press; link: (side: Side) => Press; remove: () => void;
  /** Changes what the person chose for it: the figures its faces show, how a level is drawn, the view it is in. */
  set: (change: Pick<Widget, 'shows'> | Pick<Widget, 'faces'> | Pick<Widget, 'look'> | Pick<Widget, 'view'> | Pick<Widget, 'colour'> | Pick<Widget, 'locked'>) => void;
  /** Opened out, it is as tall as what it holds, so nothing in it has to be scrolled to; this says how tall that came to. */
  fit: (height: number) => void;
}) {
  const card = useRef<HTMLElement>(null);
  useLayoutEffect(() => {
    const height = card.current?.offsetHeight ?? 0;
    if (widget.view && height > 0 && height !== widget.h) fit(height);
  });
  const label = KINDS[widget.kind]?.label ?? 'Widget';
  const part = held?.part ?? null;
  const kind = KINDS[widget.kind];
  const choices = kind?.choices;
  // A kind that names no way of choosing none always has one chosen: its first, until the person chooses.
  const shown = widget.shows ?? (kind?.unset === undefined ? Object.keys(choices ?? {})[0] : '');
  const whose = held?.whose ?? '';
  const now = standing(widget);
  let body: ReactNode;
  if (board.status === 'loading') body = <p role="status">Reading…</p>;
  else if (board.status === 'refused') body = <p className="why-not" role="alert">This could not be read. <small className="refusal-name" title={board.refused.refusal.reason}>{board.refused.refusal.refusal}</small></p>;
  else body = part ? part.body : <p className="why-not" role="alert">This page does not know a widget of the kind “{widget.kind}”.</p>;
  const figure = board.status === 'refused' ? board.refused.refusal.refusal : part ? part.figure ?? part.count : board.status === 'loading' ? '…' : 'Unknown kind';
  const turned = nextView(widget.view, !!kind?.faces);
  const next = turned === 'faces' ? 'Medium face' : turned === 'detail' ? 'Large face' : turned === 'settings' ? 'Its settings' : 'Small face';
  const medium = widget.faces ?? kind?.faces ?? [];
  // Chosen figures stay in the order the kind offers them, whichever was pressed first.
  const faced = (key: string) => Object.keys(choices ?? {}).filter((each) => each === key ? !medium.includes(key) : medium.includes(each));
  return <article className={'canvas-widget' + (widget.view ? ' wide' : '') + (morph ? ' morph' : '') + (pick ? ' picking' : '')} data-widget={widget.id} data-kind={widget.kind} data-view={widget.view ?? 'pill'}
    ref={card} aria-label={label + (whose ? ' of ' + whose : '')} style={{ left: now.x, top: now.y, width: now.w, height: widget.view ? undefined : now.h, ...tint(widget.colour) }} onPointerDownCapture={pick}
    onContextMenu={(event) => { event.preventDefault(); if (!widget.locked) set({ view: 'settings' }); }}>
    <header className="canvas-widget-bar" onPointerDown={widget.locked ? undefined : move}>
      <KindSymbol kind={widget.kind} /><h3>{whose || label}</h3>{part?.percent !== undefined && widget.look === 'dial' ? <Dial percent={part.percent} /> : null}<b className="canvas-widget-figure">{figure}</b>
      <button type="button" className="canvas-widget-turn" data-act="widget-view" aria-label={next + ': ' + label + (whose ? ' of ' + whose : '')} title={next} disabled={widget.locked} onClick={() => set({ view: turned })}>
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d={widget.view === 'settings' ? 'M7 14l5-5 5 5' : 'M7 10l5 5 5-5'} /></svg></button>
      {part?.percent !== undefined && widget.look === 'bar' ? <span className={'canvas-widget-level' + (part.percent >= 100 ? ' full' : '')} aria-hidden="true"><span style={{ width: full(part.percent) + '%' }} /></span> : null}
    </header>
    {widget.view === 'faces' ? <div className="canvas-widget-body">{board.status === 'ok' && part?.faces ? part.faces : body}</div> : null}
    {widget.view === 'detail' ? <div className="canvas-widget-body">{body}</div> : null}
    {widget.view === 'settings' ? <div className="canvas-widget-body canvas-widget-settings">
      <table className="dash-table"><tbody>
        <tr><td>Widget</td><td className="sec">{label}</td></tr>
        <tr><td>Counts</td><td className="sec">{whose || 'Nothing yet'}</td></tr>
        {choices ? <tr><td>{kind?.chooses ?? 'Shows'}</td><td><div className="canvas-widget-variants" role="toolbar" aria-label={(kind?.chooses ?? 'Shows') + ', of this ' + label + ' widget'}>
          {[...(kind?.unset === undefined ? [] : [['', kind.unset]]), ...Object.entries(choices)].map(([key, name]) => <button key={key} type="button" data-variant={key} aria-pressed={shown === key} onClick={() => set({ shows: key || undefined })}>{name}</button>)}
        </div></td></tr> : null}
        {choices && kind?.faces ? <tr><td>Medium face</td><td><div className="canvas-widget-variants" role="toolbar" aria-label={'The figures this ' + label + ' widget shows side by side'}>
          {Object.entries(choices).map(([key, name]) => <button key={key} type="button" data-face={key} aria-pressed={medium.includes(key)} onClick={() => set({ faces: faced(key) })}>{name}</button>)}
        </div></td></tr> : null}
        {part?.percent !== undefined || part?.levelled ? <tr><td>Drawn as</td><td><div className="canvas-widget-variants" role="toolbar" aria-label="How its level is drawn">
          {([[undefined, 'Number'], ['bar', 'Bar'], ['dial', 'Dial']] as const).map(([key, name]) => <button key={name} type="button" data-look={key ?? 'number'} aria-pressed={widget.look === key} onClick={() => set({ look: key })}>{name}</button>)}
        </div></td></tr> : null}
        {part?.href ? <tr><td>Whole page</td><td><a href={part.href}>Open {label.toLowerCase()}</a></td></tr> : null}
        <tr><td>Colour</td><td><div className="canvas-swatches" role="toolbar" aria-label="Its colour">
          {[undefined, ...Object.keys(COLOURS)].map((key) => <button key={key ?? 'own'} type="button" className="canvas-swatch" data-colour={key ?? 'own'} aria-label={key ?? 'Lys’s own'} title={key ?? 'Lys’s own'} aria-pressed={widget.colour === key} style={tint(key)} onClick={() => set({ colour: key })} />)}
        </div></td></tr>
      </tbody></table>
    </div> : null}
    {/* The lock: closed, the widget is not moved, changed or taken away by a stray press. */}
    <button type="button" className="canvas-widget-lock" data-act="widget-lock" aria-pressed={!!widget.locked} aria-label={(widget.locked ? 'Unlock the ' : 'Lock the ') + label + ' widget' + (whose ? ' of ' + whose : '')} title={widget.locked ? 'Unlock' : 'Lock'}
      onClick={() => set({ locked: widget.locked ? undefined : true })}>
      <svg viewBox="0 0 24 24" aria-hidden="true"><path className="canvas-lock-shackle" d="M8 11V8a4 4 0 0 1 8 0v3" /><rect x="6" y="11" width="12" height="9" rx="2" /></svg></button>
    <Anchors from={link} />
    {(chosen || widget.view === 'settings') && !widget.locked ? <button type="button" className="canvas-widget-remove" data-act="remove-widget" aria-label={'Remove the ' + label + ' widget' + (whose ? ' of ' + whose : '')} title="Remove" onClick={remove}>×</button> : null}
  </article>;
}
