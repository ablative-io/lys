import { refreshLive } from '../../live';
/** A surface of windows: each permitted session and each thing it is connected to is a window a person drags, sizes and arranges, with as many live terminals open as they choose. */
import { useEffect, useRef, useState } from 'react';
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
/** The part of the surface the window shows: where its origin sits on the page, and how much it is magnified. */
export interface View { x: number; y: number; k: number }
interface Kept { boxes: Record<string, Box>; open: string[]; view: View }

const KEPT = 'lys.canvas';
const BAR = 34;
const CARD: [number, number] = [220, 52];
const CLOSED: [number, number] = [280, BAR];
const OPENED: [number, number] = [760, 480];
/** The smallest a terminal window is dragged to: its bar's controls still fit and a prompt can still be read. */
const SMALLEST: [number, number] = [360, 180];

const isBox = (value: unknown): value is Box => !!value && typeof value === 'object' && ['x', 'y', 'w', 'h'].every((key) => Number.isFinite((value as Record<string, unknown>)[key]));

/** The arrangement this browser kept. One that cannot be read is no arrangement: the surface then lays itself out afresh. */
function kept(): Kept | null {
  try {
    const value: unknown = JSON.parse(localStorage.getItem(KEPT) ?? 'null');
    if (!value || typeof value !== 'object') return null;
    const { boxes, open, view } = value as Record<string, unknown>;
    if (!boxes || typeof boxes !== 'object' || !Object.values(boxes).every(isBox)) return null;
    if (!Array.isArray(open) || !open.every((id) => typeof id === 'string')) return null;
    if (!view || typeof view !== 'object' || !['x', 'y', 'k'].every((key) => Number.isFinite((view as Record<string, unknown>)[key])) || (view as View).k <= 0) return null;
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
  const [open, setOpen] = useState<ReadonlySet<string>>(() => {
    const asked = graph.nodes.find((node) => agent && node.session?.agent === agent)?.id;
    return new Set([...(before?.open ?? []), ...(asked ? [asked] : [])]);
  });
  const [moved, setMoved] = useState<Record<string, Box>>(() => before?.boxes ?? {});
  const [view, setView] = useState<View>(() => before?.view ?? { x: 24, y: 24, k: 1 });
  const [front, setFront] = useState<string | null>(null);
  const boxes = placed(graph, moved, open);
  const drag = useRef<Drag | null>(null);
  const surface = useRef<HTMLDivElement>(null);

  // The arrangement is this browser's own: it is kept as it changes and is there on the next visit.
  useEffect(() => {
    const here = new Set(graph.nodes.map((node) => node.id));
    localStorage.setItem(KEPT, JSON.stringify({ boxes: moved, open: [...open].filter((id) => here.has(id)), view } satisfies Kept));
  }, [graph, moved, open, view]);

  // The wheel moves the surface, and with Control or Command held it magnifies about the pointer. A terminal keeps its own wheel.
  useEffect(() => {
    const element = surface.current;
    if (!element) return;
    const wheel = (event: WheelEvent) => {
      const magnify = event.ctrlKey || event.metaKey;
      if (!magnify && event.target instanceof Element && event.target.closest('.terminal')) return;
      event.preventDefault();
      const edge = element.getBoundingClientRect();
      const [px, py] = [event.clientX - edge.left, event.clientY - edge.top];
      setView((now) => {
        if (!magnify) return { ...now, x: now.x - event.deltaX, y: now.y - event.deltaY };
        const k = now.k * Math.exp(-event.deltaY / 400);
        return { k, x: px - (px - now.x) * (k / now.k), y: py - (py - now.y) * (k / now.k) };
      });
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
    if (now.kind === 'pan') { setView({ ...now.view, x: now.view.x + dx, y: now.view.y + dy }); return; }
    const box = now.kind === 'move'
      ? { ...now.box, x: now.box.x + dx / view.k, y: now.box.y + dy / view.k }
      : { ...now.box, w: Math.max(SMALLEST[0], now.box.w + dx / view.k), h: Math.max(SMALLEST[1], now.box.h + dy / view.k) };
    setMoved((all) => ({ ...all, [now.id]: box }));
  };
  const finish = () => { drag.current = null; };
  const nudge = (id: string) => (event: KeyboardEvent) => {
    const step = STEPS[event.key];
    if (!step || event.target !== event.currentTarget) return;
    event.preventDefault();
    setMoved((all) => ({ ...all, [id]: { ...boxes[id], x: boxes[id].x + step[0], y: boxes[id].y + step[1] } }));
  };
  const toggle = (id: string) => {
    const opening = !open.has(id);
    setOpen((now) => { const next = new Set(now); if (opening) next.add(id); else next.delete(id); return next; });
    const [w, h] = opening ? OPENED : CLOSED;
    setMoved((all) => ({ ...all, [id]: { ...boxes[id], w, h } }));
    if (opening) setFront(id);
  };
  const fit = () => {
    const element = surface.current;
    const all = Object.values(boxes);
    if (!element || !all.length) return;
    const [left, top] = [Math.min(...all.map((box) => box.x)), Math.min(...all.map((box) => box.y))];
    const [wide, tall] = [Math.max(...all.map((box) => box.x + box.w)) - left, Math.max(...all.map((box) => box.y + box.h)) - top];
    const k = Math.min((element.clientWidth - 48) / wide, (element.clientHeight - 48) / tall, 1);
    setView({ k, x: 24 - left * k, y: 24 - top * k });
  };

  return <div className="session-canvas-scroll" role="region" aria-label="Agent connection canvas" tabIndex={0} ref={surface}
    onPointerDown={begin((from) => ({ kind: 'pan', from, view }))} onPointerMove={during} onPointerUp={finish} onPointerCancel={finish}>
    <div className="session-canvas-tools">
      <button type="button" className="btn" data-act="fit" onClick={fit}>Fit</button>
      <button type="button" className="btn" data-act="actual" onClick={() => setView({ x: 24, y: 24, k: 1 })}>100%</button>
    </div>
    <div className="session-canvas" style={{ transform: `translate(${view.x}px, ${view.y}px) scale(${view.k})` }}>
      <svg className="session-canvas-lines" aria-hidden="true">{graph.edges.flatMap((edge) => {
        const [from, to] = [boxes[edge.from], boxes[edge.to]];
        return from && to ? [<path key={edge.id} d={lineBetween(from, to)} data-kind={edge.kind} data-standing={edge.stands} />] : [];
      })}</svg>
      {graph.nodes.map((node) => {
        const box = boxes[node.id], shown = open.has(node.id);
        const state = !node.session ? null : graph.unanswered.some((entry) => entry.session === node.session?.session) ? 'Runner did not answer; current state unknown' : node.session.shown === 'running' ? 'Running' : 'Starting, not yet confirmed';
        return <article className={'session-canvas-node ' + node.column + (shown ? ' open' : '')} key={node.id} data-node={node.id}
          style={{ left: box.x, top: box.y, width: box.w, height: box.h, zIndex: front === node.id ? 3 : node.session ? 2 : 1 }}
          onPointerDownCapture={() => setFront(node.id)}>
          <header className="session-canvas-bar" tabIndex={0} aria-label={'Move ' + node.title + ' with the arrow keys'} onKeyDown={nudge(node.id)}
            onPointerDown={begin((from) => ({ kind: 'move', id: node.id, from, box }))}>
            <h3>{node.title}</h3><span className="note">{node.detail}</span>
            {node.session && !shown ? <span className="note">{state}</span> : null}
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
        <p className="sub">Drag a window by its bar, drag its corner to size it, drag the background to move the surface; Control or Command with the wheel magnifies. Closing a terminal view leaves the process running. Team membership does not grant access; dashed grant connections no longer stand.</p>
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
