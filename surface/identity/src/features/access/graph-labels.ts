import { useLayoutEffect } from "react";
import type { RefObject } from "react";

export interface LabelBox {
  id: string;
  x: number;
  y: number;
  width: number;
  height: number;
}
export function separateLabels(labels: LabelBox[], bounds?: { top: number; bottom: number }): Map<string, number> {
  const placed: LabelBox[] = [],
    shifts = new Map<string, number>();
  for (const label of labels) {
    const box = { ...label };
    const overlaps = (y: number, other: LabelBox) =>
      box.x < other.x + other.width &&
      box.x + box.width > other.x &&
      y < other.y + other.height &&
      y + box.height > other.y;
    if (bounds) {
      const candidates = [
        Math.max(bounds.top, Math.min(box.y, bounds.bottom - box.height)),
        ...placed.flatMap((other) => [other.y + other.height + 4, other.y - box.height - 4]),
      ]
        .filter(
          (y) => y >= bounds.top && y + box.height <= bounds.bottom && !placed.some((other) => overlaps(y, other)),
        )
        .sort((a, b) => Math.abs(a - label.y) - Math.abs(b - label.y));
      if (candidates.length) {
        box.y = candidates[0];
        placed.push(box);
        shifts.set(label.id, box.y - label.y);
        continue;
      }
    }
    for (;;) {
      const overlaps = placed.filter(
        (other) =>
          box.x < other.x + other.width &&
          box.x + box.width > other.x &&
          box.y < other.y + other.height &&
          box.y + box.height > other.y,
      );
      if (!overlaps.length) break;
      box.y = Math.max(...overlaps.map((other) => other.y + other.height)) + 4;
    }
    placed.push(box);
    shifts.set(label.id, box.y - label.y);
  }
  return shifts;
}

export function useGraphLabels(
  svg: RefObject<SVGSVGElement | null>,
  nodes: { id: string; label: string; x: number; y: number }[],
  size: number,
  focus: string | undefined,
  hover: string | null,
) {
  useLayoutEffect(() => {
    const place = () => {
      const labels: LabelBox[] = [];
      const texts = new Map<string, { text: SVGTextElement; unit: number; dx: number }>();
      const viewport = svg.current?.getBoundingClientRect();
      const bounded = Boolean(viewport?.width && viewport.height);
      for (const group of svg.current?.querySelectorAll<SVGGElement>("g.gn") ?? []) {
        const id = group.dataset.node ?? "",
          node = nodes.find((each) => each.id === id);
        const text = group.querySelector("text");
        if (!node || !text || typeof text.getBBox !== "function") continue;
        text.textContent = node.label;
        text.setAttribute("x", "13");
        text.setAttribute("y", "4");
        const matrix = bounded && typeof text.getScreenCTM === "function" ? text.getScreenCTM() : null;
        if (matrix && viewport) {
          const unit = Math.abs(matrix.a);
          if (!unit) continue;
          const available = viewport.width - 8;
          if (text.getBoundingClientRect().width > available) {
            let end = node.label.length;
            while (end && text.getBoundingClientRect().width > available) {
              end -= 1;
              text.textContent = node.label.slice(0, end) + "…";
            }
          }
          const box = text.getBoundingClientRect();
          const x = Math.max(viewport.left + 4, Math.min(box.x, viewport.right - box.width - 4));
          labels.push({ id, x, y: box.y, width: box.width, height: box.height });
          texts.set(id, { text, unit, dx: x - box.x });
          continue;
        }
        const box = text.getBBox();
        labels.push({
          id,
          x: node.x + box.x * size,
          y: node.y + box.y * size,
          width: box.width * size,
          height: box.height * size,
        });
        texts.set(id, { text, unit: size, dx: 0 });
      }
      for (const [id, shift] of separateLabels(
        labels,
        bounded && viewport ? { top: viewport.top + 4, bottom: viewport.bottom - 4 } : undefined,
      )) {
        const placed = texts.get(id);
        if (!placed) continue;
        const { text, unit, dx } = placed;
        text.setAttribute("x", String(13 + dx / unit));
        text.setAttribute("y", String(4 + shift / unit));
        const leader = text.parentElement?.querySelector<SVGLineElement>(".label-leader");
        leader?.setAttribute("x2", String(13 + dx / unit));
        leader?.setAttribute("y2", String(shift / unit));
        leader?.setAttribute("visibility", shift || dx ? "visible" : "hidden");
      }
    };
    place();
    let active = true;
    void document.fonts?.ready.then(() => {
      if (active) place();
    });
    return () => {
      active = false;
    };
  }, [svg, nodes, size, focus, hover]);
}
