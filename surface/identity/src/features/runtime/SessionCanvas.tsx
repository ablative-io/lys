import { refreshLive } from '../../live';
/** A surface of windows: each permitted session and each thing it is connected to is a window a person drags, sizes and arranges, with as many live terminals open as they choose. */
import { useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react';
import type { KeyboardEvent, PointerEvent, ReactNode } from 'react';
import { useParams, useSearchParams } from 'react-router';
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
import { GROUP, NOTE, NO_MARKS, SMALLEST_MARK, markId, nearestSide, nextColour, readArrangement, sidePoint, standing, stood, within } from './canvas-marks';
import type { Arrangement, Box, Marks, Side } from './canvas-marks';
import { keepArrangement, readKeeping, removeLayout, said, saveLayout } from './canvas-kept';
import type { Keeping, SavedLayout } from './canvas-kept';
import { ProxyView, proxyHref } from '../proxy/ProxyView';
import { CanvasDock, KindSymbol, PANELS } from './CanvasDock';
import type { Panel, Tool } from './CanvasDock';
import { Anchors, GroupBox, LinkHandles, LinkLines, NoteCard } from './CanvasMarks';
import { HOME, ZOOM, useWheel, zoomOf, zoomed } from './canvas-view';
import type { View } from './canvas-view';
import { WidgetCard } from './CanvasWidgets';
import { AGENT, KINDS, scopeOf } from './canvas-widgets';
import { readBoard } from '../dashboard/board';
import type { Board } from '../dashboard/board';
import './session-canvas.css';
import './canvas-marks.css';

import { CLOSED, OPENED, lineBetween, placed } from './canvas-place';
export type { Box } from './canvas-marks';
export { lineBetween, placed } from './canvas-place';
export type { View } from './canvas-view';

/** What this browser keeps: the arrangement, where the surface is looked at from, and the arrival last answered (an agent at a place in this tab's history). */
interface Kept extends Arrangement { view: View; shown?: string }

const KEPT = 'lys.canvas';
/** The smallest a terminal window is dragged to: its bar still shows the agent's name beside state, Stop and close, and a prompt can still be read. */
const SMALLEST: [number, number] = [560, 180];

/**
 * A window is kept under its agent's name, not its session's: a session's name is new every time the agent starts, and
 * a layout saved today has to find the same agents tomorrow.
 */
const keptName = (graph: SessionGraph) => (id: string): string => {
  const agent = graph.nodes.find((node) => node.id === id)?.session?.agent;
  return agent ? AGENT + agent : id;
};
const hereName = (graph: SessionGraph) => (id: string): string =>
  id.startsWith(AGENT) ? graph.nodes.find((node) => node.session?.agent === id.slice(AGENT.length))?.id ?? id : id;
function renamed(arrangement: Arrangement, name: (id: string) => string): Arrangement {
  return {
    ...arrangement,
    boxes: Object.fromEntries(Object.entries(arrangement.boxes).map(([id, box]) => [name(id), box])),
    open: arrangement.open.map(name),
    links: arrangement.links.map((link) => ({ ...link, from: name(link.from), to: name(link.to) })),
  };
}

/** The arrangement this browser kept. One that cannot be read is no arrangement: the surface then lays itself out afresh. */
function kept(): Kept | null {
  try {
    const value: unknown = JSON.parse(localStorage.getItem(KEPT) ?? 'null');
    const arrangement = readArrangement(value);
    const view = (value as { view?: Record<string, unknown> } | null)?.view;
    if (!arrangement || !view || typeof view !== 'object' || !['x', 'y'].every((key) => Number.isFinite(view[key]))) return null;
    if (view.k !== undefined && !(typeof view.k === 'number' && view.k >= ZOOM[0] && view.k <= ZOOM[1])) return null;
    const shown = (value as { shown?: unknown }).shown;
    return { ...arrangement, view: view as unknown as View, ...(typeof shown === 'string' ? { shown } : {}) };
  } catch { return null; }
}

/** The address of a session's terminal alone, filling its own browser window. */
/** A line being dragged, from the dot it started on to the pointer. */
const lineTo = (from: [number, number], to: [number, number]): string => `M ${from[0]} ${from[1]} L ${to[0]} ${to[1]}`;

const windowOf = (session: { session: string; agent: string | null }): string =>
  '#/window/' + encodeURIComponent(session.session) + (session.agent ? '?agent=' + encodeURIComponent(session.agent) : '');

/** How far an arrow key moves a window whose bar has the keyboard. */
const STEPS: Record<string, [number, number] | undefined> = { ArrowLeft: [-16, 0], ArrowRight: [16, 0], ArrowUp: [0, -16], ArrowDown: [0, 16] };

type Drag = { kind: 'pan'; from: [number, number]; view: View }
  /** A thing moved or sized: `least` is the smallest it sizes to, `along` what sits in a moved box and goes with it. */
  | { kind: 'move' | 'size'; id: string; from: [number, number]; box: Box; least: [number, number]; along: [string, Box][] }
  /** A box being drawn out from where the press began on the surface. */
  | { kind: 'draw'; id: string; from: [number, number]; at: [number, number] }
  /** A line being dragged from a dot on one thing's edge to whatever it is let go over. */
  | { kind: 'link'; id: string; side: Side; from: [number, number] };

function Canvas({ graph, keeping, connections, board }: { graph: SessionGraph; keeping: Keeping; connections: ReactNode; board: Load<Board> }) {
  const { agent } = useParams();
  // What the service keeps for this person comes first; without it, what this browser kept. Where the surface is looked at from is always this browser's own.
  const [before] = useState<Kept | null>(() => {
    const here = kept();
    const theirs = keeping.arrangement ?? here;
    return theirs ? { ...renamed(theirs, hereName(graph)), view: here?.view ?? HOME, shown: here?.shown } : null;
  });
  // Arriving for an agent is one place in this tab's history. Reading the page again, or coming back to it, is the same place: the canvas then stays where it was.
  const arrival = agent ? agent + '@' + String((window.history.state as { key?: unknown } | null)?.key ?? '') : undefined;
  const answered = useRef(before?.shown);
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
  const [marks, setMarks] = useState<Marks>(() => before ? { groups: before.groups, notes: before.notes, links: before.links, widgets: before.widgets } : NO_MARKS);
  const [unkept, setUnkept] = useState<string | null>(null);
  const [unsent, setUnsent] = useState<string | null>(null);
  const [front, setFront] = useState<string | null>(null);
  const [tool, setTool] = useState<Tool>(null);
  // The kind of widget in hand while the widget tool is held.
  const [kind, setKind] = useState<string | null>(null);
  const [lineFrom, setLineFrom] = useState<string | null>(null);
  const [fresh, setFresh] = useState<string | null>(null);
  // The box, note, widget or line last pressed: the Delete key takes it away.
  const [chosen, setChosen] = useState<string | null>(null);
  // The widget that has just changed view: it moves between its two sizes, and nothing else on the surface does.
  const [morph, setMorph] = useState<string | null>(null);
  // The line being dragged from a thing's dot: where it started and where the pointer is, on the surface.
  const [dragged, setDragged] = useState<{ from: string; side: Side; to: [number, number] } | null>(null);
  const [layouts, setLayouts] = useState<SavedLayout[]>(keeping.layouts);
  // The panel that is out, and a saved layout to open, are in the address: a link goes to the canvas with that panel out or that layout open.
  const [search, setSearch] = useSearchParams();
  const asked_panel = search.get('panel');
  const panel: Panel = asked_panel !== null && asked_panel in PANELS ? asked_panel as Panel : null;
  const addressed = (change: (next: URLSearchParams) => void) => setSearch((now) => { const next = new URLSearchParams(now); change(next); return next; }, { replace: true });
  const setPanel = (next: Panel) => addressed((query) => { if (next) query.set('panel', next); else query.delete('panel'); });
  const wanted = search.get('layout');
  const [missing, setMissing] = useState<string | null>(null);
  // Counts what the person changed. Each change is kept for them on the service once it is whole: a drag when it ends, words when they are left.
  const [changes, setChanges] = useState(0);
  const changed = () => setChanges((now) => now + 1);
  const sending = useRef<Promise<void>>(Promise.resolve());
  const boxes = placed(graph, moved, open);
  const windows = graph.nodes.flatMap((node) => node.session?.agent && boxes[node.id] ? [{ id: node.id, agent: node.session.agent, box: boxes[node.id] }] : []);
  const drag = useRef<Drag | null>(null);
  const surface = useRef<HTMLDivElement>(null);
  /** Where a thing on the surface is: a window, a box or a note; a line's end kept under an agent's name is that agent's window. */
  // Widgets where they stand now: one opened out moves those under it down for as long as it is open.
  const widgets = useMemo(() => stood(marks.widgets), [marks.widgets]);
  const at = (id: string): Box | undefined => boxes[id] ?? boxes[hereName(graph)(id)] ?? [...marks.groups, ...marks.notes, ...widgets].find((each) => each.id === id);
  const put = (id: string, box: Box) => {
    const onto = <T extends Box & { id: string }>(all: T[]): T[] => all.map((each) => each.id === id ? { ...each, x: box.x, y: box.y, w: box.w, h: box.h } : each);
    if (id.startsWith('group:')) setMarks((all) => ({ ...all, groups: onto(all.groups) }));
    else if (id.startsWith('note:')) setMarks((all) => ({ ...all, notes: onto(all.notes) }));
    // A pill moves; only a widget opened out has a size of its own to change.
    else if (id.startsWith('widget:')) setMarks((all) => ({ ...all, widgets: all.widgets.map((each) => each.id !== id ? each : each.view ? { ...each, x: box.x, y: box.y, w: box.w } : { ...each, x: box.x, y: box.y }) }));
    else setMoved((all) => ({ ...all, [id]: box }));
  };
  /** Everything arranged, as it is kept: every window's place, with the places kept for agents that are not running now. */
  const arrangement = (): Arrangement => {
    // A card that is not here may only be missing because its read was refused this visit, and an agent that is not running keeps its place for when it is. A session with no agent that has ended keeps none.
    const away = Object.fromEntries(Object.entries(moved).filter(([id]) => !(id in boxes) && !id.startsWith('session:') && hereName(graph)(id) === id));
    return renamed({ boxes: { ...away, ...boxes }, open: [...open], ...marks }, keptName(graph));
  };
  useEffect(() => {
    if (!changes) return;
    const now = arrangement();
    sending.current = sending.current.then(() => keepArrangement(keeping.where, now)).then(() => setUnsent(null), (error: unknown) => setUnsent(said(error)));
  }, [changes]);
  // A tool is left with Escape, and a line half drawn is dropped with it.
  useEffect(() => {
    if (!tool) setLineFrom(null);
    if (!tool && !panel) return;
    // Escape leaves the tool in hand; with none in hand it puts the panel away.
    const leave = (event: globalThis.KeyboardEvent) => { if (event.key === 'Escape') { if (tool) setTool(null); else setPanel(null); } };
    window.addEventListener('keydown', leave);
    return () => window.removeEventListener('keydown', leave);
  }, [tool, panel]);
  /** Opens a window's terminal if it is closed and brings it to the middle of the view, in front. */
  const show = (id: string, theirs = true) => {
    const element = surface.current, was = boxes[id];
    if (!element || !was) return;
    const box = open.has(id) ? was : { ...was, w: OPENED[0], h: OPENED[1] };
    if (!open.has(id)) {
      setOpen((now) => new Set([...now, id]));
      setMoved((all) => ({ ...all, [id]: box }));
      if (theirs) changed();
    }
    setFront(id);
    setView((now) => { const k = zoomOf(now); return { x: element.clientWidth / 2 - (box.x + box.w / 2) * k, y: element.clientHeight / 2 - (box.y + box.h / 2) * k, k }; });
  };

  // On arriving, and again when the route names another agent: that agent's terminal is open, at the open size, in the middle of the view.
  useLayoutEffect(() => {
    if (asked && answered.current !== arrival) show(asked, false);
    if (asked) answered.current = arrival;
    // Only when the agent asked for changes: after that the view and the windows are the person's.
  }, [asked]);

  // A window is its agent's: when the agent's session ends and another begins, the new one takes the window's place and its open terminal.
  const agents = useRef(new Map<string, string>());
  useEffect(() => {
    for (const node of graph.nodes) if (node.session?.agent) agents.current.set(node.id, AGENT + node.session.agent);
    const here = new Set(graph.nodes.map((node) => node.id));
    const now = (id: string) => hereName(graph)(here.has(id) ? id : agents.current.get(id) ?? id);
    setOpen((held) => [...held].some((id) => now(id) !== id) ? new Set([...held].map(now)) : held);
    setMoved((held) => Object.keys(held).some((id) => now(id) !== id) ? Object.fromEntries(Object.entries(held).map(([id, box]) => [now(id), box])) : held);
  }, [graph]);

  // The canvas stays where it is: this browser keeps it as it changes, under each agent's name, so it is there when the page is read again and when the agents have been started again.
  useEffect(() => {
    // A browser may refuse to keep anything. The surface still works for this visit, and says that it will not be remembered.
    try {
      localStorage.setItem(KEPT, JSON.stringify({ ...arrangement(), view, ...(answered.current ? { shown: answered.current } : {}) } satisfies Kept));
      setUnkept(null);
    } catch (error) { setUnkept(error instanceof Error ? error.message : String(error)); }
  }, [graph, moved, open, view, marks]);

  useWheel(surface, setView);
  /** Zoom by a step, or back to actual size, about the middle of the surface. */
  const zoomBy = (factor: number | null) => {
    const element = surface.current;
    const [cx, cy] = element ? [element.clientWidth / 2, element.clientHeight / 2] : [0, 0];
    setView((now) => zoomed(now, factor === null ? 1 : zoomOf(now) * factor, cx, cy));
  };
  const [menu, setMenu] = useState<string | null>(null);
  // Double-pressing a window's bar brings the view in until that window fills it; double-pressing it again goes back to the view before.
  const back = useRef<{ id: string; view: View } | null>(null);
  const zoomTo = (id: string) => {
    const [element, box] = [surface.current, at(id)];
    if (!element || !box) return;
    if (back.current?.id === id) { const was = back.current.view; back.current = null; setView(was); return; }
    const k = Math.min(ZOOM[1], Math.max(ZOOM[0], Math.min(element.clientWidth / (box.w + 64), element.clientHeight / (box.h + 64))));
    back.current = { id, view };
    setView({ x: element.clientWidth / 2 - (box.x + box.w / 2) * k, y: element.clientHeight / 2 - (box.y + box.h / 2) * k, k });
  };

  const begin = (start: (from: [number, number]) => Drag) => (event: PointerEvent<HTMLElement>) => {
    if (event.button !== 0 || (event.target instanceof Element && event.target.closest('button, a, input, select, textarea, .terminal'))) return;
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
    if (now.kind === 'link') {
      const rect = surface.current?.getBoundingClientRect();
      if (rect) setDragged({ from: now.id, side: now.side, to: [(event.clientX - rect.left - view.x) / zoomOf(view), (event.clientY - rect.top - view.y) / zoomOf(view)] });
      return;
    }
    if (now.kind === 'draw') { put(now.id, { x: now.at[0] + Math.min(0, dx), y: now.at[1] + Math.min(0, dy), w: Math.abs(dx), h: Math.abs(dy) }); return; }
    if (now.kind === 'size') { put(now.id, { ...now.box, w: Math.max(now.least[0], now.box.w + dx), h: Math.max(now.least[1], now.box.h + dy) }); return; }
    put(now.id, { ...now.box, x: now.box.x + dx, y: now.box.y + dy });
    for (const [id, box] of now.along) put(id, { ...box, x: box.x + dx, y: box.y + dy });
  };
  const finish = (event: PointerEvent) => {
    const now = drag.current;
    drag.current = null;
    if (!now || now.kind === 'pan') return;
    if (now.kind === 'link') {
      // The line ends on the thing it is let go over: a window, a note or a box's bar. Let go over nothing, it is not drawn.
      const over = document.elementFromPoint(event.clientX, event.clientY)?.closest('[data-node], [data-note], [data-group], [data-widget]');
      const to = over?.getAttribute('data-node') ?? over?.getAttribute('data-note') ?? over?.getAttribute('data-group') ?? over?.getAttribute('data-widget');
      const [rect, onto] = [surface.current?.getBoundingClientRect(), to ? at(to) : undefined];
      setDragged(null);
      if (!to || to === now.id || !rect || !onto) return;
      // It leaves the edge it was dragged from and meets the edge it was let go nearest.
      const to_side = nearestSide(onto, [(event.clientX - rect.left - view.x) / zoomOf(view), (event.clientY - rect.top - view.y) / zoomOf(view)]);
      setMarks((all) => ({ ...all, links: [...all.links, { id: markId('link'), from: now.id, to, from_side: now.side, to_side }] }));
    }
    if (now.kind === 'draw') {
      // A press that was not dragged still makes a box, at the size a box is made at. Its label then takes the keyboard.
      setMarks((all) => ({ ...all, groups: all.groups.map((each) => each.id === now.id && (each.w < SMALLEST_MARK[0] || each.h < SMALLEST_MARK[1]) ? { ...each, w: GROUP[0], h: GROUP[1] } : each) }));
      setTool(null);
      setFresh(now.id);
    }
    changed();
  };
  /** Puts a widget on the surface: the kind in hand, or the kind named. `fed` is the window it was placed on, which a line then feeds it from. */
  const widgetAt = (x: number, y: number, fed: string | null, named: string | null = kind) => {
    const made = named ? KINDS[named] : undefined;
    if (!named || !made) return null;
    const widget = { id: markId('widget'), kind: named, x, y, w: made.size[0], h: made.size[1] };
    setMarks((all) => ({ ...all, widgets: [...all.widgets, widget], links: fed ? [...all.links, { id: markId('link'), from: fed, to: widget.id, from_side: 'right', to_side: 'left' }] : all.links }));
    setTool(null);
    changed();
    return widget;
  };
  /** Adds a widget for an agent's window: beside the window, under the widgets its lines already feed, with a line from the window. */
  const addFor = (id: string, named: string) => {
    const box = boxes[id];
    const fedHere = marks.widgets.map(standing).filter((widget) => marks.links.some((link) => (link.from === id && link.to === widget.id) || (link.to === id && link.from === widget.id)));
    widgetAt(box.x + box.w + 48, Math.max(box.y, ...fedHere.map((widget) => widget.y + widget.h + 10)), id, named);
  };
  /** A widget dragged from the panel and let go: on an agent's window it is fed by that agent; elsewhere on the canvas it goes where it was let go; off the canvas nothing is placed. */
  const drop = (named: string, x: number, y: number) => {
    const [element, over] = [surface.current, document.elementFromPoint(x, y)];
    if (!element || !over || !element.contains(over)) return;
    const node = over.closest('[data-node]')?.getAttribute('data-node');
    if (node && graph.nodes.find((each) => each.id === node)?.session?.agent) { addFor(node, named); return; }
    const [rect, k] = [element.getBoundingClientRect(), zoomOf(view)];
    widgetAt((x - rect.left - view.x) / k, (y - rect.top - view.y) / k, null, named);
  };
  /** A press on the surface itself: with the box tool it starts a box where it lands; otherwise it takes hold of the surface. */
  const pressed = (from: [number, number]): Drag => {
    const element = surface.current;
    // A press on the surface itself lets go of the window in front and of whatever was chosen.
    setFront(null);
    setChosen(null);
    if (!element || (tool !== 'box' && tool !== 'note' && tool !== 'widget')) return { kind: 'pan', from, view };
    const [rect, k] = [element.getBoundingClientRect(), zoomOf(view)];
    const here: [number, number] = [(from[0] - rect.left - view.x) / k, (from[1] - rect.top - view.y) / k];
    if (tool === 'widget') {
      // A widget goes where the press lands: in a box it counts the box's agents, anywhere else every agent, until a line feeds it.
      const widget = widgetAt(here[0], here[1], null);
      return widget ? { kind: 'move', id: widget.id, from, box: widget, least: SMALLEST_MARK, along: [] } : { kind: 'pan', from, view };
    }
    if (tool === 'note') {
      // A note goes where the press lands, and can be dragged into place before the press is let go.
      const note = { id: markId('note'), text: '', x: here[0], y: here[1], w: NOTE[0], h: NOTE[1] };
      setMarks((all) => ({ ...all, notes: [...all.notes, note] }));
      setTool(null);
      setFresh(note.id);
      return { kind: 'move', id: note.id, from, box: note, least: SMALLEST_MARK, along: [] };
    }
    const id = markId('group');
    setMarks((all) => ({ ...all, groups: [...all.groups, { id, label: '', x: here[0], y: here[1], w: 0, h: 0 }] }));
    return { kind: 'draw', id, from, at: here };
  };
  const moving = (id: string) => begin((from) => {
    setChosen(id in boxes ? null : id);
    const box = at(id) ?? { x: 0, y: 0, w: 0, h: 0 };
    // A box takes along what sits in it: windows, notes and smaller boxes.
    const inside: [string, Box][] = id.startsWith('group:')
      ? [...Object.entries(boxes), ...[...marks.notes, ...marks.groups, ...widgets].filter((each) => each.id !== id).map((each): [string, Box] => [each.id, each])].filter(([, each]) => within(box, each))
      : [];
    return { kind: 'move', id, from, box, least: SMALLEST, along: inside };
  });
  const linking = (id: string) => (side: Side) => begin((from) => ({ kind: 'link', id, side, from }));
  const sizing = (id: string, least: [number, number]) => begin((from) => ({ kind: 'size', id, from, box: at(id) ?? { x: 0, y: 0, w: 0, h: 0 }, least, along: [] }));
  /** With the line tool, a press on a thing picks it: the first is where the line starts, the second where it ends. */
  const pick = tool !== 'line' ? null : (id: string) => (event: PointerEvent<HTMLElement>) => {
    event.stopPropagation();
    event.preventDefault();
    if (lineFrom === null) { setLineFrom(id); return; }
    if (lineFrom !== id) { setMarks((all) => ({ ...all, links: [...all.links, { id: markId('link'), from: lineFrom, to: id }] })); changed(); }
    setTool(null);
  };
  /** Takes a box, a note or a line away; a line that ended on what was taken away goes with it. */
  const removeMark = (id: string) => {
    setMarks((all) => ({ groups: all.groups.filter((each) => each.id !== id), notes: all.notes.filter((each) => each.id !== id), widgets: all.widgets.filter((each) => each.id !== id), links: all.links.filter((each) => each.id !== id && each.from !== id && each.to !== id) }));
    changed();
  };
  const recoloured = (id: string) => {
    const next = <T extends { id: string; colour?: string }>(all: T[]): T[] => all.map((each) => each.id === id ? { ...each, colour: nextColour(each.colour) } : each);
    setMarks((all) => ({ ...all, groups: next(all.groups), notes: next(all.notes) }));
    changed();
  };
  // Delete, or Backspace, takes away the chosen box, note, widget or line; never while words are being typed, and never a window.
  useEffect(() => {
    if (!chosen) return;
    const take = (event: globalThis.KeyboardEvent) => {
      if ((event.key !== 'Delete' && event.key !== 'Backspace') || (event.target instanceof Element && event.target.closest('input, textarea, select, .terminal, [contenteditable]'))) return;
      // A locked widget stays.
      if (marks.widgets.some((each) => each.id === chosen && each.locked)) return;
      event.preventDefault();
      removeMark(chosen);
      setChosen(null);
    };
    window.addEventListener('keydown', take);
    return () => window.removeEventListener('keydown', take);
  }, [chosen, marks.widgets]);
  const reworded = (id: string, words: string) => {
    setMarks((all) => ({ ...all, groups: all.groups.map((each) => each.id === id ? { ...each, label: words } : each), notes: all.notes.map((each) => each.id === id ? { ...each, text: words } : each) }));
    changed();
  };
  // A layout named in the address is opened once, on arriving, and its name then leaves the address, so reading the page again keeps what the person has since arranged.
  useEffect(() => {
    if (wanted === null) return;
    const layout = layouts.find((each) => each.name === wanted);
    if (layout) openLayout(layout.arrangement);
    setMissing(layout ? null : wanted);
    addressed((query) => query.delete('layout'));
  }, [wanted]);
  const save = async (name: string) => setLayouts(await saveLayout(keeping.where, name, arrangement()));
  const remove = async (name: string) => setLayouts(await removeLayout(keeping.where, name));
  /** A saved layout becomes the arrangement: its windows go where it has them, and its boxes, notes and lines replace the ones here. */
  const openLayout = (layout: Arrangement) => {
    const now = renamed(layout, hereName(graph));
    setMoved(now.boxes);
    setOpen(new Set(now.open.filter((id) => graph.nodes.some((node) => node.id === id))));
    setMarks({ groups: now.groups, notes: now.notes, links: now.links, widgets: now.widgets });
    changed();
  };
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
    changed();
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
    changed();
  };
  /** Brings the surface back so its topmost, leftmost window sits at the corner. */
  const home = () => {
    const all = Object.values(boxes);
    if (all.length) setView((now) => ({ ...now, x: HOME.x - Math.min(...all.map((box) => box.x)) * zoomOf(now), y: HOME.y - Math.min(...all.map((box) => box.y)) * zoomOf(now) }));
  };

  return <><div className={'session-canvas-scroll' + (tool ? ' tool-' + tool : '')} role="region" aria-label="Agent connection canvas" tabIndex={0} ref={surface} onKeyDown={travel}
    onPointerDown={begin(pressed)} onPointerMove={during} onPointerUp={finish} onPointerCancel={finish}>
    <div className="session-canvas" style={{ transform: `translate(${view.x}px, ${view.y}px)` + (zoomOf(view) === 1 ? '' : ` scale(${zoomOf(view)})`), transformOrigin: '0 0' }}>
      {marks.groups.map((group) => <GroupBox key={group.id} group={group} fresh={fresh === group.id} pick={pick?.(group.id)} move={moving(group.id)} size={sizing(group.id, SMALLEST_MARK)} link={linking(group.id)}
        change={(label) => reworded(group.id, label)} colour={() => recoloured(group.id)} remove={() => removeMark(group.id)} />)}
      <svg className="session-canvas-lines" aria-hidden="true">{graph.edges.flatMap((edge) => {
        const [from, to] = [boxes[edge.from], boxes[edge.to]];
        return from && to ? [<path key={edge.id} d={lineBetween(from, to)} data-kind={edge.kind} data-standing={edge.stands} />] : [];
      })}<LinkLines links={marks.links} at={at} chosen={chosen} choose={setChosen} />{dragged && at(dragged.from) ? <path className="drawing-line" d={lineTo(sidePoint(at(dragged.from) as Box, dragged.side), dragged.to)} /> : null}</svg>
      {graph.nodes.map((node) => {
        const box = boxes[node.id], shown = open.has(node.id);
        const state = !node.session ? null : graph.unanswered.some((entry) => entry.session === node.session?.session) ? 'Runner did not answer; current state unknown' : node.session.shown === 'running' ? 'Running' : 'Starting, not yet confirmed';
        const kind = node.column === 'sessions' ? 'Agent' : node.column === 'teams' ? 'Team or sender' : 'Resource or recipient';
        const unanswered = !!node.session && graph.unanswered.some((entry) => entry.session === node.session?.session);
        return <article className={'session-canvas-node ' + node.column + (shown ? ' open' : '') + (unanswered ? ' unanswered' : '') + (menu === node.id ? ' menu-open' : '')} key={node.id} data-node={node.id} aria-label={kind + ': ' + node.title}
          style={{ left: box.x, top: box.y, width: box.w, height: box.h, zIndex: front === node.id ? 3 : node.session ? 2 : 1 }}
          onPointerDownCapture={(event) => {
            if (pick) pick(node.id)(event);
            // With a widget in hand, a press on an agent's window places it beside the window, with a line from the window feeding it.
            else if (tool === 'widget' && node.session?.agent) { event.stopPropagation(); event.preventDefault(); widgetAt(box.x + box.w + 48, box.y, node.id); }
            else setFront(node.id);
          }}>
          <header className="session-canvas-bar" tabIndex={0} aria-label={'Move ' + node.title + ' with the arrow keys' + (node.session ? '; with Shift, size its terminal' : '')} onKeyDown={nudge(node.id)}
            onPointerDown={moving(node.id)} onDoubleClick={(event) => { if (!(event.target instanceof Element && event.target.closest('button, a'))) zoomTo(node.id); }}>
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
            <span className="session-canvas-grip" aria-hidden="true" onPointerDown={sizing(node.id, SMALLEST)} />
          </> : null}
          <Anchors from={linking(node.id)} />
          {/* The agent's window in front holds out each kind of widget beside it: one press adds that widget, fed by a line from this agent. */}
          {node.session?.agent && front === node.id && !tool ? <div className="canvas-adders" role="toolbar" aria-label={'Add a widget for ' + node.title}>
            {Object.entries(KINDS).map(([named, each]) => <button key={named} type="button" className="canvas-symbol" data-add-widget={named} aria-label={'Add ' + each.label + ' for ' + node.title} title={'Add ' + each.label}
              onClick={() => addFor(node.id, named)}><KindSymbol kind={named} /></button>)}
          </div> : null}
        </article>;
      })}
      {marks.notes.map((note) => <NoteCard key={note.id} note={note} fresh={fresh === note.id} pick={pick?.(note.id)} move={moving(note.id)} size={sizing(note.id, SMALLEST_MARK)} link={linking(note.id)}
        change={(text) => reworded(note.id, text)} colour={() => recoloured(note.id)} remove={() => removeMark(note.id)} />)}
      {widgets.map((widget) => <WidgetCard key={widget.id} widget={widget} board={board} morph={morph === widget.id} chosen={chosen === widget.id} scope={board.status === 'ok' ? scopeOf(widget, marks.links, marks.groups, windows, board.data.rows) : null}
        pick={pick?.(widget.id)} move={moving(widget.id)} link={linking(widget.id)} remove={() => removeMark(widget.id)}
        fit={(h) => setMarks((all) => ({ ...all, widgets: all.widgets.map((each) => each.id === widget.id ? { ...each, h } : each) }))}
        set={(change) => { setMarks((all) => ({ ...all, widgets: all.widgets.map((each) => each.id === widget.id ? { ...each, ...change } : each) })); if ('view' in change) setMorph(widget.id); changed(); }} />)}
      <LinkHandles links={marks.links} at={at} remove={removeMark} />
      {chosen && at(chosen) ? <div className="canvas-chosen" aria-hidden="true" style={{ left: at(chosen)!.x, top: at(chosen)!.y, width: at(chosen)!.w, height: at(chosen)!.h }} /> : null}
    </div>
  </div>
  <CanvasDock graph={graph} show={show} tool={tool} setTool={setTool} picking={lineFrom !== null} panel={panel} setPanel={setPanel}
    zoom={Math.round(zoomOf(view) * 100)} zoomBy={zoomBy} home={home} connections={connections} kind={kind} place={(next) => { setKind(next); setTool('widget'); setPanel(null); }} drop={drop} keeping={keeping} layouts={layouts} save={save} remove={remove}
    says={<>
      {unkept ? <span className="why-not" role="status">This browser will not keep the arrangement: {unkept}</span> : null}
      {missing !== null ? <span className="why-not" role="alert">No layout is saved as {missing}.</span> : null}
      {unsent ? <span className="why-not" role="alert">The arrangement was not kept on the service. <small className="refusal-name">{unsent}</small></span> : null}
    </>} />
  </>;
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
function Whole({ graph, messages, keeping, board }: { graph: SessionGraph; messages: Load<MessageRead>; keeping: Keeping; board: Load<Board> }) {
  const [later, setLater] = useState<MessageRead | null>(null);
  const read = later ?? (messages.status === 'ok' ? messages.data : null);
  const whole = read ? withMessages(graph, read.messages) : graph;
  return <>
    <div className="session-canvas-strip">
      {graph.notices.map((notice) => <p className="note" role="status" key={notice}>{notice}</p>)}
      {graph.unanswered.map((entry) => <Unanswered key={entry.session} graph={graph} entry={entry} />)}
    </div>
    {/* The connections in words are in the panel that slides out over the canvas: there for a reader who wants them, never a line across the page. */}
    <Canvas graph={whole} keeping={keeping} board={board} connections={<>
      {messages.status === 'loading' ? <p role="status">Reading message connections…</p> : messages.status === 'refused' ? <p className="why-not" role="status">Message connections unavailable: {messages.refused.refusal.refusal}: {messages.refused.refusal.reason}</p> : null}
      {read ? <section className="canvas-about" aria-label="Message connections"><MessageConnections value={read} change={setLater} /></section> : null}
      <Connections graph={whole} />
    </>} />
  </>;
}

export function SessionCanvas() {
  const load = useLive(readSessionGraph, 'session-canvas');
  const messages = useLoad(firstMessagePage, 'canvas-message-edges');
  const keeping = useLoad(readKeeping, 'canvas-kept');
  const board = useLive(readBoard, 'canvas-board');
  const [search] = useSearchParams();
  const proxy = search.get('view') === 'proxy';
  return <div className="page fill session-canvas-page">
    <div className="head"><div><h1>Operations</h1>{load.status === 'refused' && !proxy ? <button type="button" onClick={refreshLive}>Reconnect</button> : null}</div></div>
    {/* Operations is two views of the same agents: the canvas they are arranged on, and the model calls they made. */}
    <nav className="operations-swap" aria-label="Operations views"><a href="#/canvas" aria-current={proxy ? undefined : 'page'}>Canvas</a><a href={proxyHref({})} aria-current={proxy ? 'page' : undefined}>Proxy</a></nav>
    {proxy ? <ProxyView board={board} /> : <div className="canvas-side">
        {/* With agents running, who is running is behind the bar's Agents button. With none, or while the canvas cannot be read, it is said here. */}
        {load.status === 'ok' && load.data.nodes.some((node) => node.session) ? null : <RunningList />}
        <Gate load={load} title="Agent canvas" ok={(graph) => graph.nodes.some((node) => node.session) ? (keeping.status === 'ok' ? <Whole graph={graph} messages={messages} keeping={keeping.data} board={board} /> : <p role="status">Reading your canvas…</p>) : <>
          {graph.notices.map((notice) => <p className="note" role="status" key={notice}>{notice}</p>)}
          {graph.unanswered.map((entry) => <Unanswered key={entry.session} graph={graph} entry={entry} />)}
        </>} />
    </div>}
  </div>;
}
