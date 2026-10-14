import { describe, expect, it } from "vitest";
import { $$, mount } from "./harness";
import { SERVICE } from "./fixtures";

describe("Measured graph labels", () => {
  it("separates labels when their measured glyph bounds overlap", async () => {
    const previous = Object.getOwnPropertyDescriptor(SVGElement.prototype, "getBBox");
    Object.defineProperty(SVGElement.prototype, "getBBox", {
      configurable: true,
      value: () => ({ x: 13, y: -8, width: 12000, height: 1200 }),
    });
    try {
      await mount("#/graph", SERVICE);
      const labels = $$("g.gn text");
      expect(labels.length).toBeGreaterThan(1);
      expect(labels.filter((label) => Number(label.getAttribute("y")) !== 4).length).toBeGreaterThan(0);
    } finally {
      if (previous) Object.defineProperty(SVGElement.prototype, "getBBox", previous);
      else Reflect.deleteProperty(SVGElement.prototype, "getBBox");
    }
  });
});
