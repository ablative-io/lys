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
