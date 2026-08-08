//! Genesis as a delegation: leaf 0 **is** a `lys/delegation/v1`
//! artifact, signed by the anchor's offline root key, delegating to its
//! operational key.
//!
//! This is where DP16's two-key model stops being prose. The root key signs
//! exactly one thing — a delegation into the log — and then has no further job;
//! the operational key signs every checkpoint and receipt afterwards. Before
//! this module the two keys were a plan, and an anchor's leaf 0 was whatever
//! bytes its operator happened to hand
//! [`create`](super::Anchor::create).
//!
//! # ⛔ There are two genesis constructors, and that is a finding rather than a convenience
//!
//! [`Anchor::create`](super::Anchor::create) still exists, still takes
//! caller-supplied bytes, and is **not** gated. This module's
//! [`Anchor::create_with_delegated_genesis`] is gated on `unstable-anchor`. That
//! is not a transition state to be tidied up later; it is forced, and the force
//! is worth writing down because the obvious tidy-up is the one thing that must
//! not happen.
//!
//! The delegation format lives behind `lys-core`'s off-by-default
//! `unstable-anchor` feature **so that it stays changeable** — a format freezes
//! when a crate exposing it is published or when a durable artifact is signed
//! under its tag, and three specification bugs have already been found in the
//! draft formats by implementing them. Meanwhile **genesis is not optional**:
//! `LeafStore` offers no `insert`, so a log created without a leaf 0 can never
//! be given one, and `lys-core` declines to sign a receipt over a one-leaf tree.
//!
//! Those two facts do not both fit into one constructor:
//!
//! - A single `create` that builds a delegation would not compile in the default
//!   build, because `lys_core::delegation` does not exist there. A default build
//!   could then create no anchor at all — a worse form of the trap already
//!   recorded in [`append`](super::append)'s docs, where gating the *append*
//!   path left a default anchor frozen at tree size 1.
//! - A single `create` whose signature changes under `#[cfg]` is two functions
//!   wearing one name. Every call site, the CLI's `init` arguments and
//!   `tests/standalone_is_complete.rs` — which is compiled in **both** shapes
//!   with no `#[cfg]` in it, deliberately — would have to fork.
//! - A single `create` that took a root signer and refused when the feature is
//!   off would be a default build that cannot create an anchor. Same trap,
//!   reached by an error instead of a compile failure.
//! - A single `create` that took a root signer and wrote *something else* when
//!   the feature is off would be the worst of the four: a log whose leaf 0 looks
//!   like a delegation was intended and is not one. That is "a signed unchecked
//!   value looks checked" in durable form, at the one position that can never be
//!   corrected.
//!
//! So the two shapes genuinely need different genesis, and the honest expression
//! of that is two differently-named constructors rather than one name with two
//! meanings. **A default-features build of this crate cannot create a
//! DP16-conformant anchor, by construction**, and it will not be able to until
//! the delegation format is ratified and the gate comes off. `create` is
//! therefore documented as what it is — an uninterpreted leaf 0 — and this
//! constructor is the DP16 one.
//!
//! # Invariants
//!
//! - **Nothing is appended until the delegation exists and verifies.** The order
//!   is `check the log is empty → build the claim → sign → assemble (which
//!   verifies) → append`. It is load-bearing rather than tidy: leaf 0 cannot be
//!   replaced, so a signer that declines — a remote signer, an operator refusing
//!   authorization, a key file that has gone away — must leave an **empty** log
//!   that can still be given a genesis delegation later. Appending first and
//!   signing second would produce a log permanently holding a leaf 0 that is not
//!   a delegation, with no way back.
//! - **The subject is the store's origin under `subject_kind = domain`, never a
//!   configuration field and never a constant.** The value is read through
//!   `Log::origin` → `LeafStore::origin`, fixed when the store was created.
//!   DP15 forbids a committed origin and [`AnchorConfig`] has no field for one,
//!   so the store's value is the only one in existence for this crate to put in
//!   the payload. This is DP15 working the way it was meant to: a
//!   runtime-configured value reaching signed bytes.
//! - ⛔ **The subject kind is fixed at
//!   [`Domain`](lys_core::delegation::DelegationSubjectKind::Domain) and the role
//!   at [`Operational`](lys_core::delegation::DelegationRole::Operational) — the
//!   pair `(1, 2)` — and NEITHER is a parameter.** This is the most consequential
//!   of the fixings, and the reason is that **leaf 0 is the one leaf that can
//!   never be corrected**: `LeafStore` has no insert and no rewrite.
//!
//!   A constructor that took the kind from a caller would let **a single
//!   mis-passed argument produce an anchor whose genesis permanently claims to
//!   delegate a *seat*.** There would be no recovery path, and — this is the part
//!   worth sitting with — **no verifier would flag it.** The artifact would be
//!   perfectly signed and internally valid, because `(2 seat, 3 speaks-for)` is in
//!   `lys-core`'s pair table; the delegation would simply mean something an anchor
//!   has no use for, in the one position nothing can revise. Fixing the kind here
//!   is what makes *"an anchor's genesis is a domain delegation"* a property of
//!   the code rather than of the caller.
//!
//!   It is also the *consistent* choice rather than merely the safe one: the
//!   store's origin is a domain, so a seat genesis would put a domain string in a
//!   field typed as a seat identifier — the "signed value that reads as something
//!   it is not" failure the typed subject exists to prevent, committed by the very
//!   code that was supposed to prevent it.
//!
//!   `lys-core` validates `(kind, role)` as a pair, so this crate could not pick
//!   one of the two independently even if it wanted to; the pair is chosen once,
//!   here, in one expression.
//! - **The delegated key is the operational signer's own public key**, read from
//!   the signer this anchor will publish under. There is no parameter for it, so
//!   an anchor cannot be created delegating to a key it does not hold.
//! - ⛔ **The two keys must differ, and a request to make them the same is
//!   refused.** A delegation from the root key to the root key is a *valid*
//!   `lys/delegation/v1` artifact — correct pair, correct subject, verifies —
//!   that voids DP16's entire reason for two keys while leaving nothing
//!   malformed for any verifier to flag. It would sit at leaf 0, which cannot be
//!   corrected. This is the same argument that fixes the subject kind above,
//!   applied to the axis the constructor had left open, and it is checked
//!   **before** anything is built or signed. `lys-core` still permits the shape:
//!   whether a subject may delegate to its own signer is a format question and
//!   the format has not ruled on it, whereas DP16's two-key model is this
//!   crate's and lives here.
//! - **The root signer is a parameter and is never retained.** See below.
//! - **A root signer whose `public_key` disagrees with its `sign` is caught
//!   here.** [`Signer`]'s contract requires them to agree; this path does not
//!   take that on trust, because `assemble_delegation` re-verifies the signature
//!   against the key that is going into the protected `kid` before returning any
//!   bytes. A signer that broke the contract would otherwise write a leaf 0 that
//!   nothing can ever verify.
//!
//! # Why the root signer is a parameter and not a field
//!
//! An [`Anchor`] holds its *operational* signer from
//! construction, for the reason [`open`](super::open) records: a key passed in
//! at each signing site is a key every caller must hold. The root key is the
//! opposite case, and the same reasoning inverts.
//!
//! DP16 gives the root key exactly one job — signing delegations into the log —
//! precisely so it can live somewhere the operational key cannot: offline, on a
//! machine that never serves traffic, backed up in two physical locations. A
//! running anchor has no use for it. So it arrives as a parameter, is used once,
//! and is gone; `Anchor` has no field it could be stored in and no method that
//! takes one. **The type system, not an operator's discipline, is what prevents
//! a running anchor from ever signing with the root key again.**
//!
//! It is taken by reference rather than by value. Consuming it would read as a
//! stronger custody statement and would not be one — the guarantee is that
//! `Anchor` holds no root key, which is a property of its fields either way —
//! while forcing an operator who legitimately retains the root key for a later
//! rotation to clone it. Cloning key material to satisfy an ownership gesture is
//! a worse outcome than not consuming it.
//!
//! # Why the bound is [`Signer`] and not [`InProcessSigner`]
//!
//! **This is the first place in the crate where the custody boundary is usable
//! rather than reserved**, and it is usable here for a specific reason.
//! `lys-core`'s other signing entry points take a concrete `&Ed25519Identity`,
//! which is why every other signing operation on an anchor carries the
//! [`InProcessSigner`] bound. The delegation format is deliberately different:
//! it exposes the two-phase `delegation_preimage` / `assemble_delegation` pair
//! *because* the root key is the offline key, so that the preimage can be
//! carried to whatever holds it and 64 bytes carried back.
//!
//! This constructor is written through that pair rather than through the
//! `sign_delegation` convenience, so `R: Signer` is the whole bound. A remote
//! signer — an HSM, a KMS, an air-gapped ceremony behind a `Signer` impl — can
//! issue an anchor's genesis delegation today. The operational signer still
//! carries [`InProcessSigner`], because checkpoints and receipts still go
//! through `lys-core`'s concrete entry points, and that asymmetry is exactly the
//! custody story: the key that must be online is the one this crate must hold.
//!
//! # `sequence` is 0, by convention, and the convention is this crate's
//!
//! [`GENESIS_SEQUENCE`] is `0`. The specification is explicit that **nothing in
//! the format marks a genesis delegation** — `sequence = 0` is a convention, not
//! a claim the artifact makes — so a fold reading from a non-zero log offset
//! cannot tell whether it has seen the true minimum. That limitation is not
//! fixed here and cannot be: it is a property of the frozen payload.
//!
//! What *is* decided here is that the value is not a parameter. Two reasons:
//!
//! - Genesis is the first delegation for its `(subject_kind, subject_value,
//!   role)` by construction —
//!   there is no earlier position for one to occupy, because leaf 0 is the first
//!   leaf. Starting anywhere above 0 creates a range below the first delegation
//!   into which nothing can ever be written, which is a gap with no meaning.
//! - A caller-chosen genesis sequence is a foot-gun with the same shape as the
//!   `u64::MAX` one `lys-core` refuses: an operator who started at
//!   `u64::MAX - 1` would have exactly one rotation left for the life of the
//!   origin. The format closes the maximum; this closes the arbitrary start.
//!
//! # `not_before_unix_ms` is a parameter, and this crate does not read a clock
//!
//! It is a **claim by the signer**, and the signer is the party holding the root
//! key — the caller — not this library. Two consequences, and both point the
//! same way:
//!
//! - A library that reads a wall clock cannot be tested against a fixed
//!   expectation, and the value it produced cannot be reproduced afterwards by
//!   anyone checking what was signed. `chrono` is available and is deliberately
//!   not used here.
//! - A delegation is *meant* to be preparable offline to take effect later — the
//!   whole reason `not_before` exists as an effectivity claim that orders
//!   nothing. A clock read at creation forbids the case the field was added for.
//!
//! It is a bare parameter rather than an [`AnchorConfig`] field because it is an
//! input to one act, not a property of the anchor: `open` would otherwise have
//! to be handed a `not_before` it has no use for, on every call, forever.
//!
//! # ⛔ Creating a DP16 anchor and opening one are different guarantees, and the second is opt-in
//!
//! [`Anchor::create_with_delegated_genesis`] makes leaf 0 a delegation. It does
//! **not** follow that every anchor a process opens has one:
//! [`Anchor::open`](super::Anchor::open) checks that leaf 0 *exists* and reads no
//! byte of it, so a store created by a default-features build through
//! [`create`](super::Anchor::create) — whose leaf 0 is uninterpreted operator
//! bytes — opens without complaint under an all-features build, and the resulting
//! value is indistinguishable from a DP16 anchor at every method.
//!
//! [`Anchor::open_verifying_genesis`] is the opt-in that closes that, and
//! [`verify_genesis_delegation`] is the check itself, reachable without opening
//! anything so a read-only auditor can run it too.
//!
//! **Why opt-in rather than folded into `open`.** Exactly the argument this
//! module's four rejected alternatives already make about `create`, one layer
//! along: `open` is ungated, so it cannot call into `lys_core::delegation` —
//! that module does not exist in a default build. An `open` whose behaviour
//! changed under `#[cfg]` would be two functions wearing one name, and an `open`
//! that refused a non-delegation genesis would leave a default build unable to
//! open the anchors it is able to create. So the strict open is a second,
//! differently-named constructor, and the residual is stated rather than
//! designed away: **nothing forces a caller to use it**, and an anchor opened
//! through [`Anchor::open`](super::Anchor::open) carries no genesis guarantee
//! whatsoever.
//!
//! # What the open-time check establishes, and the three things it does not
//!
//! [`verify_genesis_delegation`] establishes, for the bytes at leaf 0:
//!
//! - they parse as a canonically-encoded `lys/delegation/v1` artifact;
//! - the signature verifies under the root key **the caller named**;
//! - the subject is `(Domain, this store's origin)` — the origin read through
//!   `LeafStore::origin`, never a caller argument, for the reason below;
//! - the delegated key is not the root key, so DP16's two-key model is not void;
//! - `sequence` is [`GENESIS_SEQUENCE`].
//!
//! It does **not** establish:
//!
//! - **That the named root key is the right one.** A delegation verifies against
//!   whatever key it carries; naming the trusted key is the caller's declaration
//!   and always was. `lys-core` refuses to offer an unattributed verify for this
//!   reason and so does this.
//! - **That this anchor's operational signer is the currently delegated key.**
//!   Leaf 0 names the key that was delegated *at genesis*. Revocation is an
//!   append of a later, superseding delegation, so after any rotation leaf 0
//!   names a key that is no longer operational — and deciding which delegation is
//!   current is a fold over the whole log, which does not exist yet. Checking
//!   `signer.public_key()` against leaf 0 here would therefore be a check that
//!   *forbids rotation*, sold as a check that confirms the key. The delegated key
//!   is returned instead, so a caller who knows their anchor has never rotated can
//!   compare it and one that has cannot be broken by an assumption this crate made
//!   for them.
//! - **That leaves 1..n are anything at all.** They are not read. A log whose
//!   genesis is impeccable and whose every later leaf revokes it passes.
//!
//! `sequence == GENESIS_SEQUENCE` is a check against **this crate's convention**
//! and nothing stronger: the format has no genesis marker, so a stranger holding
//! only the artifact cannot conclude from `sequence = 0` that it is the first
//! delegation for its subject. What the check buys is that a leaf 0 written with
//! a nonzero start — the foot-gun the constructor closes by not taking the value
//! as a parameter — is refused on the way back in rather than never noticed.
//!
//! # Why the open-time check reads the origin from the store and does not take one
//!
//! An `expected_subject` parameter is the obvious shape and it is the wrong one.
//! The creation-time invariant is that leaf 0's subject **is** the store's
//! origin; a caller-supplied subject checks leaf 0 against the caller instead,
//! so a store whose origin had drifted from its own genesis would pass as long
//! as the caller named genesis's value. That is the binding worth having,
//! silently dropped. The origin therefore comes from `LeafStore::origin` on both
//! paths, from the same accessor, and a caller who wants to pin *which store this
//! is* has [`Anchor::origin`](super::Anchor::origin) for it — a separate question
//! with a separate answer.

use lys_core::delegation::{
    Delegation, DelegationClaim, DelegationRole, DelegationSubjectKind, assemble_delegation,
    delegation_preimage, verify_delegation,
};
use lys_log_store::{LeafStore, Log};

use crate::admission::AdmissionPolicy;
use crate::config::AnchorConfig;
use crate::error::{AnchorError, AnchorResult};
use crate::keys::{InProcessSigner, Signer};

use super::Anchor;

/// The `sequence` a genesis delegation carries: `0`.
///
/// A **convention of this crate**, not a claim the artifact makes — the format
/// has no genesis marker, so nothing downstream can conclude from `sequence = 0`
/// that it is looking at the first delegation for a subject. It is named as a
/// constant so that the convention has one definition, and so that a future fold
/// which wants to *assume* the minimum has somewhere to find what this crate
/// actually wrote.
///
/// See the [module docs](self) for why genesis does not take this as a
/// parameter.
pub const GENESIS_SEQUENCE: u64 = 0;

/// Checks that `leaf_zero` is a genesis delegation for `origin` from
/// `expected_root_public_key`, returning it parsed.
///
/// This is the open-time counterpart to
/// [`Anchor::create_with_delegated_genesis`], and it is a free function rather
/// than a method so that a party holding only the bytes — a read-only anchor, an
/// auditor, a witness — can run the same check without a signer, an admission
/// policy, or the ability to append. There is one definition of the rule and
/// both paths reach it.
///
/// `origin` is the store's own origin. Callers inside this crate read it through
/// `LeafStore::origin`; the [module docs](self) say why it is not an
/// `expected_subject` argument.
///
/// # ⛔ Read what this does not establish before relying on it
///
/// It is a check on **leaf 0 alone**. It says nothing about whether the named
/// root key deserves trust, whether the delegation is still current, or what any
/// later leaf says — see the [module docs](self), which enumerate the three gaps.
/// In particular the returned [`Delegation`]'s
/// `claim.delegated_public_key` is the key delegated **at genesis**, which is the
/// operational key only until the first rotation.
///
/// The `(subject_kind, role)` pair is not re-checked here: `lys-core` validates
/// it at decode and `Domain` permits only `Operational`, so a second check in
/// this function would guard a rule already guarded and leave neither provable by
/// the obvious case. The role is asserted in this module's tests against the
/// enum variant, not against whatever the artifact carried.
///
/// # Errors
///
/// - [`AnchorError::GenesisNotADelegation`] if the bytes are not a canonical
///   `lys/delegation/v1` artifact, or do not verify under
///   `expected_root_public_key`, or name a different subject or subject kind.
///   `lys-core` collapses all of those into one value on purpose, and this
///   variant carries it rather than re-deriving a reason it was not given.
/// - [`AnchorError::GenesisDelegatesToTheRootKey`] if the delegated key equals
///   the signing root key — the open-time arm of the rule
///   [`AnchorError::GenesisRootKeyIsOperationalKey`] enforces at creation.
/// - [`AnchorError::GenesisSequenceIsNotGenesis`] if `sequence` is not
///   [`GENESIS_SEQUENCE`].
pub fn verify_genesis_delegation(
    leaf_zero: &[u8],
    expected_root_public_key: &[u8; 32],
    origin: &str,
) -> AnchorResult<Delegation> {
    // The kind is named as a constant of this crate, exactly as the constructor
    // names it: an anchor's genesis is a domain delegation, and `lys-core`
    // requires the caller to declare the kind precisely so that a seat
    // delegation whose identifier happens to equal this origin cannot pass.
    let delegation = verify_delegation(
        leaf_zero,
        expected_root_public_key,
        DelegationSubjectKind::Domain,
        origin,
    )
    .map_err(|source| AnchorError::GenesisNotADelegation {
        origin: origin.to_string(),
        source,
    })?;

    // DP16's two-key model, checked against the artifact rather than against two
    // signers. `lys-core` permits self-delegation — whether a subject may
    // delegate to its own signer is a format question the format has not ruled
    // on — so a store whose leaf 0 was written by anything other than this
    // crate's constructor can carry one, and nothing else would flag it.
    if delegation.claim.delegated_public_key == delegation.root_public_key {
        return Err(AnchorError::GenesisDelegatesToTheRootKey {
            origin: origin.to_string(),
        });
    }

    if delegation.claim.sequence != GENESIS_SEQUENCE {
        return Err(AnchorError::GenesisSequenceIsNotGenesis {
            origin: origin.to_string(),
            sequence: delegation.claim.sequence,
        });
    }

    Ok(delegation)
}

impl<S: LeafStore, K: InProcessSigner, P: AdmissionPolicy> Anchor<S, K, P> {
    /// Creates an anchor over `store` whose leaf 0 **is** a
    /// `lys/delegation/v1` delegation from `root_signer` to `signer`'s
    /// public key, for the store's origin as a
    /// [`Domain`](lys_core::delegation::DelegationSubjectKind::Domain) subject,
    /// at role [`Operational`](lys_core::delegation::DelegationRole::Operational)
    /// and [`GENESIS_SEQUENCE`].
    ///
    /// This is the DP16 constructor. [`create`](Self::create) is the one that
    /// takes uninterpreted bytes; the [module docs](self) explain why both exist
    /// and why that is not a state of affairs to tidy away.
    ///
    /// `root_signer` is used once and never retained — an `Anchor` has no field
    /// for it, so a running anchor cannot sign with the root key. It is bounded
    /// by [`Signer`] rather than [`InProcessSigner`], so an offline or remote
    /// root key can issue genesis; `signer`, which must sign checkpoints
    /// afterwards, still carries the stronger bound.
    ///
    /// `not_before_unix_ms` is the signer's own effectivity claim. This crate
    /// reads no clock: see the [module docs](self).
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
    ///   built or signed; see the [module docs](self) for why a valid-looking
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
    /// This is [`Anchor::open`](super::Anchor::open) plus
    /// [`verify_genesis_delegation`], and it is the only constructor in this
    /// crate that checks anything about leaf 0's *content* on the way in. `open`
    /// checks that leaf 0 exists and reads none of it; the [module docs](self)
    /// say why the strict version is a second name rather than a stricter `open`,
    /// and record that nothing forces a caller here.
    ///
    /// Everything `open` does still happens first: the tree is rebuilt from
    /// stored leaves and reconciled with the pin, and an interrupted append is
    /// repaired and reported through
    /// [`recovered_to`](super::Anchor::recovered_to).
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

#[cfg(test)]
#[path = "genesis_tests.rs"]
mod tests;
