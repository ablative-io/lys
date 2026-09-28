//! What a command runs: the executable, its recorded arguments and the
//! working directory, read from the reviewed profile version (ADR-087).
//!
//! The three are fields of the profile version under the roles card
//! `Ink1H1Os`'s record, added there by this card's named amendment and never
//! kept in a copy here. [`ProfileVersionRecords`] is the one seam this card
//! reads that record through; this card defines no profile version record
//! of its own and takes no dependency on the roles card's types. A field
//! the record does not hold is refused as `profile_version_field_missing`,
//! naming the field and `Ink1H1Os`, and is never taken from the request, the
//! machine, a default or the mock-up.

use crate::start::error::Refusal;

/// The profile version record `Ink1H1Os` keeps, read for the command.
pub trait ProfileVersionRecords {
    /// The executable `profile_version` records, or `None` when it holds none.
    fn executable(&self, profile_version: &str) -> Option<String>;

    /// The arguments `profile_version` records, in order, or `None` when it
    /// holds none.
    fn arguments(&self, profile_version: &str) -> Option<Vec<String>>;

    /// The working directory `profile_version` records, or `None` when it
    /// holds none.
    fn working_directory(&self, profile_version: &str) -> Option<String>;
}

/// The executable, arguments and working directory a profile version records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileCommand {
    /// The executable.
    pub executable: String,
    /// The arguments, exactly as recorded and in their order.
    pub arguments: Vec<String>,
    /// The working directory.
    pub working_directory: String,
}

/// Read the command fields of `profile_version`, refusing by name the first
/// one its record does not hold.
pub fn read(
    records: &dyn ProfileVersionRecords,
    profile_version: &str,
) -> Result<ProfileCommand, Refusal> {
    let missing = |field: &'static str| Refusal::ProfileVersionFieldMissing {
        profile_version: profile_version.to_owned(),
        field,
    };
    let executable = records
        .executable(profile_version)
        .ok_or_else(|| missing("executable"))?;
    let arguments = records
        .arguments(profile_version)
        .ok_or_else(|| missing("arguments"))?;
    let working_directory = records
        .working_directory(profile_version)
        .ok_or_else(|| missing("working directory"))?;
    Ok(ProfileCommand {
        executable,
        arguments,
        working_directory,
    })
}
