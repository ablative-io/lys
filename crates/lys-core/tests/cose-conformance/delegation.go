// Delegation mode for cosetool: verify lys/anchor-delegation/v1 tagged
// COSE_Sign1 artifacts with the veraison/go-cose reference implementation, and
// return the bytes go-cose actually signed over, VERBATIM.
//
// Usage:
//
//	cosetool delegation-verify <root-pubkey-hex>   (tagged COSE_Sign1 on stdin)
//
// On success, "<name> <value>" lines are written to stdout:
//
//	sig_structure   hex    the RFC 9052 §4.4 Sig_structure go-cose built
//	protected       hex    the protected bucket as a bstr-wrapped CBOR item
//	payload         hex    the embedded payload bstr contents
//	kid             hex    the key ID found in the protected header
//	payload_labels  list   the payload map's integer labels, ascending
//	payload_<label> typed  one line per label found, e.g. "payload_3 uint:1"
//
// # The payload fields are REPORTED, not pinned
//
// This tool knows the delegation *envelope* — tag, algorithm, content type,
// empty unprotected bucket, embedded payload — because those are what make a
// delegation a delegation. It deliberately knows nothing about which payload
// labels exist. It decodes whatever integer-keyed map it is handed and prints
// every label with its value and its CBOR type, so:
//
//   - a payload gaining a field is a new line, not a tool that must be edited
//     in lockstep with the format before the format can be tested at all;
//   - a MISSING or EXTRA field is a `payload_labels` disagreement rather than a
//     parse that quietly succeeds;
//   - a TYPE mistake — a `kid`-shaped bstr where a tstr was meant, a role
//     encoded as a negative integer — is a value disagreement, because the type
//     is printed as part of the value (`bstr:`, `tstr:`, `uint:`, `nint:`).
//
// A tool that hardcoded "a delegation payload has four fields" would have to be
// changed to test a five-field payload, and a gate you edit to make it pass is
// not a second party.
//
// # Why the signed bytes come back verbatim
//
// A caller that only learns "go-cose said yes" learns nothing about WHICH bytes
// go-cose said yes to. That is not a hypothetical distinction in this
// repository: an anchor once signed `body ‖ "extension\n"` instead of `body`,
// and all eighteen in-crate tests stayed green because lys's own parser
// discarded the trailing line. The Go gate caught it for exactly one reason —
// it returned the signed text verbatim, so the caller could compare it against
// what it believed it had signed. `sig_structure` above is that mechanism
// rebuilt for COSE.
//
// The bytes are not reconstructed here. `capturingVerifier` is handed to
// `Sign1Message.Verify`, and go-cose calls it with the ToBeSigned it built
// internally from the artifact's own protected bucket and payload. Rebuilding
// the Sig_structure in this file would make it a second lys encoder wearing a
// Go hat, which is the one thing a second party must not be.
//
// # kid is reported, never used
//
// The root key this tool verifies against is the one on the command line. The
// artifact's own `kid` is decoded and printed and is otherwise inert — it is
// not compared, not fallen back to, and never reaches `cose.NewVerifier`. A
// delegation carries its signer's key in its own protected header, so a tool
// that read the key out of the artifact would accept an attacker's perfectly
// signed delegation for anybody's origin. Printing it lets the caller decide;
// using it would make this gate fall into the trap it exists to guard.
package main

import (
	"bytes"
	"crypto/ed25519"
	"encoding/hex"
	"fmt"
	"sort"
	"strconv"
	"strings"

	"github.com/fxamacker/cbor/v2"
	cose "github.com/veraison/go-cose"
)

// The v1 delegation content type, typed out here from the specification rather
// than shared with any other constant in this scaffold. It is what separates a
// delegation from a receipt or an attestation, all three of which are
// COSE_Sign1 and all three of whose preimages begin 0x84 0x6A "Signature1".
//
// This is the one payload-adjacent thing the tool does pin, and it is pinned on
// purpose: the content type is the version, so a payload that gained a field
// without the type changing is a format that broke its own contract.
const delegationContentType = "application/vnd.lys.anchor-delegation.v1+cbor"

// capturingVerifier records the ToBeSigned go-cose constructs, then defers the
// actual signature check to the real Ed25519 verifier.
//
// This is the only honest way to obtain the signed bytes: go-cose's
// `toBeSigned` is unexported, so the alternative is transcribing RFC 9052 §4.4
// here — a second encoder, whose agreement with lys's would prove nothing about
// what the reference implementation does.
type capturingVerifier struct {
	inner   cose.Verifier
	content []byte
}

func (v *capturingVerifier) Algorithm() cose.Algorithm { return v.inner.Algorithm() }

func (v *capturingVerifier) Verify(content, signature []byte) error {
	// Captured before the check, so a refusal still reports which bytes were
	// refused.
	v.content = append([]byte(nil), content...)
	return v.inner.Verify(content, signature)
}

// requireCanonicalMap re-encodes an integer-keyed CBOR map with fxamacker's
// CoreDetEncOptions — RFC 8949 §4.2 core deterministic — and requires
// byte-identity with the input.
//
// # Why this is here and not left to the caller's byte comparison
//
// It was ADDED after a measurement, and the measurement is the argument. The
// caller compares the verbatim `sig_structure` line against its own preimage,
// which sounds like it would catch a map emitted in the wrong key order. It does
// not. A permutation applied inside the encoder reaches BOTH the artifact and
// the preimage the caller builds, go-cose reads the artifact's payload as an
// opaque byte string and hands those same permuted bytes back, and the two agree
// perfectly about the wrong encoding. Injecting `{2, 1, 3, 4}` into lys's
// payload writer left the caller's positive test **green**.
//
// That is the exact shape of blindness a verbatim return cannot fix, because the
// drift is *inside* the bytes both parties read rather than around them. What
// fixes it is a second party with an opinion about what canonical means, which
// is what fxamacker's core-deterministic encoder is. RFC 9052 §9 does not
// require map key ordering — lys elects RFC 8949 §4.2 — so without this the rule
// is pinned only by the frozen test vector.
func requireCanonicalMap(name string, raw []byte) {
	var fields map[int64]any
	if err := cbor.Unmarshal(raw, &fields); err != nil {
		fail("%s is not an integer-keyed CBOR map: %v", name, err)
	}
	em, err := cbor.CoreDetEncOptions().EncMode()
	if err != nil {
		fail("EncMode: %v", err)
	}
	canonical, err := em.Marshal(fields)
	if err != nil {
		fail("re-encoding %s: %v", name, err)
	}
	if !bytes.Equal(canonical, raw) {
		fail("%s is not RFC 8949 §4.2 canonical:\n  on the wire %s\n  canonical  %s",
			name, hex.EncodeToString(raw), hex.EncodeToString(canonical))
	}
}

// renderPayloadValue prints one payload value as "<cbor-type>:<value>".
//
// The type prefix is the point. Without it a `bstr` of "1" and a `uint` 1 would
// print the same, and a field whose encoding changed type — the mistake a
// round-trip through one implementation can never see, because both sides make
// it — would look like agreement.
func renderPayloadValue(raw cbor.RawMessage) string {
	var value any
	if err := cbor.Unmarshal(raw, &value); err != nil {
		fail("undecodable payload value: %v", err)
	}
	switch v := value.(type) {
	case []byte:
		return "bstr:" + hex.EncodeToString(v)
	case string:
		// Hex, not the text: an origin is compared by raw UTF-8 byte equality,
		// and printing it as text would invite a caller to compare it as text.
		return "tstr:" + hex.EncodeToString([]byte(v))
	case uint64:
		return "uint:" + strconv.FormatUint(v, 10)
	case int64:
		return "nint:" + strconv.FormatInt(v, 10)
	default:
		fail("payload value of unsupported CBOR type %T", v)
		return ""
	}
}

// reportPayload prints the payload map's labels and every value it found.
//
// It pins no label set and no field count; see the file docs for why a tool that
// had to be edited to accept a new field would stop being a second party at the
// moment the format changed.
func reportPayload(payload []byte) {
	var fields map[int64]cbor.RawMessage
	if err := cbor.Unmarshal(payload, &fields); err != nil {
		fail("the payload is not an integer-keyed CBOR map: %v", err)
	}
	if len(fields) == 0 {
		fail("the payload map is empty")
	}
	labels := make([]int64, 0, len(fields))
	for label := range fields {
		labels = append(labels, label)
	}
	sort.Slice(labels, func(i, j int) bool { return labels[i] < labels[j] })

	rendered := make([]string, 0, len(labels))
	for _, label := range labels {
		rendered = append(rendered, strconv.FormatInt(label, 10))
	}
	fmt.Printf("payload_labels %s\n", strings.Join(rendered, ","))
	for _, label := range labels {
		fmt.Printf("payload_%d %s\n", label, renderPayloadValue(fields[label]))
	}
}

func delegationVerify(rootPubHex string, artifact []byte) {
	pub, err := hex.DecodeString(rootPubHex)
	if err != nil || len(pub) != ed25519.PublicKeySize {
		fail("bad root public key")
	}

	// Tagged only. Sign1Message.UnmarshalCBOR requires the 0xd2 0x84 prefix and
	// refuses an untagged COSE_Sign1 — which is the format's rule, not a
	// convenience: RFC 9052 §4.2 permits either form "depending on the
	// context", so accepting both would give one statement two valid encodings.
	var msg cose.Sign1Message
	if err := msg.UnmarshalCBOR(artifact); err != nil {
		fail("UnmarshalCBOR: %v", err)
	}
	if msg.Payload == nil {
		fail("a delegation's payload must be embedded, not detached")
	}
	if len(msg.Headers.Unprotected) != 0 {
		fail("the unprotected bucket must be empty, got %d entries", len(msg.Headers.Unprotected))
	}
	if len(msg.Headers.Protected) != 3 {
		fail("the protected header has three entries, got %d", len(msg.Headers.Protected))
	}
	alg, err := msg.Headers.Protected.Algorithm()
	if err != nil || alg != cose.AlgorithmEdDSA {
		fail("wrong algorithm")
	}
	ct, ok := msg.Headers.Protected[cose.HeaderLabelContentType].(string)
	if !ok || ct != delegationContentType {
		fail("wrong content type")
	}
	kid, ok := msg.Headers.Protected[cose.HeaderLabelKeyID].([]byte)
	if !ok || len(kid) != ed25519.PublicKeySize {
		fail("no 32-byte kid in the protected header")
	}

	// The key comes from the command line. `kid` is printed below and is never
	// consulted here; see the file docs.
	inner, err := cose.NewVerifier(cose.AlgorithmEdDSA, ed25519.PublicKey(pub))
	if err != nil {
		fail("NewVerifier: %v", err)
	}
	capture := &capturingVerifier{inner: inner}
	if err := msg.Verify(nil, capture); err != nil {
		fail("Verify: %v", err)
	}
	if len(capture.content) == 0 {
		fail("go-cose reported success without handing the verifier any bytes")
	}

	protected, err := msg.Headers.MarshalProtected()
	if err != nil {
		fail("MarshalProtected: %v", err)
	}

	// Both signature-covered maps must be canonical under an encoder that has
	// never heard of lys. `protected` is the bstr-wrapped item as it sits inside
	// the Sig_structure, so it is unwrapped first.
	var protectedMap []byte
	if err := cbor.Unmarshal(protected, &protectedMap); err != nil {
		fail("the protected bucket is not a bstr: %v", err)
	}
	requireCanonicalMap("the protected header", protectedMap)
	requireCanonicalMap("the payload", msg.Payload)

	fmt.Printf("sig_structure %s\n", hex.EncodeToString(capture.content))
	fmt.Printf("protected %s\n", hex.EncodeToString(protected))
	fmt.Printf("payload %s\n", hex.EncodeToString(msg.Payload))
	fmt.Printf("kid %s\n", hex.EncodeToString(kid))
	reportPayload(msg.Payload)
}
