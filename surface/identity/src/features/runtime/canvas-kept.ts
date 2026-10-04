/**
 * Where a person's canvas is kept. It is theirs, not a browser's: the service keeps the arrangement they are working in
 * and every layout they saved by name, so it is the same on any computer they sign in at. When the service refuses to
 * keep it, this browser keeps it instead and the page says so, with the refusal by name.
 */
import { Refused, request } from '../../api';
import { readArrangement } from './canvas-marks';
import type { Arrangement } from './canvas-marks';

/** A layout saved by name. */
export interface SavedLayout { name: string; saved_at: number; arrangement: Arrangement }
export interface Keeping {
  where: 'service' | 'browser';
  /** Why the service is not keeping it, in the service's words; null while it is. */
  why: string | null;
  /** The arrangement being worked in as the service has it; null when it keeps none, or when this browser is keeping it. */
  arrangement: Arrangement | null;
  layouts: SavedLayout[];
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
    return { where: 'service', why: null, arrangement, layouts: answered(answer) };
  } catch (error) {
    const refused = error instanceof Refused ? error.refusal : { refusal: 'Unanswered', reason: String(error) };
    return { where: 'browser', why: refused.refusal + ': ' + refused.reason, arrangement: null, layouts: inBrowser() };
  }
}

/** Keeps the arrangement being worked in. This browser keeps its own copy as the canvas changes, so only the service is told here. */
export async function keepArrangement(where: Keeping['where'], arrangement: Arrangement): Promise<void> {
  if (where === 'service') await request('/canvas', { arrangement }, 'PUT');
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
