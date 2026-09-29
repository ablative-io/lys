# Codex policy adapter source contract

Measured source: `bd3798faee33820aad0044ed732841261d2bbe15` from the installed
`0.0.0-channels.3` package. The relevant files below remain byte-identical at
the inspected checkout's `7cbba483f7c83aed15c5fbc0258c72651b6fdf91`.
The executable's `codex-cli 0.0.0` banner is not a compatibility proof.
Source references below are relative to the pinned source's `codex-rs/`.

The package identity reader compares both `codex-package.json` and `bin/codex`
against the measured SHA-256 record. It returns identity only. It does not
claim that config precedence, sandboxing, hooks or event delivery were proved.
The measured artifact is aarch64-apple-darwin. No Linux artifact is approved
by this record.

## Native hook boundary

- `hooks/src/schema.rs:278` defines native PreToolUse stdin, including thread,
  turn and tool-use identities. These are caller claims, not peer credentials.
- `core/src/tools/hook_names.rs:34` keeps `apply_patch` as its native name;
  Write and Edit are matcher aliases only. Patch text is not one file path.
- `core/src/tools/handlers/unified_exec/exec_command.rs:520` maps a shell
  invocation to the Bash hook. The adapter consumes that native payload.
- `core/src/tools/handlers/unified_exec/write_stdin.rs:129` supplies no
  PreToolUse payload for continuation input. No second observation is inferred.
- `hooks/src/events/pre_tool_use.rs:220` applies control effects only for a
  trusted handler. Invalid JSON, failed execution and missing trust can leave
  `should_block` false. A native deny response is useful, but an installed
  hook alone cannot prove a required hard rule fails closed.

The adapter maps into the existing peer judge. The server proves ancestry on
the socket. A pass emits `{}`, never an allow override. An unavailable judge
produces the native deny object with a reason, without claiming durable audit.

## Rejection evidence

- `app-server-protocol/src/protocol/v2/item.rs:1410` declares item/completed
  with threadId, turnId and its typed item.
- The same file at lines 294 and 329 declares commandExecution and fileChange.
- `core/src/tools/events.rs:453` explicitly records that some setup failures
  use Declined. The adapter therefore calls it a Codex-reported rejection with
  cause unavailable. It never infers a person, rule or kernel denied the act.
- `hooks/src/legacy_notify.rs:17` carries agent-turn-complete only. Neither it,
  assistant prose nor stderr containing permission denied is a refusal source.
- `rollout/src/policy.rs:145` and `:194` omit transient ExecCommandEnd and
  HookCompleted records. Legacy history cannot prove an uninterrupted source.

Projection is not persistence. The app-server owner must bind the thread,
append each safe projection with its consumed cursor to the existing durable
store, and only then acknowledge it. Gaps remain explicit missing coverage.

## Native configuration

- `config/src/config_toml.rs:203` declares approval_policy; `:247` and `:251`
  declare default_permissions and named permission profiles.
- `config/src/permissions_toml.rs:224` declares filesystem entries; `:330`
  declares network settings. The renderer derives them from the bound native
  policy, including its digest. It writes no agent-selected overrides.
- `protocol/src/permissions.rs:94` specifies conflict precedence at equal
  path specificity; `:3768` tests more-specific path rules.
- `network-proxy/src/config.rs:386` defines Full as permitting HTTP methods,
  subject to domain policy. It does not mean unrestricted filesystem access.

The rendered fragment selects `lys-bound` and approval_policy never, denies
the root by default, writes only the declared home/workspace, and preserves
explicit host denials. It disables proxy, local-binding and socket bypasses.
The isolated config owner must preserve admitted provider/MCP/notify settings,
refuse widening project/resume/CLI overrides, and read back effective settings.

## Evidence recorded on 29 September 2026, Melbourne

The actual packaged `codex app-server --strict-config --stdio` accepted the
committed `tests/fixtures/codex-policy.toml` fixture with exit 0. Adding an
unknown table returned exit 1 naming `unrecognised_lys_probe`. Each ran in its
own isolated CODEX_HOME, with stdin closed and no model call. Logs are in the
Cambium checkout's `.dev/direct-defects-20260928/codex-policy-probe/`.

This is parser evidence only. It is not evidence of effective-policy readback,
native OS enforcement, authenticated page delivery or Linux support. Those
remain required integration proofs; no prepared session is called enforced.

A subsequent isolated `initialize` then `config/read` exchange returned the
selected profile and every expected filesystem/network setting. Its redacted
config fields are committed as `tests/fixtures/codex-policy-readback.json`.
The readback checker rejects changed approvals, a legacy sandbox override,
another default profile, an extra writable root, local network access and
profile inheritance. These checks are not a launched thread's enforcement
receipt, and do not replace binding later thread/resume parameters.
