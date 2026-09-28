//! The receipt of a grant change, and its read-only verification against the log.
//!
//! A receipt carries the event version, the operation id, the caller, the
//! grant, the change kind, the SHA-256 payload commitment and the log
//! coordinate the event's leaf completed. Anyone holding the signed message,
//! the service's public key, a checkpoint of the grant log and an inclusion
//! proof can check it: a changed caller, source, recipient, resource, action,
//! position or signature fails.

use lys_core::merkle::{InclusionProof, RootHash, raw_leaf_hash, verify_inclusion_raw};

use super::error::GrantError;
use super::events::{GRANT_EVENT_VERSION, SignedGrantEvent, change_kind, verify_grant_event};
use super::types::GrantId;
use crate::id::IdentityId;
use crate::log::Coordinate;
use crate::operation::OperationId;

/// What the grants answer for a recorded change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantReceipt {
    /// The event version the change was signed under.
    pub version: u64,
    /// The caller's operation id.
    pub operation: OperationId,
    /// The identity that made the request.
    pub caller: IdentityId,
    /// The grant changed.
    pub grant: GrantId,
    /// The change kind's wire code.
    pub change_kind: u64,
    /// SHA-256 over the event body.
    pub payload_commitment: [u8; 32],
    /// Where the event's leaf stands in the grant log.
    pub coordinate: Coordinate,
}

impl GrantReceipt {
    /// The receipt of `signed`, committed at `coordinate`.
    pub fn of(signed: &SignedGrantEvent, coordinate: Coordinate) -> Self {
        let event = signed.event();
        Self {
            version: GRANT_EVENT_VERSION,
            operation: event.operation(),
            caller: event.caller(),
            grant: event.grant(),
            change_kind: change_kind(event.change()),
            payload_commitment: signed.payload_commitment(),
            coordinate,
        }
    }

    /// The revision a decision must stand at to reflect this change.
    pub fn revision(&self) -> u64 {
        self.coordinate.index + 1
    }
}

/// Check `receipt` against the signed `message`, the service key, a
/// checkpoint of the grant log (`tree_size` and `root`) and an inclusion proof.
pub fn verify_grant_receipt(
    receipt: &GrantReceipt,
    message: &[u8],
    service_key: &[u8; 32],
    checkpoint: (u64, [u8; 32]),
    proof: &InclusionProof,
) -> Result<(), GrantError> {
    let signed = verify_grant_event(message, service_key)?;
    let expected = GrantReceipt::of(&signed, receipt.coordinate);
    if receipt.version != expected.version {
        return Err(GrantError::ReceiptInvalid {
            reason: "the receipt names another event version",
        });
    }
    if (
        receipt.operation,
        receipt.caller,
        receipt.grant,
        receipt.change_kind,
    ) != (
        expected.operation,
        expected.caller,
        expected.grant,
        expected.change_kind,
    ) {
        return Err(GrantError::ReceiptInvalid {
            reason: "the receipt's operation, caller, grant or change kind is not the event's",
        });
    }
    if receipt.payload_commitment != expected.payload_commitment {
        return Err(GrantError::ReceiptInvalid {
            reason: "the payload commitment is not the event body's SHA-256",
        });
    }
    if receipt.coordinate.leaf_hash != raw_leaf_hash(message) {
        return Err(GrantError::ReceiptInvalid {
            reason: "the leaf hash is not the message's",
        });
    }
    let (tree_size, root) = checkpoint;
    if receipt.coordinate.index >= tree_size
        || verify_inclusion_raw(
            &RootHash::from_parts(root, tree_size),
            message,
            receipt.coordinate.index,
            proof,
        )
        .is_err()
    {
        return Err(GrantError::ReceiptInvalid {
            reason: "the inclusion proof does not place the leaf at the receipt's index in the checkpoint",
        });
    }
    Ok(())
}
