import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { windowText } from "../src/features/grants/model";
import { $, $$, click, mount, text } from "./harness";
import { ADA, GRANTS, SCRIBE, SERVICE, ok } from "./fixtures";

const css = (file: string) => readFileSync(new URL("../src/" + file, import.meta.url), "utf8");
describe("Surface words and controls", () => {
  it("names an absent start without an epoch and preserves a real start", () => {
    expect(windowText({ ...GRANTS[0], window: { starts_at: 0, ends_at: null }, effective_ends_at: null })).toBe(
      "no start, no end",
    );
    expect(windowText(GRANTS[0])).not.toContain("no start");
  });
  it.each([
    "#/spelling-mistake",
    "#/settings/profiles",
    "#/access/unknown",
    "#/people/view/unknown",
  ])("names an unknown address %s", async (address) => {
    await mount(address, SERVICE);
    expect($(".page h1")?.textContent).toBe("Not found");
    expect(text()).toContain(address.slice(1));
  });
  it("gives an unknown agent name the agents list instead of an access accusation", async () => {
    await mount("#/team/Unknown%20helper", SERVICE);
    expect(text()).toContain("No agent called Unknown helper");
    expect(text()).not.toContain("PersonNotVisible");
    expect($('a[href="#/people/view/agents"]')).not.toBeNull();
  });
  it("does not offer Stop for a known offline seat and preserves the full identifiers in titles", async () => {
    await mount("#/canvas?view=sessions", {
      ...SERVICE,
      "/seats": ok({
        runner: "read",
        seats: [
          {
            name: "Ward helper",
            agent: SCRIBE,
            machine: "op-" + "e".repeat(32),
            state: "offline",
            harness: "Care",
            profile_version: 1,
            working_folder: null,
            responsible: ADA,
            session: "op-" + "f".repeat(32),
            created_by: ADA,
            created_at: 1,
            revision: 1,
          },
        ],
      }),
      "/seats/unregistered": ok({ sessions: [] }),
    });
    const row = $('tr[data-seat="Ward helper"]');
    expect(row?.querySelector('button[aria-label="Start Ward helper"]')).not.toBeNull();
    expect(row?.querySelector('button[aria-label="Stop Ward helper"]')).toBeNull();
    expect(row?.querySelector('[title="' + SCRIBE + '"]')?.textContent).toBe("Scribe");
  });
  it("matches the directory visibility on Graph and keeps retired records reachable on request", async () => {
    await mount("#/graph", SERVICE);
    expect($$("g.gn").map((node) => node.getAttribute("aria-label"))).not.toContain("Lamplighter");
    const toggle = $$("button").find((button) => button.textContent?.startsWith("Show retired"));
    await click(toggle ?? null);
    expect($$("g.gn").map((node) => node.getAttribute("aria-label"))).toContain("Lamplighter");
  });
  it("keeps the count below the Access icon and gives short values whole words", () => {
    expect(css("styles/overlays.css")).not.toMatch(/\.rail:not\(\.open\) \.cnt[^}]*position: absolute/);
    expect(css("features/grants/grants.css")).toContain('[data-col="Allows"]');
    expect(css("features/runtime/session-canvas.css")).toMatch(/operations-swap[^}]*position: static/);
    expect(css("styles/dock.css")).not.toMatch(/\.legend[^}]*position: absolute/);
  });
});
