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

## The context record

- [ ] **C14** — Every launch-template render appends one lys.given custom entry after the render event, listing the instruction documents Claude Code will load for the session's working directory and the files the render wrote, in the order measured on the named Claude Code version, each by kind, path, byte length and SHA-256, with the harness name and version, the kinds resolved, the kinds it leaves unlisted, and the environment variable names the template set; never a document's content and never a variable's value.
- [ ] **C15** — lys-home lists a session's given records with their documents, and checks a listed document against a file on disk by hash, answering matches or differs and never printing either file.
- [ ] **C16** — The Claude Code version on this Mac, the instruction load order and project slug rule measured from its own behaviour, and one real session render, are written in PROOF-GIVEN.md as paths, counts and hashes only.
