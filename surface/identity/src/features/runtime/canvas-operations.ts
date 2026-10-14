/**
 * The operations view kept with the person's canvas. `pane_width` is the tree's width the person set by dragging or
 * keying it; null until they do, when the tree is a fifth of the window (see `fifthOf`).
 */
export interface OperationsChoice { view: 'tree' | 'canvas' | 'sessions'; selected: string | null; pane_width: number | null }
/** The service accepts these pane widths, in whole pixels. */
export const PANE_WIDTH = { least: 200, most: 640 } as const;
export const FIRST_OPERATIONS: OperationsChoice = { view: 'tree', selected: null, pane_width: null };
/** A fifth of a window this wide, within the widths the service keeps. */
export const fifthOf = (window: number): number => Math.round(Math.max(PANE_WIDTH.least, Math.min(PANE_WIDTH.most, window * 0.2)));

export function readOperations(value: unknown): OperationsChoice | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return null;
  const given = value as { view: OperationsChoice['view']; selected: string | null; pane_width: number };
  if (!['tree', 'canvas', 'sessions'].includes(given.view) || (given.selected !== null && typeof given.selected !== 'string')
    || !Number.isSafeInteger(given.pane_width) || given.pane_width < PANE_WIDTH.least || given.pane_width > PANE_WIDTH.most) return null;
  return { view: given.view, selected: given.selected, pane_width: given.pane_width };
}
