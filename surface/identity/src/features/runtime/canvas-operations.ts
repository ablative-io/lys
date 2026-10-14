/** The operations view kept with the person's canvas. */
export interface OperationsChoice { view: 'tree' | 'canvas' | 'sessions'; selected: string | null; pane_width: number }
/** The service accepts these pane widths, in whole pixels. */
export const PANE_WIDTH = { least: 200, most: 640 } as const;
export const FIRST_OPERATIONS: OperationsChoice = { view: 'tree', selected: null, pane_width: 300 };

export function readOperations(value: unknown): OperationsChoice | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return null;
  const given = value as OperationsChoice;
  if (!['tree', 'canvas', 'sessions'].includes(given.view) || (given.selected !== null && typeof given.selected !== 'string')
    || !Number.isSafeInteger(given.pane_width) || given.pane_width < PANE_WIDTH.least || given.pane_width > PANE_WIDTH.most) return null;
  return { view: given.view, selected: given.selected, pane_width: given.pane_width };
}
