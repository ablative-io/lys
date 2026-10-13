/**
 * Where a person's canvas is kept. It is theirs, not a browser's: the service keeps the arrangement they are working in
 * and every layout they saved by name, so it is the same on any computer they sign in at. When the service refuses to
 * keep it, this browser keeps it instead and the page says so, with the refusal by name.
 */
import { Refused, request } from '../../api';
import { readArrangement } from './canvas-marks';
import type { Arrangement } from './canvas-marks';
import { ZOOM } from './canvas-view';
import type { View } from './canvas-view';
import { AGENT } from './canvas-widgets';
import type { SessionGraph } from './session-graph';

/** A layout saved by name. */
export interface SavedLayout { name: string; saved_at: number; arrangement: Arrangement }
export interface Keeping {
  where: 'service' | 'browser';
  /** Why the service is not keeping it, in the service's words; null while it is. */
  why: string | null;
  /** The arrangement being worked in as the service has it; null when it keeps none, or when this browser is keeping it. */
  arrangement: Arrangement | null;
  layouts: SavedLayout[];
  operations: OperationsChoice;
}

export interface OperationsChoice { view: 'tree' | 'canvas' | 'sessions'; selected: string | null; pane_width: number }
export const FIRST_OPERATIONS: OperationsChoice = { view: 'tree', selected: null, pane_width: 600 };

function operationsOf(arrangement: unknown): OperationsChoice {
  const choice = (arrangement as { operations?: unknown } | null)?.operations;
  if (choice === undefined) return FIRST_OPERATIONS;
  if (!choice || typeof choice !== 'object') throw unreadable();
  const given = choice as OperationsChoice;
  if (!['tree', 'canvas', 'sessions'].includes(given.view) || (given.selected !== null && typeof given.selected !== 'string')
    || !Number.isFinite(given.pane_width) || given.pane_width < 280 || given.pane_width > 1200) throw unreadable();
  return { view: given.view, selected: given.selected, pane_width: given.pane_width };
}

/** The view travels with the arrangement; a service that refuses its shape is named, never replaced by browser storage. */
export async function keepOperations(choice: OperationsChoice): Promise<void> {
  const before = await request<{ arrangement: unknown }>('/canvas');
  const arrangement = before.arrangement === null ? { boxes: {}, open: [], groups: [], notes: [], links: [], widgets: [] } : readArrangement(before.arrangement);
  if (!arrangement) throw unreadable();
  const answer = await request<{ arrangement: unknown }>('/canvas', { arrangement: { ...arrangement, operations: choice } }, 'PUT');
  if (!(answer.arrangement as { operations?: unknown } | null)?.operations) throw new Refused(0, { refusal: 'OperationsNotKept', reason: 'The service did not retain the operations view fields.' });
  const kept = operationsOf(answer.arrangement);
  if (JSON.stringify(kept) !== JSON.stringify(choice)) throw new Refused(0, { refusal: 'OperationsNotKept', reason: 'The service did not confirm the operations view, selected seat and pane width.' });
}

const LAYOUTS = 'lys.canvas.layouts';

/** What went wrong, said with the refusal's name when the service named one. */
export const said = (error: unknown): string => error instanceof Refused ? error.refusal.refusal + ': ' + error.refusal.reason : String(error);

function layoutsOf(value: unknown): SavedLayout[] | null {
  if (!Array.isArray(value)) return null;
  const layouts: SavedLayout[] = [];
  for (const entry of value as Record<string, unknown>[]) {
    const arrangement = readArrangement(entry?.arrangement);
    if (typeof entry?.name !== 'string' || typeof entry.saved_at !== 'number' || !arrangement) return null;
    layouts.push({ name: entry.name, saved_at: entry.saved_at, arrangement });
  }
  return layouts;
}

function inBrowser(): SavedLayout[] {
  try { return layoutsOf(JSON.parse(localStorage.getItem(LAYOUTS) ?? '[]')) ?? []; } catch { return []; }
}

const unreadable = () => new Refused(0, { refusal: 'UnreadableResponse', reason: 'the service answered a canvas that cannot be read' });

function answered(answer: { layouts: unknown }): SavedLayout[] {
  const layouts = layoutsOf(answer.layouts);
  if (!layouts) throw unreadable();
  return layouts;
}

/** The person's canvas from the service; from this browser, saying why, when the service refuses. */
export async function readKeeping(): Promise<Keeping> {
  try {
    const answer = await request<{ arrangement: unknown; layouts: unknown }>('/canvas');
    const arrangement = answer.arrangement === null ? null : readArrangement(answer.arrangement);
    if (answer.arrangement !== null && !arrangement) throw unreadable();
    return { where: 'service', why: null, arrangement, layouts: answered(answer), operations: operationsOf(answer.arrangement) };
  } catch (error) {
    const refused = error instanceof Refused ? error.refusal : { refusal: 'Unanswered', reason: String(error) };
    return { where: 'browser', why: refused.refusal + ': ' + refused.reason, arrangement: null, layouts: inBrowser(), operations: FIRST_OPERATIONS };
  }
}

/** Keeps the arrangement being worked in. This browser keeps its own copy as the canvas changes, so only the service is told here. */
export async function keepArrangement(where: Keeping['where'], arrangement: Arrangement): Promise<void> {
  if (where === 'service') {
    const before = await request<{ arrangement: { operations?: unknown } | null }>('/canvas');
    const operations = before.arrangement?.operations;
    await request('/canvas', { arrangement: { ...arrangement, ...(operations === undefined ? {} : { operations: operationsOf(before.arrangement) }) } }, 'PUT');
  }
}

/** Saves the arrangement under a name, over any layout of that name, and answers every saved layout. */
export async function saveLayout(where: Keeping['where'], name: string, arrangement: Arrangement): Promise<SavedLayout[]> {
  if (where === 'service') return answered(await request<{ layouts: unknown }>('/canvas/layouts', { name, arrangement }));
  const layouts = [...inBrowser().filter((each) => each.name !== name), { name, saved_at: Math.floor(Date.now() / 1000), arrangement }];
  localStorage.setItem(LAYOUTS, JSON.stringify(layouts));
  return layouts;
}

/** Removes a saved layout and answers the ones that remain. */
export async function removeLayout(where: Keeping['where'], name: string): Promise<SavedLayout[]> {
  if (where === 'service') return answered(await request<{ layouts: unknown }>('/canvas/layouts/remove', { name }));
  const layouts = inBrowser().filter((each) => each.name !== name);
  localStorage.setItem(LAYOUTS, JSON.stringify(layouts));
  return layouts;
}

/** What this browser keeps: the arrangement, where the surface is looked at from, and the arrival last answered (an agent at a place in this tab's history). */
export interface Kept extends Arrangement { view: View; shown?: string }

export const KEPT = 'lys.canvas';

/**
 * A window is kept under its agent's name, not its session's: a session's name is new every time the agent starts, and
 * a layout saved today has to find the same agents tomorrow.
 */
export const keptName = (graph: SessionGraph) => (id: string): string => {
  const agent = graph.nodes.find((node) => node.id === id)?.session?.agent;
  return agent ? AGENT + agent : id;
};
export const hereName = (graph: SessionGraph) => (id: string): string =>
  id.startsWith(AGENT) ? graph.nodes.find((node) => node.session?.agent === id.slice(AGENT.length))?.id ?? id : id;
export function renamed(arrangement: Arrangement, name: (id: string) => string): Arrangement {
  return {
    ...arrangement,
    boxes: Object.fromEntries(Object.entries(arrangement.boxes).map(([id, box]) => [name(id), box])),
    open: arrangement.open.map(name),
    links: arrangement.links.map((link) => ({ ...link, from: name(link.from), to: name(link.to) })),
  };
}

/** The arrangement this browser kept. One that cannot be read is no arrangement: the surface then lays itself out afresh. */
export function kept(): Kept | null {
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
