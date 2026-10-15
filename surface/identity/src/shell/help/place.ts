import type { Region } from './pages';

/** A callout's width, as liminal's console draws it. */
export const CALL_W = 260;

export interface Box {
  left: number;
  top: number;
  right: number;
  bottom: number;
  width: number;
  height: number;
}

export interface Bounds {
  /** The left edge callouts may not cross: the screen beside the rail. */
  minX: number;
  width: number;
  height: number;
}

/**
 * Where a callout goes beside its region (liminal's place()): from the callout's own measured height `ch`, then
 * held inside the screen area and the window, so no callout is ever drawn off screen.
 */
export function place(region: Region, b: Box, ch: number, bounds: Bounds): { left: number; top: number } {
  const m = 12;
  let x: number;
  let y: number;
  switch (region.at) {
    case 'c':
      x = b.left + b.width / 2 - CALL_W / 2;
      y = b.top + b.height / 2 - ch / 2;
      break;
    case 'above':
      x = b.left + b.width / 2 - CALL_W / 2;
      y = b.top - ch - m;
      break;
    case 'below':
      x = b.left + Math.min(20, b.width / 2);
      y = b.bottom + m;
      break;
    case 'left':
      x = b.left - CALL_W - m;
      y = b.top + 10;
      break;
    case 'right':
      x = b.right + m;
      y = b.top + 10;
      break;
    case 'in-tr':
      x = b.right - CALL_W - m;
      y = b.top + m + 26;
      break;
    case 'in-bl':
      x = b.left + m;
      y = b.bottom - ch - m;
      break;
  }
  y += region.dy ?? 0;
  return {
    left: Math.max(bounds.minX + 8, Math.min(bounds.width - CALL_W - 8, x)),
    top: Math.max(8, Math.min(bounds.height - ch - 8, y)),
  };
}

/** Whether an element is drawn on screen at all: a region hidden, folded or scrolled away gets no callout. */
export const onScreen = (b: Box, width: number, height: number): boolean =>
  b.width >= 4 && b.height >= 4 && b.bottom > 0 && b.right > 0 && b.top < height && b.left < width;

/** Whether two boxes share any area, with `gap` kept clear between them. */
const overlaps = (a: Box, b: Box, gap: number): boolean =>
  a.left < b.right + gap && b.left < a.right + gap && a.top < b.bottom + gap && b.top < a.bottom + gap;

const boxAt = (left: number, top: number, width: number, height: number): Box =>
  ({ left, top, width, height, right: left + width, bottom: top + height });

/**
 * Moves a placed callout off everything already drawn over the page (the keys panel and the callouts placed before
 * it), so no note is ever hidden under another. It tries left of the obstacle first, then below it, then above it,
 * each held inside the bounds; where no clear place exists the first clear-of-the-panel place is kept.
 */
export function settle(at: { left: number; top: number }, ch: number, obstacles: readonly Box[], bounds: Bounds): { left: number; top: number } {
  const m = 8;
  const clear = (left: number, top: number) => obstacles.every((o) => !overlaps(boxAt(left, top, CALL_W, ch), o, m / 2));
  const inside = (left: number, top: number) => left >= bounds.minX + m && left + CALL_W <= bounds.width - m && top >= m && top + ch <= bounds.height - m;
  if (clear(at.left, at.top)) return at;
  const tried: { left: number; top: number }[] = [];
  for (const o of obstacles) {
    if (!overlaps(boxAt(at.left, at.top, CALL_W, ch), o, m / 2)) continue;
    tried.push({ left: o.left - CALL_W - m, top: at.top }, { left: at.left, top: o.bottom + m }, { left: at.left, top: o.top - ch - m });
  }
  // A second round: past the obstacle a first move lands on.
  for (const first of [...tried]) {
    for (const o of obstacles) {
      if (overlaps(boxAt(first.left, first.top, CALL_W, ch), o, m / 2)) tried.push({ left: first.left, top: o.bottom + m }, { left: o.left - CALL_W - m, top: first.top });
    }
  }
  return tried.find((p) => inside(p.left, p.top) && clear(p.left, p.top)) ?? at;
}
