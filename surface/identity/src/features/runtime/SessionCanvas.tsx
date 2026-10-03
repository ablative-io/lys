import { refreshLive } from '../../live';
/** A surface of windows: each permitted session and each thing it is connected to is a window a person drags, sizes and arranges, with as many live terminals open as they choose. */
import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import type { KeyboardEvent, PointerEvent } from 'react';
import { useParams } from 'react-router';
import { useLive, useLoad } from '../../api';
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
 * The part of the surface the page shows: where the surface's origin sits. The surface is never magnified: a terminal
 * counts its columns from its size on the screen, so a magnified window would resize the agent's real terminal.
 */
export interface View { x: number; y: number }
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

/** How far an arrow key moves a window whose bar has the keyboard. */
const STEPS: Record<string, [number, number] | undefined> = { ArrowLeft: [-16, 0], ArrowRight: [16, 0], ArrowUp: [0, -16], ArrowDown: [0, 16] };

type Drag = { kind: 'pan'; from: [number, number]; view: View } | { kind: 'move' | 'size'; id: string; from: [number, number]; box: Box };

function Canvas({ graph }: { graph: SessionGraph }) {
  const { agent } = useParams();
  const [before] = useState(kept);
  // The agent the person came for, when the route names one: its terminal is open and in view whatever was kept.
  const [asked] = useState(() => graph.nodes.find((node) => agent && node.session?.agent === agent)?.id ?? null);
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

  useLayoutEffect(() => {
    const element = surface.current;
    const box = asked ? placed(graph, moved, open)[asked] : undefined;
    if (!element || !box) return;
    setView({ x: element.clientWidth / 2 - (box.x + box.w / 2), y: element.clientHeight / 2 - (box.y + box.h / 2) });
    // Once, on arriving: after that the view is the person's.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // The arrangement is this browser's own: it is kept as it changes and is there on the next visit.
  useEffect(() => {
    const here = new Set(graph.nodes.map((node) => node.id));
    // A window that is no longer here (a session that ended) keeps no place.
    const boxes = Object.fromEntries(Object.entries(moved).filter(([id]) => here.has(id)));
    // A browser may refuse to keep anything. The surface still works for this visit, and says that it will not be remembered.
    try {
      localStorage.setItem(KEPT, JSON.stringify({ boxes, open: [...open].filter((id) => here.has(id)), view } satisfies Kept));
      setUnkept(null);
    } catch (error) { setUnkept(error instanceof Error ? error.message : String(error)); }
  }, [graph, moved, open, view]);

  // The wheel moves the surface. A terminal keeps its own wheel.
  useEffect(() => {
    const element = surface.current;
    if (!element) return;
    const wheel = (event: WheelEvent) => {
      if (event.ctrlKey || event.metaKey || (event.target instanceof Element && event.target.closest('.terminal'))) return;
      event.preventDefault();
      setView((now) => ({ x: now.x - event.deltaX, y: now.y - event.deltaY }));
    };
    element.addEventListener('wheel', wheel, { passive: false });
    return () => element.removeEventListener('wheel', wheel);
  }, []);

  const begin = (start: (from: [number, number]) => Drag) => (event: PointerEvent<HTMLElement>) => {
    if (event.button !== 0 || (event.target instanceof Element && event.target.closest('button, a, input, .terminal'))) return;
    event.stopPropagation();
    event.currentTarget.setPointerCapture?.(event.pointerId);
    drag.current = start([event.clientX, event.clientY]);
  };
  const during = (event: PointerEvent) => {
    const now = drag.current;
    if (!now) return;
    const [dx, dy] = [event.clientX - now.from[0], event.clientY - now.from[1]];
    if (now.kind === 'pan') { setView({ x: now.view.x + dx, y: now.view.y + dy }); return; }
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
    setView((now) => ({ x: now.x - step[0] * 4, y: now.y - step[1] * 4 }));
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
    if (all.length) setView({ x: HOME.x - Math.min(...all.map((box) => box.x)), y: HOME.y - Math.min(...all.map((box) => box.y)) });
  };

  return <div className="session-canvas-scroll" role="region" aria-label="Agent connection canvas" tabIndex={0} ref={surface} onKeyDown={travel}
    onPointerDown={begin((from) => ({ kind: 'pan', from, view }))} onPointerMove={during} onPointerUp={finish} onPointerCancel={finish}>
    <div className="session-canvas-tools">
      {unkept ? <span className="why-not" role="status">This browser will not keep the arrangement: {unkept}</span> : null}
      <button type="button" className="btn" data-act="home" onClick={home}>Back to the windows</button>
    </div>
    <div className="session-canvas" style={{ transform: `translate(${view.x}px, ${view.y}px)` }}>
      <svg className="session-canvas-lines" aria-hidden="true">{graph.edges.flatMap((edge) => {
        const [from, to] = [boxes[edge.from], boxes[edge.to]];
        return from && to ? [<path key={edge.id} d={lineBetween(from, to)} data-kind={edge.kind} data-standing={edge.stands} />] : [];
      })}</svg>
      {graph.nodes.map((node) => {
        const box = boxes[node.id], shown = open.has(node.id);
        const state = !node.session ? null : graph.unanswered.some((entry) => entry.session === node.session?.session) ? 'Runner did not answer; current state unknown' : node.session.shown === 'running' ? 'Running' : 'Starting, not yet confirmed';
        const kind = node.column === 'sessions' ? 'Agent' : node.column === 'teams' ? 'Team or sender' : 'Resource or recipient';
        return <article className={'session-canvas-node ' + node.column + (shown ? ' open' : '')} key={node.id} data-node={node.id} aria-label={kind + ': ' + node.title}
          style={{ left: box.x, top: box.y, width: box.w, height: box.h, zIndex: front === node.id ? 3 : node.session ? 2 : 1 }}
          onPointerDownCapture={() => setFront(node.id)}>
          <header className="session-canvas-bar" tabIndex={0} aria-label={'Move ' + node.title + ' with the arrow keys' + (node.session ? '; with Shift, size its terminal' : '')} onKeyDown={nudge(node.id)}
            onPointerDown={begin((from) => ({ kind: 'move', id: node.id, from, box }))}>
            <span className="session-canvas-kind">{kind}</span><h3>{node.title}</h3><span className="note">{node.detail}</span>
            {node.session && !shown ? <span className="note" title={state ?? undefined}>{state}</span> : null}
            {node.session && shown ? <span className="session-canvas-slot" /> : null}
            {node.session ? <button className="btn" aria-expanded={shown} onClick={() => toggle(node.id)}>{shown ? 'Close terminal view' : 'Open terminal'}</button> : null}
          </header>
          {node.session && shown ? <>
            <Terminal key={node.session.session} session={node.session.session} agent={node.session.agent} />
            <span className="session-canvas-grip" aria-hidden="true" onPointerDown={begin((from) => ({ kind: 'size', id: node.id, from, box }))} />
          </> : null}
        </article>;
      })}
    </div>
  </div>;
}

/** The connections in words, for a reader who does not follow the lines. */
function Connections({ graph }: { graph: SessionGraph }) {
  return <details className="canvas-connections"><summary>Connections ({graph.edges.length})</summary>
    <ul>{graph.edges.map((edge) => <li key={edge.id} data-kind={edge.kind}>{graph.nodes.find((node) => node.id === edge.from)?.title} → {graph.nodes.find((node) => node.id === edge.to)?.title}: {edge.label}</li>)}</ul>
    {!graph.edges.length ? <p>No team memberships or grants connecting these sessions were returned.</p> : null}
  </details>;
}

function MessageCanvas({ graph, first }: { graph: SessionGraph; first: MessageRead }) {
  const [messages, setMessages] = useState(first);
  const whole = withMessages(graph, messages.messages);
  return <><div className="session-canvas-strip"><MessageConnections value={messages} change={setMessages} /><Connections graph={whole} /></div><Canvas graph={whole} /></>;
}

export function SessionCanvas() {
  const load = useLive(readSessionGraph, 'session-canvas');
  const messages = useLoad(firstMessagePage, 'canvas-message-edges');
  return <div className="page fill session-canvas-page">
    <div className="head"><h1>Agent canvas</h1>{load.status === 'refused' ? <button type="button" onClick={refreshLive}>Reconnect</button> : null}
      <details className="canvas-about"><summary>About this canvas</summary>
        <p className="sub">Drag a window by its bar, drag its corner to size it, drag the background or use the wheel to move the surface. With a bar focused, arrows move the window and Shift with arrows sizes its terminal; with the surface focused, arrows move the surface. Closing a terminal view leaves the process running. Team membership does not grant access; dashed grant connections no longer stand.</p>
      </details></div>
    <Gate load={load} title="Agent canvas" ok={(graph) => <>
      {graph.notices.length || graph.unanswered.length || messages.status !== 'ok' ? <div className="session-canvas-strip">
        {graph.notices.map((notice) => <p className="note" role="status" key={notice}>{notice}</p>)}
        {graph.unanswered.map((entry) => <p className="why-not" role="alert" key={entry.session}>{entry.session}: {entry.refusal}: {entry.reason}</p>)}
        {messages.status === 'loading' ? <p role="status">Reading message connections…</p> : messages.status === 'refused' ? <p className="why-not" role="status">Message connections unavailable: {messages.refused.refusal.refusal}: {messages.refused.refusal.reason}</p> : null}
        {messages.status !== 'ok' && graph.nodes.some((node) => node.session) ? <Connections graph={graph} /> : null}
      </div> : null}
      {graph.nodes.some((node) => node.session) ? messages.status === 'ok' ? <MessageCanvas graph={graph} first={messages.data} /> : <Canvas graph={graph} /> : <p>No running sessions were returned.</p>}
    </>} />
  </div>;
}
