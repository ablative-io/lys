//! The proxy's errors, named.
//!
//! Invariant: no variant carries a header value or a body byte. An error
//! names a path, a call id, an upstream base the operator gave, or the
//! underlying library's own words, which describe a connection and never
//! the bytes on it.

use std::path::PathBuf;

use crate::error::HomeError;

/// What went wrong in the proxy.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ProxyError {
    /// An I/O operation failed at a named path.
    #[error("{context} at {}: {source}", path.display())]
    Io {
        /// What the operation was doing.
        context: &'static str,
        /// The path involved.
        path: PathBuf,
        /// The underlying error.
        #[source]
        source: std::io::Error,
    },

    /// The open-call journal could not be written before the call was
    /// admitted, so the call was refused and never sent upstream.
    #[error(
        "the open-call journal could not be written at {}: {source}; the call was refused and not forwarded",
        path.display()
    )]
    JournalUnwritable {
        /// The journal record that could not be written.
        path: PathBuf,
        /// The underlying error.
        #[source]
        source: std::io::Error,
    },

    /// A journal record did not parse.
    #[error("journal record {} is not a valid open call: {source}", path.display())]
    JournalRecord {
        /// The record.
        path: PathBuf,
        /// The parser's own error.
        #[source]
        source: serde_json::Error,
    },

    /// A journal record could not be serialised.
    #[error("the journal record for call {call_id} could not be serialised: {source}")]
    JournalEncode {
        /// The call.
        call_id: String,
        /// The serialiser's own error.
        #[source]
        source: serde_json::Error,
    },

    /// An upstream base is not an absolute `http` or `https` URL.
    #[error("upstream base `{base}` is not an absolute http or https URL: {reason}")]
    BadUpstream {
        /// The base as given.
        base: String,
        /// What was wrong with it.
        reason: String,
    },

    /// A request's path could not be joined onto its upstream base.
    #[error("the request path could not be joined onto upstream `{base}`: {reason}")]
    BadTarget {
        /// The upstream base.
        base: String,
        /// The URI parser's own words, which name no part of the path.
        reason: String,
    },

    /// The TLS configuration could not be built.
    #[error("the TLS configuration could not be built: {source}")]
    Tls {
        /// The TLS library's own error.
        #[source]
        source: rustls::Error,
    },

    /// The upstream could not be reached, or its response broke off before
    /// its head arrived.
    #[error("the upstream call failed: {source}")]
    Upstream {
        /// The transport's own error.
        #[source]
        source: Box<hyper_util::client::legacy::Error>,
    },

    /// A connection could not be accepted.
    #[error("a connection could not be accepted: {source}")]
    Accept {
        /// The underlying error.
        #[source]
        source: std::io::Error,
    },

    /// The sink that records calls has stopped.
    #[error("the call sink has stopped; {what} was not handed to it")]
    SinkStopped {
        /// What could not be handed over: a call by id, or a settle request.
        what: String,
    },

    /// The home refused.
    #[error(transparent)]
    Home(Box<HomeError>),
}

impl From<HomeError> for ProxyError {
    fn from(error: HomeError) -> Self {
        Self::Home(Box::new(error))
    }
}

impl ProxyError {
    /// An I/O failure at a named path, with what the operation was doing.
    pub fn io(context: &'static str, path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            context,
            path: path.into(),
            source,
        }
    }
}
