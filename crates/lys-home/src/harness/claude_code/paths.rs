//! Claude Code's own paths under its config directory (HOME-003 R1): the
//! project slug that names the directory a working directory's sessions and
//! memory live under, `<config>/projects/<slug>/`.
//!
//! Measured on Claude Code 2.1.283 (PROOF-GIVEN.md): the slug is the working
//! directory with every character that is not an ASCII letter or digit
//! replaced by `-`. `/`, `.` and `_` each become `-`, `-` stays, and
//! consecutive `-` are not collapsed. Under the earlier rule, which replaced
//! `/` alone, a dotted working directory named a directory the harness never
//! wrote, so a memory index placed there was never read. Nothing here reads
//! or writes a file.

/// The directory Claude Code keeps a working directory's sessions and memory
/// in: every character that is not an ASCII letter or digit becomes `-`.
#[must_use]
pub fn projects_slug(cwd: &str) -> String {
    cwd.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}
