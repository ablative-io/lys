# SECRETS-003 R1 — the secrets broker's baseline, recorded from source

This document records what the code does today, read from source at pinned commits, before the broker's contract (CONTRACT.md) is written. It records where things are; it decides nothing. In particular it records where the revolver's call site is and not who owns the worker-side change, which SECRETS-002 R3 keeps open.

Every citation into a repository other than lys is spelled as the repository's name, the full commit hash it was read at, and a path relative to that repository's root with its line. Every citation into lys is read at the brief's commit, de8f29a77ff8, whose lys-core is unchanged by this brief.

## Repositories read

| Repository | Remote | Full commit read |
| --- | --- | --- |
| manifold | github.com/tomWhiting/ablative-manifold | 3df5ac5f643e0479a0bc0464ef76e36f083c0246 |
| aion | github.com/ablative-io/aion | f143dd73173ff1673a0108f38d822d0abaf386da |
| cambium | github.com/ablative-io/cambium | 1be80d8ec9cfbad84cc222d3430fba5f1459f2df |
| argus | github.com/ablative-io/argus | 424e7789be282d097a9ea3d6f02062b9a30c5379 |
| haematite | github.com/ablative-io/haematite | 27ec726bc0722ab405e3b3a48a8eb1fd212b34b2 |

Every search below is `git grep` against the commit object, run from that repository's root, so it reads the pinned tree and never a working copy. `git grep -l` prints each file prefixed by `<commit>:`; the lists below drop that prefix. The source languages searched are Rust, TypeScript, JavaScript and Elixir, the languages the five repositories are written in (argus is Elixir; cambium is Rust and TypeScript).

## The revolver's next-account call site

manifold at 3df5ac5f643e0479a0bc0464ef76e36f083c0246, crates/manifold-node/src/seat/revolver.rs:172, inside `turned()`, which begins at crates/manifold-node/src/seat/revolver.rs:158:

```
172:         document.account = Some(next.to_owned());
```

`turned()` reads the seat's record (line 170), takes its seat document (line 171), sets the live account to the next name in the seat's `accounts` list (line 172), and spawns the seat again on that document. The value it sets is an account NAME; the account's value is read from the pool file at spawn (crates/manifold-supervisor/src/spawn/seat.rs:743, below). Check: `git show 3df5ac5f643e0479a0bc0464ef76e36f083c0246:crates/manifold-node/src/seat/revolver.rs | sed -n 172p` prints the line above.

## Consumers of the account pool file

The account pool file is `<data-dir>/supervisor/accounts.json` (manifold at 3df5ac5f643e0479a0bc0464ef76e36f083c0246, crates/manifold-supervisor/src/values/pool/path.rs:8 names `accounts.json`, and :30 derives the path).

The search, run in each of the five repositories at the commit in the table above:

```
git grep -l -E 'accounts\.json|POOL_FILE|pool_path|AccountPool|PoolError|PoolEntry|values::pool' <commit> -- '*.rs' '*.ts' '*.tsx' '*.js' '*.ex' '*.exs'
```

| Repository | Commit | Files returned |
| --- | --- | --- |
| manifold | 3df5ac5f643e0479a0bc0464ef76e36f083c0246 | 25 |
| aion | f143dd73173ff1673a0108f38d822d0abaf386da | 0 |
| cambium | 1be80d8ec9cfbad84cc222d3430fba5f1459f2df | 0 |
| argus | 424e7789be282d097a9ea3d6f02062b9a30c5379 | 0 |
| haematite | 27ec726bc0722ab405e3b3a48a8eb1fd212b34b2 | 0 |

The 25 manifold files, each with the first line the search matched and what the file does with the pool:

| File:line | What it does with the pool file |
| --- | --- |
| crates/manifold-core/src/seat_document/record.rs:345 | The seat's `account` field: a name of one pool entry, copied into the seat's store at spawn |
| crates/manifold-node/src/cli/account_verbs.rs:2 | The account verbs that write the pool |
| crates/manifold-node/tests/mailbox/revolver.rs:148 | The revolver's test writes a pool under the test's data directory |
| crates/manifold-supervisor/src/lib.rs:58 | Re-exports the pool types and `pool_path` |
| crates/manifold-supervisor/src/profile/error.rs:3 | Names `PoolError` as the pattern its errors follow |
| crates/manifold-supervisor/src/profile/parse.rs:6 | Names `PoolError::Malformed` as the pattern its parse follows |
| crates/manifold-supervisor/src/profile/path.rs:3 | Places the profiles beside the pool file |
| crates/manifold-supervisor/src/profile/profiles.rs:4 | Places the profiles directory beside `accounts.json` |
| crates/manifold-supervisor/src/profile/raw.rs:3 | Names the pool's module as the pattern its fields follow |
| crates/manifold-supervisor/src/spawn/node_dir.rs:15 | Lists the pool file in the data directory's map |
| crates/manifold-supervisor/src/spawn/refusal.rs:141 | Carries a `PoolError` inside a spawn refusal |
| crates/manifold-supervisor/src/spawn/seat.rs:743 | Opens the pool at spawn to resolve the seat's account name |
| crates/manifold-supervisor/src/values/mod.rs:55 | Re-exports the pool module |
| crates/manifold-supervisor/src/values/pool/accounts.rs:9 | `AccountPool`: opens and parses the pool, and looks an entry up by name |
| crates/manifold-supervisor/src/values/pool/error.rs:14 | `PoolError`: every refusal of the pool, naming a path, never a value |
| crates/manifold-supervisor/src/values/pool/import.rs:23 | Copies one pool entry into a node's values store |
| crates/manifold-supervisor/src/values/pool/mod.rs:9 | The pool's module: its location, shape and rules |
| crates/manifold-supervisor/src/values/pool/parse.rs:21 | Parses the pool document, refusing by line and column |
| crates/manifold-supervisor/src/values/pool/path.rs:8 | `POOL_FILE` and `pool_path` |
| crates/manifold-supervisor/src/values/pool/read.rs:14 | The guarded read: directory and file modes, then the bytes |
| crates/manifold-supervisor/src/values/store/error.rs:7 | Carries a `PoolError` inside a store error |
| crates/manifold-supervisor/src/values/store/node_store.rs:29 | `set_account_from_pool`: writes one pool entry into the node's store |
| crates/manifold-supervisor/tests/supervisor/pool.rs:11 | The pool's tests |
| crates/manifold-supervisor/tests/supervisor/seating.rs:2717 | A seating test that expects `PoolError::PoolMissing` |
| crates/manifold-supervisor/tests/supervisor/seating_account.rs:32 | A seating test that writes a two-entry pool |

## Credential paths into a seat

A credential path into a seat is a place in source where a credential, or the name that selects one, is carried toward a seat's process. Each entry is one file, classed as exactly one of:

- `login exception`: the file carries the seat's own model login toward its process at spawn, the one exception SECRETS-002 R5 names;
- `proxied`: the file carries some other credential into a seat's process, which under the broker becomes a handle the proxy swaps;
- `neither`: the file matched the search but carries no credential into a seat (it validates a path, carries names only, strips a variable, filters names, or is a fixture or a record of provenance).

The search, run in each of the five repositories at the commit in the table above:

```
git grep -l -E 'CLAUDE_CODE_OAUTH_TOKEN|ANTHROPIC_API_KEY|ANTHROPIC_AUTH_TOKEN|OPENAI_API_KEY|GH_TOKEN|GITHUB_TOKEN|accounts\.json|secret_env|env_file' <commit> -- '*.rs' '*.ts' '*.tsx' '*.js' '*.ex' '*.exs' ':(exclude)**/tests/**' ':(exclude)tests/**' ':(exclude)*tests.rs' ':(exclude)**/test/**' ':(exclude)test/**' ':(exclude)*.test.ts' ':(exclude)*.test.tsx'
```

| Repository | Commit | Files returned |
| --- | --- | --- |
| manifold | 3df5ac5f643e0479a0bc0464ef76e36f083c0246 | 34 |
| aion | f143dd73173ff1673a0108f38d822d0abaf386da | 15 |
| cambium | 1be80d8ec9cfbad84cc222d3430fba5f1459f2df | 0 |
| argus | 424e7789be282d097a9ea3d6f02062b9a30c5379 | 2 |
| haematite | 27ec726bc0722ab405e3b3a48a8eb1fd212b34b2 | 0 |

### manifold at 3df5ac5f643e0479a0bc0464ef76e36f083c0246 (34 entries)

| File:line | Class | What carries what |
| --- | --- | --- |
| crates/manifold-core/src/seat_document/record.rs:345 | login exception | The seat's `account` (lines 344 to 358): the name of one pool entry, whose value is copied into the seat's store at spawn and reaches the seat as its environment and nothing else; the path by which the pool value reaches the seat |
| crates/manifold-node/src/seat/launcher.rs:13 | login exception | Builds the seat's launch; the composed environment holds at most one credential, the account variable (lines 122 to 125), and `secret_env` names it |
| crates/manifold-session/src/spawn.rs:92 | login exception | Starts the seat's process with a constructed environment that carries the account variable |
| crates/manifold-supervisor/src/spawn/environ.rs:25 | login exception | Composes the seat's environment; the cascade at step 4 (line 28) sets the account variable the kind in force is bound to |
| crates/manifold-supervisor/src/spawn/seat.rs:224 | login exception | The spawn: reads the environment files (line 224) and opens the pool for the seat's account (line 743) |
| crates/manifold-supervisor/src/values/pool/mod.rs:9 | login exception | The pool module: one entry is imported into a node and put in its launch environment |
| crates/manifold-supervisor/src/values/pool/path.rs:8 | login exception | The pool file the login is read from |
| crates/manifold-supervisor/src/spawn/env_file.rs:5 | proxied | The environment files a seat's chain names: every `NAME=value` in them reaches the seat's environment, any credential a person keeps there among them |
| crates/manifold-core/src/seat_document/boot.rs:75 | neither | A default document with no environment file |
| crates/manifold-core/src/seat_document/metadata.rs:206 | neither | Validates the `env_file` path's shape |
| crates/manifold-core/src/seat_document/metadata_fields.rs:31 | neither | Lists `env_file` as a field name |
| crates/manifold-core/src/seat_document/read.rs:269 | neither | Refuses an empty or relative `env_file` path |
| crates/manifold-node/src/cli/args.rs:1104 | neither | Documents the fields a spawn flag may carry |
| crates/manifold-node/src/cli/seat_document.rs:143 | neither | A default document with no environment file |
| crates/manifold-node/src/cli/seat_mark.rs:103 | neither | Reports the `env_file` path, never its content |
| crates/manifold-node/src/cli/session_verbs.rs:186 | neither | A hand-typed session with an empty `secret_env` |
| crates/manifold-node/src/cli/tree_reseat.rs:127 | neither | Compares two documents' `env_file` paths |
| crates/manifold-node/src/face/outline/fold.rs:313 | neither | Folds the `env_file` path into an outline |
| crates/manifold-node/src/face/outline/plan_documents.rs:244 | neither | Copies the `env_file` path into a planned document |
| crates/manifold-session/src/answer.rs:429 | neither | Refuses a wire caller that names a secret |
| crates/manifold-session/src/grammar.rs:773 | neither | A fixture in the grammar's tests (the module starts at line 580) |
| crates/manifold-session/src/handover/wire.rs:328 | neither | A handover with an empty `secret_env` |
| crates/manifold-session/src/retire.rs:249 | neither | A retire with an empty `secret_env` |
| crates/manifold-session/src/service/mod.rs:888 | neither | A record with an empty `secret_env` |
| crates/manifold-session/src/table.rs:23 | neither | The session table keeps the names of secret entries and filters their values out |
| crates/manifold-supervisor/src/profile/path.rs:3 | neither | Places the profiles beside the pool file |
| crates/manifold-supervisor/src/profile/profiles.rs:4 | neither | Places the profiles directory beside the pool file |
| crates/manifold-supervisor/src/spawn/mark_document.rs:78 | neither | Compares two documents' `env_file` paths |
| crates/manifold-supervisor/src/spawn/mod.rs:44 | neither | Declares the `env_file` module |
| crates/manifold-supervisor/src/spawn/node_dir.rs:15 | neither | The data directory's map |
| crates/manifold-supervisor/src/spawn/record.rs:489 | neither | Records which file a value came from, by path |
| crates/manifold-supervisor/src/spawn/refusal.rs:409 | neither | Refuses an environment file that could not be read, by path |
| crates/manifold-supervisor/src/values/effective.rs:45 | neither | Records the provenance of a value, by path |
| crates/manifold-supervisor/src/values/pool/error.rs:18 | neither | A refusal text naming the pool's path and shape |

### aion at f143dd73173ff1673a0108f38d822d0abaf386da (15 entries)

The aion workers spawn Norn with `OPENAI_API_KEY` removed from the child's environment so that Norn uses its own login; the file that does or declares that removal carries the seat's own login toward its process.

| File:line | Class | What carries what |
| --- | --- | --- |
| crates/aion-integration-norn/src/harness.rs:234 | login exception | `without_env`: the removal that sends a spawned Norn to its own login |
| examples/agent-dev/worker/src/main.rs:37 | login exception | The worker spawns Norn on its own login |
| examples/dev-brief/worker/src/main.rs:29 | login exception | The worker spawns Norn on its own login |
| examples/dev-pipeline/norn-worker/src/main.rs:64 | login exception | The worker spawns Norn on its own login |
| examples/general-worker/src/agent.rs:13 | login exception | The worker removes the key (lines 58 and 62) so Norn uses its own login |
| examples/incident-triage/worker/src/main.rs:23 | login exception | The worker spawns Norn on its own login |
| examples/norn-fan-worker/src/main.rs:18 | login exception | The worker spawns Norn on its own login |
| examples/pipeline-run/worker/src/main.rs:20 | login exception | The worker spawns Norn on its own login |
| examples/plan-fanout/worker/src/harness.rs:189 | login exception | The harness strips the key so Norn uses its own login |
| examples/plan-fanout/worker/src/main.rs:21 | login exception | The worker spawns Norn on its own login |
| examples/remediation/worker/src/main.rs:32 | login exception | The worker spawns Norn on its own login |
| examples/staged-rounds/worker/src/main.rs:42 | login exception | The worker spawns Norn on its own login |
| crates/aion-cli/src/main.rs:2684 | neither | A test that the retired flag is refused (the module starts at line 1495) |
| crates/aion-integration-acp/src/harness.rs:1172 | neither | A test that a removal is expressed on the child (the module starts at line 948) |
| crates/aion-server/src/config/defaults.rs:332 | neither | The fragments that make a variable name credential-shaped, for refusal |

### argus at 424e7789be282d097a9ea3d6f02062b9a30c5379 (2 entries)

| File:line | Class | What carries what |
| --- | --- | --- |
| lib/argus/seats/env.ex:14 | proxied | A seat's `env`: a name whose value is read from the environment Argus runs in when the seat starts, so any named credential reaches the seat |
| lib/argus/seats.ex:15 | neither | The registry's documented example |

### cambium and haematite

The search returned 0 files in cambium at 1be80d8ec9cfbad84cc222d3430fba5f1459f2df and 0 in haematite at 27ec726bc0722ab405e3b3a48a8eb1fd212b34b2.

### Counts

51 entries: 19 `login exception`, 2 `proxied`, 30 `neither`; 19 + 2 + 30 = 51.

## What the lys formats can carry

Read in lys at de8f29a77ff8.

### Handle

- Can carry: a seat subject with the `SpeaksFor` role, the pair a handle bound to one identity would need (crates/lys-core/src/delegation/artifact.rs:145), and an ordering among delegations of that subject by `sequence` (crates/lys-core/src/delegation/artifact.rs:405).
- Cannot carry: a consumer of that role; `SpeaksFor` has defined semantics and no implementation, and nothing reads it (crates/lys-core/src/delegation/artifact.rs:242-254). The format is behind the off-by-default `unstable-anchor` feature and no delegation may be signed outside tests (crates/lys-core/src/delegation/mod.rs:237-242). Which subject and role pair a handle would use stays open (SECRETS-002).

### Lease

- Can carry: a start of effect, `not_before_unix_ms`, which is an effectivity claim and never an ordering key (crates/lys-core/src/delegation/artifact.rs:350).
- Cannot carry: an end. lys/delegation/v1 has no `not_after` (crates/lys-core/src/delegation/mod.rs:218-221), and no count of uses or spend. The lease is therefore a broker record under the audit log, and its window is the lys-identity grant's window it counts against (ADR-020, DIRECTORY-006 R1), which covers this absence.

### Sealed record

- Can carry: a payload sealed with AES-256-GCM under lys/sealed-envelope/v1 through lys-core unchanged (crates/lys-core/src/seal/sealed_envelope.rs:182).
- Cannot carry: associated data. The envelope seals with an empty AAD (crates/lys-core/src/seal/sealed_envelope.rs:182) and opens with one, so nothing in the envelope binds a sealed record to its name, its owning identity or its sequence; that binding must be carried inside the sealed plaintext.
