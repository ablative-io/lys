# Lys: Tom's requirements register

Every requirement Tom has given for Lys, in his words, with the date, and where it is carried.
Kept by Waffles. A lead who hears a new requirement from Tom adds it here the same day, with his words
and the time, before any brief cites it. Status is one of: landed, briefed (with the brief id),
in a box (with the post id), or **not yet carried** (an open gap the leads owe a brief for).

Target (Tom, 8 Oct 2026, ~13:05): within about a week, Cambium and Lys "can go up and stay up and stay
in use, forever basically."

## Standing bars (apply to every Lys brief)

| # | Tom's words | Date | Carried by | Status |
|---|---|---|---|---|
| B1 | "when the team moves in here it can never go down, it can never slow down, it can never break and it must scale effectively" | 8 Oct ~12:55 | AGENTS-003; never-down brief (Crumpet); scale fixture (Brisket); hot-path ratchets (Scone) | in boxes |
| B2 | Every line written before anything is built or installed; one battery, one install | 3 Oct 12:36 | standing | standing |
| B3 | Storage done properly: one durable operation per batch, one flush; haematite under Lys's log; no re-replay; pruning | 3 Oct 14:27–14:32 | lys-log-store on haematite (design question) | **not yet carried** as a brief |
| B4 | Publish libraries to crates.io; apps install the latest from crates; no stopgap installs | 8 Oct 09:21 | lys-core / lys-log-store 0.3.0 | publish pending (needs Tom's credential) |
| B5 | Not a patch: "an actual solution to the problem" | 8 Oct | review of every END | standing |

## The team moves into Lys

| # | Tom's words | Date | Carried by | Status |
|---|---|---|---|---|
| T1 | Every seat started through Lys, run through the proxy, with hooks, mods and the status line; variables, countdowns and scheduling (replacing herdr and Argus) | 6 Oct ~15:2x | ARGUS-TO-LYS-2026-10-06.md; DIRECTORY-064; AGENTS-001; AGENTS-002 | briefed; 064 partly built; QUALIFIED pin empty |
| T2 | "we need a way of importing these things in" | 8 Oct ~12:55 | AGENTS-003 (Pikelet, post 2d53388b) | in a box |
| T3 | Lys-started AIs use the machine's own Claude Code or Codex; CLAUDE_CONFIG_DIR never set unless a person chose it | 3 Oct | DIRECTORY-050 / HOME-037 | landed |
| T4 | Accounts are long-lived Claude Code OAuth tokens Tom registers; Lys records who draws from which account and how much | 3 Oct | DIRECTORY-051 | landed in part |
| T5 | Every model call through Lys's own proxy, no hooks; observers and gates; capture everything whole | 3 Oct | PROXY.md | briefed |
| T6 | Services start with the login's environment, never the installer's shell | 3 Oct | build.json | landed |
| T7 | The operator token is a development profile only; service installs keep no standing token | 3 Oct | identity | landed |

## Identity first

| # | Tom's words | Date | Carried by | Status |
|---|---|---|---|---|
| I1 | "Lys identity comes first" | 25 Sep 22:37 | DIRECTORY cluster | landed in part |
| I2 | Other people and their AIs sign in to Cambium through Lys (see Cambium register P1–P3) | 8 Oct ~12:58 | Chippy readiness box (post 667aaee4) | in a box |


### I2 installed sign-in defects, 9 October 2026

Installed build d310a9ead52bfa8b5e6ff0a35e781d38f8ba046f serves the public issuer
https://lys.ablative.com.au. The existing approved `cambium` app has its profile
permission and public callback; schema v2 contains `cambium.workspace`, and Tom
holds read access to workspace `ablative`. Public discovery and JWKS returned
HTTP 200, key id `5a99f13dce7955e6`. This is configuration evidence, not a completed
sign-in. Captured native gaps:

| Gap | Evidence | Required behavior | Status |
|---|---|---|---|
| Imported approved app has no sealed client custody | POST `/api/apps/cambium/credentials/issue` returned HTTP 403 `AppClientNoCustody`; `app_client.rs:56-71` requires a sealed entry, while reapproval refuses an already decided app | Establish custody through the native durable API for the same approved app, preserving its identity and earlier approval, then issue a client credential | source direction e232d421f70509d789186c46491ec32f87c6f18cd79176bae50a2b657333eb98; implementation and qualification open |
| Approved-app bearer cannot be delivered | Approval returns `client_secret_ref` and `api_credential_ref`; no bearer issue/export route is served; both requested credential files remain absent | Native issue returns plaintext once and a reference on replay; rotation revokes the old value, survives restart, and log replay never exposes plaintext | same source direction; implementation and qualification open |
| Generated public RP origin crashes the pinned provider | Rauthy 0.36.2 panicked parsing `RAUTHY_RP_ORIGIN=https://lys.ablative.com.au`; its parser requires a numeric port | Generate a valid RP origin; a generated `:443` correction recovered the existing service without changing its browser origin | runtime correction only; generator defect open |
| Native identity stop is nonzero | `lys identity stop --json` exited 1 after `/api/runtime/stop-everything/console` returned HTTP 404 `NotAnApiRoute`; subsequent native start exited 0 | Native stop and its server route agree and preserve failure evidence | clean stop acceptance open |

Fresh `cambium-door` registration returned HTTP 400 `app_id_invalid`, because ids
permit lowercase letters, digits and underscores. No new app was created. Another
app id cannot own `cambium.workspace`; schema and grant boundaries enforce the app
prefix. The existing client id remains `cambium`. No validation was weakened and no
historical log was rewritten.

Primary evidence: `/private/tmp/scone-sign-in-20261009-0751`, including the refusals,
complete 25-session census, config hashes, public discovery/JWKS and native outputs.
END bdcbfd27b173ebe88afe734a6c197db0fb4505ac4369a1eb05880bb8f737c610 records two of eight
Lys delivery requirements unwritten (25%): the client credential and app-bearer files.
Browser sign-in, membership proof, source qualification and source-repair installation
remain unconfirmed. No Cargo, formatter, Clippy or gate result is inferred.

This register-only delta adds no production locks, whole-state clones, explicit sync
sites or history loops. Existing API durability costs and physical sync totals were
not measured.
