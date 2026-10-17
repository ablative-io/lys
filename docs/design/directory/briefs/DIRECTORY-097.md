---
type: brief
id: DIRECTORY-097
cluster: directory
title: A return address for every machine an app runs on: an https sign-in return address whose host is one wildcard label under a registered domain
---

# DIRECTORY-097: A return address for every machine an app runs on: an https sign-in return address whose host is one wildcard label under a registered domain

> **Cluster:** directory
> **Depends on:** DIRECTORY-096
> **Design anchor:**
> - ADR-115 — Lys is the only sign-in a person or a product ever sees; the issuer inside it is never shown — Lys is the single sign-on for every product: every product is a client of Lys at Lys's own origin, and no product configuration names the issuer. A person meets only Lys screens: first-run setup, sign-in, provider setup and their own account are Lys pages, and the issuer's pages, admin site, name and password files are never part of any path a person follows. First run asks the person for the administrator's name, email and password; nothing is filled from the machine.
> - ADR-116 — Every app registers with Lys through one published API; Lys depends on no app — An app is a record in Lys: an id, a name, its sign-in client, and a permission schema it owns (resource kinds under the app's own prefix, each kind's actions, relations carrying actions, and parent kinds whose relations flow down). An app registers and changes its schema only through the API, is approved by an administrator on a Lys screen before it has any effect, and cannot touch another app's kinds. Lys's own model is the schema of the app 'lys'. The API is described by one OpenAPI document generated from the routes and their types, never written by hand. The MCP server is a face over that same API with three tools, the caller's own identity on every call, and no credential or authority of its own. Lys depends on no app. It holds no app's name, kind, schema or code; it never calls an app, waits on one or reads one's store. Every app depends on Lys through this API alone, and Lys runs the same with no apps registered as with twenty.
> **Checklist:**
> - C739 — Registration and the Sign-in settings admit an https return address whose host is one wildcard label under a domain of two or more labels, and refuse by name an http wildcard, a bare wildcard, a wildcard deeper than one label and a one-label suffix.
> - C740 — A sign-in's redirect matches a wildcard entry by exactly one host label under the suffix with the entry's scheme, port and path, at /oauth/authorize and at the token exchange, and is never rewritten.
> - C741 — The app's Sign-in section and the developer registration form explain the wildcard form and the device-local http rule, and show a refused line with its reason.
> - C742 — The CHANGELOG and the apps skill's route notes record the admitted form and its refusals.
> **Stories:**
> - S420 (Person or agent using Lys through an AI assistant, Reads and changes Lys through an MCP client, with only their own rights) — As a person who runs an app on every machine of my tailnet, I want one registered return address to cover each machine's name, so that signing in at a new machine needs no edit in Lys.
> - S421 (Person or agent using Lys through an AI assistant, Reads and changes Lys through an MCP client, with only their own rights) — As a person adding a return address, I want a refused line shown with its reason, so that I learn why an http name cannot be saved instead of finding the list unchanged.

## Purpose

On 17 October 2026 Tom reached liminal's console from Dean's Mac at this Mac's tailnet name and could not sign in: Lys admits a plain-http sign-in return address only on the machine itself (lys-identity-server apps_api::redirect: https anywhere, http at localhost, 127.0.0.1 or [::1]), so the line he added for the name was refused, and the console had to be served over https at the name by Tailscale's edge before the https line could be registered. That line serves one machine. liminal runs an exchange and its console on every machine of the estate, each reached at its own tailnet name (toms-macbook-pro.tail39833a.ts.net, mervyns-macbook-pro.tail39833a.ts.net, the next one), and today each needs its own line added by hand under the app's Sign-in settings before anyone can sign in there; a machine added on a Thursday is a machine nobody can sign in at until someone edits Lys. Tom, 17 October 13:0x: "lys obviously needs to work over the network". The rule that admits https anywhere and http only at the machine itself stays: what changes is that one registered address can name every machine of a domain the person controls.

## Task

Admit, in an app's sign-in return addresses, an https address whose host is exactly one wildcard label under a registered domain suffix (R1); judge a sign-in's redirect address against such an entry by one label under that suffix and nothing more, at /oauth/authorize and at the token exchange alike, with the refusals that exist today (R2); show and explain the form on the app's Sign-in settings and on the developer registration form (R3); record it in the CHANGELOG and the apps skill's route notes, and pin that http, a bare wildcard, a wildcard deeper than one label and a wildcard on a public suffix are refused by name (R4). Out of scope: wildcards in the path, port or scheme; post-logout redirects; any change to the device-local http rule; DIRECTORY-096's return page.

## Requirements

### R1: Registration admits one wildcard host label under a domain, https only

Behavioural. apps_api::redirect admits an address whose host is `*.` followed by a domain of at least two labels (so `https://*.tail39833a.ts.net/signin/return` is admitted and `https://*.net/signin/return` is not), with the scheme https, a port given or not, a path, no user, no fragment; the wildcard is the whole first label, so `https://a*.example.com/…` and `https://*.*.example.com/…` are refused `redirect_invalid` naming the address with the reason "a wildcard is one whole leading label under a domain of two or more labels". An http wildcard is refused by the existing reason ("is neither https nor http to this machine"); a host that is only `*` is refused by the new reason. apps_sign_in::redirects and the registration path judge by the same function, so a wildcard admitted at registration is admitted on the Sign-in settings and the reverse.

**Acceptance:**
- Red first in apps_binding_tests.rs: `https://*.tail39833a.ts.net/signin/return` refused today, admitted after; `https://*.net/x`, `https://*/x`, `https://a*.example.com/x`, `https://*.*.example.com/x`, `http://*.example.com/x` refused by name, each pinned with its exact reason
- The refusal names stay redirect_invalid; no new name appears on /oauth/authorize, /oauth/token or /apps/{app}/sign_in
- apps_api.rs stays under 500 lines or the judging moves to its own module named in design.json

**Files:**
- modify: crates/lys-identity-server/src/apps_api.rs
- modify: crates/lys-identity-server/src/apps_binding_tests.rs
- modify: crates/lys-identity-server/src/apps_sign_in.rs
- modify: crates/lys-identity-server/src/apps_sign_in_tests.rs

**Checklist:**
- C739 — Registration and the Sign-in settings admit an https return address whose host is one wildcard label under a domain of two or more labels, and refuse by name an http wildcard, a bare wildcard, a wildcard deeper than one label and a one-label suffix.

**Stories:**
- S420 (Person or agent using Lys through an AI assistant, Reads and changes Lys through an MCP client, with only their own rights) — As a person who runs an app on every machine of my tailnet, I want one registered return address to cover each machine's name, so that signing in at a new machine needs no edit in Lys.

### R2: A sign-in's redirect matches a wildcard entry by one label under the suffix, at authorize and at the token exchange

Behavioural. apps_binding::listed, which /oauth/authorize and the token exchange both reach, admits a redirect when it equals a listed address byte for byte (as today) or when a listed address is a wildcard entry and the redirect's host is exactly one label (one or more of a-z, 0-9 and hyphen, compared ASCII case-insensitively) followed by `.` and the entry's suffix, and the redirect's scheme, port (explicit or the scheme's own) and path equal the entry's. `https://toms-macbook-pro.tail39833a.ts.net/signin/return` matches `https://*.tail39833a.ts.net/signin/return`; `https://a.b.tail39833a.ts.net/signin/return` (two labels), `https://tail39833a.ts.net/signin/return` (no label), `https://toms-macbook-pro.tail39833a.ts.net:5178/signin/return` (another port) and `https://toms-macbook-pro.tail39833a.ts.net/signin/other` (another path) do not. A redirect is never rewritten: the address the sign-in came with is the address the code returns to, and the token exchange judges the same redirect it was given.

**Acceptance:**
- Red first in apps_binding_tests.rs and apps_sign_in_tests.rs: with the wildcard listed, the one-label host is admitted at sign_in_client and refused today; the two-label, no-label, other-port and other-path hosts are refused redirect_invalid with the existing reason "is not an address the app's sign-in settings list"
- An end-to-end pin through /oauth/authorize and /oauth/token with a wildcard-listed app: the code issued at the one-label redirect exchanges, and the same code presented with a redirect at another host is refused as today (RedirectUnregistered or redirect_invalid, whichever that route names now; the name is pinned, not changed)

**Files:**
- modify: crates/lys-identity-server/src/apps_binding.rs
- modify: crates/lys-identity-server/src/apps_binding_tests.rs
- modify: crates/lys-identity-server/src/apps_sign_in_tests.rs

**Checklist:**
- C740 — A sign-in's redirect matches a wildcard entry by exactly one host label under the suffix with the entry's scheme, port and path, at /oauth/authorize and at the token exchange, and is never rewritten.

**Stories:**
- S420 (Person or agent using Lys through an AI assistant, Reads and changes Lys through an MCP client, with only their own rights) — As a person who runs an app on every machine of my tailnet, I want one registered return address to cover each machine's name, so that signing in at a new machine needs no edit in Lys.

### R3: The Sign-in settings and the registration form show and explain the form

Behavioural. The app card's Sign-in section (surface/identity/src/features/apps/Apps.tsx, help text in shell/help/pages.ts) keeps its one-address-per-line field and its help says, in one sentence, that an https address may name every machine of a domain with `*.` as its first label (`https://*.example.com/signin/return`), that http addresses are admitted only on this machine, and that Save replaces the list. The developer registration form ("Add your own app") says the same. A refused line is shown under the field with the server's reason, naming the line, so a person who types an http name learns why it did not save instead of finding the list unchanged.

**Acceptance:**
- bun test (surface/identity): the help sentence is present on both forms; a refusal from the sign-in route is rendered naming the line and its reason; the existing tests of the Sign-in section still pass
- No new dependency; the forms' Content-Security-Policy is unchanged

**Files:**
- modify: surface/identity/src/features/apps/Apps.tsx
- modify: surface/identity/src/shell/help/pages.ts

**Checklist:**
- C741 — The app's Sign-in section and the developer registration form explain the wildcard form and the device-local http rule, and show a refused line with its reason.

**Stories:**
- S421 (Person or agent using Lys through an AI assistant, Reads and changes Lys through an MCP client, with only their own rights) — As a person adding a return address, I want a refused line shown with its reason, so that I learn why an http name cannot be saved instead of finding the list unchanged.

### R4: The change is recorded and the refusals pinned

Documentary. CHANGELOG names the admitted form and the four refusals (http wildcard, bare `*`, a wildcard deeper than one label, a suffix of fewer than two labels); plugin/skills/apps/references/routes.md and plugins/lys/skills/apps/references/routes.md say at /apps/{app}/sign_in that a wildcard entry is admitted and how it is matched. The liminal install guide (stack/liminal docs/site/operations/install-and-restart.mdx, "Reaching the console from another device") is told by its lead to register the wildcard once for the estate's tailnet suffix; that edit is liminal's, not this brief's.

**Acceptance:**
- The CHANGELOG entry and both routes.md files name the form and the refusals
- cargo nextest run -p lys-identity-server green; cargo clippy --workspace --all-targets -- -D warnings; cargo fmt --all --check

**Files:**
- modify: CHANGELOG.md
- modify: plugin/skills/apps/references/routes.md
- modify: plugins/lys/skills/apps/references/routes.md

**Checklist:**
- C742 — The CHANGELOG and the apps skill's route notes record the admitted form and its refusals.

**Stories:**
- S421 (Person or agent using Lys through an AI assistant, Reads and changes Lys through an MCP client, with only their own rights) — As a person adding a return address, I want a refused line shown with its reason, so that I learn why an http name cannot be saved instead of finding the list unchanged.

## Boundaries

- SHALL NOT admit http anywhere but localhost, 127.0.0.1 and [::1]: the device-local rule is unchanged.
- SHALL NOT admit a wildcard in the scheme, the port or the path, a wildcard that is part of a label, more than one wildcard label, or a wildcard directly under a one-label suffix.
- SHALL NOT rewrite a redirect: the address a sign-in presents is judged and returned to as presented.
- SHALL NOT add a refusal name; redirect_invalid carries a new reason only.
- SHALL NOT touch post_logout_redirects, DIRECTORY-096's return page, or the MCP door's own registration (/oauth/mcp/register).

## Verification

- Every test red first; cargo nextest run only; clippy -D warnings, fmt --check, the surface's bun test; the design gate (validate.py and check-coverage.py on docs/design/directory) parsed for FAIL lines.
- Done means: the five refusal pins and the two admission pins green, the end-to-end authorize-and-token pin green, the two forms' help and refusal rendering tested, the CHANGELOG and routes notes written.
