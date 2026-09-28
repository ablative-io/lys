//! Check an agent before giving its start command, keep a launch record for
//! every command given, and show it running only on its signed report
//! (ADR-007: check, give, record, never run).
//!
//! A start request names the agent, the profile version and the machine,
//! and nothing else ([`request`]). The agent resolves to its enduring record
//! and the caller must be its responsible person or a directory
//! administrator ([`authority`]). Five named checks each read their owner's
//! record through their own seam and keep no copy ([`checks`]). When every
//! check passes, one signed launch record is kept ([`launch_record`]) and
//! the command given is rendered from it ([`command`], [`give`](mod@give)). Its state
//! is derived from the records it reads: running only on a verified signed
//! report naming it, unconfirmed without one, withdrawn by a signed
//! withdrawal that never says the agent did not run ([`state`],
//! [`withdrawal`]).
//!
//! # Invariants
//!
//! - Nothing in this module runs a process, opens a terminal or asks any
//!   runner to run anything. The command is text a person copies into
//!   the shell of the machine the request names.
//! - No credential value is read, held, logged, returned or put in a
//!   command, a record or an error: the handle record is read for ids and
//!   validity only.
//! - No record a check reads is built, copied, defaulted or faked here. A
//!   missing owner's record is refused by name, naming the card that makes it.
//! - A start never creates or changes an agent record and never writes a
//!   session record.

pub mod active;
pub mod authority;
pub mod checks;
pub mod command;
pub mod credentials;
pub mod egress;
pub mod error;
pub mod give;
pub mod launch_record;
pub mod machine_role;
pub mod profile_command;
pub mod profile_review;
pub mod request;
pub mod state;
pub mod withdrawal;

pub use checks::{Check, CheckReport};
pub use command::Grammars;
pub use error::{Refusal, Refused, StartError};
pub use give::{Given, Owners, give, give_again};
pub use launch_record::LaunchRecord;
pub use state::{LaunchRecords, LaunchState, Reading, state_of};
pub use withdrawal::{Withdrawal, withdraw};
