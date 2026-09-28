//! The subject kind of a delegation: the namespace its subject value is read
//! in, its frozen wire values, and the table of roles each kind permits.

use super::DelegationRole;

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
/// [`verify_delegation`](crate::delegation::sign::verify_delegation) requires the caller to
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
