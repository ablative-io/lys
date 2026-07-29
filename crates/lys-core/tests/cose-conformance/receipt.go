// Receipt modes for cosetool: sign and verify lys/anchor-receipt/v1 tagged
// COSE_Sign1 artifacts with the veraison/go-cose reference implementation.
//
// Usage:
//   cosetool receipt-sign   <seed-hex> <leaf-index> <leaf-hex>...   (tagged COSE_Sign1 on stdout)
//   cosetool receipt-verify <pubkey-hex> <leaf-index> <leaf-hex>... (receipt on stdin;
//                                                                    "<root-hex> <size> <index>\n" on stdout)
//
// # Why the tree code here is written out rather than shared
//
// The Merkle functions below are transcriptions of RFC 6962 §2.1's *recursive*
// definitions — MTH(D[n]) and PATH(m, D[n]) — deliberately not of the iterative
// upward walk lys implements in `merkle::root_from_inclusion_path`. Two
// independent algorithms for the same value is the point: because the receipt's
// payload is the detached root, go-cose's signature check only succeeds if the
// root this file computes recursively equals the root lys computed iteratively.
// A disagreement between the two shows up as a signature failure, not as a
// subtly wrong artifact that verifies anyway.
package main

import (
	"bytes"
	"crypto/ed25519"
	"crypto/rand"
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"os"
	"strconv"

	"github.com/fxamacker/cbor/v2"
	cose "github.com/veraison/go-cose"
)

const receiptContentType = "application/vnd.lys.receipt.v1+cbor"

// COSE header labels from RFC 9942.
const (
	labelVDS = int64(395)
	labelVDP = int64(396)
)

// RFC 9942 registered values.
const (
	vdsRFC9162SHA256 = int64(1)
	proofTypeInclusion = int64(-1)
)

// largestPowerOfTwoBelow returns k, the largest power of two strictly less
// than n (RFC 6962 §2.1 requires n > 1).
func largestPowerOfTwoBelow(n int) int {
	k := 1
	for k*2 < n {
		k *= 2
	}
	return k
}

// leafHash is RFC 6962's MTH for a single leaf: SHA-256(0x00 || d).
func leafHash(leaf []byte) []byte {
	h := sha256.New()
	h.Write([]byte{0x00})
	h.Write(leaf)
	return h.Sum(nil)
}

// interiorHash is RFC 6962's interior node: SHA-256(0x01 || left || right).
func interiorHash(left, right []byte) []byte {
	h := sha256.New()
	h.Write([]byte{0x01})
	h.Write(left)
	h.Write(right)
	return h.Sum(nil)
}

// mth is RFC 6962 §2.1's Merkle Tree Hash, by its recursive definition.
func mth(leaves [][]byte) []byte {
	switch len(leaves) {
	case 0:
		h := sha256.Sum256(nil)
		return h[:]
	case 1:
		return leafHash(leaves[0])
	}
	k := largestPowerOfTwoBelow(len(leaves))
	return interiorHash(mth(leaves[:k]), mth(leaves[k:]))
}

// path is RFC 6962 §2.1.1's PATH(m, D[n]), by its recursive definition.
func path(m int, leaves [][]byte) [][]byte {
	if len(leaves) == 1 {
		return nil
	}
	k := largestPowerOfTwoBelow(len(leaves))
	if m < k {
		return append(path(m, leaves[:k]), mth(leaves[k:]))
	}
	return append(path(m-k, leaves[k:]), mth(leaves[:k]))
}

func decodeLeaves(hexes []string) [][]byte {
	leaves := make([][]byte, 0, len(hexes))
	for i, h := range hexes {
		b, err := hex.DecodeString(h)
		if err != nil {
			fail("bad leaf hex at %d: %v", i, err)
		}
		leaves = append(leaves, b)
	}
	if len(leaves) == 0 {
		fail("at least one leaf is required")
	}
	return leaves
}

// encodeInclusionProof builds the RFC 9942 `bstr .cbor [tree-size, leaf-index,
// inclusion-path]` payload.
func encodeInclusionProof(treeSize, leafIndex uint64, proofPath [][]byte) []byte {
	em, err := cbor.CoreDetEncOptions().EncMode()
	if err != nil {
		fail("EncMode: %v", err)
	}
	// An empty path must still encode as an empty array, not as null.
	if proofPath == nil {
		proofPath = [][]byte{}
	}
	out, err := em.Marshal([]any{treeSize, leafIndex, proofPath})
	if err != nil {
		fail("Marshal inclusion proof: %v", err)
	}
	return out
}

func receiptSign(seedHex, indexArg string, leafHexes []string) {
	seed, err := hex.DecodeString(seedHex)
	if err != nil || len(seed) != ed25519.SeedSize {
		fail("bad seed")
	}
	leafIndex, err := strconv.ParseUint(indexArg, 10, 64)
	if err != nil {
		fail("bad leaf index: %v", err)
	}
	leaves := decodeLeaves(leafHexes)
	if leafIndex >= uint64(len(leaves)) {
		fail("leaf index %d outside a tree of %d leaves", leafIndex, len(leaves))
	}

	priv := ed25519.NewKeyFromSeed(seed)
	pub := priv.Public().(ed25519.PublicKey)

	root := mth(leaves)
	proof := encodeInclusionProof(uint64(len(leaves)), leafIndex, path(int(leafIndex), leaves))

	signer, err := cose.NewSigner(cose.AlgorithmEdDSA, priv)
	if err != nil {
		fail("NewSigner: %v", err)
	}
	msg := cose.NewSign1Message()
	msg.Headers.Protected[cose.HeaderLabelAlgorithm] = cose.AlgorithmEdDSA
	msg.Headers.Protected[cose.HeaderLabelContentType] = receiptContentType
	msg.Headers.Protected[cose.HeaderLabelKeyID] = []byte(pub)
	msg.Headers.Protected[labelVDS] = vdsRFC9162SHA256
	msg.Headers.Unprotected[labelVDP] = map[any]any{
		proofTypeInclusion: []any{proof},
	}

	// Sign over the root, then detach it: the artifact carries nil and the
	// verifier must recompute the root to check the signature at all.
	msg.Payload = root
	if err := msg.Sign(rand.Reader, nil, signer); err != nil {
		fail("Sign: %v", err)
	}
	msg.Payload = nil

	out, err := msg.MarshalCBOR()
	if err != nil {
		fail("MarshalCBOR: %v", err)
	}
	os.Stdout.Write(out)
}

// asInt normalises the several integer types a CBOR decoder may produce for a
// map key or small value.
func asInt(v any) (int64, bool) {
	switch n := v.(type) {
	case int64:
		return n, true
	case uint64:
		return int64(n), true
	case int:
		return int64(n), true
	case cose.Algorithm:
		return int64(n), true
	}
	return 0, false
}

// lookupInt finds an integer-keyed entry in a generically decoded CBOR map.
func lookupInt(m any, key int64) (any, bool) {
	switch mm := m.(type) {
	case map[any]any:
		for k, v := range mm {
			if ki, ok := asInt(k); ok && ki == key {
				return v, true
			}
		}
	case cose.UnprotectedHeader:
		return lookupInt(map[any]any(mm), key)
	case cose.ProtectedHeader:
		return lookupInt(map[any]any(mm), key)
	}
	return nil, false
}

// extractInclusionProof pulls (tree-size, leaf-index, inclusion-path) out of
// the unprotected vdp header, enforcing the RFC 9942 nesting on the way.
func extractInclusionProof(msg *cose.Sign1Message) (uint64, uint64, [][]byte) {
	vdp, ok := lookupInt(msg.Headers.Unprotected, labelVDP)
	if !ok {
		fail("no vdp (396) in the unprotected header")
	}
	proofs, ok := lookupInt(vdp, proofTypeInclusion)
	if !ok {
		fail("no inclusion proof (-1) in the vdp")
	}
	proofList, ok := proofs.([]any)
	if !ok {
		fail("the value at -1 must be an array of proofs")
	}
	if len(proofList) != 1 {
		fail("expected exactly one inclusion proof, got %d", len(proofList))
	}
	raw, ok := proofList[0].([]byte)
	if !ok {
		fail("each inclusion proof must be a bstr wrapping CBOR")
	}

	var parts []cbor.RawMessage
	if err := cbor.Unmarshal(raw, &parts); err != nil {
		fail("Unmarshal inclusion proof: %v", err)
	}
	if len(parts) != 3 {
		fail("an inclusion proof is a 3-array, got %d elements", len(parts))
	}
	var treeSize, leafIndex uint64
	var proofPath [][]byte
	if err := cbor.Unmarshal(parts[0], &treeSize); err != nil {
		fail("bad tree-size: %v", err)
	}
	if err := cbor.Unmarshal(parts[1], &leafIndex); err != nil {
		fail("bad leaf-index: %v", err)
	}
	if err := cbor.Unmarshal(parts[2], &proofPath); err != nil {
		fail("bad inclusion-path: %v", err)
	}
	return treeSize, leafIndex, proofPath
}

func receiptVerify(pubHex, indexArg string, leafHexes []string, artifact []byte) {
	pub, err := hex.DecodeString(pubHex)
	if err != nil || len(pub) != ed25519.PublicKeySize {
		fail("bad public key")
	}
	expectedIndex, err := strconv.ParseUint(indexArg, 10, 64)
	if err != nil {
		fail("bad leaf index: %v", err)
	}
	leaves := decodeLeaves(leafHexes)

	var msg cose.Sign1Message
	if err := msg.UnmarshalCBOR(artifact); err != nil {
		fail("UnmarshalCBOR: %v", err)
	}
	if msg.Payload != nil {
		fail("a receipt's payload must be detached (nil)")
	}

	alg, err := msg.Headers.Protected.Algorithm()
	if err != nil || alg != cose.AlgorithmEdDSA {
		fail("wrong algorithm")
	}
	ct, ok := msg.Headers.Protected[cose.HeaderLabelContentType].(string)
	if !ok || ct != receiptContentType {
		fail("wrong content type")
	}
	kid, ok := msg.Headers.Protected[cose.HeaderLabelKeyID].([]byte)
	if !ok || !bytes.Equal(kid, pub) {
		fail("wrong kid")
	}
	vds, ok := lookupInt(msg.Headers.Protected, labelVDS)
	if !ok {
		fail("no vds (395) in the protected header")
	}
	if v, ok := asInt(vds); !ok || v != vdsRFC9162SHA256 {
		fail("wrong vds")
	}

	treeSize, leafIndex, proofPath := extractInclusionProof(&msg)
	if treeSize != uint64(len(leaves)) {
		fail("claimed tree size %d but %d leaves were supplied", treeSize, len(leaves))
	}
	if leafIndex != expectedIndex {
		fail("claimed leaf index %d but %d was expected", leafIndex, expectedIndex)
	}

	// The path this tool derives from the RFC's recursive definition must be the
	// path the artifact carries. Checked before the signature so a mismatch is
	// reported as a path disagreement rather than as a crypto failure.
	expectedPath := path(int(leafIndex), leaves)
	if len(expectedPath) != len(proofPath) {
		fail("path length %d, expected %d", len(proofPath), len(expectedPath))
	}
	for i := range expectedPath {
		if !bytes.Equal(expectedPath[i], proofPath[i]) {
			fail("path node %d differs from the recursively derived path", i)
		}
	}

	// The detached payload: the root computed here recursively. If lys's
	// iterative walk disagreed with RFC 6962's recursive MTH, this signature
	// check would fail.
	root := mth(leaves)
	msg.Payload = root

	verifier, err := cose.NewVerifier(cose.AlgorithmEdDSA, ed25519.PublicKey(pub))
	if err != nil {
		fail("NewVerifier: %v", err)
	}
	if err := msg.Verify(nil, verifier); err != nil {
		fail("Verify: %v", err)
	}

	fmt.Printf("%s %d %d\n", hex.EncodeToString(root), treeSize, leafIndex)
}
