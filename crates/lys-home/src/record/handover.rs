//! The handover (HOME-015, building HOME-001 R12): an outgoing session's
//! letter carried into a new successor home as inherited memory (ADR-058).
//!
//! The letter is one or more entry ids of the outgoing session: a run of
//! assistant message entries standing next to each other on its
//! root-to-head path, in path order. It is copied whole: each entry keeps
//! its id, its timestamp and its message, every thinking block and
//! signature byte for byte, and only its parent link is rewritten to chain
//! onto the successor. Nothing is composed, merged or re-ordered.
//!
//! The outgoing session is read without owning it: no lock is taken and
//! nothing is written, created or renamed under the outgoing home. Every
//! refusal is checked before any file or directory is created, and names
//! ids and paths only, never the letter's text, thinking or signature.
//!
//! The successor is a new home at a path that is absent or an empty
//! directory, holding one session with a fresh id, the outgoing header's
//! cwd and no parentSession. Its first entry is a `lys.inherited` entry
//! with no rule, which is what says the successor's first memory is
//! inherited; then the letter's entries; then a `session_info` named
//! `inherited from <outgoing session id>`. No field is added to Pi's
//! grammar and no block is written to either home.
