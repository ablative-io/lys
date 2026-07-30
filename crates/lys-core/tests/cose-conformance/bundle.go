// Bundle mode for cosetool: verify a lys/verification-bundle/v1 container end
// to end using Go reference implementations for every primitive it rests on.
//
// Usage:
//   cosetool bundle-verify <log-verifier-key> <anchor-verifier-key>...
//     (bundle JSON on stdin; on success prints one summary line
//      "<leaf-hex> <origin> <tree-size> <root-hex>" followed by one line per
//      chain link, "<anchor-root-hex> <tree-size> <leaf-index>")
//
// # Why this is a whole second verifier rather than a byte comparison
//
// The receipt gate can compare bytes, because a receipt is one signed artifact
// with a deterministic encoding. A bundle is not: it is a container whose value
// is entirely in the *relationships* between the artifacts inside it, and those
// are established by checks rather than by bytes. A cross-check that only
// confirmed "Go can parse this JSON" would test nothing that matters.
//
// So this file re-implements the verification instead, and every primitive
// comes from a different source than lys's:
//
//   - signed notes are opened by golang.org/x/mod/sumdb/note, the C2SP
//     reference implementation, against lys's hand-written note verifier;
//   - receipt signatures go through veraison/go-cose;
//   - Merkle roots are rebuilt by rootFromPath, a transcription of RFC 6962
//     §2.1.1's *recursive* structure, against lys's iterative walk;
//   - verifier-key text forms are parsed by note.NewVerifier, which
//     independently recomputes the key ID lys derived.
//
// The chain logic itself is written from the wire spec, deliberately not
// transliterated from bundle/verify.rs, so that two implementations reading the
// same spec have to reach the same verdict. The Rust gate asserts parity in
// both directions: every bundle lys accepts this tool must accept, and every
// bundle lys refuses this tool must refuse. A one-sided check would let a
// missing relationship check pass unnoticed on whichever side omitted it.
package main

import (
	"bytes"
	"crypto/ed25519"
	"encoding/base64"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"strconv"
	"strings"

	"golang.org/x/mod/sumdb/note"
)

// Frozen format strings the container and its embedded proof are pinned to.
const (
	bundleFormat         = "lys/verification-bundle/v1"
	inclusionProofFormat = "lys/log-inclusion-proof/v1"
)

// Defensive bounds, matching the lys verifier so parity is meaningful: a
// bundle may carry at most 32 links, an inclusion path at most 64 nodes, and
// every tree size stays below the JSON-safe 2^53 boundary.
const (
	maxLinks           = 32
	maxInclusionHashes = 64
	maxJSONTreeSize    = uint64(1) << 53
)

// The container shape. Field names are the frozen wire contract; the decoder
// below rejects unknown fields, mirroring serde's deny_unknown_fields.
type inclusionProofArtifact struct {
	Format     string   `json:"format"`
	TreeSize   uint64   `json:"tree_size"`
	LeafIndex  uint64   `json:"leaf_index"`
	Hashes     []string `json:"hashes"`
	Checkpoint string   `json:"checkpoint"`
}

type bundleLink struct {
	Checkpoint string `json:"checkpoint"`
	Receipt    string `json:"receipt"`
}

type verificationBundle struct {
	Format         string                 `json:"format"`
	Leaf           string                 `json:"leaf"`
	InclusionProof inclusionProofArtifact `json:"inclusion_proof"`
	Links          []bundleLink           `json:"links"`
	// A pointer so that "absent or null" is distinguishable from "present and
	// empty" — a populated slot is refused, and an empty string is populated.
	CounterAnchor *string `json:"counter_anchor"`
}

// checkpointBody is the C2SP tlog-checkpoint body: origin, tree size, root.
type checkpointBody struct {
	origin   string
	treeSize uint64
	root     []byte
}

// parseCheckpointBody reads the three mandatory lines of a checkpoint body,
// tolerating (and discarding) extension lines after them per the C2SP spec.
//
// Strict on the size line: no leading zeros and no sign, because a body that
// re-encodes differently than it parsed would let one signed checkpoint be
// presented as two distinct sizes.
func parseCheckpointBody(text string) checkpointBody {
	lines := strings.Split(strings.TrimSuffix(text, "\n"), "\n")
	if len(lines) < 3 {
		fail("a checkpoint body has at least three lines, got %d", len(lines))
	}
	origin, sizeText, rootText := lines[0], lines[1], lines[2]
	if sizeText == "" || (len(sizeText) > 1 && sizeText[0] == '0') {
		fail("non-canonical tree size %q", sizeText)
	}
	treeSize, err := strconv.ParseUint(sizeText, 10, 64)
	if err != nil {
		fail("bad tree size %q: %v", sizeText, err)
	}
	root, err := base64.StdEncoding.DecodeString(rootText)
	if err != nil || len(root) != 32 {
		fail("bad root hash %q", rootText)
	}
	return checkpointBody{origin: origin, treeSize: treeSize, root: root}
}

// anchorKey is a verifier key in both the roles lys binds to one key: opening
// that party's signed notes, and verifying that party's receipt signatures.
type anchorKey struct {
	verifier note.Verifier
	public   ed25519.PublicKey
}

// parseVerifierKey reads the signed-note verifier-key text form
// "<name>+<keyid-hex>+<base64(0x01 || pubkey)>".
//
// note.NewVerifier does the authoritative parse — and, usefully here,
// independently recomputes the key ID from the name and public key and rejects
// a spec whose declared ID disagrees. The manual split only recovers the raw
// key bytes, which note.Verifier does not expose and the receipt check needs.
func parseVerifierKey(spec string) anchorKey {
	verifier, err := note.NewVerifier(spec)
	if err != nil {
		fail("NewVerifier(%q): %v", spec, err)
	}
	first := strings.Index(spec, "+")
	if first < 0 {
		fail("malformed verifier key %q", spec)
	}
	second := strings.Index(spec[first+1:], "+")
	if second < 0 {
		fail("malformed verifier key %q", spec)
	}
	encoded := spec[first+1+second+1:]
	raw, err := base64.StdEncoding.DecodeString(encoded)
	if err != nil {
		fail("bad verifier key base64 in %q: %v", spec, err)
	}
	if len(raw) != 1+ed25519.PublicKeySize || raw[0] != 0x01 {
		fail("verifier key %q is not an Ed25519 key", spec)
	}
	return anchorKey{verifier: verifier, public: ed25519.PublicKey(raw[1:])}
}

// openCheckpoint verifies a signed note under one key and returns its body.
//
// Two checks, not one: note.Open establishes that the key signed these bytes,
// and the origin comparison establishes that the note is the one this party
// publishes about *itself*. Without the second, a party could sign another
// origin's checkpoint text and have it accepted as its own.
func openCheckpoint(signedNote string, key anchorKey) checkpointBody {
	opened, err := note.Open([]byte(signedNote), note.VerifierList(key.verifier))
	if err != nil {
		fail("note.Open: %v", err)
	}
	body := parseCheckpointBody(opened.Text)
	if body.origin != key.verifier.Name() {
		fail("checkpoint origin %q is not the signing key's name %q",
			body.origin, key.verifier.Name())
	}
	return body
}

// decodeHashes decodes the inclusion path, requiring canonical padded base64
// of exactly 32 bytes per node.
func decodeHashes(hashes []string) [][]byte {
	out := make([][]byte, 0, len(hashes))
	for i, entry := range hashes {
		node, err := base64.StdEncoding.DecodeString(entry)
		if err != nil || len(node) != 32 {
			fail("bad inclusion-path node at %d", i)
		}
		out = append(out, node)
	}
	return out
}

// verifyInclusionProof checks the embedded D2 inclusion artifact and returns
// the verified checkpoint body.
//
// The root is recomputed from the leaf and the path and compared with the
// checkpoint's own signed root. The artifact's declared tree size is corroborated
// against the size inside the signature-covered checkpoint rather than trusted:
// it is the log's claim about itself, and it is checked.
func verifyInclusionProof(proof inclusionProofArtifact, leaf []byte, key anchorKey) checkpointBody {
	if proof.Format != inclusionProofFormat {
		fail("wrong inclusion-proof format %q", proof.Format)
	}
	if proof.TreeSize >= maxJSONTreeSize {
		fail("tree size %d is at or beyond the JSON-safe bound", proof.TreeSize)
	}
	if proof.LeafIndex >= proof.TreeSize {
		fail("leaf index %d is not below tree size %d", proof.LeafIndex, proof.TreeSize)
	}
	if len(proof.Hashes) > maxInclusionHashes {
		fail("inclusion path of %d nodes exceeds the cap", len(proof.Hashes))
	}
	path := decodeHashes(proof.Hashes)

	body := openCheckpoint(proof.Checkpoint, key)
	if body.treeSize != proof.TreeSize {
		fail("checkpoint size %d disagrees with the artifact's %d",
			body.treeSize, proof.TreeSize)
	}
	root := rootFromPath(leafHash(leaf), proof.LeafIndex, proof.TreeSize, path)
	if !bytes.Equal(root, body.root) {
		fail("recomputed root does not match the checkpoint root")
	}
	return body
}

// decodeBundle parses the container, refusing unknown fields and trailing
// content — a v1 bundle carrying anything not in v1 is not a v1 bundle.
func decodeBundle(raw []byte) verificationBundle {
	decoder := json.NewDecoder(bytes.NewReader(raw))
	decoder.DisallowUnknownFields()
	var bundle verificationBundle
	if err := decoder.Decode(&bundle); err != nil {
		fail("decode bundle: %v", err)
	}
	if decoder.More() {
		fail("trailing content after the bundle object")
	}
	return bundle
}

// bundleVerify is the whole verification, in the order the wire spec gives.
func bundleVerify(logKeySpec string, anchorKeySpecs []string, raw []byte) {
	bundle := decodeBundle(raw)

	if bundle.Format != bundleFormat {
		fail("wrong bundle format %q", bundle.Format)
	}
	if len(bundle.Links) > maxLinks {
		fail("%d links exceeds the cap of %d", len(bundle.Links), maxLinks)
	}
	if len(anchorKeySpecs) != len(bundle.Links) {
		fail("%d anchor keys supplied for %d links", len(anchorKeySpecs), len(bundle.Links))
	}
	// Refused rather than ignored: nothing can verify a counter-anchor in v1,
	// and carrying an unverified time attestation is how a reader comes to
	// believe it.
	if bundle.CounterAnchor != nil {
		fail("the counter_anchor slot is populated but nothing can verify one")
	}

	leaf, err := base64.StdEncoding.DecodeString(bundle.Leaf)
	if err != nil {
		fail("bad leaf base64: %v", err)
	}

	logKey := parseVerifierKey(logKeySpec)
	logBody := verifyInclusionProof(bundle.InclusionProof, leaf, logKey)

	anchors := make([]anchorKey, 0, len(anchorKeySpecs))
	for _, spec := range anchorKeySpecs {
		anchors = append(anchors, parseVerifierKey(spec))
	}

	type notarization struct {
		root      []byte
		treeSize  uint64
		leafIndex uint64
	}
	notarizations := make([]notarization, 0, len(bundle.Links))

	for index, link := range bundle.Links {
		// The join. Link 0 must notarize the very checkpoint the inclusion
		// proof was verified against; otherwise the notarization is about some
		// other log whose checkpoint also happens to verify. Later links are
		// pinned by the previous iteration's rung check below, which requires
		// strictly more than an equality check could.
		if index == 0 && link.Checkpoint != bundle.InclusionProof.Checkpoint {
			fail("link 0 notarizes a different checkpoint than the inclusion proof")
		}

		receipt, err := base64.StdEncoding.DecodeString(link.Receipt)
		if err != nil {
			fail("bad receipt base64 at link %d: %v", index, err)
		}
		anchorRoot, treeSize, leafIndex := openReceipt(
			anchors[index].public, []byte(link.Checkpoint), receipt)

		// The rung: where a next link exists, its checkpoint must be *this*
		// anchor's own published checkpoint, stating exactly the root and size
		// this anchor's receipt vouched for. Two independently signed
		// statements forced to agree — and what makes the receipt format's
		// documented tree_size malleability harmless here, since a relabelled
		// size no longer matches a note-signed one.
		if index+1 < len(bundle.Links) {
			next := openCheckpoint(bundle.Links[index+1].Checkpoint, anchors[index])
			if !bytes.Equal(next.root, anchorRoot) {
				fail("link %d's reconstructed root is not the anchor's own checkpoint root", index)
			}
			if next.treeSize != treeSize {
				fail("link %d's receipt size %d disagrees with the anchor's checkpoint size %d",
					index, treeSize, next.treeSize)
			}
		}

		notarizations = append(notarizations, notarization{
			root: anchorRoot, treeSize: treeSize, leafIndex: leafIndex,
		})
	}

	fmt.Printf("%s %s %d %s\n", hex.EncodeToString(leaf), logBody.origin,
		logBody.treeSize, hex.EncodeToString(logBody.root))
	for _, n := range notarizations {
		fmt.Printf("%s %d %d\n", hex.EncodeToString(n.root), n.treeSize, n.leafIndex)
	}
}
