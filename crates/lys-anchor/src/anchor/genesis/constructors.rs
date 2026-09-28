//! The two anchor constructors that give leaf 0 a meaning: creating an anchor
//! whose genesis is a delegation from a root key, and opening one only after
//! that delegation verifies.

use lys_core::delegation::{
    DelegationClaim, DelegationRole, DelegationSubjectKind, assemble_delegation,
    delegation_preimage,
};
use lys_log_store::{LeafStore, Log};

use super::{GENESIS_SEQUENCE, verify_genesis_delegation};
use crate::admission::AdmissionPolicy;
use crate::anchor::Anchor;
use crate::config::AnchorConfig;
use crate::error::{AnchorError, AnchorResult};
use crate::keys::{InProcessSigner, Signer};

impl<S: LeafStore, K: InProcessSigner, P: AdmissionPolicy> Anchor<S, K, P> {
    /// Creates an anchor over `store` whose leaf 0 **is** a
    /// `lys/delegation/v1` delegation from `root_signer` to `signer`'s
    /// public key, for the store's origin as a
    /// [`Domain`](lys_core::delegation::DelegationSubjectKind::Domain) subject,
    /// at role [`Operational`](lys_core::delegation::DelegationRole::Operational)
    /// and [`GENESIS_SEQUENCE`].
    ///
    /// This is the DP16 constructor. [`create`](Self::create) is the one that
    /// takes uninterpreted bytes; the [module docs](super) explain why both exist
    /// and why that is not a state of affairs to tidy away.
    ///
    /// `root_signer` is used once and never retained — an `Anchor` has no field
    /// for it, so a running anchor cannot sign with the root key. It is bounded
    /// by [`Signer`] rather than [`InProcessSigner`], so an offline or remote
    /// root key can issue genesis; `signer`, which must sign checkpoints
    /// afterwards, still carries the stronger bound.
    ///
    /// `not_before_unix_ms` is the signer's own effectivity claim. This crate
    /// reads no clock: see the [module docs](super).
    ///
    /// The store must already exist and must be empty. `policy` governs
    /// submissions and is **not** consulted here, exactly as in
    /// [`create`](Self::create).
    ///
    /// # Nothing is written unless the delegation was produced
    ///
    /// The log is left untouched if the emptiness check, the signing or the
    /// assembly fails. That ordering is an invariant rather than an incidental
    /// property of this function's shape — leaf 0 cannot be replaced, so a
    /// declined signature must leave a log that can still be given genesis.
    ///
    /// # Errors
    ///
    /// - [`AnchorError::GenesisAlreadyWritten`] if the store already holds
    ///   leaves.
    /// - [`AnchorError::GenesisRootKeyIsOperationalKey`] if `root_signer` and
    ///   `signer` advertise the same public key. Checked before anything is
    ///   built or signed; see the [module docs](super) for why a valid-looking
    ///   artifact is the worst possible outcome here.
    /// - Whatever `root_signer` returned, unchanged, if it declined to sign. The
    ///   error is the signer implementation's own and is propagated rather than
    ///   wrapped: a remote signer's reason for refusing is the only account of
    ///   that refusal in existence, and re-describing it here would replace it
    ///   with a guess.
    /// - [`AnchorError::GenesisDelegation`] if `lys-core` refused to assemble
    ///   the artifact — a claim its own decoder would reject, or a signature
    ///   that does not verify against the key going into the protected `kid`.
    ///   The latter means `root_signer` broke [`Signer`]'s contract.
    /// - [`AnchorError::Store`] for anything the log or its storage refuses,
    ///   including an integrity failure found while opening.
    pub fn create_with_delegated_genesis<R: Signer>(
        store: S,
        root_signer: &R,
        not_before_unix_ms: u64,
        signer: K,
        policy: P,
        config: AnchorConfig,
    ) -> AnchorResult<Self> {
        let mut log = Log::open(store)?;
        let tree_size = log.tree().len();
        if tree_size != 0 {
            return Err(AnchorError::GenesisAlreadyWritten {
                origin: log.origin().to_string(),
                tree_size,
            });
        }

        // ⛔ The two keys must differ, and this is the last moment anyone can
        // require it. A delegation from the root key to the root key is a
        // perfectly valid artifact that says nothing — DP16's two-key model
        // void, with no malformedness for any verifier to notice — written to
        // the one leaf a `LeafStore` can never correct.
        //
        // Compared through each signer's own `public_key()`, which is the same
        // value that reaches `kid` and the payload respectively, so this cannot
        // pass by comparing something other than what gets signed.
        let root_public_key = root_signer.public_key();
        if root_public_key == signer.public_key() {
            return Err(AnchorError::GenesisRootKeyIsOperationalKey {
                origin: log.origin().to_string(),
            });
        }

        // The subject VALUE comes from storage and from nowhere else. There is
        // no origin field on this crate to prefer over it and no constant to fall
        // back to. The subject KIND is fixed: an anchor's subject is a domain,
        // never a seat, and leaf 0 is the one leaf that can never be corrected.
        let claim = DelegationClaim {
            subject_kind: DelegationSubjectKind::Domain,
            subject_value: log.origin().to_string(),
            delegated_public_key: signer.public_key(),
            role: DelegationRole::Operational,
            not_before_unix_ms,
            sequence: GENESIS_SEQUENCE,
        };

        let signature = root_signer.sign(&delegation_preimage(&root_public_key, &claim))?;
        // `assemble_delegation` re-verifies the signature against
        // `root_public_key` before returning bytes, so a signer whose advertised
        // key does not match what it signed with is refused here rather than
        // discovered by a stranger years later.
        let genesis =
            assemble_delegation(&root_public_key, &claim, &signature).map_err(|source| {
                AnchorError::GenesisDelegation {
                    origin: log.origin().to_string(),
                    source,
                }
            })?;

        // Only now. Everything above can fail without touching the log.
        log.append(&genesis)?;
        Ok(Self {
            log,
            signer,
            config,
            policy,
        })
    }

    /// Opens an anchor over an existing `store`, refusing it unless leaf 0 is a
    /// genesis delegation from `expected_root_public_key` for the store's own
    /// origin.
    ///
    /// This is [`Anchor::open`](crate::anchor::Anchor::open) plus
    /// [`verify_genesis_delegation`], and it is the only constructor in this
    /// crate that checks anything about leaf 0's *content* on the way in. `open`
    /// checks that leaf 0 exists and reads none of it; the [module docs](super)
    /// say why the strict version is a second name rather than a stricter `open`,
    /// and record that nothing forces a caller here.
    ///
    /// Everything `open` does still happens first: the tree is rebuilt from
    /// stored leaves and reconciled with the pin, and an interrupted append is
    /// repaired and reported through
    /// [`recovered_to`](crate::anchor::Anchor::recovered_to).
    ///
    /// # ⛔ What a successful open does not tell you
    ///
    /// That leaf 0 is a well-formed genesis delegation from a key you named. It
    /// is **not** a statement that the key is trustworthy, that `signer` is the
    /// currently delegated operational key, or that any later leaf has not
    /// superseded the delegation — the last needs a fold over the log that does
    /// not exist yet. [`verify_genesis_delegation`] enumerates all three, and
    /// returns the parsed delegation for a caller who wants to compare the
    /// delegated key themselves.
    ///
    /// # Errors
    ///
    /// - [`AnchorError::NoGenesisLeaf`] if the log has no leaves.
    /// - [`AnchorError::NoSuchLeaf`] if the tree is non-empty and index 0 is
    ///   nonetheless absent from storage. Not reachable through a `LeafStore`
    ///   honouring its contract, and named rather than assumed away because the
    ///   alternative is an `unwrap` on somebody else's invariant.
    /// - Everything [`verify_genesis_delegation`] returns.
    /// - [`AnchorError::Store`] for anything the log or its storage refuses —
    ///   notably `StoreError::PinMismatch` when the stored leaves no longer
    ///   rebuild to the pinned root.
    pub fn open_verifying_genesis(
        store: S,
        expected_root_public_key: &[u8; 32],
        signer: K,
        policy: P,
        config: AnchorConfig,
    ) -> AnchorResult<Self> {
        let log = Log::open(store)?;
        let tree_size = log.tree().len();
        if tree_size == 0 {
            return Err(AnchorError::NoGenesisLeaf {
                origin: log.origin().to_string(),
            });
        }

        let leaf_zero = log.leaf_bytes(0).ok_or_else(|| AnchorError::NoSuchLeaf {
            origin: log.origin().to_string(),
            leaf_index: 0,
            tree_size,
        })?;

        // The origin comes from storage on this path exactly as it does on the
        // creation path, through the same accessor. There is no argument for a
        // caller to disagree with it through.
        verify_genesis_delegation(leaf_zero, expected_root_public_key, log.origin())?;

        Ok(Self {
            log,
            signer,
            config,
            policy,
        })
    }
}
