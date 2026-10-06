# From Argus and herdr to Lys: the map

Waffles, 6 October 2026, on Tom's words at about 15:2x the same day: every seat is started through
herdr, which Argus watches through the hooks; look at how that is done and how to do it in Lys,
given Lys starts the seats and runs them through the proxy, with hooks, mods and the status line;
and the variables, countdowns and scheduling. Every claim about today names its file.

## 1. How it is done today

- **The seat.** A pane in a herdr workspace. herdr is a third-party terminal workspace manager
  (`~/.local/bin/herdr`, a Mach-O binary updated by `herdr update`) with a socket API
  (`herdr api schema --json`, `herdr agent list`, `herdr agent prompt <pane> <text>`). Argus's
  `seat_start` creates a tab in the seat's workspace and waits for the pane to appear in
  `agent list`; `seat_stop` delivers `/exit` and waits for a bare shell (tools/argus/lib/argus/mcp.ex).
- **Seeing the seat.** The Claude Code plugin registers every hook event and forwards the whole
  payload to Argus (`tools/argus/plugin/hooks/hooks.json`, `scripts/forward-hook.sh` to
  `/api/hooks`, with the manifold identity as headers); the status line script prints one line and
  forwards the whole status payload, including `context_window.used_percentage` and the rate limits,
  to `/api/statusline` (`scripts/argus-statusline.sh`). The registry is reconciled against
  `herdr agent list`: online, not seen by herdr, seen but unregistered.
- **Warning, preparing, compacting, waking.** Typed into the pane as a user turn:
  `scripts/context-text-inject.sh` sends `[Argus context watch] ...` or `/compact` through
  `herdr agent prompt` (or `screen -X stuff`, or manifold). herdr has no durable input receipt, so a
  delivery is uncertain after a timeout (`docs/herdr-compaction-permission-20260925.md`); Warden
  holds an `operation_permission` before each send.
- **Words.** Five messages (context warning, preparation, compaction command, wake-up, scheduled
  reminder) in four layers (built-in, shared, agent, session) with named templates and a preview
  (`docs/agent-prompts.md`; `/api/prompt-settings`).
- **Variables.** `agent_variables_get/set`: JSON values per agent or session with a revision, an
  author and an optional expiry; `{{vars.key | fallback}}` and `{{goals}}` in the words.
- **Schedules.** `Argus.ScheduledMessages`: at, until, interval 60 s to a year, max occurrences,
  recipients; missed intervals coalesce; an uncertain delivery stops the schedule
  (`docs/scheduled-messages-20260907.md`).
- **The rest.** Budgets, alarms, rules, launches and the queue (Argus MCP tools).

## 2. What Lys has, brief by brief

| Argus or herdr does | Lys owner | State on 6 October |
| --- | --- | --- |
| Start the seat in a pane | DIRECTORY-050 R1, R3: the runner's own pseudo-terminal; HOME-037: the profile | landed (15 commits to 1 Oct) |
| Hooks and status line to see it | DIRECTORY-051 R1: Lys's own hooks and status line in the provisioned home, a stream follower | landed (merged 29 Sep) |
| Count tokens and context | DIRECTORY-051 R1 and the proxy usage file (`crates/lys-runner/src/tracking_proxy.rs`) | landed |
| Budgets; a reached budget acts once | DIRECTORY-051 R2, R3 | landed |
| Goals with deadlines and reminders | DIRECTORY-051 R4 | landed |
| Type, keys, compact, wake, stop | DIRECTORY-050 R4, R6 (through the runner's terminal) | landed; typing, which ADR-130 ends |
| Warn and compact without typing; deliver at turn boundaries; receipts | DIRECTORY-064 | written, 0 commits |
| Words in layers with templates | AGENTS-001 R1 | this cluster |
| Variables with revision and expiry; the agent's own | AGENTS-001 R2 | this cluster |
| `{{vars}}`, `{{goals}}`, countdowns rendered at delivery | AGENTS-001 R3 | this cluster |
| Schedules | AGENTS-001 R4 | this cluster |
| The registry with liveness | AGENTS-002 R2 | this cluster |
| Start, stop, restart by right | AGENTS-002 R3 (ADR-136) | this cluster |
| Look at a seat's terminal (herdr's job) | AGENTS-002 R4: `lys attach` | this cluster |
| The move off herdr and Argus | AGENTS-002 R5, one seat at a time | this cluster |
| Alarms, rules, launches, queue | liminal services and aion (ADR-136 section 3; liminal ACCESS-003) | briefed in liminal |

## 3. Where the data lives (ADR-137)

In Lys, beside goals, as identity-server records in the log; rendered at the moment of delivery by
064's dispatcher with every contributing revision captured in the receipt. Not a shared library:
Lodestone is a read-only query engine over a store and holds nothing (reader.rs:96-104), haematite is
already the store under Lys (Tom, 3 Oct), and a library would be a second place for the same data.
The proxy is where every call is seen; the hooks and the status line are Lys's own in the session's
home; nothing is typed into a terminal.

## 4. Order

1. DIRECTORY-064 (delivery without typing) is the next Lys box after BOX 16 and 17.
2. AGENTS-001 (words, variables, schedules), then AGENTS-002 (seats, registry, attach, the move).
3. The move: Waffles's seat first, then one at a time; Argus's delivery for each turned off as it
   moves; Argus switched off when the last moves.
