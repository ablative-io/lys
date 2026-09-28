//! The authzed v1 API as the build compiles it from the vendored protocol
//! files under crates/lys-identity-server/proto/authzed/, with the status
//! type its bulk answers carry. Nothing here is written by hand: the build
//! script generates the messages, and the client calls the services.

/// The authzed API.
pub mod authzed {
    /// The authzed API's versions.
    pub mod api {
        /// Version 1 of the authzed API, the one `SpiceDB` serves.
        pub mod v1 {
            include!(concat!(env!("OUT_DIR"), "/authzed.api.v1.rs"));
        }
    }
}

/// The Google API types the authzed API names.
pub mod google {
    /// The RPC status type.
    pub mod rpc {
        include!(concat!(env!("OUT_DIR"), "/google.rpc.rs"));
    }
}
