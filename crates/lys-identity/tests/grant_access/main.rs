//! The access tests that share the grant support, as one binary: together
//! they use every part of it (ACCESS-006 R2, R5, R6; DIRECTORY-089 R1;
//! DIRECTORY-090 R6).

#[path = "../support/mod.rs"]
mod support;

mod grant_change_range;
mod grant_live_decision;
mod grant_tail_authority;
mod membership_index_recovery;
