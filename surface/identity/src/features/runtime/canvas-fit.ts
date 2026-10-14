import { useLayoutEffect, useRef } from "react";
import type { Dispatch, RefObject, SetStateAction } from "react";
import { SWAP, around, framed, zoomOf } from "./canvas-view";
import type { View } from "./canvas-view";
import type { Box } from "./canvas-marks";

export function fitCanvasBox(box: Box, element: HTMLElement, nearest = 1): View {
  const bars = element.querySelector(".canvas-bars")?.getBoundingClientRect().height ?? 0;
  return framed(box, element.clientWidth, element.clientHeight - bars, nearest);
}

export function useCanvasFit(
  surface: RefObject<HTMLDivElement | null>,
  boxes: Record<string, Box>,
  setView: Dispatch<SetStateAction<View>>,
) {
  const latest = useRef(boxes);
  latest.current = boxes;
  useLayoutEffect(() => {
    const element = surface.current;
    if (!element) return;
    const fit = () => {
      if (!element.clientWidth || !element.clientHeight) return;
      const all = Object.values(latest.current),
        part = around(all);
      if (!part) return;
      setView((current) => {
        const k = zoomOf(current);
        const visible = all.some(
          (box) =>
            current.x + box.x * k >= 0 &&
            current.y + box.y * k >= SWAP &&
            current.x + (box.x + box.w) * k <= element.clientWidth &&
            current.y + (box.y + box.h) * k <= element.clientHeight,
        );
        return visible ? current : fitCanvasBox(part, element);
      });
    };
    fit();
    const observer = typeof ResizeObserver === "undefined" ? null : new ResizeObserver(fit);
    observer?.observe(element);
    return () => observer?.disconnect();
  }, [surface, setView]);
}
