import { refreshLive } from '../../live';
/** A surface of windows: each permitted session and each thing it is connected to is a window a person drags, sizes and arranges, with as many live terminals open as they choose. */
import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import type { KeyboardEvent, PointerEvent } from 'react';
import { useParams } from 'react-router';
import { useLive, useLoad } from '../../api';
import { RunningList } from './Sessions';
import type { Load } from '../../api';
import { Gate } from '../signin/Gate';
import { Terminal } from './Terminal';
import { readSessionGraph, withMessages } from './session-graph';
import type { SessionGraph } from './session-graph';
import { firstMessagePage } from './message-connections';
import type { MessageRead } from './message-connections';
import { MessageConnections } from './MessageConnections';
import './session-canvas.css';

/** Where a window sits on the surface and how large it is, in the surface's own units. */
export interface Box { x: number; y: number; w: number; h: number }
/**
 * The part of the surface the page shows: where the surface's origin sits and how far it is zoomed (1 when absent).
 * Zooming scales the drawing only; a window's own size, which its terminal counts its columns from, does not change.
 */
export interface View { x: number; y: number; k?: number }
const zoomOf = (view: View): number => view.k ?? 1;
/** The nearest and the farthest the surface zooms: far enough to see every window at once, near enough to read small text. */
const ZOOM: [number, number] = [0.2, 3];
/** The view zoomed to `k` with the surface point under (`cx`, `cy`) staying where it is. */
function zoomed(view: View, k: number, cx: number, cy: number): View {
  const from = zoomOf(view), to = Math.min(ZOOM[1], Math.max(ZOOM[0], k));
  return { x: cx - (cx - view.x) * to / from, y: cy - (cy - view.y) * to / from, k: to };
}
interface Kept { boxes: Record<string, Box>; open: string[]; view: View }

const KEPT = 'lys.canvas';
const HOME: View = { x: 24, y: 24 };
const BAR = 34;
const CARD: [number, number] = [220, 64];
const CLOSED: [number, number] = [440, BAR];
const OPENED: [number, number] = [760, 480];
/** The smallest a terminal window is dragged to: its bar still shows the agent's name beside state, Stop and close, and a prompt can still be read. */
const SMALLEST: [number, number] = [560, 180];

const isBox = (value: unknown): value is Box => !!value && typeof value === 'object' && ['x', 'y', 'w', 'h'].every((key) => Number.isFinite((value as Record<string, unknown>)[key]));

/** The arrangement this browser kept. One that cannot be read is no arrangement: the surface then lays itself out afresh. */
function kept(): Kept | null {
  try {
    const value: unknown = JSON.parse(localStorage.getItem(KEPT) ?? 'null');
    if (!value || typeof value !== 'object') return null;
    const { boxes, open, view } = value as Record<string, unknown>;
    if (!boxes || typeof boxes !== 'object' || !Object.values(boxes).every(isBox)) return null;
    if (!Array.isArray(open) || !open.every((id) => typeof id === 'string')) return null;
    if (!view || typeof view !== 'object' || !['x', 'y'].every((key) => Number.isFinite((view as Record<string, unknown>)[key]))) return null;
    const k = (view as Record<string, unknown>).k;
    if (k !== undefined && !(typeof k === 'number' && k >= ZOOM[0] && k <= ZOOM[1])) return null;
    return { boxes: boxes as Record<string, Box>, open, view: view as View };
  } catch { return null; }
}

/** A first place for every node that has none: teams down the left, agents in the middle two across, resources on the right. */
export function placed(graph: SessionGraph, have: Record<string, Box>, open: ReadonlySet<string>): Record<string, Box> {
  const boxes: Record<string, Box> = {};
  const sessions = graph.nodes.filter((node) => node.column === 'sessions');
  const across = Math.min(2, Math.max(1, sessions.length));
  const middle = CARD[0] + 60;
  const right = middle + across * (OPENED[0] + 40) + 20;
  const count = { teams: 0, sessions: 0, resources: 0 };
  for (const node of graph.nodes) {
    const index = count[node.column]++;
    const was = have[node.id];
    if (was) { boxes[node.id] = was; continue; }
    if (node.column === 'sessions') {
      const [w, h] = open.has(node.id) ? OPENED : CLOSED;
      boxes[node.id] = { x: middle + (index % across) * (OPENED[0] + 40), y: Math.floor(index / across) * (OPENED[1] + 40), w, h };
    } else boxes[node.id] = { x: node.column === 'teams' ? 0 : right, y: index * (CARD[1] + 20), w: CARD[0], h: CARD[1] };
  }
  return boxes;
}

/** A connection, drawn from the middle of one window's right edge to the middle of the other's left edge. */
export function lineBetween(from: Box, to: Box): string {
  const [x, y, endX, endY] = [from.x + from.w, from.y + from.h / 2, to.x, to.y + to.h / 2];
  const middle = (x + endX) / 2;
  return `M ${x} ${y} C ${middle} ${y}, ${middle} ${endY}, ${endX} ${endY}`;
}

/** The address of a session's terminal alone, filling its own browser window. */
const windowOf = (session: { session: string; agent: string | null }): string =>
  '#/window/' + encodeURIComponent(session.session) + (session.agent ? '?agent=' + encodeURIComponent(session.agent) : '');

/** How far an arrow key moves a window whose bar has the keyboard. */
const STEPS: Record<string, [number, number] | undefined> = { ArrowLeft: [-16, 0], ArrowRight: [16, 0], ArrowUp: [0, -16], ArrowDown: [0, 16] };

type Drag = { kind: 'pan'; from: [number, number]; view: View } | { kind: 'move' | 'size'; id: string; from: [number, number]; box: Box };

function Canvas({ graph }: { graph: SessionGraph }) {
  const { agent } = useParams();
  const [before] = useState(kept);
  // The agent the person came for, when the route names one: its terminal is open and in view whatever was kept.
  const asked = graph.nodes.find((node) => agent && node.session?.agent === agent)?.id ?? null;
  const [open, setOpen] = useState<ReadonlySet<string>>(() => new Set([...(before?.open ?? []), ...(asked ? [asked] : [])]));
  const [moved, setMoved] = useState<Record<string, Box>>(() => {
    const boxes = before?.boxes ?? {};
    const was = asked ? boxes[asked] : undefined;
    // A window kept closed is only its bar; opened by the route it takes the open size where it stands.
    return asked && was && !before?.open.includes(asked) ? { ...boxes, [asked]: { ...was, w: OPENED[0], h: OPENED[1] } } : boxes;
  });
  const [view, setView] = useState<View>(() => before?.view ?? HOME);
  const [unkept, setUnkept] = useState<string | null>(null);
  const [front, setFront] = useState<string | null>(null);
  const boxes = placed(graph, moved, open);
  const drag = useRef<Drag | null>(null);
  const surface = useRef<HTMLDivElement>(null);

  // On arriving, and again when the route names another agent: that agent's terminal is open, at the open size, in the middle of the view.
  useLayoutEffect(() => {
    const element = surface.current;
    if (!element || !asked) return;
    const was = placed(graph, moved, open)[asked];
    const box = open.has(asked) ? was : { ...was, w: OPENED[0], h: OPENED[1] };
    if (!open.has(asked)) {
      setOpen((now) => new Set([...now, asked]));
      setMoved((all) => ({ ...all, [asked]: box }));
    }
    setFront(asked);
    setView((now) => { const k = zoomOf(now); return { x: element.clientWidth / 2 - (box.x + box.w / 2) * k, y: element.clientHeight / 2 - (box.y + box.h / 2) * k, k }; });
    // Only when the agent asked for changes: after that the view and the windows are the person's.
  }, [asked]);

  // The arrangement is this browser's own: it is kept as it changes and is there on the next visit.
  useEffect(() => {
    const here = new Set(graph.nodes.map((node) => node.id));
    // A session that is no longer here has ended and keeps no place. A team, resource or message card that is not here may
    // only be missing because its read was refused this visit, so its place is kept.
    const boxes = Object.fromEntries(Object.entries(moved).filter(([id]) => !id.startsWith('session:') || here.has(id)));
    // A browser may refuse to keep anything. The surface still works for this visit, and says that it will not be remembered.
    try {
      localStorage.setItem(KEPT, JSON.stringify({ boxes, open: [...open].filter((id) => here.has(id)), view } satisfies Kept));
      setUnkept(null);
    } catch (error) { setUnkept(error instanceof Error ? error.message : String(error)); }
  }, [graph, moved, open, view]);

  // The wheel moves the surface, and a pinch zooms it about the pointer: a trackpad's pinch arrives as a wheel with
  // Control held, Safari's as gesture events. A terminal keeps its own wheel.
  useEffect(() => {
    const element = surface.current;
    if (!element) return;
    const at = (event: { clientX: number; clientY: number }): [number, number] => {
      const box = element.getBoundingClientRect();
      return [event.clientX - box.left, event.clientY - box.top];
    };
    const wheel = (event: WheelEvent) => {
      if (event.ctrlKey || event.metaKey) {
        event.preventDefault();
        const [cx, cy] = at(event);
        setView((now) => zoomed(now, zoomOf(now) * Math.exp(-event.deltaY * 0.01), cx, cy));
        return;
      }
      if (event.target instanceof Element && event.target.closest('.terminal')) return;
      event.preventDefault();
      setView((now) => ({ ...now, x: now.x - event.deltaX, y: now.y - event.deltaY }));
    };
    let pinched: View | null = null;
    const pinchStart = (event: Event) => { event.preventDefault(); setView((now) => { pinched = now; return now; }); };
    const pinch = (event: Event) => {
      event.preventDefault();
      const gesture = event as Event & { scale?: number; clientX?: number; clientY?: number };
      const [cx, cy] = at({ clientX: gesture.clientX ?? 0, clientY: gesture.clientY ?? 0 });
      if (pinched && typeof gesture.scale === 'number') { const from = pinched; setView(zoomed(from, zoomOf(from) * gesture.scale, cx, cy)); }
    };
    element.addEventListener('wheel', wheel, { passive: false });
    element.addEventListener('gesturestart', pinchStart);
    element.addEventListener('gesturechange', pinch);
    return () => { element.removeEventListener('wheel', wheel); element.removeEventListener('gesturestart', pinchStart); element.removeEventListener('gesturechange', pinch); };
  }, []);
  /** Zoom by a step, or back to actual size, about the middle of the surface. */
  const zoomBy = (factor: number | null) => {
    const element = surface.current;
    const [cx, cy] = element ? [element.clientWidth / 2, element.clientHeight / 2] : [0, 0];
    setView((now) => zoomed(now, factor === null ? 1 : zoomOf(now) * factor, cx, cy));
  };
  const [menu, setMenu] = useState<string | null>(null);

  const begin = (start: (from: [number, number]) => Drag) => (event: PointerEvent<HTMLElement>) => {
    if (event.button !== 0 || (event.target instanceof Element && event.target.closest('button, a, input, .terminal'))) return;
    event.stopPropagation();
    event.currentTarget.setPointerCapture?.(event.pointerId);
    drag.current = start([event.clientX, event.clientY]);
  };
  const during = (event: PointerEvent) => {
    const now = drag.current;
    if (!now) return;
    const [sx, sy] = [event.clientX - now.from[0], event.clientY - now.from[1]];
    if (now.kind === 'pan') { setView((held) => ({ ...held, x: now.view.x + sx, y: now.view.y + sy })); return; }
    // A window moves and sizes by what the pointer travelled on the surface, which is the screen's distance over the zoom.
    const [dx, dy] = [sx / zoomOf(view), sy / zoomOf(view)];
    const box = now.kind === 'move'
      ? { ...now.box, x: now.box.x + dx, y: now.box.y + dy }
      : { ...now.box, w: Math.max(SMALLEST[0], now.box.w + dx), h: Math.max(SMALLEST[1], now.box.h + dy) };
    setMoved((all) => ({ ...all, [now.id]: box }));
  };
  const finish = () => { drag.current = null; };
  const nudge = (id: string) => (event: KeyboardEvent) => {
    const step = STEPS[event.key];
    if (!step || event.target !== event.currentTarget) return;
    event.preventDefault();
    event.stopPropagation();
    const box = boxes[id];
    // Arrows move the window; with Shift they size an open terminal.
    setMoved((all) => ({ ...all, [id]: event.shiftKey && open.has(id)
      ? { ...box, w: Math.max(SMALLEST[0], box.w + step[0]), h: Math.max(SMALLEST[1], box.h + step[1]) }
      : { ...box, x: box.x + step[0], y: box.y + step[1] } }));
  };
  /** With the surface itself focused, arrows move the surface. */
  const travel = (event: KeyboardEvent) => {
    const step = STEPS[event.key];
    if (!step || event.target !== event.currentTarget) return;
    event.preventDefault();
    setView((now) => ({ ...now, x: now.x - step[0] * 4, y: now.y - step[1] * 4 }));
  };
  const toggle = (id: string) => {
    const opening = !open.has(id);
    setOpen((now) => { const next = new Set(now); if (opening) next.add(id); else next.delete(id); return next; });
    const [w, h] = opening ? OPENED : CLOSED;
    setMoved((all) => ({ ...all, [id]: { ...boxes[id], w, h } }));
    if (opening) setFront(id);
  };
  /** Brings the surface back so its topmost, leftmost window sits at the corner. */
  const home = () => {
    const all = Object.values(boxes);
    if (all.length) setView((now) => ({ ...now, x: HOME.x - Math.min(...all.map((box) => box.x)) * zoomOf(now), y: HOME.y - Math.min(...all.map((box) => box.y)) * zoomOf(now) }));
  };

  return <div className="session-canvas-scroll" role="region" aria-label="Agent connection canvas" tabIndex={0} ref={surface} onKeyDown={travel}
    onPointerDown={begin((from) => ({ kind: 'pan', from, view }))} onPointerMove={during} onPointerUp={finish} onPointerCancel={finish}>
    <div className="session-canvas-tools">
      {unkept ? <span className="why-not" role="status">This browser will not keep the arrangement: {unkept}</span> : null}
      <button type="button" className="btn" data-act="zoom-out" aria-label="Zoom out" onClick={() => zoomBy(1 / 1.25)}>−</button>
      <button type="button" className="btn" data-act="zoom-reset" aria-label="Zoom to actual size" onClick={() => zoomBy(null)}>{Math.round(zoomOf(view) * 100)}%</button>
      <button type="button" className="btn" data-act="zoom-in" aria-label="Zoom in" onClick={() => zoomBy(1.25)}>+</button>
      <button type="button" className="btn" data-act="home" onClick={home}>Back to the windows</button>
    </div>
    <div className="session-canvas" style={{ transform: `translate(${view.x}px, ${view.y}px)` + (zoomOf(view) === 1 ? '' : ` scale(${zoomOf(view)})`), transformOrigin: '0 0' }}>
      <svg className="session-canvas-lines" aria-hidden="true">{graph.edges.flatMap((edge) => {
        const [from, to] = [boxes[edge.from], boxes[edge.to]];
        return from && to ? [<path key={edge.id} d={lineBetween(from, to)} data-kind={edge.kind} data-standing={edge.stands} />] : [];
      })}</svg>
      {graph.nodes.map((node) => {
        const box = boxes[node.id], shown = open.has(node.id);
        const state = !node.session ? null : graph.unanswered.some((entry) => entry.session === node.session?.session) ? 'Runner did not answer; current state unknown' : node.session.shown === 'running' ? 'Running' : 'Starting, not yet confirmed';
        const kind = node.column === 'sessions' ? 'Agent' : node.column === 'teams' ? 'Team or sender' : 'Resource or recipient';
        const unanswered = !!node.session && graph.unanswered.some((entry) => entry.session === node.session?.session);
        return <article className={'session-canvas-node ' + node.column + (shown ? ' open' : '') + (unanswered ? ' unanswered' : '') + (menu === node.id ? ' menu-open' : '')} key={node.id} data-node={node.id} aria-label={kind + ': ' + node.title}
          style={{ left: box.x, top: box.y, width: box.w, height: box.h, zIndex: front === node.id ? 3 : node.session ? 2 : 1 }}
          onPointerDownCapture={() => setFront(node.id)}>
          <header className="session-canvas-bar" tabIndex={0} aria-label={'Move ' + node.title + ' with the arrow keys' + (node.session ? '; with Shift, size its terminal' : '')} onKeyDown={nudge(node.id)}
            onPointerDown={begin((from) => ({ kind: 'move', id: node.id, from, box }))}>
            <span className="session-canvas-kind">{kind}</span><h3>{node.title}</h3><span className="note">{node.detail}</span>
            {node.session ? <span className="note" title={state ?? undefined}>{state}</span> : null}
            {/* One slim bar: a closed window opens from it; an open one keeps its controls in a small menu over the window. */}
            {node.session && !shown ? <span className="session-canvas-acts"><button className="btn" data-act="toggle" aria-expanded={false} onClick={() => toggle(node.id)}>Open terminal</button></span> : null}
            {node.session && shown ? <span className="session-canvas-acts">
              <button type="button" className="btn" data-act="window-menu" aria-haspopup="menu" aria-expanded={menu === node.id} aria-label={'Controls of the window of ' + node.title}
                onClick={() => setMenu((now) => now === node.id ? null : node.id)}>⋯</button>
            </span> : null}
          </header>
          {node.session && shown && menu === node.id ? <div className="session-canvas-menu" role="menu" aria-label={'Controls of the window of ' + node.title}>
            <button type="button" role="menuitem" className="btn" data-act="full-screen" onClick={(event) => { setMenu(null); void event.currentTarget.closest('article')?.requestFullscreen?.(); }}>Full screen</button>
            <a role="menuitem" className="btn" data-act="separate-window" href={windowOf(node.session)} target="_blank" rel="noreferrer"
              onClick={(event) => { event.preventDefault(); setMenu(null); window.open(windowOf(node.session!), 'lys-terminal-' + node.session!.session, 'popup,width=1200,height=800'); }}>Separate window</a>
            <button type="button" role="menuitem" className="btn" data-act="toggle" aria-expanded={true} onClick={() => { setMenu(null); toggle(node.id); }}>Close terminal view</button>
          </div> : null}
          {node.session && shown ? <>
            {unanswered ? <p className="why-not session-canvas-unanswered" role="status">{state}</p> : null}
            <Terminal key={node.session.session} session={node.session.session} agent={node.session.agent} machine={node.session.machine_name ?? node.session.machine} />
            <span className="session-canvas-grip" aria-hidden="true" onPointerDown={begin((from) => ({ kind: 'size', id: node.id, from, box }))} />
          </> : null}
        </article>;
      })}
    </div>
  </div>;
}

/** A session whose runner did not answer, said with the agent's name and the computer's, never their identifiers; the runner's refusal follows, small. */
function Unanswered({ graph, entry }: { graph: SessionGraph; entry: SessionGraph['unanswered'][number] }) {
  const session = graph.nodes.find((node) => node.session?.session === entry.session)?.session;
  const agent = session?.agent ? graph.nodes.find((node) => node.session?.session === entry.session)?.title : undefined;
  const who = agent && agent !== session?.agent ? agent : session?.agent ? 'an agent outside your view' : 'a session with no agent';
  const where = session?.machine_name ?? 'its computer';
  return <p className="why-not" role="alert">The runner on {where} did not answer for {who}. <small className="refusal-name">{entry.refusal}: {entry.reason}</small></p>;
}

/** The connections in words, for a reader who does not follow the lines. */
function Connections({ graph }: { graph: SessionGraph }) {
  return <section className="canvas-connections" aria-label="Connections"><h4>Connections ({graph.edges.length})</h4>
    <ul>{graph.edges.map((edge) => <li key={edge.id} data-kind={edge.kind}>{graph.nodes.find((node) => node.id === edge.from)?.title} → {graph.nodes.find((node) => node.id === edge.to)?.title}: {edge.label}</li>)}</ul>
    {!graph.edges.length ? <p>No team memberships or grants connecting these sessions were returned.</p> : null}
  </section>;
}

/**
 * The surface with whatever message connections have been read. It is one Canvas in one place whether the messages are
 * still being read, were refused, or have arrived, so a terminal that is open stays open across that change.
 */
function Whole({ graph, messages }: { graph: SessionGraph; messages: Load<MessageRead> }) {
  const [later, setLater] = useState<MessageRead | null>(null);
  const read = later ?? (messages.status === 'ok' ? messages.data : null);
  const whole = read ? withMessages(graph, read.messages) : graph;
  return <>
    <div className="session-canvas-strip">
      {graph.notices.map((notice) => <p className="note" role="status" key={notice}>{notice}</p>)}
      {graph.unanswered.map((entry) => <Unanswered key={entry.session} graph={graph} entry={entry} />)}
      {/* The connections in words are one small fold over the canvas: there for a reader who wants them, never a line across the page. */}
      <details className="canvas-fold"><summary>Connections ({whole.edges.length})</summary><div className="canvas-fold-body">
        {messages.status === 'loading' ? <p role="status">Reading message connections…</p> : messages.status === 'refused' ? <p className="why-not" role="status">Message connections unavailable: {messages.refused.refusal.refusal}: {messages.refused.refusal.reason}</p> : null}
        {read ? <section className="canvas-about" aria-label="Message connections"><MessageConnections value={read} change={setLater} /></section> : null}
        <Connections graph={whole} />
      </div></details>
    </div>
    <Canvas graph={whole} />
  </>;
}

export function SessionCanvas() {
  const load = useLive(readSessionGraph, 'session-canvas');
  const messages = useLoad(firstMessagePage, 'canvas-message-edges');
  return <div className="page fill session-canvas-page">
    <div className="head"><div><h1>Running</h1>{load.status === 'refused' ? <button type="button" onClick={refreshLive}>Reconnect</button> : null}</div></div>
    <div className="canvas-side">
      <RunningList />
        <Gate load={load} title="Agent canvas" ok={(graph) => graph.nodes.some((node) => node.session) ? <Whole graph={graph} messages={messages} /> : <>
          {graph.notices.map((notice) => <p className="note" role="status" key={notice}>{notice}</p>)}
          {graph.unanswered.map((entry) => <Unanswered key={entry.session} graph={graph} entry={entry} />)}
        </>} />
    </div>
  </div>;
}
