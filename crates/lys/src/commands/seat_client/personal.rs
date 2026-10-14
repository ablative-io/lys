//! Personal launches keep their authority even on an identity server's own host.

use super::{Connection, Server};
use crate::commands::error::CliResult;
use crate::commands::sign_in::{Pass, base};
use lys_runner::dial::HttpClient;

impl Server {
    /// Reach only the signed-in person's recorded server, without operator authority.
    ///
    /// # Errors
    /// Refuses an unsafe or malformed recorded address.
    pub fn personal(pass: Pass) -> CliResult<Self> {
        base(&pass.server)?;
        Ok(Self {
            connection: Connection::Person {
                client: Box::new(HttpClient::new(&pass.server)?),
                pass,
            },
        })
    }

    /// Give a following read its own connection so it cannot hold up terminal input.
    ///
    /// # Errors
    /// Refuses an address that cannot be reached by the personal HTTP client.
    pub fn separate(&self) -> CliResult<Self> {
        match &self.connection {
            Connection::Person { pass, .. } => Self::personal(pass.clone()),
            Connection::Local {
                authority,
                prefix,
                token,
            } => Ok(Self {
                connection: Connection::Local {
                    authority: authority.clone(),
                    prefix: prefix.clone(),
                    token: token.clone(),
                },
            }),
        }
    }
}

#[cfg(test)]
#[path = "personal_tests.rs"]
mod tests;
