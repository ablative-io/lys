// cosetool: sign/verify lys/attestation/v2 tagged COSE_Sign1 artifacts with
// the veraison/go-cose reference implementation.
//
// Usage:
//   cosetool sign <seed-hex> <timestamp-ms>   (payload on stdin; tagged COSE_Sign1 on stdout)
//   cosetool verify <pubkey-hex>              (COSE bytes on stdin; "<hash-hex> <ts>\n" on stdout; exit 0/1)
//
// Claims are built with fxamacker/cbor CoreDetEncOptions (RFC 8949 §4.2 core
// deterministic), which is byte-identical to the lys hand encoder.
package main

import (
	"bytes"
	"crypto/ed25519"
	"crypto/rand"
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"io"
	"os"
	"strconv"

	"github.com/fxamacker/cbor/v2"
	cose "github.com/veraison/go-cose"
)

const contentType = "application/vnd.lys.attestation.v2+cbor"

func fail(format string, args ...any) {
	fmt.Fprintf(os.Stderr, format+"\n", args...)
	os.Exit(1)
}

func encodeClaims(payloadHash []byte, timestampMS int64) []byte {
	em, err := cbor.CoreDetEncOptions().EncMode()
	if err != nil {
		fail("EncMode: %v", err)
	}
	claims, err := em.Marshal(map[int64]any{1: payloadHash, 2: timestampMS})
	if err != nil {
		fail("Marshal claims: %v", err)
	}
	return claims
}

func sign(seedHex, tsArg string, payload []byte) {
	seed, err := hex.DecodeString(seedHex)
	if err != nil || len(seed) != ed25519.SeedSize {
		fail("bad seed")
	}
	timestampMS, err := strconv.ParseInt(tsArg, 10, 64)
	if err != nil {
		fail("bad timestamp: %v", err)
	}
	priv := ed25519.NewKeyFromSeed(seed)
	pub := priv.Public().(ed25519.PublicKey)

	payloadHash := sha256.Sum256(payload)
	claims := encodeClaims(payloadHash[:], timestampMS)

	signer, err := cose.NewSigner(cose.AlgorithmEdDSA, priv)
	if err != nil {
		fail("NewSigner: %v", err)
	}
	msg := cose.NewSign1Message()
	msg.Headers.Protected[cose.HeaderLabelAlgorithm] = cose.AlgorithmEdDSA
	msg.Headers.Protected[cose.HeaderLabelContentType] = contentType
	msg.Headers.Protected[cose.HeaderLabelKeyID] = []byte(pub)
	msg.Payload = claims
	if err := msg.Sign(rand.Reader, nil, signer); err != nil {
		fail("Sign: %v", err)
	}
	out, err := msg.MarshalCBOR()
	if err != nil {
		fail("MarshalCBOR: %v", err)
	}
	os.Stdout.Write(out)
}

func verify(pubHex string, artifact []byte) {
	pub, err := hex.DecodeString(pubHex)
	if err != nil || len(pub) != ed25519.PublicKeySize {
		fail("bad public key")
	}
	var msg cose.Sign1Message
	if err := msg.UnmarshalCBOR(artifact); err != nil {
		fail("UnmarshalCBOR: %v", err)
	}
	alg, err := msg.Headers.Protected.Algorithm()
	if err != nil || alg != cose.AlgorithmEdDSA {
		fail("wrong algorithm")
	}
	ct, ok := msg.Headers.Protected[cose.HeaderLabelContentType].(string)
	if !ok || ct != contentType {
		fail("wrong content type")
	}
	kid, ok := msg.Headers.Protected[cose.HeaderLabelKeyID].([]byte)
	if !ok || !bytes.Equal(kid, pub) {
		fail("wrong kid")
	}
	verifier, err := cose.NewVerifier(cose.AlgorithmEdDSA, ed25519.PublicKey(pub))
	if err != nil {
		fail("NewVerifier: %v", err)
	}
	if err := msg.Verify(nil, verifier); err != nil {
		fail("Verify: %v", err)
	}
	var claims map[int64]cbor.RawMessage
	if err := cbor.Unmarshal(msg.Payload, &claims); err != nil {
		fail("Unmarshal claims: %v", err)
	}
	if len(claims) != 2 {
		fail("wrong claims count")
	}
	var payloadHash []byte
	if err := cbor.Unmarshal(claims[1], &payloadHash); err != nil || len(payloadHash) != sha256.Size {
		fail("bad payload hash claim")
	}
	var timestampMS int64
	if err := cbor.Unmarshal(claims[2], &timestampMS); err != nil {
		fail("bad timestamp claim")
	}
	fmt.Printf("%s %d\n", hex.EncodeToString(payloadHash), timestampMS)
}

func main() {
	if len(os.Args) < 2 {
		fail("usage: cosetool sign|verify ...")
	}
	stdin, err := io.ReadAll(os.Stdin)
	if err != nil {
		fail("read stdin: %v", err)
	}
	switch os.Args[1] {
	case "sign":
		if len(os.Args) != 4 {
			fail("usage: cosetool sign <seed-hex> <timestamp-ms>")
		}
		sign(os.Args[2], os.Args[3], stdin)
	case "verify":
		if len(os.Args) != 3 {
			fail("usage: cosetool verify <pubkey-hex>")
		}
		verify(os.Args[2], stdin)
	default:
		fail("unknown mode %q", os.Args[1])
	}
}
