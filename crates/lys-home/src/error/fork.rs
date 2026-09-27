//! What a fork refuses: a lantern that cannot be found or is not lit
//! here, nothing to fork, and a fork left half written.

/// A refusal of a fork.
#[derive(Debug, thiserror::Error)]
pub enum ForkError {
    /// A lantern id was named that no session of the home holds as a
    /// `lys.lantern` entry, an id of another kind of entry included.
    #[error(
        "no session of the home holds a lantern `{lantern}`; name the entry id a light report printed"
    )]
    NoSuchLantern {
        /// The lantern id named.
        lantern: String,
    },

    /// A lantern whose data carries no lit-in session is held by more than
    /// one session, and none was named to cut from.
    #[error(
        "lantern_ambiguous: lantern `{lantern}` carries no lit-in session and is held by {}; name one of them with --session", sessions.join(", ")
    )]
    LanternAmbiguous {
        /// The lantern id.
        lantern: String,
        /// Every session holding it, in ascending byte order.
        sessions: Vec<String>,
    },

    /// A session was named to cut from that is not the one the lantern
    /// records it was lit in.
    #[error(
        "lantern_not_lit_here: lantern `{lantern}` was lit in session {lit_in}, not in {session}; fork from {lit_in}, or leave --session out"
    )]
    LanternNotLitHere {
        /// The lantern id.
        lantern: String,
        /// The session named.
        session: String,
        /// The session the lantern's data records it was lit in.
        lit_in: String,
    },

    /// A lantern's point has no assistant message at or before it on its
    /// chain, so there is nothing said yet to fork.
    #[error(
        "nothing_to_fork: lantern `{lantern}` sits before any assistant message; a fork carries what was said up to its point, and a point before the first reply would carry only a seed, which is a new session and not a fork"
    )]
    NothingToFork {
        /// The lantern id.
        lantern: String,
    },

    /// A lantern's data carries a `lit_in` that is not a session id: null,
    /// not a string, not a safe session name, or no session of the home.
    #[error(
        "lit_in_not_a_session: lantern `{lantern}` records a lit-in session that {what}; a fork cuts from the session a lantern was lit in and no other"
    )]
    LitInNotASession {
        /// The lantern id.
        lantern: String,
        /// What is wrong with the recorded value, naming no content.
        what: &'static str,
    },

    /// A fork failed after its child session was created, and removing the
    /// child failed too, so the child's files stand half-written.
    #[error(
        "fork of child {child} failed ({reason}), and removing the child failed too ({cleanup}); the child's files under sessions/ are half-written and the parent holds no lys.fork line for it"
    )]
    ForkHalfWritten {
        /// The child's session id.
        child: String,
        /// The refusal that stopped the fork, as displayed.
        reason: String,
        /// The refusal the cleanup met, as displayed.
        cleanup: String,
    },
}
