//! The Codex profile: a home session translated into a rollout in the shape
//! Codex 0.156.0 writes and reads for its own threads, with a loss account
//! beside it (HOME-009). The translation is a fork of the session, never the
//! same session: the rollout opens with an in-band marker saying so, and the
//! durable link back is the account and a `lys.translation` side leaf.

pub mod account;
#[cfg(test)]
mod account_tests;
pub mod parts;
#[cfg(test)]
mod parts_tests;
#[cfg(test)]
pub(crate) mod rollout_tests;
pub mod zone;
#[cfg(test)]
mod zone_tests;
