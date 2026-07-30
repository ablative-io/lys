// Consistency proofs from RFC 6962's RECURSIVE definitions.
//
// lys computes consistency paths and derives newer roots with iterative walks
// over node indices. This file deliberately does the opposite: it transcribes
// RFC 6962 §2.1.4.1 (SUBPROOF) and §2.1.4.2 (the verification algorithm) as the
// recursions the RFC actually writes, splitting each tree at `k`, the largest
// power of two strictly less than `n`.
//
// The point is not a second opinion from the same shape. A transcription error
// in lys's index arithmetic and an identical error in a recursion written from
// the RFC's own text by a different author in another language is not a mistake
// two people make the same way. So `consistency` emits four values and the Rust
// gate requires all four to agree with what ct-merkle and lys produce:
//
//	MTH(D[0:first])   the older root
//	MTH(D[0:second])  the newer root
//	SUBPROOF(...)     the path, concatenated, byte-for-byte
//	derived           the newer root reached by running §2.1.4.2 on the above
//
// The fourth is the one that matters most: it is produced by *this* file's
// derivation from *this* file's proof, so agreement means two independent
// derivations over two independently constructed proofs landed on one root.
//
// `mth`, `interiorHash`, `largestPowerOfTwoBelow` and `decodeLeaves` are reused
// from receipt.go rather than restated. That MTH is already required to agree
// with lys over 152 inclusion cases, so this gate inherits a tree hash that has
// been disagreed with rather than introducing a second private copy — two copies
// of a tree hash in one tool is how two gates stop testing the same tree.
//
// # The receipt modes
//
//	cosetool consistency-sign   <seed-hex> <first-size> <second-size> <leaf-hex>...
//	cosetool consistency-verify <pubkey-hex> <first-size> <second-size> <leaf-hex>...
//
// These wrap the derivation above in the lys/consistency-receipt/v1 COSE
// envelope, so the gate covers the ARTIFACT and not only the Merkle walk. The
// derivation gate proves both sides reach the same root; these prove both sides
// then encode, sign and pin it the same way — a receipt whose root is right but
// whose envelope no conforming library accepts is still worthless.
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

// The `lys/consistency-receipt/v1` content type, and the RFC 9942 proof type it
// carries.
//
// The content type MUST differ from receipt.go's `receiptContentType`. Both
// kinds are a tagged COSE_Sign1 signed by the same key over a bare 32-byte
// detached root, so with a shared type their Sig_structure bytes would be
// identical and an inclusion receipt could be presented as a consistency one
// with its real signature still verifying. `proofTypeConsistency` cannot be the
// discriminator: it lives in the UNPROTECTED header, which no signature covers.
const consistencyContentType = "application/vnd.lys.consistency-receipt.v1+cbor"

const proofTypeConsistency = int64(-2)

// maxWireSize bounds a tree size on the way from the wire into deriveNewer's
// int arithmetic. Narrowing a uint64 to int silently is how a size that no
// verifier could ever hold becomes a small positive number that passes every
// later check; refusing above this bound makes the narrowing explicit.
const maxWireSize = uint64(1) << 31

// subproof is RFC 6962 §2.1.4.1, by its recursive definition.
//
//	SUBPROOF(m, D[m], true)  = {}
//	SUBPROOF(m, D[m], false) = {MTH(D[m])}
//	m <= k: SUBPROOF(m, D[0:k], b) : MTH(D[k:n])
//	m >  k: SUBPROOF(m - k, D[k:n], false) : MTH(D[0:k])
func subproof(m int, leaves [][]byte, b bool) [][]byte {
	if m == len(leaves) {
		if b {
			return nil
		}
		return [][]byte{mth(leaves)}
	}
	k := largestPowerOfTwoBelow(len(leaves))
	if m <= k {
		return append(subproof(m, leaves[:k], b), mth(leaves[k:]))
	}
	return append(subproof(m-k, leaves[k:], false), mth(leaves[:k]))
}

// deriveNewer runs RFC 6962 §2.1.4.2 but RETURNS the second root instead of
// comparing it against a supplied one, matching what lys does and what RFC
// 9942's detached payload requires. It still checks the reconstructed FIRST
// root against firstHash, which is the caller's own earlier view.
func deriveNewer(firstHash []byte, first, second int, proof [][]byte) ([]byte, error) {
	if first <= 0 || first >= second {
		return nil, fmt.Errorf("require 0 < first < second, got %d and %d", first, second)
	}
	// Step 1: an exact power of two omits the first hash from the proof, so the
	// verifier supplies its own copy.
	if first&(first-1) == 0 {
		proof = append([][]byte{firstHash}, proof...)
	}
	if len(proof) == 0 {
		return nil, fmt.Errorf("empty proof")
	}
	// Steps 2 and 3.
	fn, sn := first-1, second-1
	for fn&1 == 1 {
		fn >>= 1
		sn >>= 1
	}
	// Step 4.
	fr := proof[0]
	sr := proof[0]
	// Step 5.
	for _, c := range proof[1:] {
		if sn == 0 {
			return nil, fmt.Errorf("proof longer than the sizes allow")
		}
		if fn&1 == 1 || fn == sn {
			fr = interiorHash(c, fr)
			sr = interiorHash(c, sr)
			for fn != 0 && fn&1 == 0 {
				fn >>= 1
				sn >>= 1
			}
		} else {
			sr = interiorHash(sr, c)
		}
		fn >>= 1
		sn >>= 1
	}
	// Step 6.
	if sn != 0 {
		return nil, fmt.Errorf("proof shorter than the sizes require")
	}
	if hex.EncodeToString(fr) != hex.EncodeToString(firstHash) {
		return nil, fmt.Errorf("reconstructed older root does not match the supplied one")
	}
	return sr, nil
}

// consistency prints "<mth_first> <mth_second> <subproof_concat> <derived>".
func consistency(firstArg, secondArg string, leafHexes []string) {
	first, err := strconv.Atoi(firstArg)
	if err != nil {
		fail("bad first size: %v", err)
	}
	second, err := strconv.Atoi(secondArg)
	if err != nil {
		fail("bad second size: %v", err)
	}
	if second != len(leafHexes) {
		fail("second size %d but %d leaves supplied", second, len(leafHexes))
	}
	leaves := decodeLeaves(leafHexes)

	firstHash := mth(leaves[:first])
	secondHash := mth(leaves)
	proof := subproof(first, leaves, true)

	concat := make([]byte, 0, len(proof)*sha256.Size)
	for _, node := range proof {
		concat = append(concat, node...)
	}

	derived, err := deriveNewer(firstHash, first, second, proof)
	if err != nil {
		fail("derive: %v", err)
	}

	fmt.Printf("%s %s %s %s\n",
		hex.EncodeToString(firstHash),
		hex.EncodeToString(secondHash),
		hex.EncodeToString(concat),
		hex.EncodeToString(derived))
	os.Exit(0)
}

// encodeConsistencyProof builds the RFC 9942 §5.3.1 `bstr .cbor [tree-size-1,
// tree-size-2, consistency-path]` payload.
//
// Deliberately written out rather than shared with encodeInclusionProof. The
// two are the same CBOR shape — a 3-array of two uints and an array of bstrs —
// holding fields of entirely different meaning, and a single writer told apart
// only by its caller's intent is how the wrong pair of numbers gets encoded
// under the right label. lys's encoder splits them for the same reason.
func encodeConsistencyProof(first, second uint64, proof [][]byte) []byte {
	em, err := cbor.CoreDetEncOptions().EncMode()
	if err != nil {
		fail("EncMode: %v", err)
	}
	if proof == nil {
		proof = [][]byte{}
	}
	out, err := em.Marshal([]any{first, second, proof})
	if err != nil {
		fail("Marshal consistency proof: %v", err)
	}
	return out
}

// extractConsistencyProof pulls (tree-size-1, tree-size-2, consistency-path)
// out of the unprotected vdp header, enforcing the RFC 9942 nesting — the
// array-of-proofs at -2 and the bstr wrapping each proof's CBOR — on the way.
//
// Nothing it returns is trusted. Both sizes are attacker-chosen until
// deriveNewer has refused every ordering it disallows and the derived root has
// satisfied the signature.
func extractConsistencyProof(msg *cose.Sign1Message) (uint64, uint64, [][]byte) {
	vdp, ok := lookupInt(msg.Headers.Unprotected, labelVDP)
	if !ok {
		fail("no vdp (396) in the unprotected header")
	}
	proofs, ok := lookupInt(vdp, proofTypeConsistency)
	if !ok {
		fail("no consistency proof (-2) in the vdp")
	}
	proofList, ok := proofs.([]any)
	if !ok {
		fail("the value at -2 must be an array of proofs")
	}
	if len(proofList) != 1 {
		fail("expected exactly one consistency proof, got %d", len(proofList))
	}
	raw, ok := proofList[0].([]byte)
	if !ok {
		fail("each consistency proof must be a bstr wrapping CBOR")
	}

	var parts []cbor.RawMessage
	if err := cbor.Unmarshal(raw, &parts); err != nil {
		fail("Unmarshal consistency proof: %v", err)
	}
	if len(parts) != 3 {
		fail("a consistency proof is a 3-array, got %d elements", len(parts))
	}
	var first, second uint64
	var proofPath [][]byte
	if err := cbor.Unmarshal(parts[0], &first); err != nil {
		fail("bad tree-size-1: %v", err)
	}
	if err := cbor.Unmarshal(parts[1], &second); err != nil {
		fail("bad tree-size-2: %v", err)
	}
	if err := cbor.Unmarshal(parts[2], &proofPath); err != nil {
		fail("bad consistency-path: %v", err)
	}
	return first, second, proofPath
}

// sizesFromArgs parses and range-checks the pair of sizes both consistency
// receipt modes take.
func sizesFromArgs(firstArg, secondArg string, leafCount int) (int, int) {
	first, err := strconv.Atoi(firstArg)
	if err != nil {
		fail("bad first size: %v", err)
	}
	second, err := strconv.Atoi(secondArg)
	if err != nil {
		fail("bad second size: %v", err)
	}
	if second != leafCount {
		fail("second size %d but %d leaves supplied", second, leafCount)
	}
	if first <= 0 || first >= second {
		fail("require 0 < first < second, got %d and %d", first, second)
	}
	return first, second
}

// consistencySign builds a lys/consistency-receipt/v1 artifact with go-cose.
//
// The detached payload is the newer root as *this file* derives it: SUBPROOF
// from RFC 6962 §2.1.4.1 by recursion, then §2.1.4.2's verification algorithm
// run forward to produce the root rather than to compare one. Signing over that
// value is what makes the Rust gate's byte-identity assertion meaningful — the
// two implementations agree on the encoding only if they first agreed on what
// the signature covers.
func consistencySign(seedHex, firstArg, secondArg string, leafHexes []string) {
	seed, err := hex.DecodeString(seedHex)
	if err != nil || len(seed) != ed25519.SeedSize {
		fail("bad seed")
	}
	leaves := decodeLeaves(leafHexes)
	first, second := sizesFromArgs(firstArg, secondArg, len(leaves))

	priv := ed25519.NewKeyFromSeed(seed)
	pub := priv.Public().(ed25519.PublicKey)

	firstHash := mth(leaves[:first])
	proofPath := subproof(first, leaves, true)
	newRoot, err := deriveNewer(firstHash, first, second, proofPath)
	if err != nil {
		fail("derive: %v", err)
	}
	proof := encodeConsistencyProof(uint64(first), uint64(second), proofPath)

	signer, err := cose.NewSigner(cose.AlgorithmEdDSA, priv)
	if err != nil {
		fail("NewSigner: %v", err)
	}
	msg := cose.NewSign1Message()
	msg.Headers.Protected[cose.HeaderLabelAlgorithm] = cose.AlgorithmEdDSA
	msg.Headers.Protected[cose.HeaderLabelContentType] = consistencyContentType
	msg.Headers.Protected[cose.HeaderLabelKeyID] = []byte(pub)
	msg.Headers.Protected[labelVDS] = vdsRFC9162SHA256
	msg.Headers.Unprotected[labelVDP] = map[any]any{
		proofTypeConsistency: []any{proof},
	}

	msg.Payload = newRoot
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

// consistencyVerify checks a consistency receipt and prints
// "<newer-root-hex> <size-1> <size-2>".
//
// The leaves supplied on the command line stand in for the verifier's own
// knowledge of the log, and the OLDER ROOT is computed from them here rather
// than read from the artifact. That is the whole design: RFC 9942 does not say
// where a verifier gets the older root, and an artifact that supplied it would
// let the anchor choose both endpoints — "consistent with an earlier version of
// my log" degrading to "consistent with whichever earlier version I nominate
// today". lys makes it a required argument for the same reason.
func consistencyVerify(pubHex, firstArg, secondArg string, leafHexes []string, artifact []byte) {
	pub, err := hex.DecodeString(pubHex)
	if err != nil || len(pub) != ed25519.PublicKeySize {
		fail("bad public key")
	}
	leaves := decodeLeaves(leafHexes)
	expectedFirst, expectedSecond := sizesFromArgs(firstArg, secondArg, len(leaves))

	msg := openEnvelope(artifact, pub, consistencyContentType)
	wireFirst, wireSecond, proofPath := extractConsistencyProof(msg)
	if wireFirst >= maxWireSize || wireSecond >= maxWireSize {
		fail("tree sizes %d and %d exceed what this tool will narrow", wireFirst, wireSecond)
	}
	if wireFirst != uint64(expectedFirst) || wireSecond != uint64(expectedSecond) {
		fail("claimed sizes %d and %d, expected %d and %d",
			wireFirst, wireSecond, expectedFirst, expectedSecond)
	}

	// The SUBPROOF this tool derives recursively must be the path the artifact
	// carries. Checked before the signature so a disagreement is reported as a
	// path mismatch rather than as a crypto failure.
	expectedPath := subproof(expectedFirst, leaves, true)
	if len(expectedPath) != len(proofPath) {
		fail("path length %d, expected %d", len(proofPath), len(expectedPath))
	}
	for i := range expectedPath {
		if !bytes.Equal(expectedPath[i], proofPath[i]) {
			fail("path node %d differs from the recursively derived SUBPROOF", i)
		}
	}

	// Derived from the path the ARTIFACT carries, not from expectedPath: the
	// signature must cover a root reached from what the anchor actually sent.
	firstHash := mth(leaves[:expectedFirst])
	newRoot, err := deriveNewer(firstHash, expectedFirst, expectedSecond, proofPath)
	if err != nil {
		fail("derive: %v", err)
	}
	msg.Payload = newRoot

	verifier, err := cose.NewVerifier(cose.AlgorithmEdDSA, ed25519.PublicKey(pub))
	if err != nil {
		fail("NewVerifier: %v", err)
	}
	if err := msg.Verify(nil, verifier); err != nil {
		fail("Verify: %v", err)
	}

	fmt.Printf("%s %d %d\n", hex.EncodeToString(newRoot), wireFirst, wireSecond)
}
