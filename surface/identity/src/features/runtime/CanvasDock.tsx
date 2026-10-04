/**
 * What hovers over the canvas and never takes it over: one small bar of symbols at its bottom right, and one small panel
 * that opens from the bar. The bar holds three buttons that open the panel: the agents, to find one and go to it; the saved layouts; the connections in words. Nothing is laid over the
 * surface until it is asked for, so the bar is the same size with three agents or three hundred. The drawing tools (a box, a
 * line, a note) are their own bar to its left, apart, and the zoom is its own small bar at the bottom left. Each bar is the
 * person's to move: dragged by its grip to another corner of the canvas, it stays there in this browser.
 */
import { useEffect, useRef, useState } from 'react';
import type { ReactNode } from 'react';
import { useLocation } from 'react-router';
import { api } from '../../api';
import { entries } from '../people/directory';
import type { Entry } from '../people/directory';
import { clock } from '../file/time';
import { RunningList } from './Sessions';
import type { SessionGraph } from './session-graph';
import { said } from './canvas-kept';
import { KINDS } from './canvas-widgets';
import type { Keeping, SavedLayout } from './canvas-kept';

export type Tool = 'box' | 'line' | 'note' | 'widget' | null;
export type Panel = 'agents' | 'layouts' | 'connections' | 'widgets' | null;
export const PANELS: Record<Exclude<Panel, null>, string> = { agents: 'Agents', layouts: 'Layouts', connections: 'Connections', widgets: 'Widgets' };

/** The bar's symbols, drawn in the rail's own line style. */
const SYMBOLS = {
  agents: <><circle cx="9" cy="8" r="3.2" /><path d="M3.5 19c.8-3.2 3-5 5.5-5s4.7 1.8 5.5 5" /><circle cx="17" cy="9" r="2.4" /><path d="M15.5 14.2c2.3.2 4 1.8 4.6 4.8" /></>,
  box: <><rect x="3.5" y="7" width="17" height="13" rx="2" strokeDasharray="3 2.5" /><path d="M6 4h6" /></>,
  line: <><rect x="3" y="4" width="7" height="6" rx="1.2" /><rect x="14" y="14" width="7" height="6" rx="1.2" /><path d="M10 9l4 6" /></>,
  note: <path d="M5 4h14v10l-6 6H5zM13 20v-6h6" />,
  widget: <><rect x="3.5" y="4" width="17" height="16" rx="2" /><path d="M7.5 16v-3M12 16V8M16.5 16v-5" /></>,
  layouts: <><rect x="3.5" y="4" width="7.5" height="7" rx="1.2" /><rect x="13" y="4" width="7.5" height="7" rx="1.2" /><rect x="3.5" y="13" width="7.5" height="7" rx="1.2" /><rect x="13" y="13" width="7.5" height="7" rx="1.2" /></>,
  connections: <><circle cx="6" cy="7" r="2.2" /><circle cx="18" cy="6" r="2.2" /><circle cx="12" cy="17" r="2.2" /><path d="M8.1 7.6 15.8 6.4M7.1 9 10.9 15M16.9 8 13.1 15" /></>,
  out: <path d="M6 12h12" />,
  in: <path d="M6 12h12M12 6v12" />,
  home: <><path d="M4 9V5a1 1 0 0 1 1-1h4M20 9V5a1 1 0 0 0-1-1h-4M4 15v4a1 1 0 0 0 1 1h4M20 15v4a1 1 0 0 1-1 1h-4" /><rect x="9" y="9" width="6" height="6" rx="1" /></>,
};

/** One symbol on the bar. What it does is its name for a reader and its tip for a pointer. */
function Symbol({ act, says, on, pressed, expanded, count }: { act: string; says: string; on: () => void; pressed?: boolean; expanded?: boolean; count?: number }) {
  return <button type="button" className="canvas-symbol" data-act={act} aria-label={says} title={says} aria-pressed={pressed} aria-expanded={expanded} onClick={on}>
    <svg viewBox="0 0 24 24" aria-hidden="true">{SYMBOLS[act.replace(/^(draw|add|zoom)-/, '') as keyof typeof SYMBOLS]}</svg>
    {count === undefined ? null : <span className="canvas-symbol-count">{count}</span>}
  </button>;
}

/** Find an agent by name: the running ones first, each going to its window here; then the ones that are not running, each going to its own page. */
function Agents({ graph, show, open }: { graph: SessionGraph; show: (node: string) => void; open: boolean }) {
  const [query, setQuery] = useState('');
  const [agents, setAgents] = useState<Entry[] | null>(null);
  const [refused, setRefused] = useState<string | null>(null);
  const field = useRef<HTMLInputElement>(null);
  // The agents that are not running are read when the panel first opens, and the search takes the keyboard each time it does.
  useEffect(() => {
    if (!open) return;
    field.current?.focus();
    if (agents) return;
    api.people().then((answer) => { setAgents(entries(answer).filter((each) => each.kind === 'agent')); setRefused(null); }, (error: unknown) => setRefused(said(error)));
  }, [open]);
  const running = new Set(graph.nodes.flatMap((node) => node.session?.agent ? [node.session.agent] : []));
  const wanted = query.trim().toLowerCase();
  const idle = (agents ?? []).filter((each) => each.state !== 'retired' && !running.has(each.id) && each.display_name.toLowerCase().includes(wanted));
  return <div className="canvas-find">
    <input ref={field} type="search" aria-label="Find an agent" placeholder="Find an agent" value={query} tabIndex={open ? 0 : -1} onChange={(event) => setQuery(event.target.value)}
      onKeyDown={(event) => { if (event.key === 'Enter') event.currentTarget.closest('.canvas-find')?.querySelector<HTMLAnchorElement>('.canvas-find-list a')?.click(); }} />
    <div className="canvas-find-list">
      <RunningList named={query} go={(session) => show('session:' + session)} />
      {idle.map((each) => <a key={each.id} href={'#/file/' + each.id} data-find="idle"><span>{each.display_name}</span><span className="sec">Not running</span></a>)}
      {refused ? <p className="why-not" role="alert">The agents that are not running could not be read. <small className="refusal-name">{refused}</small></p> : null}
      {agents && !idle.length && wanted && ![...graph.nodes].some((node) => node.session && node.title.toLowerCase().includes(wanted)) ? <p className="dim">No agent by that name.</p> : null}
    </div>
  </div>;
}

/**
 * The saved layouts as one table: each is saved over or removed where it stands, and the last row saves the canvas as it
 * is under a new name. A layout's name is its link: it opens the layout, and it is an address that can be kept or sent.
 */
function Layouts({ keeping, layouts, save, remove }: {
  keeping: Keeping; layouts: SavedLayout[]; save: (name: string) => Promise<void>; remove: (name: string) => Promise<void>;
}) {
  const { pathname } = useLocation();
  const [name, setName] = useState('');
  const [refused, setRefused] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const doing = (work: Promise<void>, after?: () => void) => {
    setBusy(true);
    work.then(() => { setRefused(null); after?.(); }, (error: unknown) => setRefused(said(error))).finally(() => setBusy(false));
  };
  const wanted = name.trim();
  return <>
    <form id="canvas-save-layout" aria-label="Save this layout" onSubmit={(event) => { event.preventDefault(); if (wanted && !busy) doing(save(wanted), () => setName('')); }} />
    <table className="canvas-layouts"><thead><tr><th>Layout</th><th>Saved</th><th>Change</th></tr></thead>
      <tbody>
        {layouts.map((layout) => <tr key={layout.name} data-layout={layout.name}>
          <td><a data-act="open-layout" href={'#' + pathname + '?panel=layouts&layout=' + encodeURIComponent(layout.name)} title="Open this layout">{layout.name}</a></td><td className="sec">{clock(layout.saved_at)}</td>
          <td className="canvas-layout-acts">
            <button type="button" className="btn" data-act="save-over" disabled={busy} onClick={() => doing(save(layout.name))}>Save over</button>
            <button type="button" className="btn" data-act="remove-layout" disabled={busy} onClick={() => doing(remove(layout.name))}>Remove</button>
          </td></tr>)}
        {layouts.length ? null : <tr className="empty"><td colSpan={3} className="dim">No layout is saved yet.</td></tr>}
      </tbody>
      <tfoot><tr data-add="layout">
        <td colSpan={2}><input form="canvas-save-layout" name="name" aria-label="Name for this layout" placeholder="Name this layout" value={name} onChange={(event) => setName(event.target.value)} /></td>
        <td><button form="canvas-save-layout" type="submit" className="btn primary" disabled={!wanted || busy}>{layouts.some((each) => each.name === wanted) ? 'Save over' : 'Save'}</button></td>
      </tr></tfoot>
    </table>
    <p className="note">{keeping.where === 'service'
      ? 'Your layouts are yours: kept on the service, the same on any computer you sign in at.'
      : <>Kept in this browser only: the service is not keeping your canvas. <small className="refusal-name">{keeping.why}</small></>}</p>
    {refused ? <p className="why-not" role="alert">That was not done. <small className="refusal-name">{refused}</small></p> : null}
  </>;
}

/** A kind of widget's own symbol, the same wherever the kind is named. */
export function KindSymbol({ kind }: { kind: string }) {
  return <svg className="canvas-kind-symbol" viewBox="0 0 24 24" aria-hidden="true"><path d={KINDS[kind]?.symbol ?? 'M5 5h14v14H5z'} /></svg>;
}

/**
 * Every kind of widget: its symbol and its name. One is dragged onto the canvas and goes where it is let go; pressed
 * without dragging it is held as a tool, and the next press on the canvas places it.
 */
function Widgets({ held, place, drop }: { held: string | null; place: (kind: string) => void; drop: (kind: string, x: number, y: number) => void }) {
  const from = useRef<[number, number] | null>(null);
  const dragged = useRef(false);
  return <div className="canvas-kinds" role="toolbar" aria-label="Widgets to place">{Object.entries(KINDS).map(([kind, each]) =>
    <button key={kind} type="button" className="btn" data-act="place-widget" data-kind={kind} aria-pressed={held === kind} title={'Drag ' + each.label + ' onto the canvas, or press it and then press where it goes'}
      onPointerDown={(event) => { from.current = [event.clientX, event.clientY]; event.currentTarget.setPointerCapture?.(event.pointerId); }}
      onPointerUp={(event) => {
        const start = from.current;
        from.current = null;
        if (!start || Math.hypot(event.clientX - start[0], event.clientY - start[1]) <= 6) return;
        dragged.current = true;
        drop(kind, event.clientX, event.clientY);
      }}
      onClick={() => { if (dragged.current) dragged.current = false; else place(kind); }}><KindSymbol kind={kind} />{each.label}</button>)}</div>;
}

/** A corner of the canvas a bar is kept in. */
export type Corner = 'top-left' | 'top-right' | 'bottom-right' | 'bottom-left';
/** The corners in the order a bar's grip goes round them when it is pressed without dragging. */
const CORNERS: Corner[] = ['top-left', 'top-right', 'bottom-right', 'bottom-left'];
type Bar = 'look' | 'draw' | 'find';
const BARS: Record<Bar, string> = { look: 'view', draw: 'drawing', find: 'tools' };
const BAR_HOMES: Record<Bar, Corner> = { look: 'bottom-left', draw: 'bottom-right', find: 'bottom-right' };
const KEPT_BARS = 'lys.canvas.bars';
/** Where this browser kept each bar; a bar it kept no corner for is in its own. */
function keptBars(): Record<Bar, Corner> {
  try {
    const kept = JSON.parse(localStorage.getItem(KEPT_BARS) ?? '{}') as Record<string, unknown>;
    const at = (bar: Bar): Corner => CORNERS.includes(kept[bar] as Corner) ? kept[bar] as Corner : BAR_HOMES[bar];
    return { look: at('look'), draw: at('draw'), find: at('find') };
  } catch { return BAR_HOMES; }
}

/**
 * A bar's grip. Dragged, the bar goes to the corner of the canvas nearest where it is let go; pressed without dragging,
 * to the next corner round.
 */
function Grip({ bar, at, move, drag }: { bar: Bar; at: Corner; move: (to: Corner) => void; drag: (on: boolean) => void }) {
  const from = useRef<[number, number] | null>(null);
  return <button type="button" className="canvas-bar-grip" data-move={bar} aria-label={'Move the ' + BARS[bar] + ' bar: it is at the ' + at.replace('-', ' ') + '. Drag it to a corner, or press for the next corner'} title="Drag to a corner"
    onPointerDown={(event) => { from.current = [event.clientX, event.clientY]; event.currentTarget.setPointerCapture?.(event.pointerId); drag(true); }}
    onPointerUp={(event) => {
      const start = from.current;
      from.current = null;
      drag(false);
      if (!start) return;
      const canvas = event.currentTarget.closest('.canvas-corner')?.parentElement?.getBoundingClientRect();
      if (!canvas || Math.hypot(event.clientX - start[0], event.clientY - start[1]) <= 6) { move(CORNERS[(CORNERS.indexOf(at) + 1) % CORNERS.length]); return; }
      move(((event.clientY < canvas.top + canvas.height / 2 ? 'top' : 'bottom') + '-' + (event.clientX < canvas.left + canvas.width / 2 ? 'left' : 'right')) as Corner);
    }}>
    <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M9 6v.01M15 6v.01M9 12v.01M15 12v.01M9 18v.01M15 18v.01" /></svg></button>;
}

export function CanvasDock({ graph, show, tool, setTool, picking, panel, setPanel, zoom, zoomBy, home, says, connections, kind, place, drop, ...layouts }: {
  graph: SessionGraph; show: (node: string) => void; tool: Tool; setTool: (tool: Tool) => void; picking: boolean;
  panel: Panel; setPanel: (panel: Panel) => void;
  /** How far the surface is zoomed, in percent; `zoomBy` steps it, or with null puts it back to actual size; `home` brings the windows back into view. */
  zoom: number; zoomBy: (factor: number | null) => void; home: () => void;
  /** What went wrong that the person has to know, each said once over the bar. */
  says: ReactNode;
  connections: ReactNode;
  /** The kind of widget held to be placed, and the act that takes one in hand. */
  kind: string | null; place: (kind: string) => void;
  /** Places a widget of a kind where it was let go on the page, when that is on the canvas. */
  drop: (kind: string, x: number, y: number) => void;
  keeping: Keeping; layouts: SavedLayout[]; save: (name: string) => Promise<void>; remove: (name: string) => Promise<void>;
}) {
  const hold = (name: Exclude<Tool, null>) => () => setTool(tool === name ? null : name);
  const slide = (name: Exclude<Panel, null>) => () => setPanel(panel === name ? null : name);
  const running = graph.nodes.filter((node) => node.session).length;
  const [corners, setCorners] = useState(keptBars);
  const [dragging, setDragging] = useState(false);
  const grip = (bar: Bar) => <Grip bar={bar} at={corners[bar]} drag={setDragging} move={(to) => setCorners((now) => {
    const next = { ...now, [bar]: to };
    try { localStorage.setItem(KEPT_BARS, JSON.stringify(next)); } catch { /* A browser that keeps nothing still moves the bar for now. */ }
    return next;
  })} />;
  const bars: Record<Bar, ReactNode> = {
    look: <div key="look" className="canvas-bar canvas-look" role="toolbar" aria-label="Canvas view">
      {grip('look')}
      <Symbol act="zoom-out" says="Zoom out" on={() => zoomBy(1 / 1.25)} />
      <button type="button" className="canvas-symbol canvas-zoom" data-act="zoom-reset" aria-label="Zoom to actual size" title="Zoom to actual size" onClick={() => zoomBy(null)}>{zoom}%</button>
      <Symbol act="zoom-in" says="Zoom in" on={() => zoomBy(1.25)} />
      <Symbol act="home" says="Back to the windows" on={home} />
    </div>,
    // Drawing is one bar and finding is another, apart: each drawing tool is held, then used on the canvas, and shows that it is held.
    draw: <div key="draw" className="canvas-bar canvas-draw" role="toolbar" aria-label="Draw on the canvas">
      {grip('draw')}
      <Symbol act="draw-box" says="Box: drag on the canvas to draw a box around windows, then label it" on={hold('box')} pressed={tool === 'box'} />
      <Symbol act="draw-line" says="Line: press one thing, then another, to draw a line between them; or drag from a dot on a thing's edge to another thing" on={hold('line')} pressed={tool === 'line'} />
      <Symbol act="draw-note" says="Note: press on the canvas where the note goes" on={hold('note')} pressed={tool === 'note'} />
      <Symbol act="draw-widget" says="Widget: choose what to show about your agents, then press where it goes" on={slide('widgets')} pressed={tool === 'widget'} expanded={panel === 'widgets'} />
    </div>,
    find: <div key="find" className="canvas-bar canvas-find-bar" role="toolbar" aria-label="Canvas tools">
      {grip('find')}
      <Symbol act="agents" says={'Agents: ' + running + ' running. Find one and go to it'} on={slide('agents')} expanded={panel === 'agents'} count={running} />
      <Symbol act="layouts" says="Layouts: save this one, or open a saved one" on={slide('layouts')} expanded={panel === 'layouts'} />
      <Symbol act="connections" says="Connections, in words" on={slide('connections')} expanded={panel === 'connections'} />
    </div>,
  };
  // The panel, and what has to be said, stand by the bar they belong to: the widgets and a held tool by the drawing bar, the rest by the tools.
  const owner: Bar = panel === 'widgets' || (tool && !panel) ? 'draw' : 'find';
  return <>
  {CORNERS.filter((corner) => Object.values(corners).includes(corner)).map((corner) => <div key={corner} className="canvas-corner" data-corner={corner}>
    {(Object.keys(bars) as Bar[]).filter((bar) => corners[bar] === corner).map((bar) => bars[bar])}
  </div>)}
  {dragging ? CORNERS.map((corner) => <span key={corner} className="canvas-corner-offer" data-corner={corner} aria-hidden="true" />) : null}
  <div className="canvas-dock" data-corner={corners[owner]}>
    <div className="canvas-dock-says">
      {says}
      {tool ? <span role="status">{tool === 'box' ? 'Drag on the canvas to draw the box.' : tool === 'note' ? 'Press on the canvas where the note goes.'
        : tool === 'widget' ? 'Press an agent’s window to feed it from that agent, or press the canvas and draw lines into it.' : picking ? 'Now press the thing the line goes to.' : 'Press the thing the line starts from.'} Escape leaves it.</span> : null}
    </div>
    {/* Each part stays on the page while the panel is away, so nothing in it is read again when it opens. */}
    <section className={'canvas-pop' + (panel ? ' open' : '')} aria-label={panel ? PANELS[panel] : 'Canvas panel'} aria-hidden={!panel}>
      <div className="canvas-pop-body" hidden={panel !== 'agents'}><Agents graph={graph} show={show} open={panel === 'agents'} /></div>
      <div className="canvas-pop-body" hidden={panel !== 'layouts'}><Layouts {...layouts} /></div>
      <div className="canvas-pop-body" hidden={panel !== 'connections'}>{connections}</div>
      <div className="canvas-pop-body" hidden={panel !== 'widgets'}><Widgets held={tool === 'widget' ? kind : null} place={place} drop={drop} /></div>
    </section>
  </div>
  </>;
}
