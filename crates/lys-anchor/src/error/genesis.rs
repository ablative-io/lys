//! Refusals about an anchor's genesis leaf: a log without one, one already
//! written, and a genesis delegation that cannot be built or does not say
//! what an anchor's leaf 0 must say.

#[cfg(feature = "unstable-anchor")]
use lys_core::error::TrustError;

/// Why an anchor's genesis leaf could not be written, or does not hold what
/// it must.
///
/// `#[non_exhaustive]` for the reason [`AnchorError`](super::AnchorError)
/// is: a new precondition in this family gets its own name without a major
/// version bump.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum GenesisError {
    /// The log holds no leaves, so it has no genesis leaf and never can.
    ///
    /// **Terminal, not a state to initialize out of.** `LeafStore` offers no
    /// `insert` and no way to rewrite a leaf, so position 0 cannot be filled
    /// after any other entry exists — and it cannot be filled *now* either
    /// without deciding, on the log's behalf, what its first entry says.
    /// [`Anchor::create`](crate::Anchor::create) is where genesis bytes are
    /// supplied, by the caller, once.
    #[error(
        "the log for {origin} has no genesis leaf: an anchor's leaf 0 is written when it is created and can never be inserted afterwards"
    )]
    NoGenesisLeaf {
        /// The origin of the log that was opened, as its store reports it.
        origin: String,
    },

    /// `lys-core` refused to assemble the genesis delegation.
    ///
    /// Reachable two ways, and they are not the same fault:
    ///
    /// - The claim is one `lys-core`'s own decoder would reject — in practice an
    ///   origin long enough to push the artifact past the size cap, since the
    ///   other refusals (an empty origin, an unusable delegated key) are
    ///   unreachable from a store whose origin was validated at creation and a
    ///   signer whose key was validated at load.
    /// - The signature the root signer produced does not verify against the key
    ///   that signer advertises. That is a broken
    ///   [`Signer`](crate::Signer) implementation, and it is caught here rather
    ///   than becoming a leaf 0 nothing can ever verify.
    ///
    /// Detailed rather than collapsed, on the rule the module docs set: creation
    /// is the operator's own act on their own store, with their own key. No
    /// stranger can drive it and nothing about a stranger's bytes is disclosed.
    ///
    /// **Nothing was appended.** The delegation is built and verified before the
    /// log is touched, because leaf 0 cannot be replaced.
    ///
    /// `#[cfg(feature = "unstable-anchor")]` because the delegation format is,
    /// so the default build has no call site and could not construct this.
    #[cfg(feature = "unstable-anchor")]
    #[error("failed to build the genesis delegation for {origin}: {source}")]
    GenesisDelegation {
        /// The origin of the log genesis was being written into, as its store
        /// reports it.
        origin: String,
        /// `lys-core`'s reason for refusing to assemble or to verify it.
        source: TrustError,
    },

    /// Creation was asked to delegate an anchor's operational role **to its own
    /// root key** — the two signers advertise the same public key.
    ///
    /// # Why this is refused rather than merely discouraged
    ///
    /// The artifact it would produce is **perfectly valid and completely
    /// hollow**. It is a well-formed `lys/delegation/v1` domain delegation, it
    /// verifies, its pair is in the table, and it says the offline root key has
    /// delegated the operational role to the offline root key. DP16's entire
    /// reason for two keys — that the key which signs every checkpoint is *not*
    /// the key an operator can keep air-gapped — is void, and **nothing
    /// downstream can tell.** There is no verifier that would flag it, because
    /// there is nothing malformed to flag.
    ///
    /// That lands at leaf 0, which `LeafStore` can never correct: no insert, no
    /// rewrite. So this is refused at the one moment refusing is still possible,
    /// on exactly the argument that fixes the subject kind in the same
    /// constructor — a single mis-passed argument must not be able to make an
    /// anchor permanently mean something other than what it appears to mean.
    ///
    /// # Why the check lives here and not in `lys-core`
    ///
    /// A delegation whose subject delegates to the signing key is a *format*
    /// question, and the format has not ruled on it: there may be subjects for
    /// which self-delegation is meaningful. What is not in question is DP16's
    /// two-key model, and that model lives in this constructor. So
    /// `sign_delegation` and `assemble_delegation` still permit it and this
    /// entry point does not.
    ///
    /// Descriptive rather than collapsed, for the same reason as
    /// [`Self::GenesisDelegation`]: this is an issuing-path fault on the
    /// operator's own store with the operator's own keys, so naming it costs
    /// nothing and saves an operator staring at a valid-looking anchor.
    ///
    /// **Nothing was appended.** The comparison happens before the claim is
    /// built.
    ///
    /// `#[cfg(feature = "unstable-anchor")]` because the delegation format is,
    /// so the default build has no call site and could not construct this.
    #[cfg(feature = "unstable-anchor")]
    #[error(
        "the root signer and the operational signer for {origin} advertise the same public key, \
         so leaf 0 would delegate the operational role to the root key itself: DP16's two-key \
         model would be void in a delegation nothing can distinguish from an honest one, at the \
         one position a log can never correct"
    )]
    GenesisRootKeyIsOperationalKey {
        /// The origin of the log genesis was being written into, as its store
        /// reports it.
        origin: String,
    },

    /// Creation was asked to write genesis into a log that already has entries.
    ///
    /// Appending here would put the genesis bytes at whatever the next free
    /// index happens to be, producing a log whose leaf 0 is something else
    /// entirely while every later check passes. Refused rather than appended:
    /// there is exactly one position genesis can occupy, and it is taken.
    #[error(
        "refusing to write genesis into the log for {origin}: it already holds {tree_size} leaves, and genesis is leaf 0 or nothing"
    )]
    GenesisAlreadyWritten {
        /// The origin of the log that was opened, as its store reports it.
        origin: String,
        /// The number of leaves already present.
        tree_size: u64,
    },

    /// A log was opened strictly and its leaf 0 is not a genesis delegation from
    /// the root key the caller named, for this store's own origin.
    ///
    /// **This is the variant that exists because creating a DP16 anchor and
    /// opening one were different guarantees.** `Anchor::open` checks that leaf 0
    /// exists and reads no byte of it, so a store whose genesis is uninterpreted
    /// operator bytes — everything a default-features build can create — opens
    /// indistinguishably from one whose genesis is a delegation.
    /// `Anchor::open_verifying_genesis` is where that stops, and this is its
    /// refusal.
    ///
    /// # The cause is deliberately not narrowed, and the source says so
    ///
    /// `source` is `lys-core`'s single collapsed
    /// [`TrustError::DelegationVerification`], which is one value for a malformed
    /// artifact, a non-canonical encoding, a delegation from a different root
    /// key, one for a different origin, one for a *seat* whose identifier equals
    /// this origin, and a forged signature alike. That collapse is a
    /// security property of `lys-core`'s verifier — a delegation verifier is the
    /// network-exposed surface where a distinguishable error becomes an oracle
    /// for the verifier's configuration — and this variant carries the value it
    /// was given rather than inferring a reason nobody supplied it.
    ///
    /// Naming the origin costs nothing: it is the first line of every checkpoint
    /// this anchor signs.
    ///
    /// `#[cfg(feature = "unstable-anchor")]` because the delegation format is,
    /// so the default build has no call site and could not construct this.
    #[cfg(feature = "unstable-anchor")]
    #[error(
        "leaf 0 of the log for {origin} is not a genesis delegation from the named root key for \
         that origin: {source}"
    )]
    GenesisNotADelegation {
        /// The origin of the log that was opened, as its store reports it —
        /// which is also the subject value leaf 0 was required to name.
        origin: String,
        /// `lys-core`'s single collapsed reason for refusing the delegation.
        source: TrustError,
    },

    /// A log was opened strictly and its leaf 0 delegates the operational role
    /// to the very key that signed it.
    ///
    /// The open-time arm of the rule
    /// [`Self::GenesisRootKeyIsOperationalKey`] enforces at creation, and the
    /// reason it needs a second arm is that the two paths cannot see the same
    /// thing. Creation compares two *signers* it was handed. Opening has no
    /// signers to compare — only the artifact — and the artifact was not
    /// necessarily written by this crate's constructor. `lys-core` permits
    /// self-delegation, because whether a subject may delegate to its own signer
    /// is a format question the format has not ruled on, so nothing between the
    /// bytes and here would otherwise flag it.
    ///
    /// What it would mean if accepted is unchanged from the creation-time
    /// variant: a perfectly valid, perfectly signed delegation in which DP16's
    /// entire reason for two keys is void, at the one position a log can never
    /// correct.
    ///
    /// `#[cfg(feature = "unstable-anchor")]` because the delegation format is,
    /// so the default build has no call site and could not construct this.
    #[cfg(feature = "unstable-anchor")]
    #[error(
        "leaf 0 of the log for {origin} delegates the operational role to its own root key, so \
         DP16's two-key model is void in an artifact nothing else would flag"
    )]
    GenesisDelegatesToTheRootKey {
        /// The origin of the log that was opened, as its store reports it.
        origin: String,
    },

    /// A log was opened strictly and its leaf 0 carries a `sequence` other than
    /// [`GENESIS_SEQUENCE`](crate::GENESIS_SEQUENCE).
    ///
    /// **This checks a convention of this crate, not a property of the format,
    /// and the distinction is the whole content of the variant.** `lys/delegation/v1`
    /// marks nothing as genesis; `sequence = 0` is what
    /// `Anchor::create_with_delegated_genesis` writes, and it writes it because a
    /// caller-chosen start would open a range below the first delegation into
    /// which nothing can ever be written. A stranger holding only the artifact
    /// still cannot conclude from `sequence = 0` that they are looking at the
    /// first delegation for a subject, and this refusal does not make them able
    /// to.
    ///
    /// The value read is carried rather than written into the message as a
    /// literal, so an operator is told what was found instead of what was
    /// expected.
    ///
    /// `#[cfg(feature = "unstable-anchor")]` because the delegation format is,
    /// so the default build has no call site and could not construct this.
    #[cfg(feature = "unstable-anchor")]
    #[error(
        "leaf 0 of the log for {origin} carries sequence {sequence}: this crate writes 0 at \
         genesis, and a higher start leaves a range below the first delegation that nothing can \
         ever fill"
    )]
    GenesisSequenceIsNotGenesis {
        /// The origin of the log that was opened, as its store reports it.
        origin: String,
        /// The `sequence` leaf 0 actually carried.
        sequence: u64,
    },
}
