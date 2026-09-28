//! [`SpiceDbError`], what the identity server's `SpiceDB` client reports.
//!
//! Every variant names the operation and the configured address, so an
//! operator can tell which call to which engine failed. None carries the
//! preshared key, and none stands in for an answer: a call `SpiceDB` did not
//! answer is reported, never substituted.

use lys_identity::grants::GrantError;

/// Errors from the identity server's `SpiceDB` client.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SpiceDbError {
    /// `SpiceDB` could not be reached at the configured address.
    #[error("spicedb_unreachable: {operation} could not reach SpiceDB at {address}: {reason}")]
    Unreachable {
        /// The call.
        operation: &'static str,
        /// The configured address.
        address: String,
        /// What the transport reported.
        reason: String,
    },
    /// `SpiceDB` answered the call with an error status.
    #[error("spicedb_refused: SpiceDB at {address} answered {operation} with {code}: {message}")]
    Refused {
        /// The call.
        operation: &'static str,
        /// The configured address.
        address: String,
        /// The gRPC status code.
        code: String,
        /// The status message.
        message: String,
    },
    /// An answer lacks what the call needs from it.
    #[error(
        "spicedb_answer_malformed: SpiceDB at {address} answered {operation} without {missing}"
    )]
    Malformed {
        /// The call.
        operation: &'static str,
        /// The configured address.
        address: String,
        /// What the answer lacks.
        missing: &'static str,
    },
    /// The client cannot be set up from the configuration it was given.
    #[error("spicedb_config_invalid: {reason}")]
    ConfigInvalid {
        /// What is wrong with the configuration.
        reason: String,
    },
    /// The configuration names no `SpiceDB` gRPC address, so there is no
    /// engine to answer a grant question and none is answered.
    #[error(
        "spicedb_grpc_absent: every grant and permission check is answered by SpiceDB over gRPC, and {reason}; no grant is decided without it"
    )]
    GrpcAbsent {
        /// What the configuration names in place of the address.
        reason: String,
    },
    /// The store holds a schema other than the one the server is built with.
    #[error(
        "spicedb_schema_mismatch: SpiceDB at {address} holds a schema that differs from crates/lys-identity-server/src/spicedb/schema.zed, and it is not overwritten"
    )]
    SchemaMismatch {
        /// The configured address.
        address: String,
    },
}

impl From<SpiceDbError> for GrantError {
    fn from(error: SpiceDbError) -> Self {
        GrantError::EngineUnanswered {
            reason: error.to_string(),
        }
    }
}
