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

## Moving a home

- [ ] **C14** — Ship commits exactly a home's session files, their index and head files and its blocks store in the home's own git repository, pushes that commit as one ref to the remote named on the command line, and reports the commit, the ref and every lock or temporary file it left behind.
- [ ] **C15** — Ship refuses by name a home in which any session's index is missing or stale, names the index step that fixes it and writes nothing; a session without a head file ships as it is, and the source home's files are byte-identical before and after every ship.
- [ ] **C16** — A separate index subcommand rebuilds a session's missing or stale index, after which the same ship command runs unchanged and succeeds.
- [ ] **C17** — Ship refuses by name every file in the home that is not a home file, and every place a tracked file holds a value from the credential values file named on the command line or matches one of five standard credential patterns, giving the file and the byte offset and never the value; nothing is redacted.
- [ ] **C18** — Fetch brings the shipped ref into a new directory and refuses by name one that already holds a home; the fetched tree equals the shipped tree, so the session file, index, head and blocks hash-match the source, and every index is verified against its file and every head against its index.
- [ ] **C19** — After the tree check, each fetched session gains exactly one appended lys.harness_event of kind arrival naming the source commit, the remote, the ref and a fresh execution id; the source session file is a byte prefix of the target's, the blocks still hash-match, and the index and head are the ones Session::append wrote for that entry.
- [ ] **C20** — The fetched home is rendered through the Claude Code launch template and resumed by its printed launch line on the Claude Code installed when the proof runs, its version recorded, written up in PROOF-SHIP.md in hashes, lengths, counts and paths only.
