#!/usr/bin/env python3
"""Verify a `lys/log-inclusion-proof/v1` artifact against a leaf file.

    usage: verify_inclusion.py <artifact.json> <leaf-file> [expected-root-base64]

    exit 0  the leaf is in the log at the stated index, under the checkpoint's root
    exit 1  usage or I/O problem — nothing was verified
    exit 2  VERIFICATION FAILED — something was verified and it did not hold

WHERE THIS CAME FROM, because that is the whole point of the file
-----------------------------------------------------------------
The Merkle logic below is transcribed from RFC 6962 section 2.1 and section
2.1.1, from the RFC's own text:

    MTH({d(0)}) = SHA-256(0x00 || d(0))
    MTH(D[n])   = SHA-256(0x01 || MTH(D[0:k]) || MTH(D[k:n]))
                  where k is the largest power of two smaller than n

    PATH(m, D[n]) = {}                                   if n = 1
    PATH(m, D[n]) = PATH(m, D[0:k]) : MTH(D[k:n])        if m < k
    PATH(m, D[n]) = PATH(m - k, D[k:n]) : MTH(D[0:k])    if m >= k

`root_from_path` is that PATH definition read backwards: the recursion peels
the OUTERMOST sibling off the end of the list, exactly as `:` appended it,
which is why the path is consumed from the right rather than the left.

**I did not read the Rust implementation of the Merkle walk** — not
`lys-core/src/merkle/`, not `lys-core/src/tlog/verify.rs`. A verifier derived
from the implementation it is meant to check is one party agreeing with itself
in a second language. What I did read, and disclose so the claim is the honest
one: the artifact's field list and the checkpoint's three-line body, both of
which are wire formats and are documented in `docs/design/WIRE-FORMATS.md`
(sections 2.1, 3.1 and 3.3); and `lys-core/src/tlog/artifact.rs`, for the same
field list in Rust form. Those describe the container. The walk is the RFC's.

WHAT THIS DOES NOT CHECK, stated up front rather than left to be discovered
--------------------------------------------------------------------------
**The checkpoint's Ed25519 signature is not verified.** There is no Ed25519 in
the Python standard library and this script takes no dependencies, so it
answers exactly one question: does the leaf you hold hash into the root the
checkpoint carries, at the index the artifact claims? Whether that checkpoint
is genuinely the log's — whether anyone signed it at all — is a separate check
requiring the log's public key, and this script cannot make it. Pass the root
you already trust as the third argument and this becomes an end-to-end answer;
without it, an attacker who can hand you the artifact can also hand you a
checkpoint of their own making.

Python 3 standard library only. No pip, no imports from the lys repository.
"""

import base64
import hashlib
import json
import sys

FORMAT = "lys/log-inclusion-proof/v1"
FIELDS = {"format", "tree_size", "leaf_index", "hashes", "checkpoint"}


class Failed(Exception):
    """A check ran and did not hold. Distinct from a file that would not open."""


def leaf_hash(data: bytes) -> bytes:
    """RFC 6962 MTH({d(0)}) — the raw leaf path: file bytes verbatim."""
    return hashlib.sha256(b"\x00" + data).digest()


def node_hash(left: bytes, right: bytes) -> bytes:
    """RFC 6962 MTH(D[n]) for n > 1."""
    return hashlib.sha256(b"\x01" + left + right).digest()


def root_from_path(m: int, n: int, mth: bytes, path: list) -> bytes:
    """RFC 6962 2.1.1 PATH(m, D[n]) read backwards, rebuilding MTH(D[n])."""
    if not 0 <= m < n:
        raise Failed(f"leaf index {m} is not inside a tree of {n} leaves")
    if n == 1:
        if path:
            raise Failed("inclusion path is longer than the tree is deep")
        return mth
    if not path:
        raise Failed("inclusion path is shorter than the tree is deep")
    k = 1 << ((n - 1).bit_length() - 1)  # largest power of two smaller than n
    sibling, inner = path[-1], path[:-1]
    if m < k:
        return node_hash(root_from_path(m, k, mth, inner), sibling)
    return node_hash(sibling, root_from_path(m - k, n - k, mth, inner))


def b64_digest(text: str, what: str) -> bytes:
    """Standard base64 with padding (D2 rule), and it must be 32 bytes."""
    try:
        raw = base64.b64decode(text, validate=True)
    except Exception as exc:
        raise Failed(f"{what} is not standard base64: {exc}") from exc
    if len(raw) != 32:
        raise Failed(f"{what} decodes to {len(raw)} bytes, not 32")
    return raw


def checkpoint_head(note: str) -> tuple:
    """The C2SP checkpoint body: origin, tree size, root — lines 1, 2, 3."""
    lines = note.split("\n")
    if len(lines) < 4 or lines[3] != "":
        raise Failed("checkpoint is not a signed note: no blank line after the body")
    origin, size_text, root_text = lines[0], lines[1], lines[2]
    if not size_text.isdigit() or (size_text != "0" and size_text.startswith("0")):
        raise Failed(f"checkpoint tree size {size_text!r} is not ASCII decimal")
    return origin, int(size_text), b64_digest(root_text, "checkpoint root")


def verify(artifact: dict, leaf: bytes, expected_root: bytes) -> str:
    """Every check, in order. Raises Failed on the first one that does not hold."""
    if set(artifact) != FIELDS:
        raise Failed(f"artifact fields {sorted(artifact)} are not the frozen v1 set")
    if artifact["format"] != FORMAT:
        raise Failed(f"format is {artifact['format']!r}, not {FORMAT!r}")
    origin, note_size, root = checkpoint_head(artifact["checkpoint"])
    # Load-bearing, not belt-and-braces. `tree_size` is NOT recoverable from the
    # proof: under RFC 6962 whole ranges of tree sizes share one path shape — for
    # leaf 3, sizes 5, 6, 7 and 8 all do — so a relabelled artifact recomputes
    # the same root and the walk below cannot tell. The signed checkpoint is the
    # only thing that pins the size. Drop this and you will report "leaf 3 of 6"
    # about a log that has five leaves.
    if artifact["tree_size"] != note_size:
        raise Failed(
            f"artifact tree_size {artifact['tree_size']} contradicts the signed "
            f"checkpoint's {note_size}"
        )
    if expected_root is not None and expected_root != root:
        raise Failed("the checkpoint's root is not the root you supplied")
    path = [b64_digest(h, f"hashes[{i}]") for i, h in enumerate(artifact["hashes"])]
    index, size = artifact["leaf_index"], artifact["tree_size"]
    computed = root_from_path(index, size, leaf_hash(leaf), path)
    if computed != root:
        raise Failed(
            f"recomputed root {base64.b64encode(computed).decode()} does not match "
            f"the checkpoint's {base64.b64encode(root).decode()}"
        )
    return (
        f"INCLUSION VERIFIED\n"
        f"  origin      {origin}\n"
        f"  leaf {index} of {size}, {len(leaf)} bytes, {len(path)} path nodes\n"
        f"  root        {base64.b64encode(root).decode()}\n"
        f"  NOT CHECKED the checkpoint's signature (no Ed25519 in the stdlib)"
    )


def main(argv: list) -> int:
    if not 3 <= len(argv) <= 4:
        print(__doc__.split("WHERE")[0].strip(), file=sys.stderr)
        return 1
    try:
        with open(argv[1], "rb") as handle:
            artifact = json.load(handle)
        with open(argv[2], "rb") as handle:
            leaf = handle.read()
    except (OSError, ValueError) as exc:
        print(f"could not read the inputs: {exc}", file=sys.stderr)
        return 1
    try:
        expected = b64_digest(argv[3], "expected root") if len(argv) == 4 else None
        print(verify(artifact, leaf, expected))
    except Failed as exc:
        print(f"VERIFICATION FAILED: {exc}", file=sys.stderr)
        return 2
    except (KeyError, TypeError, AttributeError) as exc:
        print(f"VERIFICATION FAILED: artifact is malformed: {exc!r}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
