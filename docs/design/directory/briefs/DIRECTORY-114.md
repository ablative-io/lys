---
type: brief
id: DIRECTORY-114
cluster: directory
title: Lys names no product: the agent steps, Getting started and the import read whichever app is approved, by its own registration
---

# DIRECTORY-114: Lys names no product: the agent steps, Getting started and the import read whichever app is approved, by its own registration

> **Cluster:** directory
> **Depends on:** DIRECTORY-048
> **Checklist:**
> - C368 — Lys holds no app's name or schema in code or configuration and makes no call to any app (DIRECTORY-048 R7).
> - C825 — An active agent's overview names each approved app by its registration: an access line where the agent holds a grant under the app's id, else the steps to use it there; with none approved, connect an app. DIRECTORY-114/R1.
> - C826 — The note before any app's resources are held names no app and links Access to the one approved app, or to Access when more than one is. DIRECTORY-114/R2.
> - C827 — Getting started's step is Connect an app, done when any app is approved, naming the first pending app when one asks. DIRECTORY-114/R3.
> - C828 — The estate import shows evidence as written, and the seat import excludes a server entry holding only tool approvals by its shape, never a name. DIRECTORY-114/R4.
> **Stories:**
> - S151 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As a developer of an app, I want to register my app and its permission schema through a documented API, so that my app's resources and actions are checked by Lys without anyone changing Lys.

## Purpose

DIRECTORY-048 R7 (C368): Lys depends on no app and holds no app's name in its code, configuration or screens; an app is known to Lys only by the registration it makes through the Apps API. The rule that says so (rules/ast-grep/no-app-names.yml) runs in Lys's gate and on 22 October 2026 reports 94 findings in 14 files, every one a product's name written into Lys: the agent page's next step and the access note are written for Cambium alone (CambiumNext.tsx, CambiumFirst.tsx and the three screens that use them), Getting started has a 'Connect Cambium' step, the estate import rewrites Cambium's evidence words, the seat import excludes a server by the name 'argus', and four comments credit aion's console. Waffles told Tom on 22 October that this brief would be written that day. Each finding is changed so no product's name remains; nothing is added to the rule's ignores, and the rule is not weakened.

## Task

Replace every product name the no-app-names rule finds under crates/ and surface/ with behaviour that reads the app from Lys's own apps list (its id, its registered name, its standing and its address), so the same screens serve Cambium today and any other app registered tomorrow; exclude a seat profile's leftover server entry by its shape, not its name; reword the comments. The rule's three legs (rust, tsx, typescript) report no finding on the landing tree, and the gate's ast-grep stage is green. Every red is written and shown red before its code through the gate on Dean's laptop; the whole brief is written before anything is built.

## Requirements

### R1: The agent's next step names each approved app by its registration

Behavioural. CambiumNext.tsx becomes AppNext.tsx (export AppNext, same props). WHEN an active agent's overview is shown, THE SYSTEM SHALL read /apps once and, for every app whose state is approved, say one line or one section named for that app's registered name (the apps list's name member; its id when name is absent): when the agent holds a standing grant on a kind under that app's id prefix (kind starts with '<id>.'), the line '<agent> has access in <app name>' with, when the app's address is served, a link 'Open <app name>', and a link to change what it may use on its Access tab; otherwise a section headed 'Use <agent> in <app name>' with two steps: open the app (a link when its address is served) and 'Give <agent> what it may use in <app name>' linking to the agent's Access tab. WHEN no app is approved, one section headed 'Use <agent> in an app' says, for the administrator, 'No app is connected to this Lys yet. Connect one on Apps, then come back to this page for the next step.' with Apps linked, and for anyone else that the Lys administrator connects apps on Apps. The words that only one product's screens use (Invite, channel, 'Invite an agent from Lys') are not said by Lys. Nothing is shown until both reads answer, as today; a refused read shows ErrorWords. The address is read by addressOf exactly as today. AgentOverview.tsx renders AppNext. No product's id or name is a constant anywhere in the file.

**Acceptance:**
- src/features/file/app-next.test.tsx (replacing cambium-next.test.tsx, every case it held kept): with /apps answering one approved app {id 'fixture_product', name 'Fixture Product', address 'https://fixture.example.test'} and no grants, the overview shows 'Use Scribe in Fixture Product' with 'Open Fixture Product' linked to https://fixture.example.test and the Access step linked to the agent's Access tab
- The same with a standing grant on fixture_product.space for the agent shows the single line 'Scribe has access in Fixture Product' and no section
- Two approved apps with a grant under only one: one access line for the first, one section for the second, in the apps list's order
- No approved app (one pending): the administrator sees 'No app is connected to this Lys yet.' with Apps linked; a person who is not the administrator sees no link to Apps
- A refused /apps shows its words under 'Use <agent> in an app'; nothing is rendered while /apps or /grants has not answered, as today

**Files:**
- create: surface/identity/src/features/file/AppNext.tsx
- create: surface/identity/src/features/file/app-next.test.tsx
- modify: surface/identity/src/features/file/AgentOverview.tsx
- delete: surface/identity/src/features/file/CambiumNext.tsx
- delete: surface/identity/src/features/file/cambium-next.test.tsx

**Checklist:**
- C825 — An active agent's overview names each approved app by its registration: an access line where the agent holds a grant under the app's id, else the steps to use it there; with none approved, connect an app. DIRECTORY-114/R1.

**Stories:**
- S151 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As a developer of an app, I want to register my app and its permission schema through a documented API, so that my app's resources and actions are checked by Lys without anyone changing Lys.

### R2: The note before any app's resources are held names no app

Behavioural. CambiumFirst.tsx becomes AppsFirst.tsx (export AppsFirst and anyAppResource, unchanged in what it decides). WHEN an agent would be given access and no app's resources are held (AddAgent.tsx, sections.tsx, Delegate.tsx, as today), THE SYSTEM SHALL read /apps and say: when at least one app is approved, for the administrator 'An app is approved. Give yourself access to it on Access first; what it shares appears here once you hold it.' with Access linked to #/access/issue?app=<id> of the first approved app in the apps list's order when exactly one is approved and to #/access/issue when more than one is, and for anyone else 'Ask an administrator for access to an app; what it shares appears here once you hold it.'; when none is approved, for the administrator 'Connect an app on Apps first; what it shares appears here once it is approved.' and for anyone else 'No app is connected to Lys yet. An administrator connects one on Apps.' The data attribute is data-apps-first with values access and apps. The administrator is read as today (/directory/people answers). Nothing is said until both reads answer.

**Acceptance:**
- tests/file.test.tsx asserts data-apps-first and the new words where it asserted data-cambium-first, every case kept (connect first, approved for the administrator and for anyone else, not connected, /apps refused, pending, silent once an app resource is held)
- One approved app 'fixture_product': the administrator's Access link is #/access/issue?app=fixture_product; two approved apps: the link is #/access/issue
- No approved app: the administrator's note links to #/apps; anyone else's has no link

**Files:**
- create: surface/identity/src/features/grants/AppsFirst.tsx
- modify: surface/identity/src/features/people/AddAgent.tsx
- modify: surface/identity/src/features/file/sections.tsx
- modify: surface/identity/src/features/grants/Delegate.tsx
- modify: surface/identity/tests/file.test.tsx
- delete: surface/identity/src/features/grants/CambiumFirst.tsx

**Checklist:**
- C826 — The note before any app's resources are held names no app and links Access to the one approved app, or to Access when more than one is. DIRECTORY-114/R2.

**Stories:**
- S151 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As a developer of an app, I want to register my app and its permission schema through a documented API, so that my app's resources and actions are checked by Lys without anyone changing Lys.

### R3: Getting started asks for an app, not for one product

Behavioural. Getting started's standing carries app: 'approved' | 'pending' | null in place of cambium (approved when any app in /apps is approved, else pending when any is pending, else null) and asking: string | null, the first pending app's registered name (its id when nameless) when app is pending, else null. The step's key is 'app' and its title 'Connect an app'; it is done when app is approved. For anyone but the administrator it says 'No app is connected to Lys yet. An administrator connects one on Apps; once it is, this step is done.'; for the administrator, when an app is pending, '<asking> has asked to connect. Review what it asks for and approve it on Apps.', else 'Connect an app to Lys on Apps.', Apps linked. The step is offered only when /apps answered, as today. dashboard-cherie.fixture.ts writes app: null and asking: null.

**Acceptance:**
- tests/getting-started.test.tsx: every case that read 'Connect Cambium' reads 'Connect an app'; a pending app named 'Fixture Product' shows 'Fixture Product has asked to connect.'; one approved app of any id marks the step done
- The dashboard fixture renders with app: null and the step open

**Files:**
- modify: surface/identity/src/features/dashboard/GettingStarted.tsx
- modify: surface/identity/src/features/dashboard/dashboard-cherie.fixture.ts
- modify: surface/identity/tests/getting-started.test.tsx

**Checklist:**
- C827 — Getting started's step is Connect an app, done when any app is approved, naming the first pending app when one asks. DIRECTORY-114/R3.

**Stories:**
- S151 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As a developer of an app, I want to register my app and its permission schema through a documented API, so that my app's resources and actions are checked by Lys without anyone changing Lys.

### R4: The import shows evidence as written and excludes a leftover server by its shape

Behavioural and structural. EstateApproval.tsx's evidenceWords SHALL show a resource's evidence as the plan writes it, trimmed, with no rewrite keyed on any product's words; the plan's producer owns its sentences. In the seat import (crates/lys-identity-server/src/seat_import_profiles/servers.rs), WHEN a server entry names neither a command nor an address AND every member it holds is tools (per-tool approval settings), THE SYSTEM SHALL exclude it with the reason 'approvals_only: this entry holds tool approvals for a server it does not start; the approvals are not carried', whatever the entry's name; an entry naming neither and holding any other member stays refused import_member_unsupported as today. No server name is compared anywhere in the import. Comments naming a product are reworded without the name: IssueRoot.tsx:57's example kind becomes `fixture_product.channel`, and HelpDock.tsx (three), help/concepts/types.ts and help/url.ts say what the help does without crediting another product's console.

**Acceptance:**
- tests/seat_import_sources/live_shapes.rs: the recorded entry holding only tools is excluded with a reason starting 'approvals_only' and is not among the servers; the assertion keys on the reason, not the entry's name
- A new case in the same file: an entry named 'fixture_server' holding only tools is excluded the same way; one holding tools and an unknown member is refused import_member_unsupported
- tests/surface-polish-details.test.tsx: an evidence sentence is shown as written
- sg scan reports no no-app-names-rust, no-app-names-tsx or no-app-names-typescript finding on the landing tree, and a product name planted in a surface file is found

**Files:**
- modify: surface/identity/src/features/import/EstateApproval.tsx
- modify: crates/lys-identity-server/src/seat_import_profiles/servers.rs
- modify: crates/lys-identity-server/tests/seat_import_sources/live_shapes.rs
- modify: surface/identity/src/features/grants/IssueRoot.tsx
- modify: surface/identity/src/shell/help/HelpDock.tsx
- modify: surface/identity/src/shell/help/concepts/types.ts
- modify: surface/identity/src/shell/help/url.ts
- modify: surface/identity/tests/surface-polish-details.test.tsx

**Checklist:**
- C828 — The estate import shows evidence as written, and the seat import excludes a server entry holding only tool approvals by its shape, never a name. DIRECTORY-114/R4.
- C368 — Lys holds no app's name or schema in code or configuration and makes no call to any app (DIRECTORY-048 R7).

**Stories:**
- S151 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As a developer of an app, I want to register my app and its permission schema through a documented API, so that my app's resources and actions are checked by Lys without anyone changing Lys.

## Boundaries

- No product's id, name or words are written into crates/ or surface/ outside tests and fixtures; test apps are named fixture_product, never a real product.
- Nothing is added to the no-app-names rule's ignores and the rule is not changed.
- Every app's name, standing and address come from the /apps answer; nothing about an app is assumed from its id.
- No timer, deadline or fixed wait anywhere (Tom, 15 October).
- No file over 500 lines of code.

## Verification

- Every red written and shown red at the brief commit through the gate on Dean's laptop, green at the fix commit; the whole gate (tokei, ast-grep, fmt, clippy, tsc, nextest, doc, vitest) at the landing sha, judged by its output, with the ast-grep stage reporting no finding.
- The release carrying this brief installed on the Mac, and the agent page and Getting started walked on the install with Cambium approved.
- The closing note says plainly that R1 to R4 are done, or names what is not.
