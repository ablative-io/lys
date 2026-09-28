//! What a translation to Codex refuses: a target that exists, a
//! Codex version not measured, and a time zone unnamed or unknown.

use std::path::PathBuf;

/// A refusal of a translation.
#[derive(Debug, thiserror::Error)]
pub enum TranslateError {
    /// A file translate-codex would write already exists.
    #[error("translate-codex target already exists: {}; choose another --out", path.display())]
    TranslationTargetExists {
        /// The file.
        path: PathBuf,
    },

    /// A Codex version was named whose rollout shape was never measured.
    #[error(
        "Codex {version} has no measured rollout shape: render for 0.156.0 or card a measurement of the new version"
    )]
    UnmeasuredCodexVersion {
        /// The version given.
        version: String,
    },

    /// The time zone given is absent or not shaped as an IANA name.
    #[error("TZ {value} is not an IANA time zone name: set TZ to an IANA name")]
    UnnamedTimeZone {
        /// The value given, or `unset` when none was.
        value: String,
    },

    /// The time zone given is shaped as an IANA name and the bundled time
    /// zone database does not hold it.
    #[error("time zone {zone} is not in the time zone database: set TZ to an IANA name")]
    UnknownTimeZone {
        /// The zone given.
        zone: String,
    },
}
