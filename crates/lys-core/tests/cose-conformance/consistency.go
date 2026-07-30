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
package main

import (
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"os"
	"strconv"
)

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
