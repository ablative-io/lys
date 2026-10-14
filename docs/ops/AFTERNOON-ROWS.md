# Rows following the LYS-DEVICE walk

These are design and acceptance rows, not implemented features. The LYS-DEVICE
report at `logs/LYS-DEVICE/report.md` records the walk against `d79ac8eb`: the
second computer joined and its runner answered, but seat start, send and stop
refused `PermissionEngineUnavailable` because the engine lacked the `seat`
definition. It did not prove a running seat or an independent person's sign-in.
A later installed build needs its own walk; that earlier refusal is not a
readback of the later build.

| Row | Outcome | Next act | What completes the row |
| --- | --- | --- | --- |
| `LYS-CLAUDE-WRAPPER` | `lys claude` injects Lys settings at the argument stage. | Brief the wrapper against the existing launch renderer. | An approved profile reaches the child as arguments and run files; caller settings are not rewritten; refusal and child exit status are preserved. |
| `LYS-MODEL-ACCOUNTS-PAGE` | A person adds their long-lived Claude Code OAuth token and assigns account handles to their agents in Lys. | Build the page over the served add and hold routes. | Add, grant, hold, profile review and a real provider call are proven separately; no token is returned or retained by the page. |
| `LYS-ORGANISATIONS` | A person reaches only their own permitted things in each organisation. | Tom decides the sign-in and organisation model below. | The chosen boundary is enforced on lists, direct reads, writes, grants, accounts, machines and live messages, with two-person and two-organisation proofs. |

## `LYS-CLAUDE-WRAPPER`: design only

The requested behavior is injection when constructing the command's arguments.
Use the existing declared-harness renderer in `lys-home`, including its settings,
MCP and instruction files. Do not create a second encoder or edit the person's
Claude configuration.

A proposed command is `lys claude --agent <agent-id> -- <claude-arguments>`.
The wrapper reads the person's permitted, approved profile and resolves the
declared program on this computer. It passes the generated files through the
renderer’s `--settings`, `--mcp-config` and instruction arguments. Conflicting
caller flags that would replace governed inputs receive a named refusal.
Other arguments retain their order.

Preparation must not start a second process: the existing `start-command` route
dispatches a runner start, so it cannot be used merely to obtain arguments.
The brief must provide a preparation act that records the same authority and
run inputs before the wrapper launches the child. Credentials stay in the
broker; account launches receive the existing non-secret account selectors.

Acceptance exercises two working folders, a harness update, a supplied argument,
conflicting governed arguments, an unavailable profile, a refused preparation,
terminal input and interruption, and a nonzero child exit. A file comparison
proves the person's existing configuration was not rewritten. The wrapper has
not been built or walked by this row.

## `LYS-MODEL-ACCOUNTS-PAGE`: the existing broker contract

The source-backed account procedure is [Named model accounts](../design/home/PROXY.md#named-model-accounts).
The page presents an account name and a concealed token input. For Claude Code,
it asks `POST /api/secrets/add` for a `credential` routed to
`https://api.anthropic.com`, using `authorization` with the `Bearer ` prefix.
The value is submitted once to the person's broker. The response confirms the
account by name; it does not give the value back.

The person grants the intended agent use of that secret through the existing
permission source. The page then asks `POST /api/secrets/accounts/hold`, naming
the agent, account, proxy public key, explicit use count and expiry. It reads
back the returned handle id and holder. That handle id is what the reviewed
profile's `session.accounts.accounts` list carries.

The token input is cleared after submission and is never written to local or
session storage, a URL, a receipt or diagnostic text. An uncertain add retains
the operation and account name, not the token. Before retrying, the page
establishes whether that operation was kept; it does not invent a second add.
An unavailable account or permission read is said beside its act. Each refused
add, grant, hold, review or start keeps the service's refusal name.

Acceptance begins with one person adding one account and permitting one agent.
It proves the handle is held by that agent, a start draws through the proxy,
and the upstream receives the real credential while the child and records
receive only selectors and ids. Another person and another unpermitted agent
must fail to read or use it. Revocation and provider refusal are separate cases.
Token acquisition and renewal remain the provider's acts; this page must not
claim that it refreshes an OAuth token. No new stored account class is proposed.

## `LYS-ORGANISATIONS`: a question for Tom

Does a person use one sign-in with memberships in several organisations and an
explicit organisation choice, or does each organisation have its own issuer
and sign-in? A person's scope must remain their own permitted things in either
model. An organisation switch must never widen it.

The decision determines where organisation membership is recorded and which
organisation a session and grant address. After that decision, the brief must
name the administrator's scope, membership removal, ownership of existing
records and migration of an old install. The two-organisation acceptance must
try both list discovery and guessed direct ids, account holds, agent starts,
machine admission, drafts, receipts and live subscriptions. The organisation
choice is open; no organisation implementation or migration was made here.

No release build or installation is authorised by these rows tonight.
