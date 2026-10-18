import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { $, $$, click, mount, press, text } from "./harness";
import { ADA, GRANTS, ME, SERVICE, dashboard, ok } from "./fixtures";

const css = (file: string) => readFileSync(new URL("../src/" + file, import.meta.url), "utf8");
describe("Surface polish for new users", () => {
  it("closes Help on Escape", async () => {
    await mount("#/dashboard", SERVICE);
    await click($('#rail [data-dockbtn="help"]'));
    expect($("#helpLayer")).not.toBeNull();
    await press("Escape");
    expect($("#helpLayer")).toBeNull();
  });
  it("shows the current sign-in first and names its provider with the full address in a title", async () => {
    const session = (id: string, current: boolean) => ({
      id,
      current,
      login: { issuer: "https://signin.example.test/auth/v1/", subject: "ada" },
      started_at: 1,
      ends_at: 2,
    });
    await mount("#/me", {
      ...SERVICE,
      "/sessions": ok({ person: ADA, sessions: [session("other", false), session("here", true)] }),
    });
    expect($(".you-sessions tbody tr td")?.textContent).toContain("This session");
    expect($('.you-sessions [title="https://signin.example.test/auth/v1/"]')?.textContent).toBe("signin.example.test");
    expect($(".you-sessions")?.textContent).not.toContain("/auth/v1/");
  });
  it("names grants by their resource and keeps their full identifiers in titles", async () => {
    await mount("#/me", SERVICE);
    const cell = $('.grant-table td[data-col="Grant"]');
    expect(cell?.textContent).toContain("identity");
    expect(cell?.textContent).not.toMatch(/G\/[0-9a-f]/);
    expect(cell?.querySelector('[title="' + GRANTS[0].id + '"]')).not.toBeNull();
  });
  it("names generated app credentials without changing their lookup key", async () => {
    const key = "lys-app-" + ADA + "-cambium-api-op-" + "e".repeat(32);
    await mount("#/secrets", {
      ...SERVICE,
      "/secrets": ok({
        secrets: [{ name: key, class: "credential", owner: ADA, sequence: 3, upstream: null, header: null }],
      }),
    });
    expect($('.listing [title="' + key + '"]')?.textContent).toContain("Cambium");
    expect($('.listing [title="' + key + '"]')?.textContent).toContain(ME.person.display_name);
    expect($('.listing [data-col="Recorded change"]')?.textContent).toBe("Change 3");
    const link = $('.listing a[href*="?secret="]');
    expect(link?.getAttribute("href")).toBe("#/secrets/entries?secret=" + encodeURIComponent(key));
    expect(link?.textContent).toBe("Manage");
  });
  it("shows the selected question on Ask", async () => {
    await mount("#/access/ask", SERVICE);
    expect($(".check")?.closest(".pane")?.querySelector(".seg a.on")?.getAttribute("href")).toBe("#/access/can");
  });
  it("uses the shared Lys mark on the sign-in form", async () => {
    await mount("#/sign-in", SERVICE);
    expect($(".sign-in .seal")?.textContent).toBe("L");
    expect($(".sign-in-mark")).toBeNull();
  });
  it("does not draw a dashboard team with no agent anywhere beneath it", async () => {
    await mount("#/dashboard", {
      ...SERVICE,
      "/dashboard": ok(dashboard()),
      "/teams": ok({ teams: [{ id: "team-empty", name: "Unused crew", owner: ADA, state: "active", members: [] }] }),
    });
    expect($$(".you-fold").map((button) => button.textContent)).not.toContain("Unused crew0");
    expect(text()).not.toContain("Unused crew");
  });
  it("keeps empty drafts together and uses token checkboxes and spaced settings sections", async () => {
    await mount("#/access/drafts", SERVICE);
    expect($(".drafts-body .drafts")).not.toBeNull();
    expect(css("styles/forms.css")).toMatch(/input\[type=checkbox\][^}]*appearance: none/);
    expect(css("styles/screen.css")).toContain(".effective-settings");
    expect(css("features/usage/usage.css")).toMatch(/\.team-detail > \.usage[^}]*margin-top: 12px/);
  });
});
