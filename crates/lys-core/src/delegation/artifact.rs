//! [`Delegation`], [`DelegationClaim`] and [`DelegationRole`] — the
//! parsed form of the `lys/delegation/v1` tagged `COSE_Sign1` artifact.
//!
//! # Invariants
//!
//! - **The only durable form is the COSE bytes.** These types deliberately
//!   implement no `serde` traits: nothing can persist a delegation except
//!   [`Delegation::to_cose_bytes`], so no second wire shape can ever
//!   exist alongside the frozen COSE artifact.
//! - **Round-trip identity:** `from_cose_bytes(x.to_cose_bytes()) == x` for
//!   every `Delegation` whose fields satisfy the format's constraints —
//!   a non-empty subject value, a delegated key strict Ed25519 could accept,
//!   a `(subject_kind, role)` pair this version defines, and an
//!   encoded length within the artifact cap — and `to_cose_bytes` of a parsed
//!   delegation reproduces the input bytes exactly.
//!
//!   **The qualification is not decoration and was added after it was
//!   violated.** [`Delegation::to_cose_bytes`] is infallible by design, so
//!   a hand-built value outside those constraints still encodes; the decoder
//!   then refuses what the encoder produced. The unqualified claim was false for
//!   a subject value of 3885 bytes. Every issuing entry point in
//!   [`super::sign`]
//!   refuses such a claim up front, so the only way to reach the gap is to
//!   construct the struct literal yourself and call `to_cose_bytes` on it.
//! - **Canonical-encoding strictness:** [`Delegation::from_cose_bytes`]
//!   rejects any input that is not byte-identical to the canonical re-encoding
//!   of its parsed fields — including inputs whose signature is
//!   cryptographically valid (indefinite lengths, oversized integer heads,
//!   reordered maps, a non-empty unprotected bucket, tag stripping, duplicate
//!   keys, trailing garbage). Without this, a non-canonical re-encoding of the
//!   same statement would be a *second* valid artifact for one claim, and every
//!   downstream comparison by bytes would depend on which encoding you saw.
//!
//!   **Canonical encoding is normative on the wire for this format**, not a
//!   by-product of how this crate verifies. RFC 9052 §9 does not require map key
//!   ordering, so a conforming stranger would accept a permuted map as a second
//!   valid artifact for one statement unless the format itself forbids it — and
//!   it does.
//!
//!   What the byte-compare *contributes* was stated wrongly here twice and is
//!   corrected in the `encoding` module's docs, which carry the measurement. In
//!   short: it is not what refuses a permuted map (the positional decode pins
//!   do), and it cannot be justified by "the signature catches the rest", which
//!   is true only of this verifier. Its own territory is the **envelope**, which
//!   no signature covers under any verifier's strategy — and at this parse-only
//!   entry point it is the only canonicality guard there is, because no
//!   signature is checked here at all.
//! - **The tag is required.** An untagged `COSE_Sign1` is refused, because
//!   RFC 9052 §4.2 allows both forms and accepting both would give one
//!   statement two encodings.
//! - **Parsing is not verification.** `from_cose_bytes` checks no signature and
//!   knows no expected root key and no expected subject, and is named for
//!   parsing to say so.
//!   Its output is attacker-chosen data until [`super::sign::verify_delegation`]
//!   has run.
//! - Every parse failure collapses to [`TrustError::DelegationVerification`]
//!   (non-oracle).

use crate::delegation::encoding::{self, KEY_LEN};
use crate::error::{TrustError, TrustResult};

/// What kind of thing a delegation's subject is. A **closed** enum, exactly as
/// [`DelegationRole`] is: a value on the wire that is not listed here is a
/// decode failure, never a value carried through.
///
/// # Why the subject carries a kind at all
///
/// An earlier draft of this format had a single `origin` field whose documented
/// meaning was fixed — a DNS-style origin — so "a lapsed domain orphans every
/// proof" was a property of the *artifact*. Generalising that field to a bare
/// string would have moved the protection out of the format and into the
/// caller: an anchor could pass a seat identifier, or a seat consumer a domain,
/// and **nothing would notice**. The delegation would verify perfectly and mean
/// something no verifier could pin down — a signed value whose semantics live
/// nowhere, which is the same failure the closed [`DelegationRole`] refuses.
///
/// So the subject carries its kind alongside its value, and
/// [`verify_delegation`](super::sign::verify_delegation) requires the caller to
/// **state the kind it expects**. Cross-kind confusion is refused at decode
/// rather than discovered downstream: a seat delegation cannot be presented to
/// a domain verifier and a domain delegation cannot be presented to a seat one,
/// for the same structural reason an inclusion receipt cannot be re-labelled as
/// a consistency receipt.
///
/// The kind set is closed and unknown kinds are refused. **Both inhabitants
/// therefore had to ship in `v1`**: adding a kind later is a compatibility
/// event, because an old verifier refuses what it does not recognise, so a `v1`
/// with one kind would still have needed a `v2` for the second — which defeats
/// the point of the mechanism. A third kind is a `v3`, and that is what
/// versioned wire contracts are for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DelegationSubjectKind {
    /// `1` — a DNS-style origin, for example `"example.test"`: the subject an
    /// anchor's delegations name, and the value a checkpoint's origin line
    /// carries.
    ///
    /// Pick the domain you will still control in twenty years. A lapsed domain
    /// orphans every proof issued under it, and nothing in this format can
    /// repair that.
    Domain,

    /// `2` — a seat identifier: arbitrary text minted by whatever registry owns
    /// the seat, and opaque here.
    ///
    /// The structure this exists for is *"the registry pins the handle; a
    /// delegation extends that pin one hop to a seat"*, so a verifier walks
    /// envelope → delegation → registry pin. Because the identifier is text
    /// chosen elsewhere, it can be made to collide with a domain string — which
    /// is exactly why the kind is a required argument at verification and not an
    /// inference from the value.
    Seat,
}

impl DelegationSubjectKind {
    /// The unsigned integer this kind is encoded as in the payload map.
    ///
    /// Encoding goes through this function rather than through a caller-supplied
    /// number, so no value outside the enum can be emitted by this crate at all.
    pub fn wire_value(self) -> u64 {
        match self {
            Self::Domain => 1,
            Self::Seat => 2,
        }
    }

    /// The kind for a wire value, or `None` if the value is not one this version
    /// defines.
    ///
    /// Returns `Option` rather than a `TrustResult` so this type carries no
    /// opinion about which artifact class is decoding it; the caller supplies
    /// its own non-oracle failure value.
    pub fn from_wire(value: u64) -> Option<Self> {
        match value {
            1 => Some(Self::Domain),
            2 => Some(Self::Seat),
            _ => None,
        }
    }

    /// Whether `role` is a role this kind of subject can confer.
    ///
    /// | `subject_kind` | valid `role` | meaning |
    /// |---|---|---|
    /// | `1` [`Domain`](Self::Domain) | `2` [`Operational`](DelegationRole::Operational) | the key signs checkpoints and receipts for this origin |
    /// | `2` [`Seat`](Self::Seat) | `3` [`SpeaksFor`](DelegationRole::SpeaksFor) | the key may sign on behalf of this seat |
    ///
    /// # This is the pair, and the pair is the security property
    ///
    /// **`subject_kind` and `role` are validated together, never
    /// independently.** Each kind has exactly one meaningful role today, which
    /// makes `role` *look* redundant — but the two axes are genuinely
    /// independent (a domain could later want a witness or a revocation role),
    /// so the field stays and the **pair** is what is checked. `(domain,
    /// speaks-for)` and `(seat, operational)` are made of individually valid
    /// values and are refused anyway: each would be a signed statement combining
    /// a subject with an authority nobody in the system defines over it, which is
    /// the "a signed unchecked value looks checked" failure arriving through a
    /// pair rather than through a field.
    ///
    /// # ⛔ Why `role` starts at 2, which looks arbitrary and is the opposite
    ///
    /// **Do not "tidy" [`Operational`](DelegationRole::Operational) back to 1.**
    /// A draft numbered `domain = 1, operational = 1` and `seat = 2,
    /// speaks-for = 2`, which made `subject_kind == role` for **every valid `v1`
    /// artifact** — the two fields encode to the same byte. An implementation
    /// that wired payload label 1 into its role and label 4 into its kind would
    /// then emit byte-identical output for every valid delegation, so **no test
    /// and no golden vector could ever detect the swap.** It would stay latent
    /// until a `v2` introduced a pair whose members differ, at which point every
    /// `v1`-era implementation would be carrying a silent field transposition
    /// through frozen bytes.
    ///
    /// Offsetting the role vocabulary by one buys two properties, and they are
    /// the reason for the numbering:
    ///
    /// - **No valid pair has `kind == role`**, so a field swap changes the bytes
    ///   of *every* case rather than of none.
    /// - **Every swap produces a pair outside this table** — `(1, 2)` transposed
    ///   is `(2, 1)`, `(2, 3)` is `(3, 2)` — so a swapped implementation is
    ///   *refused at decode* rather than merely producing different bytes. The
    ///   wiring is checked by the same rule that checks the semantics, which is
    ///   the strongest form this could take.
    ///
    /// A test vector cannot substitute for either property: the defect was in the
    /// numbering, so no choice of vector reaches it.
    ///
    /// # There is deliberately no wildcard arm
    ///
    /// Every combination is written out, so adding an inhabitant to either enum
    /// is a compile error **here** — at the table that decides which pairs are
    /// meaningful — rather than a new pair silently inheriting whichever default
    /// a `_` arm happened to choose. That is the difference between a closed
    /// vocabulary and a vocabulary that merely looks closed.
    pub fn permits(self, role: DelegationRole) -> bool {
        match (self, role) {
            (Self::Domain, DelegationRole::Operational)
            | (Self::Seat, DelegationRole::SpeaksFor) => true,
            (Self::Domain, DelegationRole::SpeaksFor)
            | (Self::Seat, DelegationRole::Operational) => false,
        }
    }
}

/// The role a delegation confers. A **closed** enum: a value on the wire that
/// is not listed here is a decode failure, never a value carried through.
///
/// The vocabulary is **scoped per subject kind** and the two are checked as a
/// pair — see [`DelegationSubjectKind::permits`], which holds the table.
///
/// ⛔ **The vocabulary starts at `2`, not at `1`, and that is load-bearing.**
/// `1` is deliberately *not* a role: it is
/// [`DelegationSubjectKind::Domain`]'s wire value, and numbering the roles from
/// `1` would have made `subject_kind == role` for every valid `v1` delegation,
/// rendering a field swap between the two undetectable in the encoded bytes.
/// [`DelegationSubjectKind::permits`] carries the full argument. A wire `role`
/// of `1` is therefore a **decode failure**.
///
/// # Why unknown roles are refused rather than passed along
///
/// The tolerant alternative produces the single worst artifact this system can
/// hold: a consumer handed a *verified* delegation carrying `role: 7` has a
/// cryptographically perfect statement whose meaning nobody in the system
/// defines. The signature raises its apparent authority while its semantics
/// stay empty — a signed unchecked value looks checked. Adding a role later is
/// a `v2`; that is what versioned wire contracts are for.
///
/// The enum is therefore deliberately **not** `#[non_exhaustive]`: a new role
/// is a new wire version, so downstream `match`es being forced to change is the
/// correct and intended consequence rather than an inconvenience to smooth
/// over.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DelegationRole {
    /// `2` — the operational key: the online key that signs an anchor's
    /// day-to-day artifacts, delegated by the offline root key. Valid only for
    /// a [`Domain`](DelegationSubjectKind::Domain) subject, whose own wire value
    /// is `1` — see the type docs for why these two must differ.
    Operational,

    /// `3` — the key may sign on behalf of a
    /// [`Seat`](DelegationSubjectKind::Seat).
    ///
    /// ⚠️ **This role has defined semantics, no implementation and no
    /// consumer.** Nothing in this workspace reads it, and no consumer outside
    /// it does either. It exists because unknown roles are refused at decode, so
    /// **adding it later would be a compatibility event** — an old verifier
    /// would refuse a seat delegation — and it must therefore exist before the
    /// format freezes even though it cannot be consumed until after.
    ///
    /// That is the same admitted cost `sequence` carries, and it is admitted on
    /// the same terms: the semantics are written down (a delegation names a seat,
    /// and a verifier walking envelope → delegation → registry pin learns that
    /// the delegated key may sign for it), only the implementation is pending.
    /// The alternative is a format that can never express the case at all.
    /// **Do not invent a consumer for it.**
    SpeaksFor,
}

impl DelegationRole {
    /// The unsigned integer this role is encoded as in the payload map.
    ///
    /// Encoding goes through this function rather than through a caller-supplied
    /// number, so no value outside the enum can be emitted by this crate at all.
    pub fn wire_value(self) -> u64 {
        match self {
            Self::Operational => 2,
            Self::SpeaksFor => 3,
        }
    }

    /// The role for a wire value, or `None` if the value is not one this
    /// version defines.
    ///
    /// A value this returns `Some` for is still not necessarily acceptable: it
    /// must also be a role the subject's kind permits. See
    /// [`DelegationSubjectKind::permits`].
    ///
    /// Returns `Option` rather than a `TrustResult` so this type carries no
    /// opinion about which artifact class is decoding it; the caller supplies
    /// its own non-oracle failure value.
    pub fn from_wire(value: u64) -> Option<Self> {
        match value {
            2 => Some(Self::Operational),
            3 => Some(Self::SpeaksFor),
            _ => None,
        }
    }
}

/// The six signature-covered payload fields of a delegation — the statement
/// itself, without the signer's identity or the signature.
///
/// The root key is **not** a field here. It is the identity of whoever is
/// making the claim, not part of the claim, and carrying it in the claim as
/// well as in the protected `kid` would create two places for one value to live
/// and therefore a way for them to disagree. The signing and verifying entry
/// points in [`super::sign`] take it alongside a claim.
///
/// # An invalid `(subject_kind, role)` pair is representable here and refused everywhere else
///
/// The fields are public, so a struct literal can pair a
/// [`Domain`](DelegationSubjectKind::Domain) with
/// [`SpeaksFor`](DelegationRole::SpeaksFor). Every issuing entry point in
/// [`super::sign`] refuses such a claim before it signs, and the decoder refuses
/// it on the way back in, so the pair cannot reach the wire through this crate
/// or be accepted off the wire by it. See [`DelegationSubjectKind::permits`] for
/// the rule and why it is checked as a pair.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DelegationClaim {
    /// What kind of thing [`Self::subject_value`] is. Closed enum; see
    /// [`DelegationSubjectKind`].
    ///
    /// It is signature-covered and it is a **required argument** at
    /// verification, so the two subject namespaces are disjoint by construction
    /// rather than by the hope that a domain and a seat identifier never
    /// collide.
    pub subject_kind: DelegationSubjectKind,

    /// The subject this delegation is scoped to — an origin such as
    /// `"example.test"` when [`Self::subject_kind`] is
    /// [`Domain`](DelegationSubjectKind::Domain), a seat identifier when it is
    /// [`Seat`](DelegationSubjectKind::Seat).
    ///
    /// Opaque text to this format: it is compared against the value the verifier
    /// names by **raw UTF-8 byte equality** and never parsed, case-folded,
    /// Unicode-normalised, or stripped of a trailing dot, port or scheme. See
    /// [`super::sign`] for why the strict comparison is the safe one.
    ///
    /// Scoping is what stops a delegation issued for subject A from being a
    /// valid delegation for subject B whenever B's operator holds the same root
    /// key. It rides in the signature-covered payload, and it is a
    /// runtime-configured value reaching signed bytes rather than a committed
    /// constant.
    pub subject_value: String,

    /// The raw 32-byte Ed25519 public key being delegated **to**.
    pub delegated_public_key: [u8; KEY_LEN],

    /// The role conferred. Valid only in combination with
    /// [`Self::subject_kind`] — see [`DelegationSubjectKind::permits`].
    pub role: DelegationRole,

    /// Unix milliseconds from which the signer states the delegation takes
    /// effect.
    ///
    /// **An effectivity claim, never an ordering key.** It exists so a
    /// delegation can be prepared offline and take effect later. Two
    /// delegations bearing the same value, or a later one bearing an earlier
    /// value, are both perfectly well-formed. What orders delegations is
    /// [`Self::sequence`].
    pub not_before_unix_ms: u64,

    /// The value that **orders** delegations: strictly increasing per
    /// `(subject_kind, subject_value, role)`.
    ///
    /// **The partition includes the kind, and leaving it out would be a bug.** A
    /// domain and a seat whose identifier strings collide would otherwise share
    /// one counter — which is exactly the collision the typed subject exists to
    /// stop being left to chance.
    ///
    /// # Why this field exists — it closes a keyless replay attack
    ///
    /// Every payload field is chosen by the signer and Ed25519 is deterministic,
    /// so **two issuances of the same claim are byte-identical**. Under an
    /// earlier rule that ordered delegations by their position in the
    /// append-only log, that was exploitable by an attacker holding **no key
    /// material at all**:
    ///
    /// 1. Leaf 0 delegates to `K1`.
    /// 2. `K1` is compromised, so the operator appends a later leaf delegating
    ///    to `K2`. Revocation is an append; the old leaf is never removed.
    /// 3. The attacker appends a **verbatim copy of leaf 0**. Those bytes are
    ///    public — they are in the log, which is the entire point of the log.
    /// 4. Last-wins by log position makes the revoked `K1` current again.
    ///
    /// A replay and a legitimate re-delegation to `K1` are the *same bytes*, so
    /// no fold over the log can tell them apart. With `sequence`, a replay
    /// carries a number already superseded and loses, and a replay of the
    /// *current* delegation is byte-identical to what is already current and
    /// changes nothing. **Replay becomes a no-op rather than a rollback.**
    ///
    /// The log still establishes existence and publication. It is simply not the
    /// sort key. Ordering by a monotonic value the *trusted offline signer*
    /// chose is what a certificate serial number has always been.
    ///
    /// # `u64::MAX` is refused, so that a successor always exists
    ///
    /// The largest issuable value is `u64::MAX - 1`, enforced at both encode and
    /// decode. Strictly-increasing has no successor at the maximum, so a signer
    /// who issued there would permanently disable rotation for that subject with
    /// no in-band way out. Nothing attacker-reachable leads there — issuing needs
    /// the offline root key — so it is a foot-gun rather than a vulnerability,
    /// and it is forbidden anyway because the check costs one comparison now and
    /// is impossible to add after the format freezes.
    ///
    /// # What this format does not check
    ///
    /// Monotonicity, because a single delegation cannot exhibit it. That is a
    /// fold's obligation, along with the two-branch equivocation rule in
    /// [`super`] — **byte-identical** payloads at one
    /// `(subject_kind, subject_value, role, sequence)`
    /// are a duplicate to ignore, **differing** payloads are equivocation to
    /// refuse. Collapsing those two branches into "any collision ⇒ refuse" is a
    /// denial of service the same keyless attacker can mount; see [`super`] for
    /// why.
    pub sequence: u64,
}

/// A root key's Ed25519-signed delegation, carried on the wire as a tagged
/// `COSE_Sign1` (RFC 9052) — the `lys/delegation/v1` artifact.
///
/// Constructed by parsing artifact bytes with [`Self::from_cose_bytes`] or by
/// verifying them with [`super::sign::verify_delegation`]; produced by
/// [`super::sign::sign_delegation`] and [`super::sign::assemble_delegation`],
/// which emit bytes rather than this type because the two-phase signing path
/// exists to keep the root key on a machine that need not hold this crate's
/// types.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Delegation {
    /// Ed25519 verifying key of the **root** key that signed, from the
    /// protected `kid` header — inside the signed bytes, so it cannot be
    /// swapped without invalidating `signature`.
    ///
    /// This identifies *which* key signed. It does not make that key
    /// authoritative: a delegation verifies against whatever key it carries, so
    /// an attacker's own root key here produces a perfectly valid artifact that
    /// vouches for nothing. [`super::sign::verify_delegation`] therefore
    /// requires the caller to name the root key they trust.
    pub root_public_key: [u8; KEY_LEN],

    /// The signed statement.
    pub claim: DelegationClaim,

    /// Ed25519 signature over the COSE `Sig_structure`
    /// `["Signature1", protected, h'', payload]` (`COSE_Sign1` item 3), where
    /// `payload` is the embedded payload map's bytes.
    pub signature: [u8; 64],
}

impl Delegation {
    /// Encode this delegation as its canonical tagged `COSE_Sign1` artifact
    /// (media type `application/cose`).
    ///
    /// Infallible: the artifact shape is fixed and the hand encoder is total
    /// over the field types. The emitted bytes verify with any off-the-shelf
    /// COSE library.
    pub fn to_cose_bytes(&self) -> Vec<u8> {
        encoding::artifact_bytes(&self.root_public_key, &self.claim, &self.signature)
    }

    /// Parse a tagged `COSE_Sign1` `lys/delegation/v1` artifact.
    ///
    /// Enforces the full structural algorithm: the input cap, the required tag
    /// 18, the exact protected header pin (`alg = -8`, the v1 delegation content
    /// type, a 32-byte `kid`), the **empty** unprotected bucket, the embedded
    /// payload pin including a known role, the 64-byte signature — and then
    /// **canonical-encoding strictness**: the input must be byte-identical to
    /// the canonical re-encoding of the parsed fields, so no non-canonical
    /// artifact is ever accepted even when its signature would verify.
    ///
    /// # This function must stay monomorphic — the caller names the type
    ///
    /// **Which artifact kind is expected is the caller's declaration, made by
    /// naming this type, and the wire has no vote in it.** The content type is
    /// signature-covered, and parsing pins this module's own constant rather
    /// than reading the wire's copy — but both of those protect the label, not
    /// the *choice* of which label to expect. A convenience entry point that
    /// sniffed the type and dispatched would move that choice to the attacker
    /// while every one of those properties still held.
    ///
    /// This performs **no signature verification**, and knows neither the root
    /// key nor the subject the caller expects — follow with
    /// [`super::sign::verify_delegation`], which is the only entry point that
    /// can establish anything. A parsed delegation's fields are attacker-chosen
    /// values until then, including `root_public_key`.
    ///
    /// # Errors
    ///
    /// Returns [`TrustError::DelegationVerification`] for every rejected input —
    /// oversize, malformed CBOR, wrong shape, wrong header pins, unknown role,
    /// an empty subject value, an unknown subject kind, an invalid
    /// `(subject_kind, role)` pair, an unusable delegated key, a non-empty
    /// unprotected
    /// bucket and non-canonical encoding all return that one value.
    ///
    /// **"Same error value" is not the same as "indistinguishable", and the
    /// distinction is not hedging** — it was measured elsewhere in this module
    /// and the docs that conflated the two were wrong. Parsing deliberately does
    /// *not* equalise the work done: the length cap rejects before any CBOR is
    /// touched, and a shape failure returns before the payload is examined. That
    /// is acceptable here in a way it is not in
    /// [`super::sign::verify_delegation`], because what a parse failure reveals
    /// is a property of the **input**, which whoever supplied it already knows —
    /// not a property of the verifier's configuration, which is the secret the
    /// non-oracle discipline exists to protect.
    pub fn from_cose_bytes(bytes: &[u8]) -> TrustResult<Self> {
        let fields = encoding::decode_fields(bytes)?;
        let canonical =
            encoding::artifact_bytes(&fields.root_public_key, &fields.claim, &fields.signature);
        if canonical != bytes {
            return Err(TrustError::DelegationVerification);
        }
        Ok(Self {
            root_public_key: fields.root_public_key,
            claim: fields.claim,
            signature: fields.signature,
        })
    }
}

#[cfg(test)]
#[path = "artifact_tests.rs"]
mod tests;
