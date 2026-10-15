import { act } from "react";
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { Listing } from "../src/shell/Listing";
import { createRoot } from "react-dom/client";
import { mount, type } from "./harness";
import { SERVICE, ok, ADA } from "./fixtures";

const css = (file: string) => readFileSync(new URL("../src/" + file, import.meta.url), "utf8");
describe("Surface use on laptops and phones", () => {
  it("offers the confirmed computer because another computer's runner refusal applies only to that computer", async () => {
    sessionStorage.clear();
    const first = "op-" + "e".repeat(32),
      second = "op-" + "f".repeat(32);
    const machine = (id: string) => ({
      id,
      name: id === first ? "Own computer" : "Ward laptop",
      state: "in_use",
      kind: "Computer",
      runtime: "lys-runner",
      may_run: [],
      may_run_roles: [],
      named_by: ADA,
    });
    const contract = SERVICE["/surface-contract"];
    if (typeof contract === "function") throw new Error("Expected a schema fixture");
    const schema = contract.body as { paths: Record<string, unknown> };
    const program = {
      name: "Care",
      line: "Care program",
      models: [{ id: "care", label: "Care model" }],
      modes: [{ id: "workspace-write", meaning: "Own folder" }],
      instructions_modes: ["keep"],
      builds: [{ name: "Installed", program: "/opt/bin/care", package: "care", from: "runner" }],
      description: {
        models: { minimum: 1, maximum: 1, further_encoding: { kind: "array" } },
        permissions: { modes: ["workspace-write"], rule_forms: [] },
        mcp: { transports: ["stdio"], working_directory: false, handle_variables: false, channel_policies: ["off"] },
        rendering_contract: "test",
      },
    };
    const { container, posted } = await mount("#/agents/new", {
      ...SERVICE,
      "/surface-contract": ok({
        paths: {
          ...schema.paths,
          "/agents": {
            post: { requestBody: { content: { "application/json": { schema: { properties: { answers_to: {} } } } } } },
          },
        },
      }),
      "/network": ok({ machines: [machine(first), machine(second)], reports_served: true }),
      ["/network/machines/" + first + "/runner"]: ok({ machine: first, runner: { kind: "lys" }, answers: true }),
      ["/network/machines/" + second + "/runner"]: ok({ machine: second, runner: null }),
      "/harnesses": ok({ programs: [program] }),
      "/skills": ok({ skills: [] }),
    });
    expect(container.querySelector(".add-agent-two")).not.toBeNull();
    const form = container.querySelector("form");
    // No name is pre-filled, so Add is off until one is typed; once it is, the confirmed computer alone keeps the form live.
    expect(form?.querySelector<HTMLButtonElement>('button[type="submit"]')?.disabled).toBe(true);
    await type(form?.querySelector('input[name="display_name"]') ?? null, "Clover");
    expect(form?.querySelector<HTMLButtonElement>('button[type="submit"]')?.disabled).toBe(false);
    expect(form?.querySelector<HTMLSelectElement>('select[name="computer"]')?.selectedOptions[0]?.textContent).toBe("Own computer");
    expect(form?.textContent).toContain("Ward laptop: not available: The computer’s runner is missing or could not be confirmed.");
    expect(form?.querySelector('select[name="computer"] option[value="' + second + '"]')).toBeNull();
    expect(form?.textContent).toContain("runner");
    expect(form?.textContent).not.toContain("RunnerReceiptMismatch");
    expect(form?.querySelector('a[href="#/network"]')).not.toBeNull();
    expect(posted.filter((call) => call.path === "/agents")).toEqual([]);
  });

  it("keeps headers and every row in one keyboard reachable scroll region", () => {
    const container = document.createElement("div");
    document.body.append(container);
    const root = createRoot(container);
    act(() =>
      root.render(
        <Listing
          groups={[{ id: "", name: "", lead: "", depth: 0, items: ["row"], within: ["row"] }]}
          columns={[{ head: "Name", cell: (item: string) => item }]}
          id={(item) => item}
          href={() => "#/people"}
          words={(item) => item}
          noun="people"
          holds={() => "1 person"}
          selected={null}
          select={() => undefined}
        />,
      ),
    );
    const pane = container.querySelector(".table-scroll");
    expect(pane?.getAttribute("role")).toBe("region");
    expect(pane?.getAttribute("tabindex")).toBe("0");
    expect(pane?.querySelector("thead")?.textContent).toBe("Name");
    expect(pane?.querySelector("tbody")?.textContent).toBe("row");
    act(() => root.unmount());
  });

  it("stacks the form, review panels and account cards and gives dashboard cards readable widths", () => {
    expect(css("styles/forms.css")).toMatch(/add-agent-two[^}]*auto-fit/);
    expect(css("styles/screen.css")).toMatch(/\.fill > \.body\.work[^}]*grid-template-columns: minmax\(0, 1fr\)/);
    expect(css("features/dashboard/dashboard.css")).toContain("container-type: inline-size");
    expect(css("features/dashboard/dashboard.css")).toMatch(/dash-widgets[^}]*auto-fit/);
    expect(css("features/me/you.css")).toMatch(/you-account-row[^}]*auto-fit/);
  });
});
