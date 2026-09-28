# Log snapshots: starting without replaying the whole log

A service that folds its log into state (the identity directory, the grant
book, the secrets broker's leases) used to rebuild that state at every start.
It read every leaf, verified every signature, rehashed the Merkle tree and
applied every event, so a start took longer every time the log grew. A
snapshot records the folded state at one tree size. A start loads it, checks
it, and reads only the leaves written after it.

## What lys-log-store provides

- `Frontier` is the compact form of an RFC 6962 tree: the roots of its perfect
  subtrees, largest first, one per set bit of the tree size. The size and root
  can be extended from it without any earlier leaf.
- `FrontierLog` is an append-only log that holds only its frontier. It opens
  either from nothing, reading every leaf once, or from a frontier the caller
  has already checked, reading only the leaves after it. Opening reconciles
  with the store's pin by the same rules as `Log`: a clean match passes, one
  interrupted append is repaired and reported, and anything else is
  `PinMismatch`. Leaves are read from the store when they are asked for. The
  whole tree that inclusion proofs need is built the first time a proof is
  asked for, and its root must equal the frontier's.
- `LeafStore::snapshot` and `LeafStore::put_snapshot` give each store one
  snapshot slot. It is local state, like the pin. The file store keeps it in
  `snapshot.bin`, written to `snapshot.bin.tmp`, flushed, renamed over the
  slot, and then the directory is flushed.
- `start(store, domain, public_key)` opens the log from its snapshot, or from
  the whole log when the snapshot is refused. It returns the log, the
  snapshot's state if the snapshot was used, the leaves the owner still has to
  apply, and a `Start` that says which path it took.

## Format: `lys/log-snapshot/v1`

Each variable-length field is an 8-byte big-endian length followed by its
bytes. The file holds two fields: the body, then the signature.

| Body part | Content |
|---|---|
| format | the literal `lys/log-snapshot/v1` |
| domain | the kind of state, named by its owner |
| origin | the origin of the log the state was folded from |
| tree size | 8 bytes, big-endian, not framed |
| root | the 32-byte RFC 6962 root at that size, not framed |
| frontier | the subtree roots at that size, 32 bytes each, largest first |
| state | the owner's encoding of its folded state |

The signature is Ed25519 over the whole body. It is made with the key the
owner already uses to sign its log's leaves. No bytes may follow the
signature, and none may follow the last field of the body.

## When a snapshot is written

The owner writes a snapshot when a set number of entries has been appended
since its last one. A timer never triggers it. The state must be the fold of
exactly the leaves the log holds at that moment, because the snapshot ties
that state to that size and root.

## What a start checks, and what it refuses

The checks run in this order. The first one that fails names the refusal:

| Refusal | Cause |
|---|---|
| `SnapshotMissing` | the store holds no snapshot |
| `SnapshotMalformed` | the bytes do not parse, the node count does not match the size, or bytes are left over |
| `SnapshotUnsigned` | no signature, or an empty one |
| `SnapshotSignatureInvalid` | the signature does not verify under the owner's key |
| `SnapshotWrongKind` | the domain is not the owner's |
| `SnapshotWrongLog` | the origin is not this log's |
| `SnapshotWrongRoot` | the frontier does not fold to the signed root, or the leaves after it do not reach the log's pinned root |
| `SnapshotBeyondLog` | the snapshot's size is larger than the log's pinned size |
| `SnapshotStateUnreadable` | the owner cannot decode the state |

After any refusal the log is opened from nothing: every leaf is read, and
nothing from the refused snapshot is used. The refusal is carried in
`Start::Rebuilt`, which the owner reports in its log output. After replaying
the whole log, the owner writes a fresh snapshot.

## What a resumed start does not check

A resumed start does not read the leaves before the snapshot, so a damaged
leaf in that range is not found at start. It is found when the proof tree is
first built: that rebuilds from every leaf and refuses a root that differs
from the frontier's. The snapshot itself cannot hide such damage from a
reader of the log. Its root was checked against the pinned root through every
leaf written after it, and its signature binds it to the owner's key.

## What still grows with the log

Starting no longer reads, verifies or hashes each leaf. The file store still
lists the leaves directory when it opens, to check that the leaf files are
numbered without gaps (its `LeafStore::extent` promise). That listing reads
file names only, never their contents. The snapshot's size follows the size
of the owner's folded state, and a service whose state keeps one entry per
operation, for retry answers, has a snapshot that grows with that count.
