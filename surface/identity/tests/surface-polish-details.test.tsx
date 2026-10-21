import { act } from "react";
import { createRoot } from "react-dom/client";
import { describe, expect, it, vi } from "vitest";
import { RecordCopy } from "../src/shell/RecordCopy";
import { mayStart } from "../src/features/network/Network";
import { plain } from "../src/features/usage/reasons";
import { $, $$, click, mount, text } from "./harness";
import { ADA, COURIER, DIRECTORY, SCRIBE, SERVICE, dashboard, ok } from "./fixtures";
import type { Machine } from "../src/features/network/contract";

describe("Readable surface details", () => {
  it("names the app on its sign-in permission and retains its identifier in a title", async () => {
    const id = "op-" + "d".repeat(32);
    const app = {
      id,
      name: "Care records",
      state: "approved",
      redirects: [],
      sign_in: null,
      schema: { kinds: {} },
      version: 1,
      versions: [1],
      pending: null,
      client_id: null,
      service_account: null,
      client_credentials: [],
      registered_by: { kind: "person", id: ADA },
      registered_at: 1,
    };
    await mount("#/apps", { ...SERVICE, "/apps": ok({ apps: [app] }) });
    const label = $(".app-signin .tick");
    expect(label?.textContent).toBe(" Give Care records the person’s name");
    expect(label?.getAttribute("title")).toBe(id);
  });
  it("keeps the complete estate evidence in a title and the Access navigation above it", async () => {
    // The plan's producer owns its sentences: Lys shows them as written and rewrites no product's words.
    const evidence = "  Read from the fixture product's place handover in workspace ablative; used by this crew.  ";
    const plan = {
      version: 1,
      agents: [{ display_name: "Scribe" }],
      resources: [
        { kind: "fixture_product.place", id: "handover", relation: "reader", actions: ["read"], seats: ["Scribe"], evidence },
      ],
    };
    await mount("#/access/import", {
      ...SERVICE,
      "/identity/estate-plan": ok({ plan, loader: "op-" + "e".repeat(32) }),
    });
    expect($(".page > .tabs")).not.toBeNull();
    expect($('[title="' + evidence + '"]')?.textContent).toBe(
      "Read from the fixture product's place handover in workspace ablative; used by this crew.",
    );
    expect(text()).toContain(evidence.trim());
  });

  it("names context use without losing the refusal reason", () => {
    expect(plain("context_percent was not reported", { id: SCRIBE, name: "Scribe" })).toEqual([
      "Context use was not reported.",
    ]);
  });

  it("counts repeated agent references once and retains different identities with the same name", () => {
    const one = { id: SCRIBE, display_name: "Scribe", state: "active" as const };
    const two = { ...one, id: COURIER };
    const machine = { runtime: "lys-runner", may_run: [one, one, two] } as Machine;
    expect(mayStart(machine)).toBe("Scribe, Scribe");
  });

  it("puts running proxy agents first and exposes their full identifiers", async () => {
    const session = {
      session: "op-" + "a".repeat(32),
      agent: COURIER,
      machine: "op-" + "b".repeat(32),
      machine_name: "Ward laptop",
      runtime: "lys-runner",
      shown: "running",
      last_reported: "running",
      first_report_at: 1,
      last_report_at: 2,
      what: "Ready",
      stopped: null,
      reported_by: ADA,
    };
    await mount("#/canvas?view=proxy", {
      ...SERVICE,
      "/dashboard": ok(dashboard({ [COURIER]: { sessions: [session] } })),
    });
    expect($(".proxy-agents tbody tr")?.getAttribute("data-agent")).toBe(COURIER);
    expect($(".proxy-agents tbody a")?.getAttribute("title")).toBe(COURIER);
    expect($$(".proxy-agents tbody tr").length).toBe(3);
  });

  it("states both the total and active agents on each person", async () => {
    await mount("#/people", { ...SERVICE, "/directory/people": ok(DIRECTORY) });
    const row = $('tr[data-href="#/file/' + ADA + '"]');
    expect(row?.textContent).toContain("3 agents");
    expect(row?.textContent).toContain("(1 active)");
  });

  it("shows the configured cookie flag faithfully", async () => {
    const route = SERVICE["/configuration"];
    if (typeof route === "function") throw new Error("Expected configuration fixture");
    const configuration = route.body as { sign_in: object };
    await mount("#/settings/signin", {
      ...SERVICE,
      "/configuration": ok({ ...configuration, sign_in: { ...configuration.sign_in, secure_cookie: false } }),
    });
    const label = $$(".facts dt").find((row) => row.textContent === "HTTPS-only session cookie");
    expect(label?.nextElementSibling?.textContent).toBe("No");
  });

  it("names control sessions while retaining their actual values and titles", async () => {
    const session = "op-" + "c".repeat(32);
    const budget = (SERVICE["/dashboard"] as { body: ReturnType<typeof dashboard> }).body.agents[0].budget;
    await mount("#/file/" + SCRIBE + "/budgets", {
      ...SERVICE,
      ["/budgets/agent/" + SCRIBE]: ok(budget),
      ["/agents/" + SCRIBE + "/usage"]: ok({ agent: SCRIBE, used: [], receipts: [], last_reported_ms: null }),
      ["/agents/" + SCRIBE + "/goals"]: ok({ goals: [] }),
      // Managed controls are shown only when the saved setup asks for them (885b2f54), so this setup asks.
      ["/agents/" + SCRIBE + "/provisioning"]: ok({ agent: SCRIBE, profile: { session: { requires_controls: true } } }),
      ["/agents/" + SCRIBE + "/control-sessions"]: ok({ agent: SCRIBE, sessions: [session], after: null }),
      ["/runtime/sessions/" + session + "/controls"]: ok({ session, control: null }),
      ["/runtime/sessions/" + session + "/control-receipts"]: ok({ session, receipts: [], after: null }),
    });
    const option = $('select[aria-label="Control session"] option');
    expect(option?.textContent).toBe("Scribe: session 1");
    expect(option?.getAttribute("value")).toBe(session);
    expect(option?.getAttribute("title")).toBe(session);
    expect($(".usage .managed-controls")).not.toBeNull();
  });

  it.each([false, true])("copies the full identifier or exposes a copy failure (rejected=%s)", async (rejected) => {
    const previous = Object.getOwnPropertyDescriptor(navigator, "clipboard");
    const writeText = vi.fn(() => (rejected ? Promise.reject(new Error("Denied")) : Promise.resolve()));
    Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText } });
    const container = document.createElement("div");
    document.body.append(container);
    const root = createRoot(container);
    try {
      await act(async () => root.render(<RecordCopy name="Signing key" value="complete-key" />));
      await click(container.querySelector("button"));
      expect(writeText).toHaveBeenCalledExactlyOnceWith("complete-key");
      if (rejected) expect(container.querySelector("[role=alert]")?.textContent).toContain("complete-key");
      else expect(container.querySelector("button")?.textContent).toBe("Copied");
    } finally {
      await act(async () => root.unmount());
      container.remove();
      if (previous) Object.defineProperty(navigator, "clipboard", previous);
      else Reflect.deleteProperty(navigator, "clipboard");
    }
  });
});
