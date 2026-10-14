import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { $, mount } from "./harness";
import { ADA, SCRIBE, SERVICE, ok } from "./fixtures";
const css = (file: string) => readFileSync(new URL("../src/" + file, import.meta.url), "utf8");

describe("Full form controls in scrollable tables", () => {
  it("keeps the connected-tools table and every add control inside one scroll region", async () => {
    const description = {
      models: { minimum: 1, maximum: 1, further_encoding: { kind: "array" } },
      permissions: { modes: ["default"], rule_forms: [] },
      mcp: { transports: ["stdio"], working_directory: false, handle_variables: false, channel_policies: ["off"] },
      rendering_contract: "test",
    };
    const program = {
      name: "Care",
      models: [{ id: "care", label: "Care model" }],
      modes: [{ id: "default", meaning: "Asks first" }],
      builds: [{ name: "Installed", program: "/opt/care", package: "care", from: "runner" }],
      description,
    };
    const profile = {
      version: 1,
      operation: "op-" + "e".repeat(32),
      harness: { name: "Care", description, program: "/opt/care", package: "care" },
      model_access: ["care"],
      permissions: { default_mode: "default" },
      instructions: "",
      tools: [],
      skills: [],
      mcp_servers: [],
      note: "",
      set_by: ADA,
      set_at: 1,
      reviewed_by: ADA,
      session: null,
    };
    await mount("#/file/" + SCRIBE + "/provisioning", {
      ...SERVICE,
      ["/agents/" + SCRIBE + "/provisioning"]: ok({ agent: SCRIBE, profile, versions: [], enforced: false }),
      "/harnesses": ok({ programs: [program] }),
      "/skills": ok({ skills: [] }),
    });
    const table = $(".mcp-servers table");
    expect(table).not.toBeNull();
    expect(table?.closest(".table-scroll")?.getAttribute("role")).toBe("region");
    expect(table?.closest(".table-scroll")?.querySelector("tfoot select")).not.toBeNull();
  });
  it("sizes the policy select from its complete options and allows table columns to grow", () => {
    const forms = css("styles/forms.css");
    const usage = css("features/usage/usage.css");
    expect(forms).toMatch(/\.permissions select[^}]*min-width: max-content/);
    expect(usage).toMatch(/\.table-scroll \.usage-table[^}]*table-layout: auto/);
  });
});
