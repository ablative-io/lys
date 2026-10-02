# Log window fixture

`log-window.key` is a public, test-only Ed25519 seed. It must never be used
by a deployed broker. The generator reads it from an explicit argument;
no key is compiled into the generator or the broker.

Generate on the gate worker from the reviewed, pushed commit:

```sh
cargo run --locked -p lys-secrets --example log_window_pack -- \
  crates/lys-secrets/tests/fixtures/log-window.key NEW_OUTPUT_DIRECTORY
```

The output directory must not exist. Generation performs all 10,000 real
durable appends, rebuilds the broker's state from those records, verifies
every record's signature and contents, checks the complete Merkle root
against the pin, and requires a fresh open to resume the full snapshot.
The generator closes and removes its temporary broker on success or error.
Its only successful outputs are `log-window.pack` and `log-window.json`.

The pack contains an eight-byte `LYSLWP01` header, a big-endian u64 record
count, then four metadata frames (log identity, pin, snapshot, anchor),
then one frame per leaf in order. Every frame is a big-endian u32 byte
length followed by the exact recorded bytes. It carries no arbitrary
filenames. The manifest records the pack's SHA-256, each frame's SHA-256 in archive
order, record count, byte
counts, public-key fingerprint, Merkle root and snapshot format.

The store key and empty encrypted store are deliberately absent from the
pack. A test creates its own store and installs the recorded audit log.
This preserves independent writable test directories without regenerating
the large signed history. The fixture format is test tooling; it does not
change any signed production format.

Ordinary tests read the recorded archive and manifest from this directory.
They reject missing artifacts, unknown fields or formats, wrong counts,
archive or frame hashes, truncated frames, trailing bytes, and mismatched
key or pinned root. They never generate the archive as a fallback.

Each log-window case unpacks into its own temporary directory. Archive
frames borrow one bounded input buffer; no shared cache or cache lock is
used. The empty broker is created through its normal API, then its audit
files are replaced with the recorded frames. Copying frames makes no
explicit disk sync and calls no audit append. The original paging,
full-signature-audit, tamper and snapshot-resume assertions remain intact.
