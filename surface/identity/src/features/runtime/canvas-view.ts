/** Where the canvas is looked at from: its place and its zoom, and the wheel and pinch that change them. */
import { useEffect } from 'react';
import type { Dispatch, RefObject, SetStateAction } from 'react';

/**
 * The part of the surface the page shows: where the surface's origin sits and how far it is zoomed (1 when absent).
 * Zooming scales the drawing only; a window's own size, which its terminal counts its columns from, does not change.
 */
export interface View { x: number; y: number; k?: number }
export const zoomOf = (view: View): number => view.k ?? 1;
/** The nearest and the farthest the surface zooms: far enough to see every window at once, near enough to read small text. */
export const ZOOM: [number, number] = [0.2, 3];
/** The view zoomed to `k` with the surface point under (`cx`, `cy`) staying where it is. */
export function zoomed(view: View, k: number, cx: number, cy: number): View {
  const from = zoomOf(view), to = Math.min(ZOOM[1], Math.max(ZOOM[0], k));
  return { x: cx - (cx - view.x) * to / from, y: cy - (cy - view.y) * to / from, k: to };
}
export const HOME: View = { x: 24, y: 24 };
/**
 * The view that holds one part of the surface whole, in the middle of a page `width` by `height`, with `clear` left
 * around it; zoomed in no further than `nearest`.
 */
export function framed(part: { x: number; y: number; w: number; h: number }, width: number, height: number, nearest = ZOOM[1], clear = 64): View {
  const k = Math.min(nearest, Math.max(ZOOM[0], Math.min(width / (part.w + clear), height / (part.h + clear))));
  return { x: width / 2 - (part.x + part.w / 2) * k, y: height / 2 - (part.y + part.h / 2) * k, k };
}
/** The part of the surface that holds every one of `things`; none when there are no things. */
export function around(things: { x: number; y: number; w: number; h: number }[]): { x: number; y: number; w: number; h: number } | undefined {
  if (!things.length) return undefined;
  const [x, y] = [Math.min(...things.map((each) => each.x)), Math.min(...things.map((each) => each.y))];
  return { x, y, w: Math.max(...things.map((each) => each.x + each.w)) - x, h: Math.max(...things.map((each) => each.y + each.h)) - y };
}

/**
 * The wheel moves the surface, and a pinch zooms it about the pointer: a trackpad's pinch arrives as a wheel with
 * Control held, Safari's as gesture events. A terminal keeps its own wheel.
 */
export function useWheel(surface: RefObject<HTMLDivElement | null>, setView: Dispatch<SetStateAction<View>>): void {
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
      if (event.target instanceof Element && event.target.closest('.terminal, textarea')) return;
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
}
