#![cfg(test)]
//! A disposable `SpiceDB` for one test: the contract harness's launcher,
//! which starts the release deploy/identity/versions.json pins in its
//! in-memory testing mode and kills it when the test drops it, with a client
//! of its own store wrapped to count every call.

use std::error::Error;
use std::ops::Deref;
use std::sync::Arc;

use lys_identity_server::spicedb::SpiceDbClient;

use super::Counting;

pub use identity_contract::spicedb::unique;

/// One running disposable `SpiceDB`; its gRPC address, `http://127.0.0.1:port`,
/// is the launcher's `address`.
pub struct SpiceDb {
    server: identity_contract::spicedb::SpiceDb,
}

impl Deref for SpiceDb {
    type Target = identity_contract::spicedb::SpiceDb;

    fn deref(&self) -> &Self::Target {
        &self.server
    }
}

impl SpiceDb {
    /// Start the pinned release in its testing mode, taking at most
    /// `max_updates_per_write` updates in one write when a cap is named, and
    /// return once it serves.
    pub fn start(max_updates_per_write: Option<u16>) -> Result<Self, Box<dyn Error>> {
        let server = identity_contract::spicedb::SpiceDb::start(max_updates_per_write)?;
        Ok(Self { server })
    }

    /// A client of this `SpiceDB` presenting a preshared key of its own, so
    /// it reaches a store of its own, wrapped to count every call.
    pub fn client(&self) -> Result<Arc<Counting>, Box<dyn Error>> {
        let config = self.server.client_config(&unique("spicedb-test-key"));
        Ok(Counting::around(Arc::new(SpiceDbClient::connect(&config)?)))
    }
}
