import { describe, expect, it, vi } from "vitest";
import { $, click, mount } from "./harness";
import { SCRIBE, SERVICE, ok } from "./fixtures";
import { mockTerminal } from "./terminal-double";
vi.mock("@gespenst/core", () => ({ createTerminal: mockTerminal }));
const session = "op-" + "7".repeat(32);
describe("Canvas fit to the available surface", () => {
  it.each([342, 1232])("opens a whole terminal in %s pixels of canvas", async (width) => {
    vi.stubGlobal(
      "ResizeObserver",
      class {
        observe() {}
        disconnect() {}
      },
    );
    vi.spyOn(HTMLElement.prototype, "clientWidth", "get").mockReturnValue(width);
    vi.spyOn(HTMLElement.prototype, "clientHeight", "get").mockReturnValue(844);
    await mount("#/canvas?view=canvas", {
      ...SERVICE,
      "/runtime/live": ok({
        sessions: [
          {
            session,
            agent: SCRIBE,
            machine: "test-machine",
            machine_name: "Test runner",
            runtime: "sh",
            shown: "running",
            last_reported: "running",
            last_report_at: 1,
            stopped: null,
          },
        ],
        unanswered: [],
      }),
    });
    if (width === 1232) await click($('[data-act="zoom-out"]'));
    await click($('[data-node="session:' + session + '"] [data-act="toggle"]'));
    const transform = $(".session-canvas")?.style.transform ?? "";
    const values = /translate\(([-\d.]+)px, ([-\d.]+)px\)(?: scale\(([-\d.]+)\))?/.exec(transform);
    expect(values).not.toBeNull();
    const node = $('[data-node="session:' + session + '"]');
    const scale = Number(values?.[3] ?? 1),
      x = Number(values?.[1] ?? 0),
      y = Number(values?.[2] ?? 0);
    if (width === 1232) expect(scale).toBe(1);
    const left = x + Number.parseFloat(node?.style.left ?? "0") * scale;
    const top = y + Number.parseFloat(node?.style.top ?? "0") * scale;
    expect(left).toBeGreaterThanOrEqual(0);
    expect(left + Number.parseFloat(node?.style.width ?? "0") * scale).toBeLessThanOrEqual(width);
    expect(top).toBeGreaterThanOrEqual(56);
  });
});
