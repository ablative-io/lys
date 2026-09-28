//! The audit receipt (P7), and its read-only verification against the log.
//!
//! A receipt carries the envelope version, the operation id, the actor as the
//! service attested them, the identity, the change kind, the SHA-256 payload
//! commitment and the log coordinate the event's leaf completed. It carries no
//! secret and no whole context object. Anyone holding the signed message, the
//! service's public key, a checkpoint of the log and an inclusion proof can
//! check it: a changed actor, payload, position or signature fails.

use ciborium::Value;
use lys_core::merkle::{InclusionProof, RootHash, raw_leaf_hash, verify_inclusion_raw};

use crate::error::IdentityError;
use crate::event::{EVENT_VERSION, IdentityEvent, wire};
use crate::id::IdentityId;
use crate::log::Coordinate;
use crate::operation::OperationId;
use crate::provenance::{Actor, Provenance};
use crate::signer::{SignedEvent, verify_event};
use crate::state_value::{
    Unreadable, array, binding, bytes, coordinate, identity, operation, read_binding,
    read_coordinate, read_fixed, read_identity, read_operation, read_uint, tuple, uint,
};

/// What the directory answers for a recorded change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Receipt {
    version: u64,
    operation: OperationId,
    actor: Actor,
    identity: IdentityId,
    change_kind: u64,
    payload_commitment: [u8; 32],
    coordinate: Coordinate,
}

impl Receipt {
    /// The receipt of `signed`, committed at `coordinate`.
    pub fn of(signed: &SignedEvent, coordinate: Coordinate) -> Self {
        let event = signed.event();
        Self {
            version: EVENT_VERSION,
            operation: event.operation(),
            actor: event.actor().clone(),
            identity: event.identity(),
            change_kind: wire::change(event.change()),
            payload_commitment: signed.payload_commitment(),
            coordinate,
        }
    }

    /// The envelope version the event was signed under.
    pub fn version(&self) -> u64 {
        self.version
    }

    /// The caller's operation id.
    pub fn operation(&self) -> OperationId {
        self.operation
    }

    /// The actor, as the service attested them.
    pub fn actor(&self) -> &Actor {
        &self.actor
    }

    /// The identity the change is about.
    pub fn identity(&self) -> IdentityId {
        self.identity
    }

    /// The change kind's wire code.
    pub fn change_kind(&self) -> u64 {
        self.change_kind
    }

    /// SHA-256 over the event body.
    pub fn payload_commitment(&self) -> [u8; 32] {
        self.payload_commitment
    }

    /// Where the event's leaf stands in the log.
    pub fn coordinate(&self) -> Coordinate {
        self.coordinate
    }

    fn matches(&self, event: &IdentityEvent) -> bool {
        self.operation == event.operation()
            && &self.actor == event.actor()
            && self.identity == event.identity()
            && self.change_kind == wire::change(event.change())
    }
}

/// The receipt as a snapshot state holds it.
pub(crate) fn encode_receipt(receipt: &Receipt) -> Value {
    let provenance = receipt.actor.provenance();
    array(vec![
        uint(receipt.version),
        operation(receipt.operation),
        array(vec![
            binding(receipt.actor.binding()),
            uint(wire::method(provenance.method())),
            uint(provenance.authenticated_at()),
        ]),
        identity(receipt.identity),
        uint(receipt.change_kind),
        bytes(&receipt.payload_commitment),
        coordinate(receipt.coordinate),
    ])
}

/// The receipt a snapshot state holds.
pub(crate) fn decode_receipt(value: Value) -> Result<Receipt, Unreadable> {
    let [version, op, actor, identity, change_kind, commitment, at] =
        tuple::<7>(value, "a receipt")?;
    let [bound, method, authenticated_at] = tuple::<3>(actor, "a receipt's actor")?;
    let method = read_uint(&method, "an authentication method")?;
    let method = wire::method_from(method)
        .ok_or_else(|| format!("authentication method {method} is not a method"))?;
    Ok(Receipt {
        version: read_uint(&version, "a receipt's version")?,
        operation: read_operation(op)?,
        actor: Actor::new(
            read_binding(bound)?,
            Provenance::new(
                method,
                read_uint(&authenticated_at, "an authentication time")?,
            ),
        ),
        identity: read_identity(identity)?,
        change_kind: read_uint(&change_kind, "a receipt's change kind")?,
        payload_commitment: read_fixed::<32>(commitment, "a payload commitment")?,
        coordinate: read_coordinate(at)?,
    })
}

/// Check `receipt` against the signed `message`, the service's public key, a
/// checkpoint of the log (`tree_size` and `root`) and an inclusion proof of the
/// receipt's leaf in that checkpoint.
pub fn verify_receipt(
    receipt: &Receipt,
    message: &[u8],
    service_key: &[u8; 32],
    checkpoint: (u64, [u8; 32]),
    proof: &InclusionProof,
) -> Result<(), IdentityError> {
    let signed = verify_event(message, service_key)?;
    if receipt.version != EVENT_VERSION {
        return Err(IdentityError::ReceiptInvalid {
            reason: "the receipt names another envelope version",
        });
    }
    if !receipt.matches(signed.event()) {
        return Err(IdentityError::ReceiptInvalid {
            reason: "the receipt's operation, actor, identity or change kind is not the event's",
        });
    }
    if receipt.payload_commitment != signed.payload_commitment() {
        return Err(IdentityError::ReceiptInvalid {
            reason: "the payload commitment is not the event body's SHA-256",
        });
    }
    if receipt.coordinate.leaf_hash != raw_leaf_hash(message) {
        return Err(IdentityError::ReceiptInvalid {
            reason: "the leaf hash is not the message's",
        });
    }
    let (tree_size, root) = checkpoint;
    if receipt.coordinate.index >= tree_size {
        return Err(IdentityError::ReceiptInvalid {
            reason: "the leaf index is outside the checkpoint",
        });
    }
    if verify_inclusion_raw(
        &RootHash::from_parts(root, tree_size),
        message,
        receipt.coordinate.index,
        proof,
    )
    .is_err()
    {
        return Err(IdentityError::ReceiptInvalid {
            reason: "the inclusion proof does not place the leaf in the checkpoint",
        });
    }
    Ok(())
}
