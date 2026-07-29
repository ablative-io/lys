//! Serde shape of the `lys/verification-bundle/v1` container.
//!
//! # Invariants
//!
//! - **Field declaration order below IS the serialization order** and matches
//!   the wire contract exactly, following the [`crate::tlog`] artifacts'
//!   precedent. The shape freezes once bundles are emitted; a `v2` gets a new
//!   `format` string, never field changes here.
//! - **`deny_unknown_fields`**: unknown fields in a v1 bundle are not valid v1.
//!   There is no field smuggling into a frozen shape, and duplicate JSON keys
//!   are rejected (serde-derive behaviour, pinned by test).
//! - **There is no signing path, at all.** Not by convention — by absence.
//!   [`VerificationBundle`] has no signature field, no `sign` function exists
//!   anywhere in this module, and nothing here holds a private key. A signature
//!   over the container would invite verifiers to check the wrapper and skip
//!   the contents, which is the exact failure mode where something reports
//!   success having verified nothing.
//! - **The bundle carries no keys.** Not the log's, not any anchor's. Trust
//!   inputs are the verifier's own, supplied at verification time — a container
//!   that named the keys to trust would be asserting who is trustworthy, and
//!   the bundle is packaging, never a trust statement.
//! - Embedded artifacts are carried **verbatim**: the inclusion proof as the
//!   `lys/log-inclusion-proof/v1` structure it already is, and each receipt as
//!   base64 of its exact `COSE_Sign1` bytes. Re-encoding a frozen artifact
//!   inside a container is how byte-identity gets lost.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::{Deserialize, Serialize};

/// `format` value of every verification bundle. FROZEN.
pub const VERIFICATION_BUNDLE_FORMAT: &str = "lys/verification-bundle/v1";

/// Hard cap on chain links, so an untrusted bundle cannot ask a verifier to do
/// unbounded work. A chain this deep would already be pathological: each link
/// is one anchor notarizing the one below it.
pub const MAX_LINKS: usize = 32;

/// A self-contained provenance chain: the artifacts a stranger needs to check
/// that a leaf was logged and that the log's checkpoint was notarized.
///
/// The bundle adds nothing cryptographic. Every security property comes from
/// the artifacts inside; the container's only job is to not lose or reorder
/// them, and to make the *relationships* between them checkable.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationBundle {
    /// Artifact kind marker; must equal [`VERIFICATION_BUNDLE_FORMAT`].
    ///
    /// First field, mandatory, exact string match. A bundle whose format string
    /// is unrecognised is rejected before any other field is read.
    pub format: String,

    /// The proven leaf bytes: standard base64, with padding.
    ///
    /// Base64 rather than a nested JSON string because leaf bytes are arbitrary
    /// binary and JSON string escaping is not a byte-preserving channel.
    pub leaf: String,

    /// The `lys/log-inclusion-proof/v1` artifact, embedded verbatim — proving
    /// `leaf` was in the log whose signed checkpoint it carries.
    pub inclusion_proof: crate::tlog::InclusionProofArtifact,

    /// The notarization chain, **ordered ascending**: index 0 is the anchor's
    /// receipt over the log's own checkpoint, index 1 the next anchor's receipt
    /// over *that* anchor's checkpoint, and so on.
    ///
    /// Order is load-bearing and is validated rather than assumed. An
    /// unvalidated chain of individually-valid receipts proves nothing about
    /// their relationship.
    ///
    /// An empty chain is permitted and means exactly what it says: the leaf is
    /// in a log that nobody has notarized. That is a weaker claim, not an
    /// invalid bundle, and [`super::verify::VerifiedBundle`] reports it rather
    /// than letting a reader assume otherwise.
    pub links: Vec<BundleLink>,

    /// The OpenTimestamps-style slot for an independent time attestation:
    /// standard base64, with padding, of an opaque artifact.
    ///
    /// **Present in v1 from day one even though it is always `null` at
    /// launch**, because adding a field to a frozen container means a v2.
    ///
    /// Deliberately an opaque base64 string rather than nested structure. Two
    /// reasons, and the second is the load-bearing one: a counter-anchor proof
    /// (an `.ots` file, say) is binary, matching how `leaf` and `receipt` are
    /// already carried; and typing this as arbitrary JSON would make `lys-core`
    /// depend on a JSON codec, which it deliberately does not — the crate only
    /// derives `Serialize`/`Deserialize` and lets the consumer choose.
    ///
    /// The *format* of the artifact is unspecified in v1 (DP11), and
    /// [`verify_bundle`](super::verify::verify_bundle) **refuses** a bundle that
    /// populates this slot. Carrying an attestation nothing can check is how a
    /// reader comes to believe it; the slot exists so a future version can fill
    /// it without a v2, not so today's bundles can gesture at time.
    pub counter_anchor: Option<String>,
}

/// One rung of the notarization chain: an anchor's receipt, together with the
/// exact bytes that receipt proves.
///
/// Both halves are needed and neither is redundant. A receipt's payload is the
/// **detached** Merkle root, so verifying one requires the leaf bytes it proves
/// — and those bytes are a signed checkpoint, which is what makes the *next*
/// link's comparison possible. A bundle carrying receipts alone would be
/// unverifiable beyond its first link: an anchor's own checkpoint appears
/// nowhere in its receipts, and cannot be reconstructed from one (a receipt
/// yields a root and a size, but deliberately carries no origin and no
/// signature).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BundleLink {
    /// The signed note this link's receipt proves as its leaf, verbatim,
    /// including its trailing newline.
    ///
    /// For link 0 this must be byte-identical to
    /// `inclusion_proof.checkpoint`; that equality is what joins the
    /// notarization to the inclusion proof rather than leaving two unrelated
    /// facts side by side.
    pub checkpoint: String,

    /// The anchor's `lys/anchor-receipt/v1` artifact: standard base64, with
    /// padding, of the exact tagged `COSE_Sign1` bytes.
    pub receipt: String,
}

impl VerificationBundle {
    /// Assemble a bundle around artifacts the caller already holds.
    ///
    /// Sets the frozen `format` string, encodes `leaf` as standard base64 with
    /// padding, and leaves `counter_anchor` empty. The embedded artifacts are
    /// moved in, not re-encoded.
    ///
    /// This is the only constructor with the encoding rules built in, so a
    /// caller cannot get the base64 alphabet or padding subtly wrong; the fields
    /// stay public because a bundle is packaging and there is nothing to
    /// encapsulate.
    ///
    /// There is deliberately no signing counterpart. See the module docs.
    pub fn new(
        leaf: &[u8],
        inclusion_proof: crate::tlog::InclusionProofArtifact,
        links: Vec<BundleLink>,
    ) -> Self {
        Self {
            format: VERIFICATION_BUNDLE_FORMAT.to_string(),
            leaf: STANDARD.encode(leaf),
            inclusion_proof,
            links,
            counter_anchor: None,
        }
    }
}

impl BundleLink {
    /// Build one chain link from a notarized checkpoint and the exact receipt
    /// bytes over it.
    ///
    /// `checkpoint` must be the signed note verbatim, including its trailing
    /// newline — it is the leaf the receipt proves, so a single altered byte
    /// makes the receipt unverifiable. `receipt_bytes` are the tagged
    /// `COSE_Sign1` bytes as
    /// [`AnchorReceipt::to_cose_bytes`](crate::receipt::AnchorReceipt::to_cose_bytes)
    /// emits them.
    pub fn new(checkpoint: &str, receipt_bytes: &[u8]) -> Self {
        Self {
            checkpoint: checkpoint.to_string(),
            receipt: STANDARD.encode(receipt_bytes),
        }
    }
}
