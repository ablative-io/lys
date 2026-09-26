# Home — Checklist

## The common record

- [ ] **C1** — The home record is Pi's session tree: a home file parses with Pi's parser unchanged, and lys's harness events and call records are custom entries.
- [ ] **C2** — Content blocks are stored once by SHA-256 and referenced; an identical block put twice occupies one entry.

## Claude Code in and out

- [ ] **C3** — A Claude Code JSONL imports to events: user turns, assistant turns with tool calls, tool results, compaction summaries, and sidechains as child branches; harness bookkeeping records are counted and left in the byte-for-byte original.
- [ ] **C4** — Events render to a Claude Code JSONL under a chosen uuid at the harness path, with a loss account beside it.
- [ ] **C5** — Provider-native opaque blocks are kept whole, keyed by provider, model family and branch, and rendered only to their own provider with the intervening events.

## The two proofs

- [ ] **C10** — A hand-written few-shot session file resumes Claude Code by path from a directory outside the config root; the source is unchanged and the session reports authored.
- [ ] **C6** — One real session imported, rendered and resumed with --fork-session on Claude Code 2.1.281: it continues, no completed tool action repeats, the original's hash is unchanged.
- [ ] **C7** — One seat with a subscription login completes a call through a pass-through proxy; the measurement is written down.

## The two captures

- [ ] **C8** — Claude Code's harness-local records (hook outcomes, permission mode, tool completion) become lys.harness_event entries attached under the message entry they followed.
- [ ] **C9** — A proxy call record (lys.call) names its request and response blocks by hash with provider, api, model and timing, and can be ingested from a captured request and response pair.
- [ ] **C11** — The little proxy passes Messages, Chat Completions and Responses streams through unchanged and appends one lys.call entry per call under the session it links to.

## The canon

- [ ] **C12** — The canon, one curated versioned series of short examples (a rule stated short plus a real exchange that shows it lived), drawn from every agent's sessions and changed only through review, seeds every new session as lys.inherited entries naming each example's source; nothing in it is authored thinking; canon-seeded against plain is measured on a card.

## The handover

- [ ] **C13** — At compaction or retirement the outgoing session's letter to its successor, with its real thinking, becomes the successor's first entry as lys.inherited; it is never authored and replays only to the same provider, api and model; seeded against plain is measured on a card.

## Codex out

- [ ] **C14** — lys-home's codex-render subcommand renders a Claude Code-imported home session into a Codex 0.156.0 rollout under a target directory, and the rollout's conversation items equal those of a rollout Codex 0.156.0 wrote, apart from ids and timestamps.
- [ ] **C15** — Every text part, tool call and tool result on the context path is carried whole as a Codex message, function_call and function_call_output item; a tool result longer than 4,000 characters is carried whole; a compaction is carried as Codex 0.156.0's own compaction record with its summary whole.
- [ ] **C16** — Readable thinking is carried as assistant text and listed as changed; redacted and empty thinking is dropped and listed as lost by hash with its reason.
- [ ] **C17** — The rollout's first response item is a developer message announcing a translated context with the source session id, the lys-home version and commit, the mapping codex-0.156.0 render v1, and whether its working directory came from --cwd or the source session header, its metadata naming the kind lys.translation_marker unless Codex 0.156.0 was measured refusing to replay that kind.
- [ ] **C18** — A JSON account beside the rollout lists by entry id and block hash every kept block, every changed block with its before and after kinds and how, and every lost block or entry with its reason.
- [ ] **C19** — An authored turn is carried as assistant output behind the authored marker and an inherited entry in its own role behind the inherited marker naming its source session, both listed as changed.
- [ ] **C20** — A sidechain is carried as a spawned Codex thread in the shape 0.156.0 writes for its own when that shape is measured here, and is otherwise listed as lost with that reason.
- [ ] **C21** — A render whose rollout or account path exists is refused by that path and writes nothing; the report is JSON of paths and counts with no content.
- [ ] **C22** — Codex 0.156.0, run with a scratch Codex home, resumes the rendered rollout and answers from its content, recorded in PROOF-CODEX.md with hashes, counts and paths only.
- [ ] **C23** — The Claude Code render of the fixture session hashes the same as the hash recorded at the base commit.
